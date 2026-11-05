"""Exercise shared-directory installation with actual shell/filesystem operations."""
import gzip
import hashlib
import importlib.util
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
COMPONENTS = (ROOT / "scripts/install-components.sh").read_text()


class LauncherTest(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory(prefix="khor-launcher-")
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name).resolve()
        self.bin = self.root / "shared-home/bin"
        self.bin.mkdir(parents=True)
        self.environment = {k: v for k, v in os.environ.items() if not k.startswith("KHOR_")}
        self.environment["KHOR_RUNTIME_DIR"] = str(self.root / "runtime")
        self.installation = hashlib.sha256(os.fsencode(self.bin.resolve())).hexdigest()
        result = subprocess.run(["sh"], input=COMPONENTS + "\nkhor_launcher\n",
                                text=True, capture_output=True, check=True, timeout=5)
        (self.bin / "khor").write_text(result.stdout)
        (self.bin / "khor").chmod(0o755)

    def selection(self, asset):
        return self.root / "runtime/cli" / self.installation / asset / "current"

    def place(self, asset):
        current = self.selection(asset)
        program = current.parent / "versions/fixture/khor"
        program.parent.mkdir(parents=True)
        program.write_text('#!/bin/sh\nprintf "%s\\n" ' + shlex.quote(asset) +
                           ' "$KHOR_INSTALL_SOURCE" "$@"\n')
        program.chmod(0o755)
        current.symlink_to("versions/fixture/khor")

    def run_installed(self, system, arch, *args):
        tools = self.root / "tools"
        tools.mkdir(exist_ok=True)
        uname = tools / "uname"
        uname.write_text(f'#!/bin/sh\ncase "$1" in -s) echo {system} ;; -m) echo {arch} ;; esac\n')
        uname.chmod(0o755)
        return subprocess.run([str(self.bin / "khor"), *args], text=True, capture_output=True,
                              env=dict(self.environment, PATH=str(tools) + os.pathsep + os.environ["PATH"]), timeout=5)

    def test_shared_launcher_selects_each_node_local_platform_and_preserves_arguments(self):
        platforms = [("Linux", "x86_64", "khor-linux-x86_64"),
                     ("Linux", "aarch64", "khor-linux-aarch64"),
                     ("Darwin", "arm64", "khor-macos-arm64"),
                     ("Darwin", "x86_64", "khor-macos-x86_64")]
        for _, _, asset in platforms:
            self.place(asset)
        for system, arch, asset in platforms:
            result = self.run_installed(system, arch, "two words", "--version")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.splitlines(),
                             [asset, str(self.selection(asset)), "two words", "--version"])
        self.assertFalse((self.bin / ".khor").exists())

    def test_missing_platform_does_not_execute_another_architecture(self):
        self.place("khor-linux-x86_64")
        result = self.run_installed("Linux", "aarch64")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("install khor-linux-aarch64", result.stderr)

    def test_keeper_copy_uses_the_original_installation_and_node_selection(self):
        asset = {("Darwin", "arm64"): "khor-macos-arm64", ("Darwin", "x86_64"): "khor-macos-x86_64",
                 ("Linux", "x86_64"): "khor-linux-x86_64", ("Linux", "aarch64"): "khor-linux-aarch64"}[(os.uname().sysname, os.uname().machine)]
        self.place(asset)
        copy = self.root / "keeper-copy"
        shutil.copy2(self.bin / "khor", copy)
        result = subprocess.run([str(copy), "version"], text=True, capture_output=True, timeout=5,
                                env=dict(self.environment, KHOR_SHIELD_FROM=str(self.bin / "khor")))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), [asset, str(self.selection(asset)), "version"])


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
                       input="""#include <stdio.h>
#include <string.h>
int main(int n, char **v) {
  if (n > 1 && strcmp(v[1], "_install_cli") == 0) {
    if (n != 7) return 2;
    char path[8192]; snprintf(path, sizeof(path), "%s/native-install.txt", v[2]);
    FILE *out = fopen(path, "w"); if (!out) return 3;
    fprintf(out, "%s\\n", v[3]);
    for (int i = 4; i < 7; i++) {
      FILE *in = fopen(v[i], "rb"); if (!in) return 4;
      int count = 0; while (fgetc(in) != EOF) count++;
      fclose(in); fprintf(out, "%d\\n", count);
    }
    fclose(out); return 0;
  }
  puts("1.2.3"); return 0;
}
""",
                       text=True, check=True, capture_output=True, timeout=30)
        self.archive = self.root / "archive.gz"
        self.archive.write_bytes(gzip.compress(self.program.read_bytes(), mtime=0))
        self.digest = hashlib.sha256(self.archive.read_bytes()).hexdigest()
        self.manifest = self.root / "manifest.json"
        self.manifest.write_bytes(b"signed release fixture")
        self.signature = self.root / "manifest.sig"
        self.signature.write_bytes(b"signature fixture")

    def install(self, version="v1.2.3", mirror="", preplaced=True):
        environment = {key: value for key, value in os.environ.items() if not key.startswith("KHOR_")}
        if preplaced:
            environment.update(KHOR_ASSET_FILE=str(self.archive), KHOR_ASSET_SHA256=self.digest,
                               KHOR_RELEASE_FILE=str(self.manifest), KHOR_RELEASE_SIGNATURE_FILE=str(self.signature))
        return subprocess.run(["sh", "-s", "--", version, mirror, str(self.bin)],
                              input=COMPONENTS + '\nkhor_install_release owner/repo "$1" "$2" khor-linux-x86_64 "$3"\n',
                              text=True, capture_output=True, env=environment, timeout=30)

    def test_preplaced_install_passes_original_archive_and_signed_metadata_to_native_verification(self):
        archive_bytes = self.archive.stat().st_size
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(self.archive.exists())
        self.assertEqual((self.bin / "native-install.txt").read_text().splitlines(),
                         ["v1.2.3", str(self.manifest.stat().st_size), str(self.signature.stat().st_size), str(archive_bytes)])
        self.assertEqual(self.previous.read_bytes(), b"previous installation",
                         "the Shell dispatcher delegates publication entirely to the native verifier")
        self.assertEqual(list(self.bin.glob(".khor.*.*")), [])

    def test_rc_install_preserves_the_exact_publication_tag_for_native_verification(self):
        result = self.install(version="v1.2.3-rc.2")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.bin / "native-install.txt").read_text().splitlines()[0], "v1.2.3-rc.2")

    def test_rc_tag_still_rejects_a_different_application_version(self):
        result = self.install(version="v1.2.4-rc.2")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("different version", result.stderr)
        self.assertEqual(self.previous.read_bytes(), b"previous installation")

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
                    name + ".sha256": (self.digest + "  khor-linux-x86_64.gz\n").encode(),
                    "/releases/v1.2.3/khor-release.json": self.manifest.read_bytes(),
                    "/releases/v1.2.3/khor-release.json.sig": self.signature.read_bytes()}
        with fixture.loopback(contents) as mirror:
            result = self.install(mirror=mirror, preplaced=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.bin / "native-install.txt").read_text().splitlines()[0], "v1.2.3")


if __name__ == "__main__":
    unittest.main()
