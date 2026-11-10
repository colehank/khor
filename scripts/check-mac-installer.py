#!/usr/bin/env python3
"""Exercise the actual package tools with an isolated, unsigned fixture."""
import plistlib
from pathlib import Path
import subprocess
import tempfile

import mac_installer


def main():
    with tempfile.TemporaryDirectory(prefix="khor-installer-check-") as tmp:
        work = Path(tmp)
        app = work / "Khor.app"
        contents = app / "Contents"
        programs = contents / "MacOS"
        programs.mkdir(parents=True)
        (contents / "Info.plist").write_bytes(plistlib.dumps({
            "CFBundleIdentifier": "io.github.colehank.khor",
            "CFBundleExecutable": "khor-gui",
            "CFBundlePackageType": "APPL",
            "CFBundleShortVersionString": "0.2.0",
            "CFBundleVersion": "1",
        }))
        for name in ["khor", "khor-gui"]:
            program = programs / name
            program.write_text("#!/bin/sh\nexit 0\n")
            program.chmod(0o755)
        package = work / "Khor.pkg"
        mac_installer.build(app, package, "0.2.0", signed=False)
        expanded = work / "expanded"
        installed = mac_installer.extract(package, expanded)
        assert (installed / "Contents/MacOS/khor").read_bytes() == (programs / "khor").read_bytes()
        scripts = list((expanded / "Scripts").rglob("postinstall"))
        assert len(scripts) == 1, scripts
        volume = work / "isolated volume"
        volume.mkdir()

        def link_cli():
            subprocess.run(["/bin/sh", str(scripts[0]), str(package), "/", str(volume)], check=True)

        link_cli()
        path = volume / "usr/local/bin/khor"
        assert path.is_symlink() and str(path.readlink()) == mac_installer.CLI
        link_cli()  # Existing App launcher is idempotent.
        assert str(path.readlink()) == mac_installer.CLI
        path.unlink()
        path.write_text("Independent CLI installation")
        link_cli()
        assert path.read_text() == "Independent CLI installation"
        print("Mac PKG payload, CLI link and independent installation preservation passed")


if __name__ == "__main__":
    main()
