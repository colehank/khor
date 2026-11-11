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
        create(args.output, TARGETS[args.target][1] + "-app", args.frontend,
               app, agent, executable_info(app))
