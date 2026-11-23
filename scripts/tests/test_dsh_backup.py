from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.backup import plan, prune, run


class BackupTests(unittest.TestCase):
    def test_plan_covers_the_session_store(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory)
            (home / 'sessions').mkdir()
            (home / 'profiles' / 'desktop').mkdir(parents=True)
            (home / 'profiles' / 'desktop' / 'cordis.patch.yml').write_text('[]', encoding='utf-8')
            relatives = [relative for _, relative in plan(home)]
            self.assertIn('sessions', relatives)
            self.assertIn('profiles/desktop/cordis.patch.yml', relatives)
            self.assertNotIn('storages', relatives)

    def test_run_copies_the_store(self):
        with tempfile.TemporaryDirectory() as directory:
            home = Path(directory) / 'home'
            destination = Path(directory) / 'out'
            (home / 'sessions').mkdir(parents=True)
            (home / 'sessions' / 'a.jsonl').write_text('{}\n', encoding='utf-8')
            receipt = run(home, destination, keep=7)
            target = Path(receipt['target'])
            self.assertTrue((target / 'sessions' / 'a.jsonl').is_file())
            self.assertEqual(receipt['copied'], ['sessions'])

    def test_prune_keeps_the_newest_copies(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for stamp in ('20260101T000000Z', '20260102T000000Z', '20260103T000000Z'):
                (root / stamp).mkdir()
            removed = prune(root, 2)
            self.assertEqual([Path(path).name for path in removed], ['20260101T000000Z'])
            self.assertEqual(sorted(path.name for path in root.iterdir()),
                             ['20260102T000000Z', '20260103T000000Z'])


if __name__ == '__main__':
    unittest.main()
