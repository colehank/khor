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
    if ref.startswith("refs/tags/") and ref != f"refs/tags/v{value}":
        raise ValueError("Release tag and source version differ")
    return value


def verify(root, sha=None):
    receipt = json.loads((root / "receipt.json").read_text())
    if receipt["schema"] != 1 or receipt["files"] != files(root):
        raise ValueError(f"Artifact bytes changed: {root}")
    if sha and receipt["source_sha"] != sha:
        raise ValueError(f"Artifact belongs to a different source commit: {root}")
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


def create(root, kind, frontend=None, executable=None, agent=None):
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
            if kind.startswith("khor-linux-"):
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
    (root / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["create", "verify", "version"])
    parser.add_argument("--root", type=Path)
    parser.add_argument("--kind")
    parser.add_argument("--frontend", type=Path)
    parser.add_argument("--executable", type=Path)
    parser.add_argument("--agent", type=Path)
    parser.add_argument("--sha")
    args = parser.parse_args()
    if args.action == "version":
        print(version())
        return
    if args.root is None:
        parser.error("--root is required")
    if args.action == "create":
        if not args.kind:
            parser.error("--kind is required")
        result = create(args.root, args.kind, args.frontend, args.executable, args.agent)
    else:
        result = verify(args.root, args.sha or source_sha())
    print(json.dumps({key: result[key] for key in ["kind", "version", "source_sha", "ui_revision"]}))


if __name__ == "__main__":
    main()
