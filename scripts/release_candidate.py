"""Assemble one candidate from verified platform receipts, without publishing it."""
import json
from pathlib import Path
import sys
from release_manifest import digest, source_sha, verify, version

PLATFORMS = {
    "khor-linux-x86_64", "khor-linux-aarch64", "khor-macos-arm64",
    "khor-macos-x86_64", "khor-macos-arm64-app", "khor-macos-x86_64-app", "ios-archive",
}


def assemble(root):
    receipts = {p.name: verify(p, source_sha()) for p in root.iterdir() if p.is_dir()}
    if receipts.keys() != PLATFORMS:
        raise ValueError(f"Candidate platform set differs: {sorted(receipts)}")
    if len({v["ui_revision"] for v in receipts.values()}) != 1:
        raise ValueError("Platforms were built with different frontend artifacts")
    installer = root / "install.sh"
    return {"schema": 1, "source_sha": source_sha(), "version": version(), "artifacts": receipts,
            "installer": {"sha256": digest(installer), "bytes": installer.stat().st_size}}


if __name__ == "__main__":
    root = Path(sys.argv[1])
    (root / "manifest.json").write_text(json.dumps(assemble(root), indent=2) + "\n")
