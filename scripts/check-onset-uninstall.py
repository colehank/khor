#!/usr/bin/env python3
"""Run onset's real shell against a scratch home and a local SSH substitute.

No network, real processes, builds, or user stores are used. Every destructive
command in the remote shell is replaced or confined to this test's temp tree.
"""
import hashlib
import itertools
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile


def mock(tool, args):
    scope = Path(os.environ["ONSET_TEST_SCOPE"]).resolve()
    def log(event, **fields):
        with (scope / "events.jsonl").open("a") as stream:
            stream.write(json.dumps({"event": event, **fields}) + "\n")
    if tool == "ssh":
        log("ssh", args=args)
        if "-O" in args:
            return 0
        if os.environ.get("ONSET_TEST_SSH_FAIL"):
            return 255
        if args[-1] != "sh -s":
            raise RuntimeError(f"Unexpected remote command: {args}")
        script = sys.stdin.read()
        with (scope / "remote.sh").open("a") as stream:
            stream.write(script + "\n")
        prelude = '''kill() { onset-test-kill "$@"; }
sleep() { :; }
'''
        return subprocess.run(["/bin/sh", "-s"], input=prelude + script, text=True).returncode
    if tool == "hostname":
        print(os.environ.get("ONSET_TEST_HOSTNAME", "B"))
        return 0
    if tool == "khor":
        log("root", args=args, inherited_override=os.environ.get("KHOR_HOME"))
        assert args == ["root"]
        if os.environ.get("ONSET_TEST_ROOT_FAIL"):
            return 23
        print(os.environ.get("KHOR_HOME", os.environ["ONSET_TEST_ROOT"]))
        return 0
    if tool in ("ps", "kill"):
        log(tool, args=args)
        state = scope / "running"
        if tool == "ps":
            if not state.exists():
                return 1
            print(os.environ.get("ONSET_TEST_COMMAND", "/scratch/khor serve"))
            return 0
        if args[0] == "-0":
            return 0 if state.exists() else 1
        if os.environ.get("ONSET_TEST_KILL_FAIL"):
            return 1
        if not os.environ.get("ONSET_TEST_KILL_STUCK"):
            state.unlink(missing_ok=True)
        return 0
    if tool == "rm":
        paths = [Path(arg) for arg in args if not arg.startswith("-")]
        for path in paths:
            if not path.is_absolute() or not path.resolve().is_relative_to(scope):
                raise RuntimeError(f"Refusing removal outside scratch: {path}")
        log("rm", paths=[str(path) for path in paths])
        if any(str(path).endswith(os.environ.get("ONSET_TEST_RM_FAIL", "\0")) for path in paths):
            return 13
        return subprocess.run(["/bin/rm", *args]).returncode
    raise RuntimeError(f"Unexpected tool: {tool}")


def snapshot(home):
    result = {}
    for path in sorted(home.rglob("*")):
        relative = str(path.relative_to(home))
        if path.is_symlink():
            result[relative] = ["link", os.readlink(path)]
        elif path.is_file():
            try:
                result[relative] = hashlib.sha256(path.read_bytes()).hexdigest()
            except PermissionError:
                result[relative] = "unreadable"
    return result


