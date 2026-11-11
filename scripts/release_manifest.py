"""Receipts for immutable CI artifacts; validation never installs or starts Khor."""
import argparse
import hashlib
import json
import mmap
import os
from pathlib import Path
import re
import subprocess
import tomllib
from release_policy import compiler_identity, parse_tag


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def files(root):
    result = {}
    for path in sorted(root.rglob("*")):
        if path.is_symlink():
            raise ValueError(f"Artifact must contain regular files: {path}")
        if path.is_file() and path.name != "receipt.json":
            result[path.relative_to(root).as_posix()] = {"sha256": digest(path), "bytes": path.stat().st_size}
    if not result:
        raise ValueError(f"Empty artifact: {root}")
    return result


def source_sha():
    return subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()


def version():
    value = tomllib.loads(Path("Cargo.toml").read_text())["workspace"]["package"]["version"]
    if value != json.loads(Path("apps/gui/src-tauri/tauri.conf.json").read_text())["version"]:
        raise ValueError("Cargo and App versions differ")
    ref = os.environ.get("GITHUB_REF", "")
    if ref.startswith("refs/tags/") and parse_tag(ref.removeprefix("refs/tags/")).version != value:
        raise ValueError("Release tag and source version differ")
    return value


def verify(root, sha=None, *, kind=None, run_id=None):
    receipt = json.loads((root / "receipt.json").read_text())
    if receipt["schema"] != 1 or receipt["files"] != files(root):
        raise ValueError(f"Artifact bytes changed: {root}")
    if sha and receipt["source_sha"] != sha:
        raise ValueError(f"Artifact belongs to a different source commit: {root}")
    if kind and receipt["kind"] != kind:
        raise ValueError(f"Artifact has a different kind: {root}")
    if run_id and receipt["run_id"] != run_id:
        raise ValueError(f"Artifact belongs to a different workflow run: {root}")
    return receipt


def assert_embedded(executable, marker, agent=None):
    with executable.open("rb") as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as binary:
        if binary.find(marker.encode()) < 0:
            raise ValueError(f"Built executable does not embed this UI: {executable}")
        if agent and binary.find(agent.read_bytes()) < 0:
            raise ValueError(f"Built executable does not embed this Pi runtime: {executable}")


def check_linux_abi(executable):
    report = subprocess.check_output(["readelf", "--version-info", str(executable)], text=True)
    versions = {tuple(map(int, m.split("."))) for m in re.findall(r"\bGLIBC_(\d+\.\d+)\b", report)}
    if not versions or max(versions) > (2, 31):
        raise ValueError(f"{executable} does not meet the glibc 2.31 distribution floor: {sorted(versions)}")


def executable_info(program, command=()):
    output = subprocess.check_output([*command, str(program), "_release_info"], timeout=30)
    if len(output) > 64 * 1024:
        raise ValueError("Executable release report exceeds its limit")
    info = json.loads(output)
    if info.get("schema") != 1:
        raise ValueError("Executable has no supported release report")
    return info


def create(root, kind, frontend=None, executable=None, agent=None, program_info=None):
    sha = source_sha()
    if kind == "frontend":
        html = (root / "dist/index.html").read_text()
        match = re.search(r'src="(/assets/index-[^"/]+\.js)"', html)
        if not match or not (root / "dist" / match[1].lstrip("/")).is_file():
            raise ValueError("Frontend has no built entrypoint")
        ui = "khor-ui-assets:" + match[1]
    else:
        if frontend is None:
            raise ValueError("A product needs its verified frontend artifact")
        ui = verify(frontend, sha)["ui_revision"]
        if executable:
            assert_embedded(executable, ui, agent)
            # Standalone servers retain the glibc 2.31 distribution floor.
            # Desktop Debian packages declare their GTK/WebKit dependencies.
            if kind.startswith("khor-linux-") and not kind.endswith("-app"):
                check_linux_abi(executable)
                if agent:
                    check_linux_abi(agent)
    receipt = {
        "schema": 1, "kind": kind, "version": version(), "source_sha": sha,
        "ui_revision": ui, "files": files(root),
        "run_id": os.environ.get("GITHUB_RUN_ID"), "attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "runner_image": os.environ.get("ImageVersion"),
        "agent_sha256": digest(agent) if agent else None,
    }
    if program_info is not None:
        build = program_info.get("build", {})
        expected = compiler_identity(sha, receipt["version"], os.environ)
        expected_release = {"tag": expected["KHOR_RELEASE_TAG"], "source": sha,
                            "build": int(expected["KHOR_RELEASE_BUILD"])} if expected["KHOR_RELEASE_TAG"] else None
        if program_info.get("schema") != 1 or build.get("source") != sha or build.get("version") != receipt["version"] or build.get("release") != expected_release:
            raise ValueError("Executable release identity differs from the verified build")
        receipt["program"] = program_info
    (root / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["create", "verify", "version", "build-env"])
    parser.add_argument("--root", type=Path)
    parser.add_argument("--kind")
    parser.add_argument("--frontend", type=Path)
    parser.add_argument("--executable", type=Path)
    parser.add_argument("--agent", type=Path)
    parser.add_argument("--program-info", type=Path)
    parser.add_argument("--sha")
    parser.add_argument("--run-id")
    args = parser.parse_args()
    if args.action == "build-env":
        values = compiler_identity(source_sha(), version(), os.environ)
        destination = os.environ.get("GITHUB_ENV")
        if not destination:
            parser.error("build-env requires GitHub's step environment file")
        with Path(destination).open("a") as stream:
            for key, value in values.items():
                stream.write(f"{key}={value}\n")
        return
    if args.action == "version":
        print(version())
        return
    if args.root is None:
        parser.error("--root is required")
    if args.action == "create":
        if not args.kind:
            parser.error("--kind is required")
        info = json.loads(args.program_info.read_text()) if args.program_info else None
        result = create(args.root, args.kind, args.frontend, args.executable, args.agent, info)
    else:
        result = verify(args.root, args.sha or source_sha(), kind=args.kind, run_id=args.run_id)
    print(json.dumps({key: result[key] for key in ["kind", "version", "source_sha", "ui_revision"]}))


if __name__ == "__main__":
    main()
