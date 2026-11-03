"""Compile the native Mac icon and reject packages that silently lose it."""
import argparse
import json
from pathlib import Path
import plistlib
import subprocess


SOURCE = Path(__file__).resolve().parents[1] / "apps/gui/src-tauri/icons/Icon.icon"


def catalog_name(path):
    if not path.is_file():
        raise ValueError(f"Missing native app icon catalog: {path}")
    entries = json.loads(subprocess.check_output(["xcrun", "assetutil", "--info", str(path)], text=True))
    names = {entry["Name"] for entry in entries if entry.get("AssetType") == "Icon Image" and entry.get("Name")}
    layered = any(entry.get("AssetType", "").replace(" ", "") == "IconImageStack" for entry in entries)
    if not layered or len(names) != 1:
        raise ValueError("The app icon catalog must contain one named, layered app icon")
    return names.pop()


def compile_icon(output):
    output.mkdir(parents=True, exist_ok=True)
    subprocess.run(["xcrun", "actool", str(SOURCE), "--compile", str(output),
                    "--platform", "macosx", "--minimum-deployment-target", "26.0",
                    "--target-device", "mac", "--app-icon", SOURCE.stem,
                    "--include-all-app-icons", "--output-partial-info-plist", str(output / "Info.plist")], check=True)
    catalog_name(output / "Assets.car")


def verify_app(app):
    contents = app / "Contents"
    info = plistlib.loads((contents / "Info.plist").read_bytes())
    name = catalog_name(contents / "Resources/Assets.car")
    if info.get("CFBundleIconName") != name:
        raise ValueError("The app does not select its native icon catalog")
    fallback = info.get("CFBundleIconFile", "")
    if fallback and not Path(fallback).suffix:
        fallback += ".icns"
    if not fallback or not (contents / "Resources" / fallback).is_file():
        raise ValueError("Missing ICNS app icon for older macOS releases")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    command = parser.add_subparsers(dest="action", required=True)
    command.add_parser("compile").add_argument("--output", type=Path, required=True)
    command.add_parser("verify").add_argument("--app", type=Path, required=True)
    args = parser.parse_args()
    if args.action == "compile":
        compile_icon(args.output)
    else:
        verify_app(args.app)
