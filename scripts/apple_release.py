"""Apple CI build and export stages. Never submits App Store review or acceptance."""
import argparse
import base64
from contextlib import contextmanager
import json
import os
from pathlib import Path
import plistlib
import re
import secrets
import subprocess
import tarfile
import tempfile
from release_manifest import version

GUI = Path("apps/gui")
APPLE = GUI / "src-tauri/gen/apple"
ARCHIVE = APPLE / "build/khor-gui_iOS.xcarchive"


def required(name):
    value = os.environ.get(name)
    if not value:
        raise ValueError(f"Configure the GitHub environment secret {name}")
    return value


def run(args, **kwargs):
    # Credential commands are deliberately not echoed, including on failure.
    result = subprocess.run(args, **kwargs)
    if result.returncode:
        raise RuntimeError(f"{Path(args[0]).name} failed ({result.returncode})")


def build_number():
    number = int(required("GITHUB_RUN_NUMBER")) + 1000
    attempt = int(required("GITHUB_RUN_ATTEMPT"))
    if not (1000 < number <= 9999 and 1 <= attempt <= 99):
        raise ValueError("CI build number exceeds Apple's component limits")
    return f"{number}.{attempt}"


@contextmanager
def api_key():
    with tempfile.TemporaryDirectory() as temporary:
        path = Path(temporary) / "AuthKey.p8"
        path.write_text(required("APPLE_API_KEY_CONTENT"))
        path.chmod(0o600)
        yield path


@contextmanager
def ios_signing():
    required("APPLE_DEVELOPMENT_TEAM")
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        certificate = root / "distribution.p12"
        certificate.write_bytes(base64.b64decode(required("IOS_CERTIFICATE_P12"), validate=True))
        profile = root / "distribution.mobileprovision"
        profile.write_bytes(base64.b64decode(required("IOS_PROFILE"), validate=True))
        decoded = subprocess.check_output(["security", "cms", "-D", "-i", str(profile)])
        info = plistlib.loads(decoded)
        team = required("APPLE_DEVELOPMENT_TEAM")
        if not re.fullmatch(r"[A-Z0-9]{10}", team):
            raise ValueError("Invalid Apple team identifier")
        app = info["Entitlements"]["application-identifier"]
        if info["TeamIdentifier"] != [team] or app != f"{team}.io.github.colehank.khor" or info["Entitlements"].get("get-task-allow"):
            raise ValueError("iOS profile must be this app's distribution profile")
        profile_id = info["UUID"]
        if not re.fullmatch(r"[A-Fa-f0-9-]+", profile_id):
            raise ValueError("Invalid provisioning profile UUID")
        installed = Path.home() / "Library/MobileDevice/Provisioning Profiles" / f"{profile_id}.mobileprovision"
        installed.parent.mkdir(parents=True, exist_ok=True)
        previous = installed.read_bytes() if installed.exists() else None
        chain = root / "signing.keychain-db"
        password = secrets.token_hex(32)
        old_chains = subprocess.check_output(["security", "list-keychains", "-d", "user"], text=True)
        old_chains = re.findall(r'"([^"]+)"', old_chains)
        try:
            run(["security", "create-keychain", "-p", password, str(chain)])
            run(["security", "set-keychain-settings", "-lut", "21600", str(chain)])
            run(["security", "unlock-keychain", "-p", password, str(chain)])
            run(["security", "import", str(certificate), "-P", required("IOS_CERTIFICATE_PASSWORD"), "-k", str(chain), "-T", "/usr/bin/codesign"])
            run(["security", "set-key-partition-list", "-S", "apple-tool:,apple:", "-s", "-k", password, str(chain)], stdout=subprocess.DEVNULL)
            run(["security", "list-keychains", "-d", "user", "-s", str(chain), *old_chains])
            installed.write_bytes(profile.read_bytes())
            yield profile_id
        finally:
            run(["security", "list-keychains", "-d", "user", "-s", *old_chains])
            subprocess.run(["security", "delete-keychain", str(chain)], capture_output=True)
            if previous is None:
                installed.unlink(missing_ok=True)
            else:
                installed.write_bytes(previous)


