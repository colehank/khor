"""Build the desktop installation package and its bundled CLI from one source."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess

from release_manifest import create, executable_info

GUI = Path("apps/gui")
TARGETS = {
    "x86_64-unknown-linux-gnu": ("deb", "khor-linux-x86_64", ""),
    "aarch64-unknown-linux-gnu": ("deb", "khor-linux-aarch64", ""),
    "x86_64-pc-windows-msvc": ("msi", "khor-windows-x86_64", ".exe"),
}


def msi_installed_bytes(package):
    """Read the payload's installed size directly from the MSI File table."""
    from ctypes import POINTER, WinDLL, byref
    from ctypes.wintypes import LPCWSTR, UINT

    api = WinDLL("msi.dll")
    api.MsiOpenDatabaseW.argtypes = [LPCWSTR, LPCWSTR, POINTER(UINT)]
    api.MsiDatabaseOpenViewW.argtypes = [UINT, LPCWSTR, POINTER(UINT)]
    api.MsiViewExecute.argtypes = [UINT, UINT]
    api.MsiViewFetch.argtypes = [UINT, POINTER(UINT)]
    api.MsiRecordGetInteger.argtypes = [UINT, UINT]
    api.MsiCloseHandle.argtypes = [UINT]
    db, view, record = UINT(), UINT(), UINT()

    def check(status):
        if status != 0:
            raise RuntimeError(f"Cannot read the MSI File table (Windows Installer error {status})")

    try:
        check(api.MsiOpenDatabaseW(str(package.resolve()), None, byref(db)))
        check(api.MsiDatabaseOpenViewW(db, "SELECT `FileSize` FROM `File`", byref(view)))
        check(api.MsiViewExecute(view, 0))
        total = 0
        while (status := api.MsiViewFetch(view, byref(record))) != 259:  # ERROR_NO_MORE_ITEMS
            check(status)
            try:
                size = api.MsiRecordGetInteger(record, 1)
                if size < 0:
                    raise ValueError("MSI contains an invalid installed file size")
                total += size
            finally:
                api.MsiCloseHandle(record)
        if not 0 < total <= 16 * 1024**3:
            raise ValueError("MSI installed payload size is invalid")
        return total
    finally:
        if view.value:
            api.MsiCloseHandle(view)
        if db.value:
            api.MsiCloseHandle(db)


def build(target, output):
    bundle, name, suffix = TARGETS[target]
    programs = Path("target") / target / "release"
    subprocess.run(["cargo", "build", "--locked", "--release", "--target", target,
                    "-p", "khor-cli", "--bin", "khor"], check=True)
    binaries = GUI / "src-tauri/binaries"
    binaries.mkdir(exist_ok=True)
    shutil.copy2(programs / ("khor" + suffix), binaries / ("khor-" + target + suffix))
    config = {
        "build": {"beforeBuildCommand": ""},
        "bundle": {
            "active": True,
            "icon": ["icons/icon.ico", "icons/icon.png"],
            "externalBin": ["binaries/khor"],
        },
    }
    if bundle == "deb":
        config["bundle"]["linux"] = {"deb": {
            "files": {"/usr/share/khor/cli-owner.json": "packaging/cli-owner.json"},
            "preInstallScript": "packaging/debian-preinstall.sh",
        }}
    else:
        config["bundle"]["resources"] = {"packaging/cli-owner.json": "cli-owner.json"}
        config["bundle"]["windows"] = {"wix": {
            # WiX emits one MSI per language. The updater and release manifest
            # expect one MSI per target; Khor localizes its own UI at runtime.
            "language": "en-US",
            "fragmentPaths": ["packaging/cli-path.wxs"],
            "componentRefs": ["KhorCliPath"],
        }}
    subprocess.run(["npm.cmd" if os.name == "nt" else "npm", "run", "tauri", "--",
                    "build", "--ci", "--target", target, "--config", json.dumps(config),
                    "--bundles", bundle, "--", "--locked"], cwd=GUI, check=True)
    app = programs / ("khor-gui" + suffix)
    cli = programs / ("khor" + suffix)
    app_info, cli_info = executable_info(app), executable_info(cli)
    if (app_info["component"] != "desktop-app" or cli_info["component"] != "cli"
            or any(app_info[key] != cli_info[key] for key in ("build", "platform", "contract"))):
        raise ValueError("The application and its CLI must share their build and runtime contract")
    packages = list((programs / "bundle" / bundle).glob("*." + bundle))
    if len(packages) != 1:
        raise ValueError("Expected one desktop installation package")
    output.mkdir(parents=True, exist_ok=True)
    shutil.copy2(packages[0], output / (name + "." + bundle))
    return app


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--frontend", type=Path)
    args = parser.parse_args()
    if os.environ.get("GITHUB_ACTIONS") != "true":
        parser.error("Desktop package builds run on the build runner")
    app = build(args.target, args.output)
    if args.frontend is not None:
        agent = Path(os.environ["KHOR_AGENT_BUNDLE"])
        bundle, name, _ = TARGETS[args.target]
        unpacked_bytes = msi_installed_bytes(args.output / (name + ".msi")) if bundle == "msi" else None
        create(args.output, name + "-app", args.frontend, app, agent, executable_info(app),
               msi_unpacked_bytes=unpacked_bytes)
