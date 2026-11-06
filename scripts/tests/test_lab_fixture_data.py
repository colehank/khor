import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.fixture_data import INPUTS, OUTPUT, cache_record, ensure, ready


class FixtureDataCache(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        for name in (*INPUTS, 'apps/gui/dev/lab/data/workspace/README.md'):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text('original source\n')
        ids = ['e' * 64] + [format(index, 'x') * 64 for index in range(2, 7)]
        self.data = dict(schema=1, machine_ids=ids,
                         avatars={key: dict(radius_ratio=.5, mark=dict(ink='dark')) for key in ids},
                         directories={key: {} for key in ('', 'project', 'project/src', 'project/docs', 'project/.config')},
                         documents={key: '' for key in ('project/README.md', 'project/src/navigation.ts', 'project/docs/中文说明.md')})

    def save(self):
        (self.root / OUTPUT).write_text(json.dumps(cache_record(self.data, self.root)))

    def test_ui_edits_reuse_domain_data(self):
        self.save()
        ui = self.root / 'apps/gui/src/App.tsx'
        ui.parent.mkdir(parents=True)
        ui.write_text('changed UI')
        self.assertTrue(ready(self.root))

    def test_backend_and_template_edits_require_regeneration(self):
        for name in ('crates/core/src/avatar.rs', 'apps/gui/dev/lab/data/workspace/README.md'):
            self.save()
            (self.root / name).write_text('changed domain input')
            self.assertFalse(ready(self.root))

    def test_generation_is_reused_and_malformed_records_refused(self):
        result = subprocess.CompletedProcess([], 0, stdout=json.dumps(self.data))
        with patch('khor_lab.fixture_data.subprocess.run', return_value=result) as generate:
            ensure(self.root)
            ensure(self.root)
            generate.assert_called_once()
        self.assertTrue(ready(self.root))
        data = json.loads((self.root / OUTPUT).read_text())
        del data['avatars'][data['machine_ids'][0]]
        (self.root / OUTPUT).write_text(json.dumps(data))
        self.assertFalse(ready(self.root))

    def test_hand_changed_crop_is_not_accepted_with_old_provenance(self):
        self.save()
        data = json.loads((self.root / OUTPUT).read_text())
        data['avatars'][data['machine_ids'][0]]['radius_ratio'] = .18
        (self.root / OUTPUT).write_text(json.dumps(data))
        self.assertFalse(ready(self.root))


if __name__ == '__main__':
    unittest.main()