def archive_ios(signed):
    config = {"build": {"beforeBuildCommand": ""}, "bundle": {"iOS": {"bundleVersion": build_number()}}}
    command = ["npm", "run", "tauri", "--", "ios", "build", "--ci", "--target", "aarch64", "--archive-only", "--config", json.dumps(config)]
    if not signed:
        run([*command, "--no-sign"], cwd=GUI)
        return
    project = APPLE / "khor-gui.xcodeproj/project.pbxproj"
    original = project.read_bytes()
    try:
        with ios_signing() as profile:
            source, count = re.subn(r'CODE_SIGN_IDENTITY = "iPhone Developer";',
                f'CODE_SIGN_IDENTITY = "Apple Distribution";\n\t\t\t\tCODE_SIGN_STYLE = Manual;\n\t\t\t\tDEVELOPMENT_TEAM = {required("APPLE_DEVELOPMENT_TEAM")};\n\t\t\t\tPROVISIONING_PROFILE_SPECIFIER = "{profile}";', original.decode())
            if count != 2:
                raise ValueError("Xcode signing settings changed; review the CI adapter")
            project.write_text(source)
            run(command, cwd=GUI)
    finally:
        project.write_bytes(original)


def mac_build(target, signed):
    command = ["npm", "run", "tauri", "--", "build", "--ci", "--target", target,
               "--config", "src-tauri/tauri.ci.json", "--bundles", "app", "--", "--locked"]
    if not signed:
        command.insert(command.index("--bundles"), "--no-sign")
        run(command, cwd=GUI)
        return
    for name in ["APPLE_CERTIFICATE", "APPLE_CERTIFICATE_PASSWORD", "APPLE_SIGNING_IDENTITY", "APPLE_API_KEY", "APPLE_API_ISSUER"]:
        required(name)
    with api_key() as path:
        run(command, cwd=GUI, env={**os.environ, "APPLE_API_KEY_PATH": str(path)})
    app = Path("target") / target / "release/bundle/macos/Khor.app"
    run(["codesign", "--verify", "--deep", "--strict", str(app)])
    run(["xcrun", "stapler", "validate", str(app)])


def package_ios():
    app = ARCHIVE / "Products/Applications/Khor.app"
    info = plistlib.loads((app / "Info.plist").read_bytes())
    if info["CFBundleIdentifier"] != "io.github.colehank.khor" or info["CFBundleShortVersionString"] != version():
        raise ValueError("Archive identity/version differs from the candidate")
    if info["CFBundleVersion"] != build_number():
        raise ValueError("Archive build number differs from this run")
    signed = (app / "embedded.mobileprovision").is_file()
    if signed:
        run(["codesign", "--verify", "--deep", "--strict", str(app)])
    output = Path("ios")
    output.mkdir(exist_ok=True)
    run(["tar", "-czf", str(output / "Khor.xcarchive.tar.gz"), "-C", str(ARCHIVE.parent), ARCHIVE.name])
    (output / "build.json").write_text(json.dumps({"version": version(), "build": info["CFBundleVersion"], "signed": signed}, indent=2) + "\n")


def upload_ios(root):
    build = json.loads((root / "build.json").read_text())
    if not build["signed"]:
        raise ValueError("This candidate is unsigned. Run a signed candidate first.")
    with tempfile.TemporaryDirectory() as temporary, ios_signing() as profile, api_key() as key:
        with tarfile.open(root / "Khor.xcarchive.tar.gz") as archive:
            archive.extractall(temporary, filter="data")
        options = Path(temporary) / "ExportOptions.plist"
        options.write_bytes(plistlib.dumps({"method": "app-store-connect", "destination": "upload",
            "teamID": required("APPLE_DEVELOPMENT_TEAM"), "signingStyle": "manual",
            "signingCertificate": "Apple Distribution", "manageAppVersionAndBuildNumber": False,
            "uploadSymbols": True, "testFlightInternalTestingOnly": False,
            "provisioningProfiles": {"io.github.colehank.khor": profile}}))
        run(["xcodebuild", "-exportArchive", "-archivePath", str(Path(temporary) / ARCHIVE.name),
             "-exportOptionsPlist", str(options), "-exportPath", str(Path(temporary) / "export"),
             "-authenticationKeyPath", str(key), "-authenticationKeyID", required("APPLE_API_KEY"),
             "-authenticationKeyIssuerID", required("APPLE_API_ISSUER")])
    print(f"Uploaded {build['version']} ({build['build']}); Apple processing and testing are separate.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["mac-build", "ios-build", "package-ios", "upload-ios"])
    parser.add_argument("--target")
    parser.add_argument("--signed", choices=["true", "false"], default="false")
    parser.add_argument("--root", type=Path, default=Path("ios"))
    args = parser.parse_args()
    if os.environ.get("GITHUB_ACTIONS") != "true":
        parser.error("Release stages run on GitHub; local ship is disabled")
    if args.action == "mac-build":
        if args.target not in {"aarch64-apple-darwin", "x86_64-apple-darwin"}:
            parser.error("Choose a supported Mac target")
        mac_build(args.target, args.signed == "true")
    elif args.action == "ios-build":
        archive_ios(args.signed == "true")
    elif args.action == "package-ios":
        package_ios()
    else:
        upload_ios(args.root)


if __name__ == "__main__":
    main()
