"""Promote a verified iOS archive independently; never accepts the desktop release."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import zipfile

from release_manifest import verify

QUALITY_JOBS = {"mac-icon", "agent", "frontend / build", "windows-agent-process", "scripts",
                "runtime-store / verify (ubuntu-24.04)", "runtime-store / verify (macos-26)",
                "runtime-store / verify (windows-2025)", "developer-lab", "ios-native-ui",
                "node-native", "gui", "mac-updates"}


def api(path):
    repo = os.environ["GITHUB_REPOSITORY"]
    return json.loads(subprocess.check_output(["gh", "api", f"repos/{repo}/{path}"], text=True))


def require_quality(run, jobs, source):
    if (run["head_sha"] != source or run["head_branch"] != "main"
            or run["status"] != "completed" or run["conclusion"] != "success"
            or run["path"] != ".github/workflows/quality.yml"):
        raise ValueError("The exact product source has not passed main Quality")
    # Full Quality includes this separately requested DSH suite as a known
    # conditional skip. Never filter arbitrary skipped or failed shared gates.
    conditional = [j for j in jobs if j["name"] == "dsh-integration"]
    if (len(conditional) > 1
            or any(j["head_sha"] != source or j["status"] != "completed"
                   or j["conclusion"] != "skipped" for j in conditional)):
        raise ValueError("The optional DSH job must be the original conditional skip")
    jobs = [j for j in jobs if j["name"] != "dsh-integration"]
    if (len(jobs) != len(QUALITY_JOBS) or {j["name"] for j in jobs} != QUALITY_JOBS
            or any(j["head_sha"] != source or j["status"] != "completed"
                   or j["conclusion"] != "success" for j in jobs)):
        raise ValueError("Every original shared quality gate must succeed")


def require_ios_producer(run, jobs, source, tag):
    if (run["head_sha"] != source or run["head_branch"] != tag
            or run["path"] != ".github/workflows/release.yml"):
        raise ValueError("Archive producer belongs to another source or release")
    ios = [j for j in jobs if j["name"] == "ios"]
    if (len(ios) != 1 or ios[0]["head_sha"] != source
            or ios[0]["status"] != "completed" or ios[0]["conclusion"] != "success"):
        raise ValueError("The original signed iOS archive job must succeed")
    required = {"quality / " + n for n in QUALITY_JOBS if n != "frontend / build"}
    shared = [j for j in jobs if j["name"] in required]
    frontend = [j for j in jobs if j["name"] == "frontend / build"]
    if (len(shared) != len(required) or {j["name"] for j in shared} != required
            or len(frontend) != 1
            or any(j["head_sha"] != source or j["status"] != "completed"
                   or j["conclusion"] != "success" for j in shared + frontend)):
        raise ValueError("The producer's shared quality gates and frontend must also succeed")


def check_archive(root, source, run_id, version, build):
    receipt = verify(root, source, kind="ios-archive", run_id=str(run_id))
    info = json.loads((root / "build.json").read_text())
    if (receipt["version"] != version or info["version"] != version
            or str(info["build"]) != build or info["signed"] is not True):
        raise ValueError("Signed iOS version/build differs from the requested candidate")
    return receipt, info


def remote_gates(args):
    ref = api("git/ref/tags/" + args.tag)["object"]
    sha = api("git/tags/" + ref["sha"])["object"]["sha"] if ref["type"] == "tag" else ref["sha"]
    if sha != args.source:
        raise ValueError("Candidate tag no longer identifies the requested source")
    quality = api(f"actions/runs/{args.quality_run}")
    quality_jobs = api(f"actions/runs/{args.quality_run}/jobs?per_page=100")["jobs"]
    require_quality(quality, quality_jobs, args.source)
    producer = api(f"actions/runs/{args.run_id}")
    jobs = api(f"actions/runs/{args.run_id}/jobs?per_page=100")["jobs"]
    require_ios_producer(producer, jobs, args.source, args.tag)
    return dict(quality=quality, quality_jobs=quality_jobs, producer=producer, producer_jobs=jobs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=["stage", "upload"])
    parser.add_argument("--run-id", type=int, required=True)
    parser.add_argument("--quality-run", type=int, required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--source", required=True)
    parser.add_argument("--build", required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    if (not re.fullmatch(r"[0-9a-f]{40}", args.source)
            or not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+-rc\.[0-9]+", args.tag)
            or not re.fullmatch(r"[0-9]+\.[0-9]+", args.build)):
        raise ValueError("An exact rc source, tag and Apple build are required")
    version = args.tag[1:].split("-rc.")[0]
    gates = remote_gates(args)
    args.report.mkdir(parents=True, exist_ok=True)
    (args.report / (args.phase + "-gates.json")).write_text(json.dumps(gates, indent=2) + "\n")
    proof_path = args.report / "ios-candidate-proof.json"
    if args.phase == "stage":
        items = api(f"actions/runs/{args.run_id}/artifacts?per_page=100")["artifacts"]
        matches = [a for a in items if a["name"] == "ios-archive" and not a["expired"]]
        if len(matches) != 1:
            raise ValueError("Require exactly one original iOS archive")
        artifact = matches[0]
        if (artifact["workflow_run"]["id"] != args.run_id
                or artifact["workflow_run"]["head_sha"] != args.source
                or not 0 < artifact["size_in_bytes"] <= 256 * 1024**2):
            raise ValueError("Archive artifact identity differs")
        archive = args.report / "ios-archive.zip"
        with archive.open("xb") as output:
            subprocess.run(["gh", "api", f"repos/{os.environ['GITHUB_REPOSITORY']}/actions/artifacts/{artifact['id']}/zip"],
                           stdout=output, check=True, timeout=180)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        if artifact["digest"] != "sha256:" + digest or archive.stat().st_size != artifact["size_in_bytes"]:
            raise ValueError("Downloaded archive differs from GitHub digest")
        with zipfile.ZipFile(archive) as bundle:
            if (set(bundle.namelist()) != {"receipt.json", "build.json", "Khor.xcarchive.tar.gz"}
                    or sum(x.file_size for x in bundle.infolist()) > 300 * 1024**2
                    or bundle.testzip() is not None):
                raise ValueError("Unexpected or corrupt iOS archive")
            args.root.mkdir(parents=True, exist_ok=False)
            bundle.extractall(args.root)
        receipt, info = check_archive(args.root, args.source, args.run_id, version, args.build)
        proof = dict(schema=1, source_sha=args.source, run_id=str(args.run_id), tag=args.tag,
                     quality_run=args.quality_run, version=version, build=info["build"],
                     artifact_id=artifact["id"], artifact_sha256=digest, receipt=receipt,
                     upload_control_source=os.environ["GITHUB_SHA"],
                     upload_control_run=os.environ["GITHUB_RUN_ID"], platform_scope="ios",
                     full_multi_platform_release_accepted=False)
        proof_path.write_text(json.dumps(proof, indent=2) + "\n")
        print(f"Verified signed iOS {version} ({info['build']}) from {args.source}; desktop acceptance remains separate")
    else:
        proof = json.loads(proof_path.read_text())
        receipt, _ = check_archive(args.root, args.source, args.run_id, version, args.build)
        if (proof["source_sha"] != args.source or proof["run_id"] != str(args.run_id)
                or proof["tag"] != args.tag or proof["receipt"] != receipt
                or proof["upload_control_source"] != os.environ["GITHUB_SHA"]
                or proof["upload_control_run"] != os.environ["GITHUB_RUN_ID"]):
            raise ValueError("Prepared iOS candidate identity changed before upload")
        from apple_release import upload_ios
        uploaded = upload_ios(args.root)
        uploaded.update(source_sha=args.source, run_id=str(args.run_id), tag=args.tag,
                        platform_scope="ios", upload_control_source=os.environ["GITHUB_SHA"],
                        upload_control_run=os.environ["GITHUB_RUN_ID"],
                        full_multi_platform_release_accepted=False)
        (args.report / "testflight.json").write_text(json.dumps(uploaded, indent=2) + "\n")


if __name__ == "__main__":
    main()
