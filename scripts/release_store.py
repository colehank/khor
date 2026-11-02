"""Keep verified candidates in immutable GitHub Releases, beyond Actions retention."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
from release_candidate import PLATFORMS
from release_manifest import digest, verify
from release_policy import parse_tag


def gh(*args, data=None):
    result = subprocess.run(["gh", *args], input=json.dumps(data) if data is not None else None,
                            capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(result.stderr.strip())
    return result.stdout


def api(path, method="GET", data=None):
    args = ["api", path, "--method", method]
    if data is not None:
        args += ["--input", "-"]
    output = gh(*args, data=data)
    return json.loads(output) if output.strip() else None


def repository():
    return os.environ["GITHUB_REPOSITORY"]


def read_manifest(root):
    return json.loads((root / "candidate-manifest/manifest.json").read_text())


def validate_manifest(record):
    if record["schema"] != 1 or set(record["artifacts"]) != PLATFORMS:
        raise ValueError("Incomplete platform manifest")
    tag = parse_tag(record["tag"])
    if not tag.prerelease or tag.version != record["version"]:
        raise ValueError("Candidate manifest needs its matching rc tag")
    if not re.fullmatch(r"[0-9a-f]{40}", record["source_sha"]) or not str(record["run_id"]).isdecimal():
        raise ValueError("Invalid candidate source or run")
    for name, receipt in record["artifacts"].items():
        if any(receipt[key] != record[key] for key in ["source_sha", "version", "run_id"]):
            raise ValueError("Mixed candidate provenance")
        if receipt["kind"] != name or receipt["schema"] != 1:
            raise ValueError("Invalid platform receipt")
    if len({r["ui_revision"] for r in record["artifacts"].values()}) != 1:
        raise ValueError("Mixed frontend artifacts")
    return record


def verify_candidate(root):
    record = validate_manifest(read_manifest(root))
    for name, expected in record["artifacts"].items():
        if verify(root / name, record["source_sha"]) != expected:
            raise ValueError(f"Artifact receipt differs: {name}")
    if digest(root / "candidate-manifest/install.sh") != record["installer"]["sha256"]:
        raise ValueError("Candidate installer changed")
    signed_build(root, record)
    return record


def signed_build(root, record):
    for name in ["khor-macos-arm64-app", "khor-macos-x86_64-app"]:
        if (root / name / "signed.txt").read_text().strip() != "true":
            raise ValueError("Mac candidates must be signed and notarized")
        proof = json.loads((root / name / "notarization.json").read_text())
        if (proof.get("status") != "Accepted" or proof.get("signed") is not True
                or proof.get("source_sha") != record["source_sha"] or proof.get("run_id") != record["run_id"]
                or proof.get("asset") != name.removesuffix("-app")
                or not re.fullmatch(r"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}", proof.get("request_id", ""))
                or not re.fullmatch(r"[0-9a-f]{64}", proof.get("archive_sha256", ""))):
            raise ValueError("Mac candidate has no accepted notarization for this exact build")
    build = json.loads((root / "ios-archive/build.json").read_text())
    if build["signed"] is not True or build["version"] != record["version"]:
        raise ValueError("iOS candidate must be signed at the same version")
    return build


def asset_map(record):
    assets = {}
    for platform, receipt in record["artifacts"].items():
        for filename, info in receipt["files"].items():
            if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", filename):
                raise ValueError("Release assets must have flat safe filenames")
            name = f"{platform}-{filename}" if filename in {"signed.txt", "build.json", "notarization.json"} else filename
            if name in assets or name in {"manifest.json", "install.sh", "release.json"}:
                raise ValueError("Release asset name collision")
            assets[name] = {"path": f"{platform}/{filename}", **info}
    assets["install.sh"] = {"path": "candidate-manifest/install.sh", **record["installer"]}
    return assets


def validate_testflight(record, build, uploaded):
    expected = {"source_sha": record["source_sha"], "run_id": record["run_id"],
                "version": build["version"], "build": build["build"]}
    if any(uploaded.get(key) != value for key, value in expected.items()):
        raise ValueError("TestFlight receipt belongs to a different candidate")
    if uploaded.get("processing_state") != "VALID" or not uploaded.get("apple_build_id"):
        raise ValueError("Apple has not processed this candidate successfully")


def release_info(tag):
    result = subprocess.run(["gh", "api", f"repos/{repository()}/releases/tags/{tag}"], capture_output=True, text=True)
    if result.returncode == 0:
        return json.loads(result.stdout)
    if "HTTP 404" in result.stderr:
        pages = json.loads(gh("api", f"repos/{repository()}/releases?per_page=100", "--paginate", "--slurp"))
        matches = [release for page in pages for release in page if release["tag_name"] == tag]
        if len(matches) > 1:
            raise ValueError("Multiple release drafts use this tag; refusing to choose one")
        return api(f"repos/{repository()}/releases/{matches[0]['id']}") if matches else None
    raise RuntimeError(result.stderr.strip())


def verify_remote_assets(release, expected, complete=True):
    actual = {asset["name"]: asset for asset in release["assets"]}
    if set(actual) - set(expected) or (complete and set(actual) != set(expected)):
        raise ValueError("Release asset set differs from the selected candidate")
    for name, asset in actual.items():
        info = expected[name]
        if asset.get("digest") != "sha256:" + info["sha256"] or asset["size"] != info["bytes"]:
            raise ValueError(f"Release asset differs: {name}; never overwriting it")
    return actual


def download_asset(tag, name, destination):
    with tempfile.TemporaryDirectory() as temporary:
        gh("release", "download", tag, "--repo", repository(), "--pattern", name, "--dir", temporary)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(Path(temporary) / name), destination)


def fetch_release(tag, root, *, full=True):
    release = release_info(tag)
    if release is None or release["draft"] or not release["prerelease"] or not release.get("immutable"):
        raise ValueError("Promotion requires a published immutable candidate release")
    download_asset(tag, "manifest.json", root / "candidate-manifest/manifest.json")
    download_asset(tag, "release.json", root / "release.json")
    record = validate_manifest(read_manifest(root))
    descriptor = json.loads((root / "release.json").read_text())
    if descriptor["schema"] != 1 or record["tag"] != tag or descriptor["tag"] != tag or descriptor["candidate_tag"] != tag:
        raise ValueError("Candidate release identity differs")
    if descriptor["manifest_sha256"] != digest(root / "candidate-manifest/manifest.json"):
        raise ValueError("Candidate manifest changed")
    assets = asset_map(record)
    for name, path in [("manifest.json", root / "candidate-manifest/manifest.json"), ("release.json", root / "release.json")]:
        assets[name] = {"sha256": digest(path), "bytes": path.stat().st_size}
    verify_remote_assets(release, assets)
    for name, info in asset_map(record).items():
        if full or info["path"].endswith(("/signed.txt", "/build.json", "/notarization.json")):
            target = root / info["path"]
            download_asset(tag, name, target)
            if digest(target) != info["sha256"] or target.stat().st_size != info["bytes"]:
                raise ValueError(f"Downloaded release asset changed: {name}")
    for platform, receipt in record["artifacts"].items():
        (root / platform).mkdir(parents=True, exist_ok=True)
        (root / platform / "receipt.json").write_text(json.dumps(receipt))
    build = signed_build(root, record)
    validate_testflight(record, build, descriptor["testflight"])
    if full:
        verify_candidate(root)
    return record, descriptor


def publish(tag, root, uploaded):
    record = verify_candidate(root)
    release_tag = parse_tag(tag)
    if release_tag.version != record["version"] or (release_tag.prerelease and tag != record["tag"]):
        raise ValueError("Publishing tag and candidate version differ")
    source = gh("api", f"repos/{repository()}/commits/{tag}")
    if json.loads(source)["sha"] != record["source_sha"]:
        raise ValueError("Publishing tag moved away from the candidate")
    validate_testflight(record, signed_build(root, record), uploaded)
    descriptor = {"schema": 1, "tag": tag, "candidate_tag": record["tag"],
                  "manifest_sha256": digest(root / "candidate-manifest/manifest.json"), "testflight": uploaded}
    descriptor_path = root / "release.json"
    descriptor_path.write_text(json.dumps(descriptor, sort_keys=True, indent=2) + "\n")
    assets = asset_map(record)
    for name, path in [("manifest.json", root / "candidate-manifest/manifest.json"), ("release.json", descriptor_path)]:
        assets[name] = {"path": str(path.relative_to(root)), "sha256": digest(path), "bytes": path.stat().st_size}
    release = release_info(tag)
    if release and not release["draft"]:
        if release["prerelease"] != release_tag.prerelease or not release.get("immutable"):
            raise ValueError("Existing release stage or immutability differs")
        verify_remote_assets(release, assets)
        return
    if release is None:
        args = ["release", "create", tag, "--repo", repository(), "--verify-tag", "--draft", "--generate-notes"]
        if release_tag.prerelease:
            args.append("--prerelease")
        gh(*args)
        release = release_info(tag)
    existing = verify_remote_assets(release, assets, complete=False)
    with tempfile.TemporaryDirectory(dir=root.parent) as temporary:
        for name, info in assets.items():
            if name in existing:
                continue
            staged = Path(temporary) / name
            os.link(root / info["path"], staged)
            gh("release", "upload", tag, str(staged), "--repo", repository())
    verify_remote_assets(release_info(tag), assets)
    published = api(f"repos/{repository()}/releases/{release['id']}", "PATCH",
                    {"draft": False, "prerelease": release_tag.prerelease,
                     "make_latest": "false" if release_tag.prerelease else "true"})
    if not published.get("immutable"):
        raise ValueError("Enable repository immutable releases before publishing")
