"""Publish rc tags and promote their exact immutable artifacts without compilation."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
from apple_api import AppleClient
from apple_release import required, run, upload_ios
from release_candidate import PLATFORMS
from release_policy import matching_candidate, parse_tag
from release_store import api, fetch_release, gh, publish, release_info, repository, verify_candidate

ROOT = Path("candidate")


def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()


def tag_source(tag):
    release = parse_tag(tag)
    sha = git("rev-parse", f"refs/tags/{tag}^{{commit}}")
    run(["git", "merge-base", "--is-ancestor", sha, "origin/main"])
    cargo = tomllib.loads(git("show", f"{sha}:Cargo.toml"))["workspace"]["package"]["version"]
    app = json.loads(git("show", f"{sha}:apps/gui/src-tauri/tauri.conf.json"))["version"]
    if cargo != release.version or app != release.version:
        raise ValueError("Tag must match the application version at its exact source commit")
    return sha


def output(values):
    with open(required("GITHUB_OUTPUT"), "a") as stream:
        for name, value in values.items():
            stream.write(f"{name}={value}\n")


def check_build():
    ref = required("GITHUB_REF")
    rc = ref.startswith("refs/tags/")
    if rc:
        tag = parse_tag(ref.removeprefix("refs/tags/"))
        if not tag.prerelease:
            raise ValueError("Stable tags promote a tested candidate; they never build")
        tag_source(tag.name)
        existing = release_info(tag.name)
        if existing and not existing["draft"]:
            raise ValueError("This candidate is published; use distribution retry or a new rc tag")
    output({"rc": str(rc).lower()})


def plan():
    tag = parse_tag(required("RELEASE_TAG"))
    sha = tag_source(tag.name)
    if tag.prerelease:
        run_id = required("CANDIDATE_RUN_ID")
        if not run_id.isdecimal():
            raise ValueError("Candidate run ID must be numeric")
        selected = tag.name
    else:
        candidates = []
        for name in git("tag", "--points-at", sha, "--list", f"v{tag.version}-rc.*").splitlines():
            try:
                parse_tag(name)
            except ValueError:
                continue
            release = release_info(name)
            if release and not release["draft"] and release["prerelease"] and release.get("immutable"):
                candidates.append(name)
        selected = matching_candidate(tag.name, candidates, os.environ.get("CANDIDATE_TAG", ""))
        with tempfile.TemporaryDirectory() as temporary:
            record, _ = fetch_release(selected, Path(temporary), full=False)
            if record["source_sha"] != sha:
                raise ValueError("Published candidate differs from the stable tag source")
            run_id = record["run_id"]
    output({"rc": str(tag.prerelease).lower(), "tag": tag.name, "candidate_tag": selected,
            "source_sha": sha, "run_id": run_id})


def fetch():
    tag = parse_tag(required("RELEASE_TAG"))
    run_id = required("CANDIDATE_RUN_ID")
    if not tag.prerelease or not run_id.isdecimal():
        raise ValueError("Download a numbered rc build run")
    source = tag_source(tag.name)
    result = api(f"repos/{repository()}/actions/runs/{run_id}")
    if result["path"] != ".github/workflows/release.yml" or result["event"] not in {"push", "workflow_dispatch"}:
        raise ValueError("Only Release candidates artifacts can be distributed")
    if result["head_sha"] != source:
        raise ValueError("Run source differs from the release tag")
    pages = json.loads(gh("api", f"repos/{repository()}/actions/runs/{run_id}/jobs?filter=all&per_page=100", "--paginate", "--slurp"))
    if not any(job["name"] == "candidate" and job["conclusion"] == "success" for page in pages for job in page["jobs"]):
        raise ValueError("The candidate aggregation and all required checks must succeed first")
    for name in sorted(PLATFORMS | {"candidate-manifest"}):
        gh("run", "download", run_id, "--repo", repository(), "--name", name, "--dir", str(ROOT / name))
    record = verify_candidate(ROOT)
    if record["source_sha"] != source or record["tag"] != tag.name or record["run_id"] != run_id:
        raise ValueError("Candidate manifest does not belong to this exact tag and run")


def testflight():
    record = verify_candidate(ROOT)
    uploaded = upload_ios(ROOT / "ios-archive")
    uploaded.update(source_sha=record["source_sha"], run_id=record["run_id"])
    Path("testflight.json").write_text(json.dumps(uploaded, sort_keys=True, indent=2) + "\n")


def published_candidate(full):
    tag = required("RELEASE_TAG")
    selected = required("CANDIDATE_TAG")
    source = tag_source(tag)
    if tag_source(selected) != source or not parse_tag(selected).prerelease:
        raise ValueError("Promotion must use a candidate from this exact source")
    record, descriptor = fetch_release(selected, ROOT, full=full)
    if record["source_sha"] != source or record["version"] != parse_tag(tag).version:
        raise ValueError("Candidate source or version differs")
    return descriptor


def publish_github():
    tag = parse_tag(required("RELEASE_TAG"))
    if tag.prerelease:
        uploaded = json.loads(Path("testflight.json").read_text())
    else:
        uploaded = published_candidate(full=True)["testflight"]
    publish(tag.name, ROOT, uploaded)


def appstore_review():
    if parse_tag(required("RELEASE_TAG")).prerelease:
        raise ValueError("Only stable tags can submit App Store review")
    descriptor = published_candidate(full=False)
    build = descriptor["testflight"]
    with AppleClient() as client:
        processed = client.processed_build(build["version"], build["build"], timeout=0)
        if processed["id"] != build["apple_build_id"]:
            raise ValueError("Apple build differs from the tested candidate")
        if client.already_submitted(build["version"], processed["id"]):
            print("This exact build is already submitted; keeping manual release unchanged.")
            return
    with tempfile.TemporaryDirectory() as temporary:
        key = Path(temporary) / "api-key.json"
        key.write_text(json.dumps({"key_id": required("APPLE_API_KEY"), "issuer_id": required("APPLE_API_ISSUER"),
                                   "key": required("APPLE_API_KEY_CONTENT"), "in_house": False}))
        key.chmod(0o600)
        run(["fastlane", "deliver", "--api_key_path", str(key), "--app_identifier", "io.github.colehank.khor",
             "--app_version", build["version"], "--build_number", build["build"], "--skip_binary_upload",
             "--skip_screenshots", "--skip_metadata", "--submit_for_review", "--automatic_release", "false", "--force"])


if __name__ == "__main__":
    actions = {"check-build": check_build, "plan": plan, "fetch": fetch, "testflight": testflight,
               "publish": publish_github, "appstore-review": appstore_review}
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=actions)
    args = parser.parse_args()
    if os.environ.get("GITHUB_ACTIONS") != "true":
        parser.error("Distribution runs on GitHub, not on a developer machine")
    actions[args.action]()
