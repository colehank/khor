"""Installer cleanup must handle the App's asynchronous resident startup."""
import importlib.util
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from release_manifest import files

spec = importlib.util.spec_from_file_location(
    "desktop_installer", Path(__file__).resolve().parents[1] / "check-desktop-installer.py")
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)


class ProcessRetirementTests(unittest.TestCase):
    def setUp(self):
        self.now = 0

    def sleep(self, seconds):
        self.now += seconds

    def retire(self, processes, terminate):
        installer.retire_processes(processes, terminate, timeout=3, quiet=1,
                                   clock=lambda: self.now, sleep=self.sleep)

    def test_initial_empty_snapshot_does_not_miss_late_resident(self):
        stopped = []
        process = (123, Path("/private-fixture/runtime/khor"))

        def processes():
            return [process] if self.now >= 0.2 and not stopped else []

        self.retire(processes, stopped.append)
        self.assertEqual(stopped, [process])
        self.assertGreaterEqual(self.now, 1.2)

    def test_a_replacement_worker_resets_the_quiet_period(self):
        stopped = []
        first = (123, Path("/private-fixture/runtime/khor"))
        replacement = (456, first[1])

        def processes():
            if first not in stopped:
                return [first]
            if self.now >= 0.6 and replacement not in stopped:
                return [replacement]
            return []

        self.retire(processes, stopped.append)
        self.assertEqual(stopped, [first, replacement])
        self.assertGreaterEqual(self.now, 1.6)

    def test_a_worker_that_never_exits_still_fails_the_gate(self):
        process = (123, Path("/private-fixture/runtime/khor"))
        with self.assertRaisesRegex(AssertionError, "must retire its own processes"):
            self.retire(lambda: [process], lambda _: None)
        self.assertLess(self.now, 3.2)


class InstallerLogIsolationTests(unittest.TestCase):
    def check_logs(self, action, use_runner_temp):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            product = root / "release-product"
            product.mkdir()
            package = product / "khor-windows-x86_64.msi"
            package.write_bytes(b"immutable MSI fixture")
            expected = files(product)
            diagnostics = root / "runner temp"
            diagnostics.mkdir()
            environment = {**os.environ, "SystemRoot": "C:/Windows"}
            environment.pop("RUNNER_TEMP", None)
            if use_runner_temp:
                environment["RUNNER_TEMP"] = str(diagnostics)

            def msiexec(command, **kwargs):
                self.assertEqual(kwargs["timeout"], 600)
                match = re.search(r'/L\*V "([^"]+)"', command)
                self.assertIsNotNone(match)
                log = Path(match[1])
                self.assertTrue(log.parent.is_dir())
                log.write_text("MSI diagnostic output\n")
                self.assertEqual(log.parent, diagnostics / "khor-installer-logs")
                self.assertIn(f'{action} "{package}" /qn /norestart', command)
                return subprocess.CompletedProcess(command, 0)

            with patch.dict(os.environ, environment, clear=True), \
                    patch.object(installer.tempfile, "gettempdir", return_value=str(diagnostics)), \
                    patch.object(installer.subprocess, "run", side_effect=msiexec):
                result = installer.invoke_msi(action, package, root / "Installed Khor" if action == "/i" else None)
            self.assertEqual(result.returncode, 0)
            self.assertEqual(files(product), expected, "Installation must preserve the complete artifact receipt")

    def test_install_logs_never_change_the_release_directory(self):
        self.check_logs("/i", use_runner_temp=True)

    def test_uninstall_logs_use_the_temp_directory_outside_actions(self):
        self.check_logs("/x", use_runner_temp=False)


if __name__ == "__main__":
    unittest.main()
