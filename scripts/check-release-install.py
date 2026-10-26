#!/usr/bin/env python3
"""Run actual installer components in scratch; never run its main entry point."""
import contextlib
import gzip
import hashlib
import http.server
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import unittest


ROOT = Path(__file__).resolve().parents[1]
INSTALL = ROOT / "scripts/install.sh"
BEGIN = "# KHOR_INSTALL_DECLARATIONS_BEGIN\n"
END = "# KHOR_INSTALL_DECLARATIONS_END\n"
MAIN = 'khor_install_main "$@"\n'


def component_declarations(source):
    if source.count(BEGIN) != 1 or source.count(END) != 1:
        raise ValueError("installer declaration markers must be unique")
    prefix, rest = source.split(BEGIN)
    block, suffix = rest.split(END)
    if any(line.strip() and not line.startswith("#") for line in prefix.splitlines()):
        raise ValueError("unexpected executable installer prefix")
    if suffix != MAIN or source.count("\n" + MAIN) != 1:
        raise ValueError("installer must end in its unique main invocation")
    main_header = "khor_install_main() {\n"
    if block.count(main_header) != 1 or not block.endswith("}\n"):
        raise ValueError("installer main declaration boundary changed")
    helpers, _ = block.split(main_header)
    if not helpers.startswith("khor_fetch_verify_place() (\n") or not helpers.endswith(")\n\n"):
        raise ValueError("unexpected executable component boundary")
    syntax = subprocess.run(["/bin/sh", "-n"], input=helpers, text=True, capture_output=True)
    if syntax.returncode:
        raise ValueError("installer component syntax is invalid")
    return helpers


@contextlib.contextmanager
def loopback(contents):
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            payload = contents.get(self.path)
            self.send_response(200 if payload is not None else 404)
            self.end_headers()
            if payload is not None:
                self.wfile.write(payload)

        def log_message(self, *_):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}"
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


class InstallComponentsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = INSTALL.read_text()
        cls.helpers = component_declarations(cls.source)

    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="release-install-")
        self.addCleanup(self.scratch.cleanup)
        self.directory = Path(self.scratch.name)
        self.destination = self.directory / "bin"
        self.destination.mkdir()
        self.binary = self.destination / "khor"
        self.binary.write_bytes(b"original binary")
        self.binary.chmod(0o755)
        self.payload = Path("/usr/bin/true").read_bytes()
        self.archive = gzip.compress(self.payload, mtime=0)

    def run_component(self, url, preplaced=None):
        # The wrapper confines curl to this exact loopback fixture and ignores
        # user curl configuration. No HOME override or installer product switch.
        wrapper = self.directory / "tools"
        wrapper.mkdir(exist_ok=True)
        endpoint = url.rsplit("/", 1)[0] + "/"
        (wrapper / "curl").write_text(
            "#!/bin/sh\nfor arg do\ncase \"$arg\" in\n"
            f"http://*|https://*) case \"$arg\" in '{endpoint}'*) : ;; *) exit 66 ;; esac ;;\n"
            "esac\ndone\nexec /usr/bin/curl --disable --noproxy '*' \"$@\"\n"
        )
        (wrapper / "curl").chmod(0o755)
        environment = {key: value for key, value in os.environ.items() if not key.startswith("KHOR_")}
        environment["PATH"] = str(wrapper) + os.pathsep + os.environ.get("PATH", os.defpath)
        if preplaced:
            environment["KHOR_ASSET_FILE"] = str(preplaced)
        script = self.helpers + '\nkhor_fetch_verify_place "$1" "$2" "$3" "$4"\n'
        return subprocess.run(
            ["/bin/sh", "-s", "--", url, str(self.destination), "fixture", "v1.2.3"],
            input=script, text=True, capture_output=True, env=environment,
        )

    def assert_no_stage(self):
        self.assertEqual(list(self.destination.glob(".khor.download.*")), [])

    def test_load_declares_only_components(self):
        result = subprocess.run(["/bin/sh", "-s"], input=self.helpers + "\nprintf loaded\n", text=True, capture_output=True)
        self.assertEqual((result.returncode, result.stdout), (0, "loaded"))
        self.assertNotIn("$HOME", self.helpers)

    def test_loader_refuses_changed_entry_boundaries(self):
        mutations = [
            self.source.replace(BEGIN, BEGIN + BEGIN), self.source.replace(END, ""),
            "printf unexpected\n" + self.source, self.source + "printf unexpected\n",
            self.source.replace(MAIN, MAIN + MAIN), self.source.replace("khor_install_main() {\n", "printf unexpected\n"),
        ]
        for source in mutations:
            with self.subTest(source_sha256=hashlib.sha256(source.encode()).hexdigest()):
                with self.assertRaises(ValueError):
                    component_declarations(source)

    def test_fetch_place_preserves_open_old_inode(self):
        with self.binary.open("rb") as old, loopback({"/asset.gz": self.archive}) as base:
            result = self.run_component(base + "/asset.gz")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(old.read(), b"original binary")
        self.assertEqual(self.binary.read_bytes(), self.payload)
        self.assertTrue(os.access(self.binary, os.X_OK))
        self.assert_no_stage()

    def test_failed_fetch_preserves_old_binary(self):
        with loopback({}) as base:
            result = self.run_component(base + "/missing.gz")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.binary.read_bytes(), b"original binary")
        self.assert_no_stage()

    def test_bad_gzip_preserves_old_binary(self):
        with loopback({"/asset.gz": b"not gzip"}) as base:
            result = self.run_component(base + "/asset.gz")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.binary.read_bytes(), b"original binary")
        self.assert_no_stage()

    def test_nonprogram_preserves_old_binary(self):
        with loopback({"/asset.gz": gzip.compress(b"not executable", mtime=0)}) as base:
            result = self.run_component(base + "/asset.gz")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.binary.read_bytes(), b"original binary")
        self.assert_no_stage()

    def test_preplaced_archive_preserves_existing_consumption(self):
        archive = self.directory / "provided.gz"
        archive.write_bytes(self.archive)
        result = self.run_component("http://127.0.0.1:1/unused", archive)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.binary.read_bytes(), self.payload)
        self.assertFalse(archive.exists())
        self.assert_no_stage()


def run_suite():
    return 0 if unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(InstallComponentsTest)).wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(run_suite())
