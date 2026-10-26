"""Exercise the real mirror writer against scratch files and a fake network."""
import base64
import gzip
import hashlib
import json
import os
import shutil
import signal
import stat
from pathlib import Path
import subprocess
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]
ASSETS = ("khor-linux-x86_64", "khor-linux-aarch64", "khor-macos-arm64", "khor-macos-x86_64")
CURL = r'''#!/usr/bin/env python3
import base64, json, os, sys
from pathlib import Path
args = sys.argv[1:]
url = next((arg for arg in args if arg.startswith('https://')), '')
with open(os.environ['MIRROR_REQUESTS'], 'a') as log: log.write(url + '\n')
routes = json.loads(Path(os.environ['MIRROR_ROUTES']).read_text())
tag = os.environ.get('MIRROR_LATEST_TAG', 'v1.2.3')
if url.endswith('/releases/latest') and any('I' in arg for arg in args if arg.startswith('-')):
    sys.stdout.write(f'HTTP/1.1 302 Found\r\nLocation: /colehank/khor/releases/tag/{tag}\r\n\r\n'); sys.exit(0)
if url.endswith('/releases/tag/' + tag):
    sys.stdout.write('HTTP/1.1 200 OK\r\n\r\n'); sys.exit(0)
if url not in routes: sys.exit(22)
if url.endswith('/khor-linux-x86_64.gz') and os.environ.get('MIRROR_PAUSE'):
    import time
    pause = Path(os.environ['MIRROR_PAUSE'])
    pause.with_suffix('.ready').touch()
    deadline = time.monotonic()+15
    while not pause.exists():
        if time.monotonic()>deadline: sys.exit(28)
        time.sleep(.02)
payload = base64.b64decode(routes[url])
if '-o' in args: Path(args[args.index('-o')+1]).write_bytes(payload)
else: sys.stdout.buffer.write(payload)
'''