def run_suite():
    repo = Path(__file__).resolve().parent.parent
    onset = repo / "scripts/onset.sh"
    report = {"files": {name: hashlib.sha256((repo / name).read_bytes()).hexdigest()
                         for name in ["scripts/onset.sh", "scripts/onset-lib.sh", "scripts/check-onset-uninstall.py"]},
              "boundary": "Scratch filesystem and mock SSH/processes only; no network or real service operations.",
              "results": []}
    output = Path(os.environ.get("ONSET_TEST_OUT") or tempfile.mkdtemp(prefix="khor-onset-evidence-"))
    output.mkdir(parents=True, exist_ok=True)

    def case(name, *, args=None, shared=False, host="B", root=None, owner=True,
             binary=True, purge=False, dry=False, error=False, no_remote=False, env=None, setup=None):
        with tempfile.TemporaryDirectory(prefix="khor-onset-check-") as directory:
            scope = Path(directory)
            home = scope / "home with space"
            tools = scope / "bin"
            tools.mkdir()
            (home / ".local/bin").mkdir(parents=True)
            (scope / "tmp").mkdir()
            remote_root = home / ".khor-hosts" / host if shared else home
            if root is not None:
                remote_root = home / root
            def store(path, name):
                data = path / ".khor"
                data.mkdir(parents=True, exist_ok=True)
                (data / "identity.key").write_text(f"identity-{name}")
                if owner:
                    (data / "owner").write_text(f"machine fingerprint-{name}\nname {name}\n")
            if shared:
                store(home, "A")
                (home / ".khor/serve.pid").write_text("737373\n")
            store(remote_root, host)
            store(home / ".khor-hosts" / ("B" if host == "A" else "C"), "unrelated-machine")
            (remote_root / "unrelated.txt").write_text("outside store")
            (remote_root / ".khor/serve.pid").write_text("424242\n")
            (scope / "running").touch()
            script = str(Path(__file__).resolve())
            def launcher(path, tool):
                path.write_text("#!/bin/sh\nexec " + shlex.join([sys.executable, script, "--mock", tool]) + ' "$@"\n')
                path.chmod(0o755)
            for tool in ["ssh", "hostname", "ps", "rm"]:
                launcher(tools / tool, tool)
            launcher(tools / "onset-test-kill", "kill")
            if binary:
                launcher(home / ".local/bin/khor", "khor")
            environment = {"PATH": f"{tools}:/usr/bin:/bin:/usr/sbin:/sbin", "HOME": str(home),
                           "TMPDIR": str(scope / "tmp"), "ONSET_TEST_SCOPE": str(scope),
                           "ONSET_TEST_ROOT": str(remote_root), "ONSET_TEST_HOSTNAME": host}
            if env:
                environment.update({key: value.replace("{home}", str(home)) for key, value in env.items()})
            if setup:
                setup(home, remote_root, scope)
            before = snapshot(home)
            argv = args if args is not None else [host, "--uninstall"] + (["--purge"] if purge else []) + (["--dry-run"] if dry else [])
            result = subprocess.run(["/bin/sh", str(onset), *argv], env=environment, text=True, capture_output=True, timeout=15)
            after = snapshot(home)
            events = [json.loads(line) for line in (scope / "events.jsonl").read_text().splitlines()] if (scope / "events.jsonl").exists() else []
            entry = {"name": name, "code": result.returncode, "stdout": result.stdout, "stderr": result.stderr,
                     "events": events, "changed": [key for key in sorted(before.keys() | after.keys()) if before.get(key) != after.get(key)]}
            try:
                if error:
                    assert result.returncode != 0, "failure must propagate"
                    if error == "after-stop":
                        assert (remote_root / ".khor/identity.key").is_file(), "failed removal must not continue to purge"
                        assert "store purged" not in result.stdout
                    else:
                        assert before == after, "invalid/failed preflight must not mutate home"
                    if no_remote:
                        assert not events, "invalid arguments must fail before SSH"
                elif dry:
                    assert result.returncode == 0, result.stderr
                    assert not events, "dry-run must not call SSH, root, or mutation tools"
                    assert before == after, "dry-run changed the home"
                    assert "dry run" in result.stdout
                    if purge:
                        assert "purge" in result.stdout
                else:
                    assert result.returncode == 0, result.stderr
                    assert not (home / ".local/bin/khor").exists(), "binary remains"
                    assert (remote_root / "unrelated.txt").is_file(), "purge exceeded store"
                    assert not (remote_root / ".khor").exists() if purge else (remote_root / ".khor/identity.key").exists()
                    if shared:
                        assert {key: value for key, value in before.items() if key.startswith(".khor/")} == {
                            key: value for key, value in after.items() if key.startswith(".khor/")}, "shared A store changed"
                    data_prefix = str((remote_root / ".khor").relative_to(home)) + "/"
                    allowed = {".local/bin/khor", ".local/bin/.khor-root", data_prefix + "serve.pid"}
                    outside = lambda values: {key: value for key, value in values.items()
                                              if key not in allowed and not (purge and key.startswith(data_prefix))}
                    assert outside(before) == outside(after), "uninstall changed an unrelated file or machine store"
                    if binary:
                        assert len([event for event in events if event["event"] == "root"]) == 1, "store must be selected once before deleting binary"
                entry["status"] = "passed"
            except AssertionError as failure:
                entry.update(status="failed", assertion=str(failure))
            report["results"].append(entry)
            (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")
            print(entry["status"].upper(), name, entry.get("assertion", ""))

    for tokens in itertools.permutations(["B", "--uninstall", "--purge", "--dry-run"]):
        case("dry-order-" + "_".join(tokens), args=list(tokens), shared=True, purge=True, dry=True)
    case("dry-uninstall", dry=True)
    case("owner-A-purge", host="A", purge=True)
    case("independent-B-purge", purge=True)
    case("shared-B-purge", shared=True, purge=True)
    case("shared-B-retain-data", shared=True)
    case("shared-B-inherited-override", shared=True, purge=True, env={"KHOR_HOME": "{home}"})
    case("renamed-B-room", shared=True, root=".khor-hosts/old-B", purge=True)
    case("missing-binary-modern-claim", shared=True, binary=False, purge=True, error=True)
    case("missing-binary-same-name-different-fingerprint", binary=False, purge=True, error=True,
         setup=lambda home, root, scope: (root / ".khor/owner").write_text("machine fingerprint-other-machine\nname B\n"))
    for fingerprint in ["0", "00", "0.0"]:
        case("missing-binary-zero-fingerprint-" + fingerprint, binary=False, purge=True, error=True,
             setup=lambda home, root, scope, fingerprint=fingerprint:
                 (root / ".khor/owner").write_text(f"machine {fingerprint}\nname B\n"))
    case("duplicate-zero-fingerprint", purge=True, error=True,
         setup=lambda home, root, scope: (root / ".khor/owner").write_text("machine 0\nmachine 0\nname B\n"))
    def legacy_claims(home, root, scope):
        (home / ".khor/owner").write_text("A\n")
        (root / ".khor/owner").write_text("B\n")
    case("missing-binary-legacy-fallback", shared=True, binary=False, purge=True, setup=legacy_claims)
    case("legacy-owner", purge=True, setup=lambda home, root, scope: (root / ".khor/owner").write_text("B\n"))
    case("unknown-owner", owner=False, error=True, purge=True)
    case("empty-owner", error=True, purge=True, setup=lambda home, root, scope: (root / ".khor/owner").write_text(""))
    case("unreadable-owner", error=True, purge=True, setup=lambda home, root, scope: (root / ".khor/owner").chmod(0))
    case("root-command-failure", env={"ONSET_TEST_ROOT_FAIL": "1"}, error=True, purge=True)
    case("root-outside-home", env={"ONSET_TEST_ROOT": "/"}, error=True, purge=True)
    case("root-multiline", env={"ONSET_TEST_ROOT": "{home}\n/"}, error=True, purge=True)
    case("root-traversal", shared=True, env={"ONSET_TEST_ROOT": "{home}/.khor-hosts/B/../.."}, error=True, purge=True)
    case("wrong-process", env={"ONSET_TEST_COMMAND": "/scratch/editor unrelated"}, error=True, purge=True)
    case("stop-permission-failure", env={"ONSET_TEST_KILL_FAIL": "1"}, error=True, purge=True)
    case("stop-timeout", env={"ONSET_TEST_KILL_STUCK": "1"}, error=True, purge=True)
    case("remove-permission-failure", env={"ONSET_TEST_RM_FAIL": "/.local/bin/khor"}, error="after-stop", purge=True)
    case("empty-root", env={"ONSET_TEST_ROOT": ""}, error=True, purge=True)
    case("pid-group-zero", error=True, purge=True, setup=lambda home, root, scope: (root / ".khor/serve.pid").write_text("0\n"))
    case("pid-group-zero-padded", error=True, purge=True, setup=lambda home, root, scope: (root / ".khor/serve.pid").write_text("00\n"))
    case("pid-option", error=True, purge=True, setup=lambda home, root, scope: (root / ".khor/serve.pid").write_text("-1\n"))
    case("malformed-owner", error=True, purge=True, setup=lambda home, root, scope: (root / ".khor/owner").write_text("name B\nname A\n"))
    case("fallback-wrong-owner", shared=True, binary=False, error=True, purge=True,
         setup=lambda home, root, scope: (root / ".khor/owner").write_text("C\n"))
    case("fingerprint-only", purge=True, setup=lambda home, root, scope: (root / ".khor/owner").write_text("machine fingerprint-B\n"))
    case("fallback-fingerprint-only", binary=False, error=True, purge=True,
         setup=lambda home, root, scope: (root / ".khor/owner").write_text("machine fingerprint-B\n"))
    def symlink_store(home, root, scope):
        store = root / ".khor"
        store.rename(root / "saved-store")
        store.symlink_to(root / "saved-store", target_is_directory=True)
    case("symlink-store", setup=symlink_store, error=True, purge=True)
    def symlink_room(home, root, scope):
        root.rename(root.with_name("saved-room"))
        root.symlink_to(root.with_name("saved-room"), target_is_directory=True)
    case("symlink-room", shared=True, setup=symlink_room, error=True, purge=True)
    def symlink_owner(home, root, scope):
        claim = root / ".khor/owner"
        claim.rename(root / ".khor/saved-owner")
        claim.symlink_to(root / ".khor/saved-owner")
    case("symlink-owner", setup=symlink_owner, error=True, purge=True)
    def symlink_pid(home, root, scope):
        pid = root / ".khor/serve.pid"
        pid.unlink()
        pid.symlink_to(home / ".khor/serve.pid")
    case("symlink-pid-other-machine", shared=True, setup=symlink_pid, error=True, purge=True)
    def symlink_hosts(home, root, scope):
        rooms = home / ".khor-hosts"
        rooms.rename(home / "saved-hosts")
        rooms.symlink_to(home / "saved-hosts", target_is_directory=True)
    case("symlink-hosts-parent", shared=True, setup=symlink_hosts, error=True, purge=True)
    case("binary-not-executable", error=True, purge=True, setup=lambda home, root, scope: (home / ".local/bin/khor").chmod(0o600))
    case("ssh-failure", env={"ONSET_TEST_SSH_FAIL": "1"}, error=True, purge=True)
    case("purge-without-uninstall", args=["B", "--purge"], error=True, no_remote=True)
    case("two-hosts", args=["A", "B", "--uninstall"], error=True, no_remote=True)
    case("empty-host", args=["", "--uninstall"], error=True, no_remote=True)
    case("uninstall-release", args=["B", "--uninstall", "--release"], error=True, no_remote=True)
    case("conflicting-mode", args=["--print-serve-up", "B", "--uninstall"], error=True, no_remote=True)
    case("conflicting-mode-reversed", args=["B", "--uninstall", "--print-serve-up"], error=True, no_remote=True)
    case("unsafe-host", args=["B'", "--uninstall"], error=True, no_remote=True)
    (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    print("Evidence:", output)
    return 1 if any(result["status"] == "failed" for result in report["results"]) else 0


if __name__ == "__main__":
    sys.exit(mock(sys.argv[2], sys.argv[3:]) if len(sys.argv) > 1 and sys.argv[1] == "--mock" else run_suite())
