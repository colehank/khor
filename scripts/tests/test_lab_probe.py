from datetime import timedelta
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.probe import MARKER, gc_probes, new_probe


class LabProbeTests(unittest.TestCase):
    def test_new_probe_creates_marker(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            probe = new_probe(root, 'open')
            self.assertEqual(probe.parent, root.resolve() / '.probe')
            self.assertTrue((probe / MARKER).is_file())
            self.assertEqual(probe, probe.resolve())

    def test_gc_keeps_unmarked_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            unmarked = root / '.probe' / 'keep-me'
            unmarked.mkdir(parents=True)
            removed = gc_probes(root, timedelta(seconds=0), dry_run=False)
            self.assertEqual(removed, [])
            self.assertTrue(unmarked.is_dir())

    def test_gc_dry_run_keeps_everything(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            probe = new_probe(root, 'open')
            planned = gc_probes(root, timedelta(seconds=0), dry_run=True)
            self.assertEqual(planned, [probe.resolve()])
            self.assertTrue(probe.is_dir())

    def test_gc_removes_old_marked_probe(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            probe = new_probe(root, 'open')
            removed = gc_probes(root, timedelta(seconds=0), dry_run=False)
            self.assertEqual(removed, [probe.resolve()])
            self.assertFalse(probe.exists())


if __name__ == '__main__':
    unittest.main()
