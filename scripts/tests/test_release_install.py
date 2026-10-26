"""Expose scratch-only installer component checks to the shared unittest entry."""
import importlib.util
from pathlib import Path
import unittest


class ReleaseInstallTest(unittest.TestCase):
    def test_components(self):
        path = Path(__file__).resolve().parents[1] / "check-release-install.py"
        spec = importlib.util.spec_from_file_location("release_install_check", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        self.assertEqual(module.run_suite(), 0)


if __name__ == "__main__":
    unittest.main()
