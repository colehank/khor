"""Explicit distribution of a successful immutable GitHub candidate; no compilation."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
from apple_release import required, run
from release_candidate import PLATFORMS
from release_manifest import digest, verify

ROOT = Path("candidate")


def gh_json(*args):
    return json.loads(subprocess.check_output(["gh", *args], text=True))


def manifest():
    return json.loads((ROOT / "candidate-manifest/manifest.json").read_text())


def verify_candidate():
    record = manifest()
    if set(record["artifacts"]) != PLATFORMS:
        raise ValueError("Incomplete platform manifest")
    for name, expected in record["artifacts"].items():
        if verify(ROOT / name, record["source_sha"]) != expected:
            raise ValueError(f"Artifact receipt differs from candidate manifest: {name}")
    if digest(ROOT / "candidate-manifest/install.sh") != record["installer"]["sha256"]:
        raise ValueError("Candidate installer changed")
    return record


def fetch():
    run_id = required("CANDIDATE_RUN_ID")
    if not run_id.isdecimal():
        raise ValueError("Candidate run ID must be numeric")
    repo = required("GITHUB_REPOSITORY")
    result = gh_json("api", f"repos/{repo}/actions/runs/{run_id}")
    if result["conclusion"] != "success" or result["path"] != ".github/workflows/release.yml":
        raise ValueError("Choose a successful Release candidates run")
    if result["event"] not in {"push", "workflow_dispatch"}:
        raise ValueError("Distribution cannot consume pull request artifacts")
    run(["git", "merge-base", "--is-ancestor", result["head_sha"], "origin/main"])
    for name in sorted(PLATFORMS | {"candidate-manifest"}):
        run(["gh", "run", "download", run_id, "--repo", repo, "--name", name, "--dir", str(ROOT / name)])
    if verify_candidate()["source_sha"] != result["head_sha"]:
        raise ValueError("Candidate receipt does not belong to the selected run")


def github_draft():
    record = verify_candidate()
    tag = "v" + record["version"]
    tag_sha = subprocess.check_output(["git", "rev-parse", f"{tag}^{{commit}}"], text=True).strip()
    if tag_sha != record["source_sha"]:
        raise ValueError("Release tag must identify this exact candidate commit")
    # Public Mac assets must be Developer ID signed and notarized candidates.
    for name in ["khor-macos-arm64-app", "khor-macos-x86_64-app"]:
        if (ROOT / name / "signed.txt").read_text().strip() != "true":
            raise ValueError("Choose a signed candidate before publishing Mac Apps")
    assets = []
    for name in sorted(PLATFORMS - {"ios-archive"}):
        assets.extend(str(p) for p in (ROOT / name).iterdir() if p.name not in {"receipt.json", "signed.txt"})
    assets.extend(str(ROOT / "candidate-manifest" / name) for name in ["manifest.json", "install.sh"])
    run(["gh", "release", "create", tag, "--verify-tag", "--draft", "--generate-notes", *assets])


def appstore_review():
    verify_candidate()
    build = json.loads((ROOT / "ios-archive/build.json").read_text())
    if not build["signed"]:
        raise ValueError("Unsigned candidates cannot be submitted")
    with tempfile.TemporaryDirectory() as temporary:
        key = Path(temporary) / "api-key.json"
        key.write_text(json.dumps({"key_id": required("APPLE_API_KEY"), "issuer_id": required("APPLE_API_ISSUER"),
            "key": required("APPLE_API_KEY_CONTENT"), "in_house": False}))
        key.chmod(0o600)
        # deliver selects this already processed build; it does not upload another binary.
        run(["fastlane", "deliver", "--api_key_path", str(key), "--app_identifier", "io.github.colehank.khor",
            "--app_version", build["version"], "--build_number", build["build"], "--skip_binary_upload",
            "--skip_screenshots", "--skip_metadata", "--submit_for_review", "--automatic_release", "false", "--force"])


def receipt():
    record = verify_candidate()
    Path("distribution.json").write_text(json.dumps({"candidate_run_id": required("CANDIDATE_RUN_ID"),
        "source_sha": record["source_sha"], "version": record["version"], "stage": required("CHANNEL"),
        "result": "completed", "note": "Apple processing, user acceptance and public availability are separate."}, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["fetch", "github-draft", "appstore-review", "receipt"])
    args = parser.parse_args()
    if os.environ.get("GITHUB_ACTIONS") != "true":
        parser.error("Distribution runs on GitHub, not on a developer machine")
    {"fetch": fetch, "github-draft": github_draft, "appstore-review": appstore_review, "receipt": receipt}[args.action]()