class ReleaseMirrorTest(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="khor-release-mirror-")
        self.addCleanup(self.scratch.cleanup)
        self.base = Path(self.scratch.name)
        self.mirror = self.base / "mirror"
        self.mirror.mkdir()
        (self.mirror / "VERSION").write_text("v1.2.2\n")
        self.tools = self.base / "tools"
        self.tools.mkdir()
        curl = self.tools / "curl"
        curl.write_text(CURL)
        curl.chmod(0o755)
        self.routes = {}
        self.requests = self.base / "requests"
        self.route_file = self.base / "routes.json"
        self.env = dict(os.environ, PATH=f"{self.tools}:{os.environ['PATH']}",
                        KHOR_MIRROR_DIR=str(self.mirror), KHOR_REPO="colehank/khor",
                        MIRROR_REQUESTS=str(self.requests), MIRROR_ROUTES=str(self.route_file))
        for key in ["KHOR_VERSION", "KHOR_MIRROR", "KHOR_ASSET_FILE", "KHOR_ASSET_SHA256"]:
            self.env.pop(key, None)
        self.release = "https://github.com/colehank/khor/releases/download/v1.2.3/"
        self.installer = (ROOT / "scripts/install.sh").read_bytes()
        self.route(self.release + "install.sh", self.installer)
        # The old writer requests the API twice. Both calls are deterministic;
        # missing assets and bad digests below are actual publication failures.
        api = {"tag_name": "v1.2.3", "assets": [
            {"browser_download_url": self.release + name + suffix}
            for name in ASSETS for suffix in (".gz", ".gz.sha256")]}
        self.route("https://api.github.com/repos/colehank/khor/releases/latest", json.dumps(api, indent=2).encode())
        self.route("https://raw.githubusercontent.com/colehank/khor/main/scripts/install.sh", self.installer)
        for index, asset in enumerate(ASSETS):
            archive = gzip.compress(b"\x7fELF" + bytes([index]) * 128, mtime=0)
            self.route(self.release + asset + ".gz", archive)
            self.route(self.release + asset + ".gz.sha256",
                       f"{hashlib.sha256(archive).hexdigest()}  {asset}.gz\n".encode())

    def route(self, url, payload):
        self.routes[url] = base64.b64encode(payload).decode()

    def run_writer(self):
        self.route_file.write_text(json.dumps(self.routes))
        return subprocess.run(["/bin/sh", str(ROOT / "scripts/mirror-sync.sh")],
                              env=self.env, capture_output=True, text=True, timeout=30)

    def assert_old_version(self):
        self.assertEqual((self.mirror / "VERSION").read_text(), "v1.2.2\n")
        self.assertFalse((self.mirror / "releases/v1.2.3").exists())

    def test_complete_release_is_immutable_and_pins_one_selection(self):
        old = self.mirror / "releases/v1.2.2"
        old.mkdir(parents=True)
        (old / "reader-open").write_bytes(b"old reader")
        result = self.run_writer()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.mirror / "VERSION").read_text(), "v1.2.3\n")
        selected = self.mirror / "releases/v1.2.3"
        self.assertEqual((selected / "install.sh").read_bytes(), self.installer)
        for asset in ASSETS:
            self.assertEqual((selected / f"{asset}.gz").read_bytes(), base64.b64decode(self.routes[self.release + asset + ".gz"]))
        self.assertEqual((old / "reader-open").read_bytes(), b"old reader")
        self.assertFalse(any("raw.githubusercontent.com" in url or "api.github.com" in url for url in self.requests.read_text().splitlines()))

    def test_bootstrap_and_pointer_directory_targets_cannot_publish(self):
        for target_name in ['install.sh', 'VERSION']:
            for symlink in [False, True]:
                with self.subTest(target=target_name, symlink=symlink):
                    shutil.rmtree(self.mirror)
                    self.mirror.mkdir()
                    (self.mirror/'VERSION').write_text('v1.2.2\n')
                    target = self.mirror/target_name
                    if target.exists():
                        target.unlink()
                    directory = self.base/f'blocked-{target_name}-{symlink}' if symlink else target
                    directory.mkdir()
                    (directory/'old-data').write_bytes(b'old destination data')
                    if symlink:
                        target.symlink_to(directory, target_is_directory=True)
                    result = self.run_writer()
                    self.assertNotEqual(result.returncode, 0, 'mv into a directory must not report a committed release')
                    if target_name == 'install.sh':
                        self.assertEqual((self.mirror/'VERSION').read_text(), 'v1.2.2\n')
                    self.assertEqual(sorted(item.name for item in directory.iterdir()), ['old-data'])
                    self.assertEqual((directory/'old-data').read_bytes(), b'old destination data')
                    self.assertFalse((self.mirror/'releases/v1.2.3').exists())
                    self.assertEqual(target.is_symlink(), symlink)

    def test_existing_generation_special_files_are_rejected_without_orphans(self):
        for entry in ['install.sh', ASSETS[0] + '.gz']:
            with self.subTest(entry=entry):
                shutil.rmtree(self.mirror)
                self.mirror.mkdir()
                (self.mirror/'VERSION').write_text('v1.2.2\n')
                old = self.mirror/'releases/v1.2.2'
                old.mkdir(parents=True)
                (old/'reader-open').write_bytes(b'old reader remains')
                generation = self.mirror/'releases/v1.2.3'
                generation.mkdir()
                for name in ['install.sh'] + [asset + suffix for asset in ASSETS for suffix in ['.gz', '.gz.sha256']]:
                    (generation/name).write_bytes(base64.b64decode(self.routes[self.release + name]))
                (generation/entry).unlink()
                os.mkfifo(generation/entry)
                self.route_file.write_text(json.dumps(self.routes))
                child = subprocess.Popen(['/bin/sh', str(ROOT/'scripts/mirror-sync.sh')],
                                         env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                         text=True, start_new_session=True)
                timed_out = False
                try:
                    try:
                        _, error = child.communicate(timeout=3)
                    except subprocess.TimeoutExpired:
                        timed_out = True
                        os.killpg(child.pid, signal.SIGTERM)
                        try:
                            _, error = child.communicate(timeout=3)
                        except subprocess.TimeoutExpired:
                            os.killpg(child.pid, signal.SIGKILL)
                            _, error = child.communicate(timeout=3)
                finally:
                    if child.poll() is None:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.communicate(timeout=3)
                self.assertFalse(timed_out, 'a special file must be refused without waiting for an external guard')
                self.assertNotEqual(child.returncode, 0)
                self.assertIn('refusing to replace immutable', error)
                self.assertIn(f'({entry})', error)
                self.assertEqual((self.mirror/'VERSION').read_text(), 'v1.2.2\n')
                self.assertEqual((old/'reader-open').read_bytes(), b'old reader remains')
                self.assertTrue(stat.S_ISFIFO((generation/entry).stat().st_mode))
                with self.assertRaises(ProcessLookupError):
                    os.killpg(child.pid, 0)

    def test_published_mirror_has_public_read_and_traverse_modes(self):
        self.mirror.chmod(0o700)
        result = self.run_writer()
        self.assertEqual(result.returncode, 0, result.stderr)
        generation = self.mirror/'releases/v1.2.3'
        directories = {'root': self.mirror, 'releases': self.mirror/'releases', 'generation': generation}
        files = {'bootstrap': self.mirror/'install.sh', 'pointer': self.mirror/'VERSION',
                 'installer': generation/'install.sh'}
        files.update({f'{asset}{suffix}': generation/f'{asset}{suffix}'
                      for asset in ASSETS for suffix in ['.gz', '.gz.sha256']})
        actual = {name: stat.S_IMODE(path.stat().st_mode) for name, path in dict(directories, **files).items()}
        expected = dict.fromkeys(directories, 0o755) | dict.fromkeys(files, 0o644)
        self.assertEqual(actual, expected, 'a separate webserver identity must be able to traverse and read the published mirror')

    def test_failed_asset_cannot_publish_version_or_partial_release(self):
        del self.routes[self.release + ASSETS[-1] + ".gz"]
        result = self.run_writer()
        self.assertNotEqual(result.returncode, 0)
        self.assert_old_version()

    def test_wrong_checksum_cannot_publish(self):
        self.route(self.release + ASSETS[0] + ".gz.sha256", f"{'0' * 64}  {ASSETS[0]}.gz\n".encode())
        result = self.run_writer()
        self.assertNotEqual(result.returncode, 0)
        self.assert_old_version()

    def test_same_tag_flat_layout_is_migrated_instead_of_skipped(self):
        (self.mirror / "VERSION").write_text("v1.2.3\n")
        result = self.run_writer()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.mirror / "releases/v1.2.3" / f"{ASSETS[-1]}.gz").is_file())

    def test_existing_generation_with_other_bytes_cannot_be_overwritten(self):
        selected = self.mirror / "releases/v1.2.3"
        selected.mkdir(parents=True)
        (selected / "install.sh").write_bytes(b"existing generation")
        result = self.run_writer()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.mirror / "VERSION").read_text(), "v1.2.2\n")
        self.assertEqual((selected / "install.sh").read_bytes(), b"existing generation")

    def test_another_writer_lock_is_preserved_without_network(self):
        lock = self.mirror / ".sync.lock"
        lock.mkdir()
        (lock / "owner").write_text("another writer")
        result = self.run_writer()
        self.assertNotEqual(result.returncode, 0)
        self.assert_old_version()
        self.assertEqual((lock / "owner").read_text(), "another writer")
        self.assertFalse(self.requests.exists())

    def test_same_bytes_do_not_replace_an_existing_generation(self):
        self.assertEqual(self.run_writer().returncode, 0)
        asset = self.mirror / "releases/v1.2.3" / f"{ASSETS[0]}.gz"
        before = asset.stat()
        result = self.run_writer()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((asset.stat().st_ino, asset.stat().st_mtime_ns), (before.st_ino, before.st_mtime_ns))

    def test_next_release_retains_the_previously_selected_bytes(self):
        self.assertEqual(self.run_writer().returncode, 0)
        old = self.mirror / "releases/v1.2.3" / f"{ASSETS[0]}.gz"
        original = old.read_bytes()
        for url, payload in list(self.routes.items()):
            if self.release in url:
                self.routes[url.replace('/v1.2.3/', '/v1.2.4/')] = payload
        self.env['MIRROR_LATEST_TAG'] = 'v1.2.4'
        result = self.run_writer()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.mirror/'VERSION').read_text(), 'v1.2.4\n')
        self.assertEqual(old.read_bytes(), original)

    def test_invalid_gzip_and_legacy_installer_preserve_the_selection(self):
        invalid = b'not gzip'
        self.route(self.release + ASSETS[0] + '.gz', invalid)
        self.route(self.release + ASSETS[0] + '.gz.sha256',
                   f"{hashlib.sha256(invalid).hexdigest()}  {ASSETS[0]}.gz\n".encode())
        result = self.run_writer()
        self.assertNotEqual(result.returncode, 0)
        self.assert_old_version()
        self.assertFalse((self.mirror/'.sync.lock').exists())
        self.assertEqual(list(self.mirror.glob('.release.*')), [])
        self.route(self.release + 'install.sh', b'#!/bin/sh\nprintf legacy\n')
        result = self.run_writer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('predates', result.stderr)
        self.assert_old_version()

    def test_concurrent_writer_cannot_replace_the_active_attempts_stage(self):
        pause = self.base/'resume'
        self.env['MIRROR_PAUSE'] = str(pause)
        self.route_file.write_text(json.dumps(self.routes))
        first = subprocess.Popen(['/bin/sh', str(ROOT/'scripts/mirror-sync.sh')],
                                 env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            deadline = time.monotonic()+10
            while not pause.with_suffix('.ready').exists():
                self.assertIsNone(first.poll(), 'first writer exited before the controlled download')
                self.assertLess(time.monotonic(), deadline)
                time.sleep(.02)
            stages = list(self.mirror.glob('.release.*'))
            self.assertEqual(len(stages), 1)
            second = self.run_writer()
            self.assertNotEqual(second.returncode, 0)
            self.assertTrue(stages[0].is_dir())
            self.assertTrue((self.mirror/'.sync.lock').is_dir())
            pause.touch()
            _, error = first.communicate(timeout=20)
            self.assertEqual(first.returncode, 0, error)
            self.assertEqual((self.mirror/'VERSION').read_text(), 'v1.2.3\n')
        finally:
            pause.touch()
            if first.poll() is None:
                first.terminate()
            first.communicate(timeout=20)


if __name__ == "__main__":
    unittest.main()
