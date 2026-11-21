from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.clean import clean, inventory
from khor_lab.safepaths import UnsafePath


def make_target(base, name):
    target = base / name
    target.mkdir(parents=True)
    (target / '.rustc_info.json').write_text('', encoding='utf-8')
    (target / 'CACHEDIR.TAG').write_text('', encoding='utf-8')
    (target / 'artifact.bin').write_bytes(b'x' * 10)
    return target


class LabCleanTests(unittest.TestCase):
    def test_inventory_classifies_target(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            make_target(base, 'khor-target-a')
            manifest = inventory(base, 'khor-*')
            self.assertEqual(len(manifest['items']), 1)
            self.assertIn('cargo', manifest['items'][0]['signatures'])
            self.assertEqual(manifest['items'][0]['bytes'], 10)

    def test_rejects_c_path_and_deletes_nothing(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            target = make_target(base, 'khor-target-a')
            manifest = {'items': [
                {'path': str(target), 'signatures': ['cargo']},
                {'path': 'C:\\Users\\admin\\x', 'signatures': ['cargo']},
            ]}
            with self.assertRaises(UnsafePath):
                clean(manifest, [str(base)], dry_run=False, yes=True)
            self.assertTrue(target.is_dir())

    def test_skips_item_without_signature(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            plain = base / 'khor-plain'
            plain.mkdir()
            manifest = {'items': [{'path': str(plain), 'signatures': ['cargo']}]}
            receipt = clean(manifest, [str(base)], dry_run=False, yes=True)
            self.assertEqual(receipt['removed'], [])
            self.assertEqual(len(receipt['skipped']), 1)
            self.assertTrue(plain.is_dir())

    def test_removes_item_and_reports_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            target = make_target(base, 'khor-target-a')
            manifest = {'items': [{'path': str(target), 'signatures': ['cargo', 'cachedir']}]}
            receipt = clean(manifest, [str(base)], dry_run=False, yes=True)
            self.assertEqual(receipt['removed'], [str(target.resolve())])
            self.assertEqual(receipt['bytes'], 10)
            self.assertFalse(target.exists())

    def test_requires_yes(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            target = make_target(base, 'khor-target-a')
            manifest = {'items': [{'path': str(target), 'signatures': ['cargo']}]}
            with self.assertRaises(ValueError):
                clean(manifest, [str(base)], dry_run=False, yes=False)
            self.assertTrue(target.is_dir())


if __name__ == '__main__':
    unittest.main()
