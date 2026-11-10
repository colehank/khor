"""Build a nonrelocatable Khor.pkg from the signed App and its bundled CLI."""
import base64
from contextlib import contextmanager
import os
from pathlib import Path
import plistlib
import re
import secrets
import subprocess
import tempfile

from apple_release import required, run

IDENTIFIER = "io.github.colehank.khor.installer"
CLI = "/Applications/Khor.app/Contents/MacOS/khor"


@contextmanager
def signing():
    identity = required("MAC_INSTALLER_SIGNING_IDENTITY")
    if not re.fullmatch(r"Developer ID Installer: .+ \([A-Z0-9]{10}\)", identity):
        raise ValueError("Use a Developer ID Installer signing identity")
    with tempfile.TemporaryDirectory(prefix="khor-pkg-signing-") as tmp:
        directory = Path(tmp)
        certificate = directory / "installer.p12"
        certificate.write_bytes(base64.b64decode(required("MAC_INSTALLER_CERTIFICATE_P12"), validate=True))
        certificate.chmod(0o600)
        chain = str(directory / "installer.keychain-db")
        password = secrets.token_hex(32)
        try:
            run(["security", "create-keychain", "-p", password, chain])
            run(["security", "unlock-keychain", "-p", password, chain])
            run(["security", "import", str(certificate), "-k", chain,
                 "-P", required("MAC_INSTALLER_CERTIFICATE_PASSWORD"), "-T", "/usr/bin/pkgbuild"])
            run(["security", "set-key-partition-list", "-S", "apple-tool:,apple:", "-s",
                 "-k", password, chain], stdout=subprocess.DEVNULL)
            yield ["--sign", identity, "--keychain", chain, "--timestamp"]
        finally:
            subprocess.run(["security", "delete-keychain", chain], capture_output=True)


def build(app, output, version, signed):
    with tempfile.TemporaryDirectory(prefix="khor-pkg-") as tmp:
        work = Path(tmp)
        payload, scripts = work / "root", work / "scripts"
        (payload / "Applications").mkdir(parents=True)
        scripts.mkdir()
        run(["ditto", str(app), str(payload / "Applications/Khor.app")])
        postinstall = scripts / "postinstall"
        postinstall.write_text(
            "#!/bin/sh\n"
            'volume=${3:?Missing target volume}\n'
            'bin="${volume%/}/usr/local/bin"\n'
            'link="$bin/khor"\n'
            'if [ -e "$link" ] || [ -L "$link" ]; then\n'
            '  if [ "$(/usr/bin/readlink "$link")" != "' + CLI + '" ]; then\n'
            '    echo "Preserved existing $link; the bundled CLI is at ' + CLI + '"\n'
            '  fi\n'
            '  exit 0\n'
            'fi\n'
            '/bin/mkdir -p "$bin" || exit 1\n'
            '/bin/ln -sh "' + CLI + '" "$link"\n'
        )
        postinstall.chmod(0o755)
        components = work / "components.plist"
        components.write_bytes(plistlib.dumps([{
            "RootRelativeBundlePath": "Applications/Khor.app",
            "BundleIsRelocatable": False,
            "BundleHasStrictIdentifier": True,
            "BundleIsVersionChecked": True,
            "BundleOverwriteAction": "upgrade",
        }]))
        command = ["/usr/bin/pkgbuild", "--root", str(payload), "--component-plist", str(components),
                   "--scripts", str(scripts), "--install-location", "/", "--identifier", IDENTIFIER,
                   "--version", version]
        if signed:
            with signing() as options:
                run([*command, *options, str(output)])
            run(["/usr/sbin/pkgutil", "--check-signature", str(output)])
        else:
            run([*command, str(output)])


def extract(package, destination):
    run(["/usr/sbin/pkgutil", "--expand-full", str(package), str(destination)])
    app = destination / "Payload/Applications/Khor.app"
    if not app.is_dir() or not (app / "Contents/MacOS/khor").is_file():
        raise ValueError("Installer payload has no Khor.app with its CLI")
    return app
