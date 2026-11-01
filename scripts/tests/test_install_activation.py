"""Exercise shared-directory installation with actual shell/filesystem operations."""
from concurrent.futures import ThreadPoolExecutor
import gzip
import hashlib
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
COMPONENTS = (ROOT / "scripts/install-components.sh").read_text()


class InstallActivationTest(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory(prefix="khor-activation-")
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.bin = self.root / "shared home" / "bin"
        self.bin.mkdir(parents=True)

    def program(self, name):
        program = self.root / name
        program.write_text('#!/bin/sh\nprintf "%s\\n" ' + name + ' "$KHOR_INSTALL_SOURCE" "$@"\n')
        program.chmod(0o755)
        return program

    def activate(self, program, asset="khor-linux-x86_64", version="v1.2.3"):
        return subprocess.run(
            ["sh", "-s", "--", str(program), str(self.bin), asset, version],
            input=COMPONENTS + '\nkhor_activate_release "$1" "$2" "$3" "$4"\n',
            text=True, capture_output=True, timeout=10,
        )

    def run_installed(self, system, arch, *args):
        tools = self.root / "tools"
        tools.mkdir(exist_ok=True)
        uname = tools / "uname"
        uname.write_text(f'#!/bin/sh\ncase "$1" in -s) echo {system} ;; -m) echo {arch} ;; esac\n')
        uname.chmod(0o755)
        return subprocess.run([str(self.bin / "khor"), *args], text=True, capture_output=True,
                              env=dict(os.environ, PATH=str(tools) + os.pathsep + os.environ["PATH"]), timeout=5)

    def test_shared_home_dispatches_each_platform_and_preserves_arguments(self):
        platforms = [("Linux", "x86_64", "khor-linux-x86_64"),
                     ("Linux", "aarch64", "khor-linux-aarch64"),
                     ("Darwin", "arm64", "khor-macos-arm64"),
                     ("Darwin", "x86_64", "khor-macos-x86_64")]
        for system, arch, asset in platforms:
            result = self.activate(self.program(asset), asset)
            self.assertEqual(result.returncode, 0, result.stderr)
        for system, arch, asset in platforms:
            result = self.run_installed(system, arch, "two words", "--version")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.splitlines(),
                             [asset, str(self.bin.resolve() / ".khor" / asset / "current"), "two words", "--version"])

    def test_concurrent_installs_never_rewrite_a_published_executable(self):
        old, new = self.program("old"), self.program("new")
        self.assertEqual(self.activate(old).returncode, 0)
        pointer = self.bin / ".khor/khor-linux-x86_64/current"
        previous = pointer.resolve()
        inode = previous.stat().st_ino
        with ThreadPoolExecutor(max_workers=8) as pool:
            results = list(pool.map(lambda _: self.activate(new, version="v1.2.4"), range(16)))
        for result in results:
            self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotEqual(previous, pointer.resolve())
        self.assertEqual(previous.stat().st_ino, inode)
        self.assertEqual(previous.read_bytes(), old.read_bytes())
        self.assertEqual(pointer.read_bytes(), new.read_bytes())
        self.assertEqual(len(list(pointer.parent.glob("versions/*/khor"))), 2)
        self.assertEqual(list(self.bin.glob(".khor.activate.*")), [])

    def test_collision_or_invalid_identity_preserves_active_version(self):
        old, new = self.program("old"), self.program("new")
        self.assertEqual(self.activate(old).returncode, 0)
        pointer = self.bin / ".khor/khor-linux-x86_64/current"
        previous = pointer.resolve()
        digest = hashlib.sha256(new.read_bytes()).hexdigest()
        collision = pointer.parent / "versions" / f"v1.2.4-{digest}" / "khor"
        collision.parent.mkdir(parents=True)
        collision.write_text("interrupted or altered artifact")
        for asset, version in [("khor-linux-x86_64", "v1.2.4"), ("../../escape", "v1.2.4"),
                               ("khor-linux-x86_64", "latest")]:
            result = self.activate(new, asset, version)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(pointer.resolve(), previous)
            self.assertEqual(pointer.read_bytes(), old.read_bytes())

    def test_missing_platform_reports_installation_without_using_another_architecture(self):
        self.assertEqual(self.activate(self.program("x86")).returncode, 0)
        result = self.run_installed("Linux", "aarch64")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("install khor-linux-aarch64", result.stderr)

    def test_keeper_copy_can_transition_from_the_direct_binary_install(self):
        asset = {("Darwin", "arm64"): "khor-macos-arm64", ("Darwin", "x86_64"): "khor-macos-x86_64",
                 ("Linux", "x86_64"): "khor-linux-x86_64", ("Linux", "aarch64"): "khor-linux-aarch64"}[(os.uname().sysname, os.uname().machine)]
        self.assertEqual(self.activate(self.program("installed"), asset).returncode, 0)
        copy = self.root / "keeper-copy"
        shutil.copy2(self.bin / "khor", copy)
        result = subprocess.run([str(copy), "version"], text=True, capture_output=True, timeout=5,
                                env=dict(os.environ, KHOR_SHIELD_FROM=str(self.bin / "khor")))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines()[0], "installed")
        self.assertEqual(result.stdout.splitlines()[1], str(self.bin.resolve() / ".khor" / asset / "current"))


