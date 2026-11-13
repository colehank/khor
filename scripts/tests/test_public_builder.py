"""Public source submission must use the explicitly authorized build repository."""
import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import dev
from khor_lab import builders


class PublicBuilderTest(unittest.TestCase):
    def test_cli_submits_exact_source_to_the_authorized_public_builder(self):
        request = 'a' * 32
        info = dict(base_commit='b' * 40, tree='c' * 40, snapshot_commit='d' * 40,
                    excluded_untracked_count=0)
        found = dict(databaseId=123, displayTitle='net-unit ' + request, url='https://example.invalid/run')

        def command(*args):
            if args[:2] == ('gh', 'api'):
                return json.dumps(dict(private=False, full_name='colehank/khor-build'))
            if args[:3] == ('gh', 'run', 'list'):
                return json.dumps([found])
            return ''

        with tempfile.TemporaryDirectory() as folder:
            args = SimpleNamespace(builder_repo='colehank/khor-build', builder_ref='feat/net', task='net-unit')
            with patch.object(dev, 'command', side_effect=command) as calls, \
                    patch.object(dev, 'snapshot', return_value=info), \
                    patch.object(dev, 'upload_snapshot') as upload, \
                    patch.object(dev, 'state_dir', return_value=Path(folder)), \
                    patch.object(dev.uuid, 'uuid4', return_value=SimpleNamespace(hex=request)), \
                    contextlib.redirect_stdout(io.StringIO()):
                dev.submit(args)
            upload.assert_called_once_with(info)
            self.assertEqual(info['repository'], 'colehank/khor-build')
            dispatch = next(c.args for c in calls.call_args_list if c.args[:3] == ('gh', 'workflow', 'run'))
            self.assertIn('snapshot_sha=' + info['snapshot_commit'], dispatch)
            self.assertEqual(dispatch[dispatch.index('--ref') + 1], 'feat/net')
            receipt = json.loads((Path(folder) / 'runs' / (request + '.json')).read_text())
            self.assertEqual(receipt['tree'], info['tree'])
            self.assertEqual(receipt['databaseId'], 123)

    def test_lab_uploads_and_dispatches_to_the_authorized_public_builder(self):
        info = dict(base_commit='b' * 40, tree='c' * 40, snapshot_commit='d' * 40)
        config = dict(builder_repo='colehank/khor-build', mode='full', port=23962)
        metadata = json.dumps(dict(private=False, full_name='colehank/khor-build'))
        with tempfile.TemporaryDirectory() as folder:
            with patch.object(builders, 'run', return_value=metadata) as calls, \
                    patch.object(builders, 'source', return_value=info), \
                    patch.object(dev, 'upload_snapshot') as upload:
                result = builders.remote(config, Path(folder), lambda *args: None, lambda: True)
            upload.assert_called_once_with(info)
            dispatch = next(c.args for c in calls.call_args_list if c.args[:3] == ('gh', 'workflow', 'run'))
            self.assertIn('snapshot_sha=' + info['snapshot_commit'], dispatch)
            self.assertIn('native_required=true', dispatch)
            self.assertEqual(result['status'], 'cancelled')
            receipt = json.loads((Path(folder) / 'request.json').read_text())
            self.assertEqual(receipt['repository'], 'colehank/khor-build')
            self.assertEqual(receipt['tree'], info['tree'])

    def test_other_public_repositories_and_redirected_identities_never_receive_source(self):
        for repository, identity in [('someone/build', 'someone/build'),
                                     ('colehank/khor-build', 'someone/build')]:
            with self.subTest(repository=repository, identity=identity):
                metadata = json.dumps(dict(private=False, full_name=identity))
                args = SimpleNamespace(builder_repo=repository, task='net-unit')
                with patch.object(dev, 'command', return_value=metadata), \
                        patch.object(dev, 'snapshot') as snapshot, \
                        patch.object(dev, 'upload_snapshot') as upload:
                    with self.assertRaisesRegex(ValueError, 'Public source snapshots'):
                        dev.submit(args)
                    snapshot.assert_not_called()
                    upload.assert_not_called()
                with patch.object(builders, 'run', return_value=metadata), \
                        patch.object(builders, 'source') as source, \
                        patch.object(dev, 'upload_snapshot') as upload:
                    with self.assertRaisesRegex(ValueError, 'Public source snapshots'):
                        builders.remote(dict(builder_repo=repository), Path('unused'), lambda *args: None, lambda: True)
                    source.assert_not_called()
                    upload.assert_not_called()


if __name__ == '__main__':
    unittest.main()
