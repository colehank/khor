#!/usr/bin/env python3
"""Build and run the production iOS menu adapter in an isolated simulator app."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import time


HERE = Path(__file__).resolve().parent
GUI = HERE.parent.parent
REPO = GUI.parent.parent
BUNDLE = "dev.khor.native-menu-probe"


def run(*args, **kwargs):
    print("+", " ".join(map(str, args)), flush=True)
    return subprocess.run(list(map(str, args)), check=True, **kwargs)


def output(*args):
    return subprocess.check_output(list(map(str, args)), text=True).strip()


def pinned(lock, name):
    for block in lock.split("[[package]]"):
        if re.search(rf'^name = "{re.escape(name)}"$', block, re.M):
            return re.search(r'^version = "([^"]+)"$', block, re.M)[1]
    raise RuntimeError(f"Cargo.lock does not pin {name}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True,
                        help="external artifact directory (empty, or previously created by this probe)")
    parser.add_argument("--simulator", default="EBBE393D-4ABB-4AFB-B360-44812E929058",
                        help="dedicated available iOS simulator UUID")
    parser.add_argument("--snapshot", type=Path,
                        help="existing production snapshot; otherwise native-menu-ui exports one")
    args = parser.parse_args()
    scratch = args.out.expanduser().resolve()
    if scratch == REPO or REPO in scratch.parents:
        raise RuntimeError("Probe artifacts must be outside the repository")
    marker = scratch / ".khor-native-menu-probe"
    if scratch.exists() and any(scratch.iterdir()) and not marker.exists():
        raise RuntimeError("Refusing to write into a nonempty directory not owned by this probe")
    scratch.mkdir(parents=True, exist_ok=True)
    marker.write_text("Isolated khor native-menu fixture\n")
    snapshot = scratch / "menu.html"
    if args.snapshot:
        source_snapshot = args.snapshot.expanduser().resolve()
        if source_snapshot != snapshot:
            shutil.copyfile(source_snapshot, snapshot)
    else:
        env = {**os.environ, "NATIVE_MENU_SNAPSHOT_OUT": str(snapshot), "KHOR_GATE_TMP": str(scratch)}
        run("npm", "run", "native-menu-ui", cwd=GUI, env=env)
    if not snapshot.is_file() or snapshot.stat().st_size == 0:
        raise RuntimeError("No production menu snapshot was exported")

    rust = scratch / "rust"
    module = rust / "src/menu_overlay"
    module.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(HERE / "lib.rs", rust / "src/lib.rs")
    # mod.rs keeps Rust's child-module resolution in this isolated directory.
    # Both links use unchanged production source; neither is a copied adapter.
    production = GUI / "src-tauri/src/menu_overlay.rs"
    ios = GUI / "src-tauri/src/menu_overlay/ios.rs"
    for destination, source in [(module / "mod.rs", production), (module / "ios.rs", ios)]:
        if destination.is_symlink():
            destination.unlink()
        elif destination.exists():
            raise RuntimeError(f"Refusing to replace unexpected file {destination}")
        destination.symlink_to(source)
    lock = (REPO / "Cargo.lock").read_text()
    shutil.copyfile(REPO / "Cargo.lock", rust / "Cargo.lock")
    versions = {name: pinned(lock, name) for name in
                ["serde", "serde_json", "objc2", "objc2-foundation", "objc2-ui-kit", "block2"]}
    (rust / "Cargo.toml").write_text(f'''[package]
name = "khor-menu-native-probe"
version = "0.0.0"
edition = "2024"
[lib]
crate-type = ["staticlib"]
[dependencies]
serde = {{ version = "={versions['serde']}", features = ["derive"] }}
serde_json = "={versions['serde_json']}"
objc2 = "={versions['objc2']}"
objc2-foundation = {{ version = "={versions['objc2-foundation']}", default-features = false, features = ["std", "NSString", "NSURL", "NSDictionary", "NSGeometry", "NSNotification", "NSOperation", "block2", "objc2-core-foundation"] }}
objc2-ui-kit = {{ version = "={versions['objc2-ui-kit']}", default-features = false, features = ["std", "block2", "UIView", "UIApplication", "UIResponder", "UIFeedbackGenerator", "UIImpactFeedbackGenerator", "UISelectionFeedbackGenerator", "UINotificationFeedbackGenerator"] }}
block2 = "={versions['block2']}"
[workspace]
''')
    hashes = {str(path): hashlib.sha256(path.read_bytes()).hexdigest()
              for path in [production, ios, HERE / "Probe.swift", HERE / "lib.rs", snapshot]}
    target = rust / "target"
    run("cargo", "build", "--offline", "--manifest-path", rust / "Cargo.toml",
        "--target", "aarch64-apple-ios-sim", "--target-dir", target)
    app = scratch / "KhorMenuProbe.app"
    app.mkdir(exist_ok=True)
    shutil.copyfile(snapshot, app / "menu.html")
    with (app / "Info.plist").open("wb") as file:
        plistlib.dump({"CFBundleIdentifier": BUNDLE, "CFBundleName": "KhorMenuProbe",
                      "CFBundleExecutable": "KhorMenuProbe", "CFBundlePackageType": "APPL",
                      "CFBundleVersion": "1", "CFBundleShortVersionString": "1.0",
                      "LSRequiresIPhoneOS": True, "UILaunchScreen": {},
                      "UISupportedInterfaceOrientations": ["UIInterfaceOrientationPortrait"],
                      "UIRequiredDeviceCapabilities": ["arm64"]}, file)
    sdk = output("xcrun", "--sdk", "iphonesimulator", "--show-sdk-path")
    run("xcrun", "--sdk", "iphonesimulator", "swiftc", "-parse-as-library",
        "-target", "arm64-apple-ios17.0-simulator", "-sdk", sdk,
        "-import-objc-header", HERE / "bridge.h", HERE / "Probe.swift",
        target / "aarch64-apple-ios-sim/debug/libkhor_menu_native_probe.a",
        "-framework", "UIKit", "-framework", "WebKit", "-framework", "Foundation",
        "-framework", "QuartzCore", "-framework", "CoreGraphics", "-lobjc",
        "-o", app / "KhorMenuProbe")
    run("codesign", "--force", "--sign", "-", app)

    devices = json.loads(output("xcrun", "simctl", "list", "devices", "available", "--json"))
    device = next((d for group in devices["devices"].values() for d in group
                   if d["udid"] == args.simulator), None)
    if device is None:
        raise RuntimeError("Requested simulator is not available")
    booted_here = device["state"] != "Booted"
    installed = False
    try:
        if booted_here:
            run("xcrun", "simctl", "boot", args.simulator)
        run("xcrun", "simctl", "bootstatus", args.simulator, "-b")
        run("xcrun", "simctl", "install", args.simulator, app)
        installed = True
        container = Path(output("xcrun", "simctl", "get_app_container", args.simulator, BUNDLE, "data"))
        result_path = container / "Documents/probe-results.json"
        result_path.unlink(missing_ok=True)
        run("xcrun", "simctl", "launch", "--terminate-running-process", args.simulator, BUNDLE)
        deadline = time.monotonic() + 120
        while not result_path.exists() and time.monotonic() < deadline:
            time.sleep(0.25)
        if not result_path.exists():
            raise RuntimeError("Native probe did not finish within 120 seconds")
        result = json.loads(result_path.read_text())
        shutil.copyfile(result_path, scratch / "probe-results.json")
        run("xcrun", "simctl", "io", args.simulator, "screenshot", scratch / "native-menu.png")
        changed = [path for path, digest in hashes.items()
                   if hashlib.sha256(Path(path).read_bytes()).hexdigest() != digest]
        (scratch / "probe-manifest.json").write_text(json.dumps({
            "simulator": args.simulator, "target": "aarch64-apple-ios-sim", "bundle": BUNDLE,
            "production_source_symlinked": True, "hashes": hashes,
            "sources_changed_during_probe": changed,
        }, indent=2))
        if changed:
            raise RuntimeError("Source changed during probe; results are not a reproducible source receipt")
        if not result["passed"]:
            failed = [c["name"] for c in result["checks"] if not c["passed"]]
            raise RuntimeError(f"Native probe failed: {failed}")
        print(f"PASS: {len(result['checks'])} actual native adapter checks. Evidence: {scratch}")
    finally:
        if installed:
            subprocess.run(["xcrun", "simctl", "terminate", args.simulator, BUNDLE], check=False)
            subprocess.run(["xcrun", "simctl", "uninstall", args.simulator, BUNDLE], check=False)
        if booted_here:
            subprocess.run(["xcrun", "simctl", "shutdown", args.simulator], check=False)


if __name__ == "__main__":
    main()
