"""Behavioral boundaries for Lab configuration, snapshots, activation and lifecycle."""
import json
import os
from unittest.mock import patch
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import unittest
from http.server import ThreadingHTTPServer
from urllib.error import HTTPError
from urllib.request import Request, urlopen
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.config import load
from khor_lab.contract import digest
from khor_lab.artifacts import unpack, prune, validate, write_json
from khor_lab.server import Handler, Lab


class LabTests(unittest.TestCase):
    def setUp(self):
        environment = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
        environment_patch = patch.dict(os.environ, environment, clear=True)
        environment_patch.start()
        self.addCleanup(environment_patch.stop)
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def test_developer_can_choose_both_dimensions_independently(self):
        for mode in ('frontend', 'full'):
            for builder in ('local', 'remote'):
                config = load(self.root, {'mode': mode, 'builder': builder, 'scene': 'live' if mode == 'full' else 'gui'})
                self.assertEqual((config['mode'], config['builder']), (mode, builder))
        write_json(self.root / '.khor-lab.local.json', {'builder': 'remote'})
        self.assertEqual(load(self.root)['builder'], 'remote')
        self.assertEqual(load(self.root, {'builder': 'local'})['builder'], 'local')

    def test_native_contract_changes_for_backend_but_not_frontend(self):
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        (self.root / 'lib.rs').write_text('fn one() {}')
        (self.root / 'view.tsx').write_text('first')
        subprocess.run(['git', '-C', str(self.root), 'add', '.'], check=True)
        original = digest(self.root)
        (self.root / 'view.tsx').write_text('second')
        self.assertEqual(digest(self.root), original)
        (self.root / 'lib.rs').write_text('fn two() {}')
        self.assertNotEqual(digest(self.root), original)

    def test_rejects_wrong_source_and_unsafe_archives(self):
        (self.root / 'index.html').write_text('App')
        write_json(self.root / 'lab-manifest.json', dict(schema=1, entry='production-app', tree='wrong'))
        with self.assertRaises(ValueError):
            validate(self.root, {'tree': 'expected'})
        archive = self.root / 'unsafe.zip'
        with zipfile.ZipFile(archive, 'w') as bundle:
            bundle.writestr('../outside', 'escape')
        with self.assertRaises(ValueError):
            unpack(archive, self.root / 'extracted')
        self.assertFalse((self.root / 'outside').exists())

    def test_serves_real_entry_and_preserves_pinned_assets_without_exposing_store(self):
        home = self.root / 'home'
        lab = Lab(load(self.root), home)
        self.addCleanup(lab.cancel.set)
        version = home / 'versions/current'
        (version / 'assets').mkdir(parents=True)
        write_json(version / 'lab-manifest.json', {'tree': 'pinned-identity'})
        (version / 'index.html').write_text('<script src="./assets/app.js"></script>')
        (version / 'assets/app.js').write_text('production App')
        (home / 'store').mkdir()
        (home / 'store/secret').write_text('must not be served')
        lab.value.update(version='current', available=True, source={'tree': 'identity'})
        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        server.lab = lab
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        origin = f'http://127.0.0.1:{server.server_port}'
        with urlopen(origin + '/app/') as response:
            self.assertIn(b'/versions/current/assets/app.js', response.read())
        with urlopen(origin + '/lab/runtime?version=current') as response:
            self.assertEqual(json.load(response)['tree'], 'pinned-identity')
        with urlopen(origin + '/snapshots/current/app/') as response:
            self.assertIn(b'/versions/current/assets/app.js', response.read())
        with urlopen(origin + '/lab/status') as response:
            self.assertNotIn('token', json.load(response))
        for path in ('/store/secret', '/versions/current/../../manager.json'):
            try:
                urlopen(origin + path)
                self.fail('Sensitive path was served')
            except HTTPError as error:
                error.close()
        try:
            urlopen(Request(origin + '/lab/control/reset', data=b'{}'))
            self.fail('Unauthenticated control was accepted')
        except HTTPError as error:
            error.close()
        body = json.dumps({'scene': 'terminal'}).encode()
        req = Request(origin + '/lab/control/scene', data=body, headers={'authorization': 'Bearer ' + lab.value['token']})
        with urlopen(req) as response:
            self.assertEqual(json.load(response)['scene'], 'terminal')
        self.assertEqual(lab.status()['source']['tree'], 'identity')

    def test_failed_update_preserves_available_bundle(self):
        lab = Lab(load(self.root), self.root / 'manager')
        lab.value.update(version='reviewed', available=True)
        with patch('khor_lab.builders.local', side_effect=ValueError('Build failed')):
            lab.build_latest()
        self.assertEqual(lab.status()['version'], 'reviewed')
        self.assertTrue(lab.status()['available'])
        self.assertEqual(lab.status()['checks'], 'failed')

    def test_manager_does_not_launch_duplicate_builds_or_publish_after_stop(self):
        lab = Lab(load(self.root), self.root / 'manager')
        entered, release = threading.Event(), threading.Event()
        def build(*_args):
            entered.set()
            release.wait(timeout=5)
            return {'checks': 'done'}
        with patch('khor_lab.builders.local', side_effect=build) as builder:
            lab.update()
            self.assertTrue(entered.wait(timeout=2))
            lab.update()
            self.assertEqual(builder.call_count, 1)
            lab.cancel.set()
            lab.publish('stale', 'frontend', {}, {'tree': 'must-not-activate'})
            self.assertFalse(lab.status()['available'])
            release.set()
            lab.build.join(timeout=2)

    def test_retention_keeps_active_version_even_if_older(self):
        for name in ('a', 'b', 'c'):
            (self.root / name).mkdir()
        prune(self.root, 1, ['a'])
        self.assertTrue((self.root / 'a').exists())
        self.assertLessEqual(len(list(self.root.iterdir())), 2)


if __name__ == '__main__':
    unittest.main()
