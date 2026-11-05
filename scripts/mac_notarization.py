"""Persist signed Mac builds and resume notarization without rebuilding them."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import uuid

from apple_release import api_key, required, run
from release_manifest import create, digest, executable_info, files, source_sha, verify
from release_store import gh, repository

TARGETS = {"aarch64-apple-darwin": "khor-macos-arm64", "x86_64-apple-darwin": "khor-macos-x86_64"}


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    temporary.replace(path)


def package_build(target, signed, root, frontend, agent):
    asset = TARGETS[target]
    app = Path("target") / target / "release/bundle/macos/Khor.app"
    root.mkdir(parents=True, exist_ok=True)
    archive = root / "upload.zip"
    run(["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", str(app), str(archive)])
    archive.rename(root / f"{asset}-{required('GITHUB_RUN_ID')}-{digest(archive)}.zip")
    (root / "signed.txt").write_text(str(signed).lower() + "\n")
    program = app / "Contents/MacOS/khor-gui"
    create(root, f"{asset}-app-build", frontend, program, agent, executable_info(program))


def build_identity(root, asset):
    if asset not in TARGETS.values():
        raise ValueError("Unsupported Mac artifact")
    receipt = verify(root, source_sha())
    if receipt["kind"] != f"{asset}-app-build" or receipt["run_id"] != required("GITHUB_RUN_ID"):
        raise ValueError("Mac build belongs to a different platform or run")
    archives = list(root.glob("*.zip"))
    if len(archives) != 1:
        raise ValueError("Mac build must contain exactly one upload archive")
    archive = archives[0]
    sha = digest(archive)
    if archive.name != f"{asset}-{receipt['run_id']}-{sha}.zip":
        raise ValueError("Notary upload name does not identify these exact bytes")
    signed = (root / "signed.txt").read_text().strip()
    if signed not in {"true", "false"}:
        raise ValueError("Missing signing decision")
    return {"schema": 1, "asset": asset, "source_sha": receipt["source_sha"], "run_id": receipt["run_id"],
            "build_receipt_sha256": digest(root / "receipt.json"), "archive": archive.name,
            "archive_sha256": sha, "signed": signed == "true"}


def read_bound(path, identity):
    record = json.loads(path.read_text())
    if any(record.get(key) != value for key, value in identity.items()):
        raise ValueError("Notarization record belongs to different build bytes, source or run")
    return record


def request_id(value):
    if not isinstance(value, str) or str(uuid.UUID(value)) != value.lower():
        raise ValueError("Invalid notarization submission ID")
    return value


class Notary:
    def __init__(self, key):
        self.auth = ["--key", str(key), "--key-id", required("APPLE_API_KEY"), "--issuer", required("APPLE_API_ISSUER")]

    def call(self, action, *args):
        try:
            result = subprocess.run(["xcrun", "notarytool", action, *map(str, args), *self.auth,
                                     "--output-format", "json"], capture_output=True, text=True,
                                    timeout=300 if action == "submit" else 60)
        except subprocess.TimeoutExpired:
            raise RuntimeError(f"notarytool {action} timed out") from None
        if result.returncode:
            raise RuntimeError(f"notarytool {action} failed ({result.returncode}); credentials are not logged")
        return json.loads(result.stdout)


def prepare(root, asset, state):
    identity = build_identity(root, asset)
    path = state / "intent.json"
    if path.exists():
        read_bound(path, identity)
    else:
        write_json(path, identity)


def submit(root, asset, state, allow_submit, client):
    identity = build_identity(root, asset)
    read_bound(state / "intent.json", identity)
    path = state / "request.json"
    if path.exists():
        record = read_bound(path, identity)
        if identity["signed"]:
            request_id(record["request_id"])
        return record
    submission = None
    if identity["signed"]:
        history = client.call("history")
        matches = [item for item in history["history"] if item["name"] == identity["archive"]]
        if len(matches) > 1:
            raise ValueError("Multiple Apple submissions match this archive; refusing to guess")
        if matches:
            submission = request_id(matches[0]["id"])
        elif allow_submit:
            result = client.call("submit", root / identity["archive"], "--no-wait")
            submission = request_id(result["id"])
        else:
            raise ValueError("Previous submission outcome is unknown and Apple history has no match; retry recovery later, do not resubmit")
    record = {**identity, "request_id": submission}
    write_json(path, record)
    print(f"Saved notarization request: {submission or 'unsigned diagnostic build'}", flush=True)
    return record


def wait_for_acceptance(record, state, client, timeout, interval=30):
    deadline = time.monotonic() + timeout
    submission = request_id(record["request_id"])
    while True:
        try:
            result = client.call("info", submission)
        except RuntimeError as error:
            result = {"status": "Query unavailable", "message": str(error)}
        else:
            if result.get("id") != submission or result.get("name") != record["archive"]:
                raise ValueError("Apple response does not match the persisted submission")
        status = result["status"]
        write_json(state / "status.json", {**record, "status": status})
        print(f"Notarization {submission}: {status}", flush=True)
        if status == "Accepted":
            return
        if status in {"Invalid", "Rejected"}:
            write_json(state / "rejection.json", client.call("log", submission))
            raise ValueError("Apple rejected this build; see the notarization diagnostics, do not resubmit unchanged bytes")
        if status not in {"In Progress", "Query unavailable"}:
            raise ValueError(f"Unexpected notarization status: {status}")
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError(f"Notarization {submission} is not complete. Re-run this failed job to query the same request; the build is preserved.")
        time.sleep(min(interval, remaining))


def finalize(root, asset, state, output, client, timeout):
    identity = build_identity(root, asset)
    record = read_bound(state / "request.json", identity)
    if identity["signed"]:
        request_id(record["request_id"])
    proof = {**record, "status": "Accepted" if identity["signed"] else "Unsigned"}
    if (output / "receipt.json").exists():
        receipt = verify(output, identity["source_sha"])
        if receipt["run_id"] != identity["run_id"] or receipt["kind"] != f"{asset}-app":
            raise ValueError("Restored Mac App belongs to a different run or platform")
        if json.loads((output / "notarization.json").read_text()) != proof:
            raise ValueError("Restored Mac App has a different notarization proof")
        return
    if identity["signed"]:
        wait_for_acceptance(record, state, client, timeout)
    with tempfile.TemporaryDirectory() as temporary:
        unpacked = Path(temporary)
        run(["ditto", "-x", "-k", str(root / identity["archive"]), str(unpacked)])
        app = unpacked / "Khor.app"
        if not app.is_dir():
            raise ValueError("Mac archive has no Khor.app")
        if identity["signed"]:
            run(["codesign", "--verify", "--deep", "--strict", str(app)])
            run(["xcrun", "stapler", "staple", str(app)])
            run(["xcrun", "stapler", "validate", str(app)])
            run(["codesign", "--verify", "--deep", "--strict", str(app)])
        output.mkdir(parents=True, exist_ok=True)
        run(["tar", "-czf", str(output / f"{asset}.app.tar.gz"), "-C", str(unpacked), "Khor.app"])
    (output / "signed.txt").write_text(str(identity["signed"]).lower() + "\n")
    write_json(output / "notarization.json", proof)
    receipt = json.loads((root / "receipt.json").read_text())
    receipt.update(kind=f"{asset}-app", files=files(output))
    write_json(output / "receipt.json", receipt)


def restore_artifact(name, destination):
    run_id = required("GITHUB_RUN_ID")
    pages = json.loads(gh("api", f"repos/{repository()}/actions/runs/{run_id}/artifacts?per_page=100", "--paginate", "--slurp"))
    matches = [item for page in pages for item in page["artifacts"] if item["name"] == name]
    if len(matches) > 1 or any(item["expired"] for item in matches):
        raise ValueError("Recovery artifact is ambiguous or expired; do not rebuild into this candidate")
    if matches:
        gh("run", "download", run_id, "--repo", repository(), "--name", name, "--dir", str(destination))
    with open(required("GITHUB_OUTPUT"), "a") as stream:
        stream.write(f"found={str(bool(matches)).lower()}\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["package-build", "restore", "prepare", "submit", "finish"])
    parser.add_argument("--target", choices=TARGETS)
    parser.add_argument("--asset", choices=TARGETS.values())
    parser.add_argument("--signed", choices=["true", "false"], default="false")
    parser.add_argument("--root", type=Path, default=Path("mac-build"))
    parser.add_argument("--state", type=Path, default=Path("notary"))
    parser.add_argument("--output", type=Path, default=Path("desktop"))
    parser.add_argument("--name")
    parser.add_argument("--allow-submit", choices=["true", "false"], default="false")
    parser.add_argument("--timeout", type=int, default=1800)
    args = parser.parse_args()
    if os.environ.get("GITHUB_ACTIONS") != "true":
        parser.error("Mac release stages run on GitHub; local ship is disabled")
    if args.action == "restore":
        if not args.name:
            parser.error("--name is required")
        restore_artifact(args.name, args.output)
    elif args.action == "package-build":
        if args.target is None:
            parser.error("--target is required")
        package_build(args.target, args.signed == "true", args.root, Path("frontend"), Path("apps/agent/dist/khor-agent"))
    elif args.action == "prepare":
        prepare(args.root, args.asset, args.state)
    else:
        if not 0 <= args.timeout <= 1800:
            parser.error("Polling timeout must be between 0 and 1800 seconds")
        identity = build_identity(args.root, args.asset)
        def execute(client):
            if args.action == "submit":
                submit(args.root, args.asset, args.state, args.allow_submit == "true", client)
            else:
                finalize(args.root, args.asset, args.state, args.output, client, args.timeout)
        if identity["signed"]:
            with api_key() as key:
                execute(Notary(key))
        else:
            execute(None)


if __name__ == "__main__":
    main()
