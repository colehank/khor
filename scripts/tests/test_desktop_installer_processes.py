"""Installer cleanup must handle the App's asynchronous resident startup."""
import importlib.util
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
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


if __name__ == "__main__":
    unittest.main()
