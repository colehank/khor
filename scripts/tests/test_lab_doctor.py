import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.doctor import doctor


def results(root, cache_root):
    return {name: (ok, hint) for name, ok, hint in doctor(root, {'cache_root': cache_root})}


class LabDoctorTests(unittest.TestCase):
    def test_reports_cache_root_outside_workspace(self):
        with tempfile.TemporaryDirectory() as workspace, tempfile.TemporaryDirectory() as outside:
            checks = results(Path(workspace), outside)
            self.assertFalse(checks['cache_root'][0])
            self.assertIn('.khor-lab.local.json', checks['cache_root'][1])

    def test_accepts_cache_root_inside_workspace(self):
        with tempfile.TemporaryDirectory() as workspace:
            checks = results(Path(workspace), str(Path(workspace) / '.cache' / 'lab'))
            self.assertTrue(checks['cache_root'][0])

    def test_reports_cargo_target_outside_workspace(self):
        with tempfile.TemporaryDirectory() as workspace, tempfile.TemporaryDirectory() as outside:
            with patch.dict(os.environ, {'CARGO_TARGET_DIR': outside}):
                checks = results(Path(workspace), str(Path(workspace) / '.cache' / 'lab'))
            self.assertFalse(checks['cargo_target'][0])

    def test_toolchain_outside_is_a_known_exception(self):
        with tempfile.TemporaryDirectory() as workspace, tempfile.TemporaryDirectory() as toolchain:
            with patch.dict(os.environ, {'CARGO_HOME': toolchain}):
                checks = results(Path(workspace), str(Path(workspace) / '.cache' / 'lab'))
            ok, hint = checks['cargo_home']
            self.assertTrue(ok)
            self.assertIn('full-access', hint)


if __name__ == '__main__':
    unittest.main()
