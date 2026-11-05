"""Sign update metadata for verified original candidates; never rebuild a product."""
import argparse
import gzip
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

from release_manifest import files
from release_policy import parse_tag

TARGETS = {
    "khor-linux-x86_64": ("cli", "linux", "x86_64"),
    "khor-linux-aarch64": ("cli", "linux", "aarch64"),
    "khor-macos-arm64": ("cli", "macos", "aarch64"),
    "khor-macos-x86_64": ("cli", "macos", "x86_64"),
    "khor-macos-arm64-app": ("desktop-app", "macos", "aarch64"),
    "khor-macos-x86_64-app": ("desktop-app", "macos", "x86_64"),
}
MANIFEST = "khor-release.json"
TAURI = "khor-tauri-update.json"


def write_json(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n")


def identity(record, tag):
    selected, built = parse_tag(tag), parse_tag(record["tag"])
    if selected.version != record["version"] or (selected.prerelease and tag != built.name):
        raise ValueError("Update publication must use this candidate or its stable promotion")
    return dict(tag=tag, source=record["source_sha"], build=int(record["run_id"]))


def products(record):
    expected_build = dict(version=record["version"], source=record["source_sha"],
                          release=identity(record, record["tag"]))
    for name, (component, system, arch) in TARGETS.items():
        receipt = record["artifacts"][name]
        info = receipt.get("program", {})
        platform = dict(os=system, arch=arch)
        if (info.get("schema") != 1 or info.get("build") != expected_build
                or info.get("component") != component or info.get("platform") != platform
                or not isinstance(info.get("contract"), dict)):
            raise ValueError(f"Missing or mismatched executable identity: {name}")
        filename = name.removesuffix("-app") + (".app.tar.gz" if component == "desktop-app" else ".gz")
        yield name, filename, info, receipt["files"][filename]


def unpacked_size(path, component):
    if component == "desktop-app":
        with tarfile.open(path, "r|gz") as archive:
            size = sum(member.size for member in archive if member.isfile())
    else:
        size = 0
        with gzip.open(path, "rb") as stream:
            while chunk := stream.read(1024 * 1024):
                size += len(chunk)
    if not 0 < size <= 16 * 1024**3:
        raise ValueError("Original update archive has an invalid expanded size")
    return size


def sign(path, signer, version):
    # Key and passphrase are consumed by the official signer through its environment.
    result = subprocess.run([str(signer), "signer", "sign", "--app-version", version, str(path)],
                            capture_output=True, timeout=300)
    if result.returncode:
        raise RuntimeError("Official update signer failed; credentials and output are not logged")
    signature = Path(str(path) + ".sig").read_text().strip()
    if not signature or len(signature) > 4096:
        raise ValueError("Official signer produced an invalid signature size")
    return signature


def verify_native(root, publication, program):
    environment = {k: v for k, v in os.environ.items()
                   if not k.startswith(("TAURI_SIGNING_", "APPLE_")) and k not in {"GH_TOKEN", "GITHUB_TOKEN"}}
    manifest = json.loads((publication / MANIFEST).read_text())
    with tempfile.TemporaryDirectory(prefix="khor-update-verify-") as temporary:
        archives = Path(temporary)
        for name, filename, _, _ in products(root_record(root)):
            (archives / filename).symlink_to((root / name / filename).resolve())
        result = subprocess.run([str(program.resolve()), "_verify_release", str((publication / MANIFEST).resolve()),
                                 str((publication / (MANIFEST + ".sig")).resolve()), str(archives)],
                                env=environment, capture_output=True, text=True, timeout=300)
    if result.returncode or json.loads(result.stdout) != manifest["identity"]:
        raise ValueError("Native trust root rejected the update publication or its original archives")


def root_record(root):
    return json.loads((root / "candidate-manifest/manifest.json").read_text())


def verify_publication(publication, record, tag):
    receipt = json.loads((publication / "receipt.json").read_text())
    expected_identity = identity(record, tag)
    if (receipt.get("schema") != 1 or receipt.get("identity") != expected_identity
            or receipt.get("built_from") != record["tag"] or receipt.get("files") != files(publication)):
        raise ValueError("Update publication receipt differs from the selected original candidate")
    manifest = json.loads((publication / MANIFEST).read_text())
    if (manifest.get("schema") != 1 or manifest.get("identity") != expected_identity
            or manifest.get("built_from") != record["tag"]):
        raise ValueError("Signed update identity differs from publication")
    assets = manifest["artifacts"]
    expected = list(products(record))
    if len(assets) != len(expected):
        raise ValueError("Update publication has an incomplete target set")
    platforms, names = {}, {MANIFEST, MANIFEST + ".sig", TAURI}
    prefix = f"https://github.com/{os.environ['GITHUB_REPOSITORY']}/releases/download/{record['tag']}/"
    for asset, (_, filename, program, original) in zip(assets, expected):
        expected_asset = dict(component=program["component"], platform=program["platform"], file=filename,
                              url=prefix + filename, bytes=original["bytes"], sha256=original["sha256"],
                              contract=program["contract"], unpacked_bytes=asset.get("unpacked_bytes"),
                              signature=(publication / (filename + ".sig")).read_text().strip())
        if asset != expected_asset or not isinstance(asset["unpacked_bytes"], int) or not 0 < asset["unpacked_bytes"] <= 16 * 1024**3:
            raise ValueError("Update publication changed an original archive or executable contract")
        names.add(filename + ".sig")
        if program["component"] == "desktop-app":
            platforms["darwin-" + program["platform"]["arch"]] = dict(url=asset["url"], signature=asset["signature"])
    expected_tauri = dict(version=tag.removeprefix("v"), notes=manifest["notes"], platforms=platforms)
    if json.loads((publication / TAURI).read_text()) != expected_tauri or set(receipt["files"]) != names:
        raise ValueError("Tauri metadata differs from the signed component selection")
    return receipt


def create(root, output, tag, signer, program):
    from release_store import verify_candidate
    record = verify_candidate(root)
    expected_identity = identity(record, tag)
    if output.exists():
        receipt = verify_publication(output, record, tag)
        verify_native(root, output, program)
        return receipt
    previous = root / "update-publication"
    reuse = None
    if previous.exists():
        verify_publication(previous, record, record["tag"])
        verify_native(root, previous, program)
        reuse = json.loads((previous / MANIFEST).read_text())
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="khor-sign-publication-", dir=output.parent) as temporary:
        staging = Path(temporary) / "publication"
        staging.mkdir()
        assets, platforms = [], {}
        prefix = f"https://github.com/{os.environ['GITHUB_REPOSITORY']}/releases/download/{record['tag']}/"
        for name, filename, info, original in products(record):
            archive = root / name / filename
            if reuse:
                signature = next(asset["signature"] for asset in reuse["artifacts"] if asset["file"] == filename)
            else:
                copied = Path(temporary) / filename
                shutil.copyfile(archive, copied)
                signature = sign(copied, signer, record["tag"].removeprefix("v"))
                copied.unlink()
                Path(str(copied) + ".sig").unlink()
            (staging / (filename + ".sig")).write_text(signature + "\n")
            asset = dict(component=info["component"], platform=info["platform"], file=filename,
                         url=prefix + filename, bytes=original["bytes"], sha256=original["sha256"],
                         unpacked_bytes=unpacked_size(archive, info["component"]),
                         signature=signature, contract=info["contract"])
            assets.append(asset)
            if info["component"] == "desktop-app":
                platforms["darwin-" + info["platform"]["arch"]] = dict(url=asset["url"], signature=signature)
        notes = f"Khor {tag}"
        write_json(staging / MANIFEST, dict(schema=1, identity=expected_identity, built_from=record["tag"],
                                           notes=notes, artifacts=assets))
        sign(staging / MANIFEST, signer, tag.removeprefix("v"))
        write_json(staging / TAURI, dict(version=tag.removeprefix("v"), notes=notes, platforms=platforms))
        write_json(staging / "receipt.json", dict(schema=1, identity=expected_identity,
                                                 built_from=record["tag"], files=files(staging)))
        receipt = verify_publication(staging, record, tag)
        verify_native(root, staging, program)
        staging.rename(output)
        return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("candidate"))
    parser.add_argument("--output", type=Path, default=Path("signed-updates"))
    parser.add_argument("--tag", required=True)
    parser.add_argument("--signer", type=Path, required=True)
    parser.add_argument("--program", type=Path, required=True)
    args = parser.parse_args()
    if os.environ.get("GITHUB_ACTIONS") != "true" or os.environ.get("GITHUB_REF") != "refs/tags/" + args.tag:
        parser.error("Update signing requires its exact release tag on the trusted GitHub runner")
    create(args.root, args.output, args.tag, args.signer, args.program)


if __name__ == "__main__":
    main()
