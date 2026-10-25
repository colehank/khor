"""Expose the scratch-only shell regression to the shared unittest entry point."""
import importlib.util
from pathlib import Path
import unittest


class OnsetUninstallTest(unittest.TestCase):
    def test_uninstall_boundaries(self):
        script = Path(__file__).resolve().parents[1] / "check-onset-uninstall.py"
        spec = importlib.util.spec_from_file_location("onset_uninstall_check", script)
        suite = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(suite)
        self.assertEqual(suite.run_suite(), 0)


if __name__ == "__main__":
    unittest.main()
