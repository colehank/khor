"""Submit tracked source snapshots to the private development worker; no Node/Rust/SSH needed."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
import uuid


def command(*args, cwd=None, env=None):
    return subprocess.check_output(args, cwd=cwd, env=env, text=True).strip()


def git(*args, **kwargs):
    return command("git", *args, **kwargs)


def root():
    return Path(git("rev-parse", "--show-toplevel"))


def state_dir():
    return Path(git("rev-parse", "--path-format=absolute", "--git-common-dir")) / "khor-development"


def configure(restore=False, install_command=False):
    destination = state_dir() / "hooks"
    receipt = destination / "configuration.json"
    current = subprocess.run(["git", "config", "--local", "--get", "core.hooksPath"], text=True, capture_output=True).stdout.strip()
    if restore:
        saved = json.loads(receipt.read_text())
        if current != str(destination):
            raise ValueError("core.hooksPath changed since installation; preserve that configuration instead of overwriting it")
        if saved["previous_hooks_path"]:
            git("config", "--local", "core.hooksPath", saved["previous_hooks_path"])
        else:
            git("config", "--local", "--unset", "core.hooksPath")
        print("Restored the previous hooks. Local full checks require their documented toolchains.")
        return
    destination.mkdir(parents=True, exist_ok=True)
    if current != str(destination):
        receipt.write_text(json.dumps({"previous_hooks_path": current}, indent=2) + "\n")
    for name in ["dev.py", "source_hook.py"]:
        source = Path(__file__).with_name(name)
        if source.resolve() != (destination / name).resolve():
            shutil.copyfile(source, destination / name)
    for stage in ["pre-commit", "pre-push"]:
        hook = destination / stage
        graph = destination.parent.parent / "hooks/pre-commit.legacy"
        legacy = ""
        if stage == "pre-commit" and graph.is_file() and "Installed by code-review-graph" in graph.read_text():
            legacy = shlex.quote(str(graph)) + ' "$@"\n'
        hook.write_text("#!/bin/sh\n" + legacy + "exec " + shlex.join([sys.executable, str(destination / "source_hook.py"), stage]) + ' "$@"\n')
        hook.chmod(0o755)
    # Existing pre-commit and graph hooks remain on disk and are restored with --restore.
    git("config", "--local", "core.hooksPath", str(destination))
    if install_command:
        entry = Path.home() / ".local/bin/khor-dev"
        marker = "# Managed Khor development command"
        if entry.exists() and marker not in entry.read_text():
            raise ValueError(f"Preserving existing command at {entry}; install without --install-command")
        entry.parent.mkdir(parents=True, exist_ok=True)
        entry.write_text("#!/bin/sh\n" + marker + "\nexec " + shlex.join([sys.executable, str(destination / "dev.py")]) + ' "$@"\n')
        entry.chmod(0o755)
        print(f"Installed source-only command: {entry}")
    print(f"Installed offline hooks for all worktrees of this clone: {destination}")
    print("GitHub Quality and release checks are unchanged. No build dependencies are installed by these hooks.")


def snapshot():
    base = git("rev-parse", "HEAD")
    # stash create writes a temporary commit object without stashing, resetting or adding untracked files.
    working = git("stash", "create", "Khor development snapshot") or base
    tree = git("rev-parse", working + "^{tree}")
    info = {"schema": 1, "base_commit": base, "tree": tree, "dirty": working != base,
            "created_at": datetime.now(timezone.utc).isoformat()}
    env = dict(os.environ, GIT_AUTHOR_NAME="Khor developer snapshot", GIT_AUTHOR_EMAIL="snapshot@localhost",
               GIT_COMMITTER_NAME="Khor developer snapshot", GIT_COMMITTER_EMAIL="snapshot@localhost")
    # Keep source workflows below source/, so a snapshot push cannot activate them
    # as workflows of the private control repository.
    container = subprocess.check_output(["git", "mktree"], input=f"040000 tree {tree}\tsource\n", text=True).strip()
    commit = command("git", "commit-tree", container, "-m", "Khor developer snapshot\n\n" + json.dumps(info, sort_keys=True), env=env)
    untracked = subprocess.check_output(["git", "ls-files", "--others", "--exclude-standard", "-z"]).split(b"\0")
    info["excluded_untracked_count"] = len(list(filter(None, untracked)))
    info["snapshot_commit"] = commit
    return info


def upload_snapshot(info):
    # The transport repository carries an artifact, not a branch being merged.
    # Its own hooks do not invoke the developer clone's full pre-push toolchain.
    with tempfile.TemporaryDirectory(prefix="khor-source-transport-") as directory:
        git("init", "--bare", "--quiet", directory)
        git("-C", directory, "-c", "protocol.file.allow=always", "fetch", "--quiet", str(root()), info["snapshot_commit"])
        git("-C", directory, "-c", "credential.helper=", "-c", "credential.helper=!gh auth git-credential", "push",
            "https://github.com/" + info["repository"] + ".git", info["snapshot_commit"] + ":refs/heads/" + info["snapshot_ref"])


def submit(args):
    repository = args.builder_repo
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("Expected owner/repository")
    details = json.loads(command("gh", "api", f"repos/{repository}"))
    if not details["private"]:
        raise ValueError("Source snapshots must go to a PRIVATE build-control repository")
    info = snapshot()
    identifier = uuid.uuid4().hex
    info.update({"repository": repository, "task": args.task, "request_id": identifier,
                 "snapshot_ref": "dev-snapshots/" + identifier})
    records = state_dir() / "runs"
    records.mkdir(parents=True, exist_ok=True)
    record = records / (identifier + ".json")
    record.write_text(json.dumps(info, indent=2) + "\n")
    print(f"Source tree {info['tree']}; base {info['base_commit']}; untracked files excluded: {info['excluded_untracked_count']}", flush=True)
    upload_snapshot(info)
    command("gh", "workflow", "run", "development.yml", "--repo", repository, "--ref", "main",
            "-f", "snapshot_sha=" + info["snapshot_commit"], "-f", "task=" + args.task, "-f", "request_id=" + identifier)
    for _ in range(20):
        runs = json.loads(command("gh", "run", "list", "--repo", repository, "--workflow", "development.yml",
                                  "--limit", "30", "--json", "databaseId,displayTitle,url"))
        run = next((item for item in runs if identifier in item["displayTitle"]), None)
        if run:
            info.update(run)
            record.write_text(json.dumps(info, indent=2) + "\n")
            print(json.dumps(info, ensure_ascii=False, indent=2))
            print(f"Inspect: python3 scripts/dev.py status {run['databaseId']} --builder-repo {repository}")
            return
        time.sleep(2)
    raise ValueError(f"Dispatched, but run discovery timed out. Request {identifier}; receipt {record}. Inspect gh run list before resubmitting.")


def status(args):
    print(command("gh", "run", "view", args.run_id, "--repo", args.builder_repo,
                  "--json", "databaseId,status,conclusion,url,displayTitle,jobs"))


def submitted_source(run_id, repository):
    for path in (state_dir() / "runs").glob("*.json"):
        info = json.loads(path.read_text())
        if str(info.get("databaseId")) == run_id and info.get("repository") == repository:
            return info
    return None


def result(args):
    run = json.loads(command("gh", "run", "view", args.run_id, "--repo", args.builder_repo,
                             "--json", "status,conclusion,url"))
    if run["status"] != "completed":
        raise ValueError("Run is still active; inspect status before downloading its results")
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    try:
        command("gh", "run", "download", args.run_id, "--repo", args.builder_repo, "--name", "dev-evidence", "--dir", str(output))
    except subprocess.CalledProcessError:
        # Portable preparation can fail before a Mac job is allowed to start.
        command("gh", "run", "download", args.run_id, "--repo", args.builder_repo, "--name", "dev-portable-evidence", "--dir", str(output))
    expected = submitted_source(args.run_id, args.builder_repo)
    actual = json.loads((output / "source.json").read_text())
    if expected and any(actual.get(key) != expected[key] for key in ["snapshot_commit", "base_commit", "tree"]):
        raise ValueError("Downloaded evidence does not match this machine's submitted source receipt")
    # The worker may be compromised; this receipt labels provenance, it is not release attestation.
    print(f"{run['conclusion']}: {run['url']}\nEvidence: {output}")
    if args.lab:
        if run["conclusion"] != "success":
            raise ValueError("Refusing to download a failed Lab as ready to use")
        command("gh", "run", "download", args.run_id, "--repo", args.builder_repo, "--name", "dev-lab", "--dir", str(output / "lab"))
        lab = json.loads((output / "lab/source.json").read_text())
        if any(lab.get(key) != actual[key] for key in ["snapshot_commit", "base_commit", "tree"]):
            raise ValueError("Lab does not match the checked source")
        with (output / "lab/KhorLab.zip").open("rb") as archive:
            if hashlib.file_digest(archive, "sha256").hexdigest() != lab["archive_sha256"]:
                raise ValueError("Lab archive digest mismatch")
        print("Downloaded an unnotarized DEVELOPMENT build from the rented worker. Do not install it over /Applications/Khor.app. Follow docs/REMOTE-DEVELOPMENT.md for isolated testing.")
    if run["conclusion"] != "success":
        raise ValueError(f"Development run did not pass: {run['conclusion']}; evidence was retained")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    config = sub.add_parser("configure-local", help="Install offline hooks, preserving the previous configuration")
    config.add_argument("--restore", action="store_true")
    config.add_argument("--install-command", action="store_true", help="Also install ~/.local/bin/khor-dev")
    check = sub.add_parser("check", help="Submit a tracked WIP snapshot asynchronously")
    check.add_argument("--task", choices=["fast", "native", "lab", "resolve", "updates"], default="fast")
    for action in [check, sub.add_parser("status"), sub.add_parser("result")]:
        action.add_argument("--builder-repo", default="colehank/khor-build")
        if action != check:
            action.add_argument("run_id")
    download = sub.choices["result"]
    download.add_argument("--output", required=True)
    download.add_argument("--lab", action="store_true")
    args = parser.parse_args()
    try:
        os.chdir(root())
        if args.action == "configure-local":
            configure(args.restore, args.install_command)
        else:
            {"check": submit, "status": status, "result": result}[args.action](args)
        return 0
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        if isinstance(error, subprocess.CalledProcessError) and error.output:
            print(error.output, file=sys.stderr)
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
