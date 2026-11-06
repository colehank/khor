"""Behavioral boundaries for Lab configuration, snapshots, activation and lifecycle."""
import json
import io
import hashlib
from urllib.error import HTTPError
from email.message import Message
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
        (version / 'native').mkdir()
        (version / 'native/private.json').write_text('must not be served')
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
        for path in ('/store/secret', '/versions/current/../../manager.json', '/versions/current/assets/../native/private.json'):
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

    def test_artifact_redirect_keeps_github_token_off_storage_requests(self):
        from khor_lab.download import acquire
        memory = io.BytesIO()
        with zipfile.ZipFile(memory, 'w') as archive:
            archive.writestr('index.html', 'actual App bundle')
        payload = memory.getvalue()
        headers = Message()
        headers['Location'] = 'https://test.blob.core.windows.net/artifact?sig=test-only'
        redirect = HTTPError('https://api.github.com/artifact', 302, 'Found', headers, None)
        response = io.BytesIO(payload)
        metadata = {'id': 1, 'size_in_bytes': len(payload), 'digest': 'sha256:' + hashlib.sha256(payload).hexdigest()}
        with patch('khor_lab.download.subprocess.check_output', return_value='test-github-token'), \
             patch('khor_lab.download.build_opener') as opener, \
             patch('khor_lab.download.urlopen', return_value=response) as storage:
            opener.return_value.open.side_effect = redirect
            acquire('owner/private', metadata, self.root)
            api_request = opener.return_value.open.call_args.args[0]
            self.assertEqual(api_request.get_header('Authorization'), 'Bearer test-github-token')
            self.assertIsInstance(storage.call_args.args[0], str)
            self.assertNotIn('headers', storage.call_args.kwargs)
        self.assertEqual((self.root / 'index.html').read_text(), 'actual App bundle')

    def test_parallel_artifact_ranges_are_checked_and_assembled_in_order(self):
        from khor_lab.download import _ranges
        payload = b'first' * (1024 * 1024)
        class Response(io.BytesIO):
            status = 206
        def storage(request, **_kwargs):
            self.assertIsNone(request.get_header('Authorization'))
            start, end = map(int, request.get_header('Range').removeprefix('bytes=').split('-'))
            response = Response(payload[start:end + 1])
            response.headers = {'Content-Range': f'bytes {start}-{end}/{len(payload)}'}
            return response
        archive = self.root / 'ranges.zip'
        with patch('khor_lab.download.urlopen', side_effect=storage):
            _ranges('https://test.blob.core.windows.net/object', archive, len(payload), lambda *_: None)
        self.assertEqual(hashlib.sha256(archive.read_bytes()).digest(), hashlib.sha256(payload).digest())

    def test_interrupted_artifact_acquisition_reuses_completed_ranges(self):
        from khor_lab.download import _ranges
        from urllib.error import URLError
        payload = b'part' * (1280 * 1024)
        archive, receipt = self.root / 'partial.zip', self.root / 'ranges.json'
        def response(request):
            start, end = map(int, request.get_header('Range').removeprefix('bytes=').split('-'))
            value = io.BytesIO(payload[start:end + 1])
            value.status = 206
            value.headers = {'Content-Range': f'bytes {start}-{end}/{len(payload)}'}
            return value
        def interrupted(request, **_kwargs):
            if request.get_header('Range').startswith('bytes=4194304-'):
                raise URLError('temporary interruption')
            return response(request)
        with patch('khor_lab.download.urlopen', side_effect=interrupted), patch('khor_lab.download.time.sleep'):
            with self.assertRaises(URLError):
                _ranges('https://test.blob.core.windows.net/object', archive, len(payload), lambda *_: None, receipt)
        self.assertEqual(json.loads(receipt.read_text()), [0])
        def retry(request, **_kwargs):
            self.assertTrue(request.get_header('Range').startswith('bytes=4194304-'))
            return response(request)
        with patch('khor_lab.download.urlopen', side_effect=retry):
            _ranges('https://test.blob.core.windows.net/object', archive, len(payload), lambda *_: None, receipt)
        self.assertEqual(archive.read_bytes(), payload)

    def test_local_npm_cache_preserves_and_tracks_file_dependencies(self):
        from khor_lab.builders import local_dependencies
        gui = self.root / 'gui'
        stub = gui / 'vendor/stub'
        stub.mkdir(parents=True)
        (stub / 'index.js').write_text('first stub')
        (gui / 'package.json').write_text('{"name":"fixture"}')
        write_json(gui / 'package-lock.json', {'packages': {'node_modules/stub': {'link': True, 'resolved': 'vendor/stub'}}})
        installed = []
        def install(*_args, cwd, **_kwargs):
            installed.append((cwd / 'vendor/stub/index.js').read_text())
            (cwd / 'node_modules').mkdir()
        with patch('khor_lab.builders.run', side_effect=install):
            first = local_dependencies(gui, self.root / 'cache')
            self.assertEqual(local_dependencies(gui, self.root / 'cache'), first)
            (stub / 'index.js').write_text('updated stub')
            self.assertNotEqual(local_dependencies(gui, self.root / 'cache'), first)
        self.assertEqual(installed, ['first stub', 'updated stub'])

    def test_restart_serves_last_valid_cached_frontend_while_next_build_is_pending(self):
        home = self.root / 'manager'
        lab = Lab(load(self.root), home)
        version = home / 'versions/available'
        version.mkdir(parents=True)
        (version / 'index.html').write_text('cached App')
        source = dict(tree='tree', base_commit='base', snapshot_commit='snapshot', native_contract='native')
        write_json(version / 'lab-manifest.json', dict(schema=1, entry='production-app', **source))
        lab.value.update(available=True, version='available', source=source)
        lab.save()
        restored = Lab(load(self.root), home)
        self.assertTrue(restored.status()['available'])
        self.assertEqual(restored.status()['source'], source)
        self.assertNotEqual(restored.value['token'], lab.value['token'])

    def test_local_unverified_host_cannot_satisfy_a_remote_host_request(self):
        local = Lab(load(self.root, {'mode': 'full', 'scene': 'live', 'builder': 'local'}), self.root / 'local')
        remote = Lab(load(self.root, {'mode': 'full', 'scene': 'live', 'builder': 'remote'}), self.root / 'remote')
        built = {'managed_frontend': True, 'real_backend_verified': False}
        self.assertTrue(local.host_usable(built))
        self.assertFalse(remote.host_usable(built))
        self.assertTrue(remote.host_usable({**built, 'real_backend_verified': True}))

    def test_local_full_build_uses_its_existing_manager_without_binding_a_second_server(self):
        from khor_lab.check_native import probe
        home, frontend, output = self.root / 'manager', self.root / 'frontend', self.root / 'output'
        frontend.mkdir();output.mkdir()
        write_json(frontend / 'lab-manifest.json', {'tree': 'exact-tree', 'snapshot_commit': 'snapshot'})
        lab = Lab(load(self.root, {'mode': 'full', 'scene': 'live', 'builder': 'local'}), home)
        lab.value['source'] = {'tree': 'exact-tree'}
        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        server.lab = lab
        threading.Thread(target=server.serve_forever, daemon=True).start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        result = probe(self.root / 'not-built-yet.app', frontend, self.root, server.server_port, output)
        self.assertTrue(result['managed_frontend'])
        self.assertFalse(result['real_backend'])
        self.assertTrue((output / 'native-probe.json').exists())

    def test_retention_keeps_active_version_even_if_older(self):
        for name in ('a', 'b', 'c'):
            (self.root / name).mkdir()
        prune(self.root, 1, ['a'])
        self.assertTrue((self.root / 'a').exists())
        self.assertLessEqual(len(list(self.root.iterdir())), 2)


if __name__ == '__main__':
    unittest.main()