class ServeReportTest(unittest.TestCase):
    def test_shared_user_unit_resolves_home_at_start_and_tracks_the_keeper(self):
        result = subprocess.run(["sh", "-s"], input=COMPONENTS + '\nkhor_systemd_unit user\n',
                                text=True, capture_output=True, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('ExecStart="%h/.local/bin/khor" serve', result.stdout)
        self.assertIn('Type=exec', result.stdout)
        self.assertIn('KillMode=process', result.stdout)
        self.assertNotIn('PIDFile=', result.stdout)
        self.assertNotIn(str(Path.home()), result.stdout)

    def test_healthy_serve_with_no_new_log_exits_successfully(self):
        source = (ROOT / "scripts/onset.sh").read_text()
        script = source.split('cat > "$STAGE/serve.sh" <<\'REMOTE\'\n', 1)[1].split('\nREMOTE\n', 1)[0]
        with tempfile.TemporaryDirectory(prefix="khor-serve-report-") as directory:
            root = Path(directory)
            (root / ".khor").mkdir()
            # Only the status/report logic runs: no real resident or kill.
            prelude = '''khor_root() { printf '%s\\n' "$1"; }
'''.replace('"$1"', "'" + directory + "'") + '''
khor_stop_serve() { :; }
env() { :; }
sleep() { :; }
kill() { :; }
'''
            result = subprocess.run(["sh", "-s"], input=prelude + script, text=True, capture_output=True, timeout=5)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn("serve is up", result.stdout)


class InstallReleaseTest(unittest.TestCase):
    def setUp(self):
        compiler = shutil.which("cc")
        if not compiler:
            self.skipTest("C compiler needed for a tiny runnable archive fixture")
        scratch = tempfile.TemporaryDirectory(prefix="khor-install-release-")
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.previous = self.bin / "khor"
        self.previous.write_bytes(b"previous installation")
        self.program = self.root / "program"
        subprocess.run([compiler, "-x", "c", "-", "-o", str(self.program)],
                       input='#include <stdio.h>\nint main(void) { puts("1.2.3"); return 0; }\n',
                       text=True, check=True, capture_output=True, timeout=30)
        self.archive = self.root / "archive.gz"
        self.archive.write_bytes(gzip.compress(self.program.read_bytes(), mtime=0))
        self.digest = hashlib.sha256(self.archive.read_bytes()).hexdigest()

    def install(self, version="v1.2.3", mirror="", preplaced=True):
        environment = {key: value for key, value in os.environ.items() if not key.startswith("KHOR_")}
        if preplaced:
            environment.update(KHOR_ASSET_FILE=str(self.archive), KHOR_ASSET_SHA256=self.digest)
        return subprocess.run(["sh", "-s", "--", version, mirror, str(self.bin)],
                              input=COMPONENTS + '\nkhor_install_release owner/repo "$1" "$2" khor-linux-x86_64 "$3"\n',
                              text=True, capture_output=True, env=environment, timeout=30)

    def test_preplaced_install_activates_only_after_preflight(self):
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(self.archive.exists())
        self.assertEqual((self.bin / ".khor/khor-linux-x86_64/current").read_bytes(), self.program.read_bytes())
        self.assertTrue(self.previous.read_bytes().startswith(b"#!/bin/sh"))
        self.assertEqual(list(self.bin.glob(".khor.*.*")), [])

    def test_version_mismatch_retains_old_install_and_preplaced_archive(self):
        result = self.install(version="v1.2.4")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("different version", result.stderr)
        self.assertEqual(self.previous.read_bytes(), b"previous installation")
        self.assertTrue(self.archive.exists())
        self.assertFalse((self.bin / ".khor").exists())

    def test_download_uses_the_same_activation_path(self):
        spec = importlib.util.spec_from_file_location("release_install_fixture", ROOT / "scripts/check-release-install.py")
        fixture = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(fixture)
        name = "/releases/v1.2.3/khor-linux-x86_64.gz"
        contents = {name: self.archive.read_bytes(),
                    name + ".sha256": (self.digest + "  khor-linux-x86_64.gz\n").encode()}
        with fixture.loopback(contents) as mirror:
            result = self.install(mirror=mirror, preplaced=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.bin / ".khor/khor-linux-x86_64/current").read_bytes(), self.program.read_bytes())


if __name__ == "__main__":
    unittest.main()
