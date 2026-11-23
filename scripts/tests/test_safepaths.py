import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.safepaths import (DENIED_ROOTS, UnsafePath, assert_no_reparse,
                                require_marker, resolve_owned)


class SafePathsTests(unittest.TestCase):
    def test_rejects_home(self):
        with self.assertRaises(UnsafePath):
            resolve_owned(Path.home(), [Path.cwd()])

    def test_rejects_drive_root(self):
        with self.assertRaises(UnsafePath):
            resolve_owned(Path('C:/'), [Path('C:/anything')])

    def test_rejects_outside_allow_roots(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            with self.assertRaises(UnsafePath):
                resolve_owned(base / 'x', [base / 'allowed'])

    def test_accepts_child(self):
        with tempfile.TemporaryDirectory() as directory:
            allowed = Path(directory) / 'allowed'
            allowed.mkdir()
            self.assertEqual(resolve_owned(allowed / 'a', [allowed]), (allowed / 'a').resolve())

    def test_rejects_trailing_space_and_reserved_names(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            for name in ['nul', 'x. ']:
                with self.assertRaises(UnsafePath):
                    resolve_owned(base / name, [base])

    def test_rejects_marker_loss(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(UnsafePath):
                require_marker(Path(directory))

    def test_reads_marker(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            (base / '.khor-probe.json').write_text('{"id": "a", "kind": "probe"}', encoding='utf-8')
            self.assertEqual(require_marker(base)['id'], 'a')

    def test_denied_roots_include_profile(self):
        self.assertIn(Path.home(), DENIED_ROOTS)
        self.assertIn((Path.home() / '.dsh').resolve(), DENIED_ROOTS)


if __name__ == '__main__':
    unittest.main()
