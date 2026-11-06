import json
from pathlib import Path
import socket
import os
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.config import load
from khor_lab.hot import develop, development_plan, simulator


class DevelopmentTests(unittest.TestCase):
    def setUp(self):
        clean = patch.dict(os.environ, {k:v for k,v in os.environ.items() if not k.startswith('GIT_')}, clear=True)
        clean.start()
        self.addCleanup(clean.stop)
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)

    def test_source_only_preference_rejects_toolchain_execution(self):
        config = load(self.root, {'builder': 'remote'})
        with patch('khor_lab.hot.prepare') as prepare, self.assertRaisesRegex(ValueError, 'builder local'):
            develop(config)
        prepare.assert_not_called()

    def test_busy_port_is_rejected_before_installation(self):
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            listener.listen()
            config = load(self.root, {'port': listener.getsockname()[1]})
            with patch('khor_lab.hot.prepare') as prepare, self.assertRaisesRegex(ValueError, 'already occupied'):
                develop(config)
            prepare.assert_not_called()

    def test_development_is_mutable_and_full_app_is_isolated(self):
        config = load(self.root, {'mode': 'full', 'scene': 'live'})
        plan = development_plan(config, self.root / 'home')
        self.assertTrue(plan['runtime']['mutable'])
        self.assertNotEqual(plan['tauri']['identifier'], 'io.github.colehank.khor')
        self.assertIn('--no-watch', plan['command'])
        self.assertEqual(plan['tauri']['build']['devUrl'], plan['url'])

    def test_ios_selects_only_an_available_named_ios_simulator(self):
        devices = {'devices': {'com.apple.CoreSimulator.SimRuntime.iOS-26-0': [
            {'name': 'iPhone test', 'udid': 'iphone-udid', 'isAvailable': True},
            {'name': 'iPhone unavailable', 'udid': 'old', 'isAvailable': False}],
            'com.apple.CoreSimulator.SimRuntime.tvOS-26-0': [
            {'name': 'Apple TV', 'udid': 'tv-udid', 'isAvailable': True}]}}
        with patch('khor_lab.hot.platform.system', return_value='Darwin'), \
             patch('khor_lab.hot.subprocess.check_output', return_value=json.dumps(devices)):
            self.assertEqual(simulator('iPhone test'), 'iphone-udid')
            with self.assertRaisesRegex(ValueError, 'Choose an available'):
                simulator('tv-udid')
            with self.assertRaisesRegex(ValueError, 'Choose an available'):
                simulator(None)

    def test_native_generation_preserves_checkout_and_reuses_host_after_css_edits(self):
        from khor_lab.workspace import materialize
        from khor_lab.builders import run as actual_run
        root = self.root / 'source'
        gui = root / 'apps/gui'
        (gui / 'src-tauri').mkdir(parents=True)
        (gui / 'src/gen').mkdir(parents=True)
        (gui / 'node_modules').mkdir()
        (gui / 'src/gen/catalog.ts').write_text('generated')
        manifest = gui / 'src-tauri/Cargo.toml'
        original = '[features]\ndefault = ["custom-protocol", "devtools"]\n'
        manifest.write_text(original)
        css = gui / 'app.css'
        css.write_text('initial')
        (root / 'lib.rs').write_text('fn initial() {}')
        (root / '.gitignore').write_text('node_modules/\nsrc/gen/\n')
        actual_run('git', 'init', '-q', str(root))
        actual_run('git', '-C', str(root), 'add', '.')
        actual_run('git', '-C', str(root), '-c', 'user.name=Lab', '-c', 'user.email=lab@localhost', 'commit', '-qm', 'Input')
        config = load(root, {'mode':'full', 'scene':'live'})
        home = self.root / 'owned-cache'
        plan = development_plan(config, home)
        def tools(*args, **kwargs):
            return actual_run(*args, **kwargs) if args[0] == 'git' else ''
        before = Path.cwd()
        with patch('khor_lab.builders.run', side_effect=tools):
            native = materialize(config, home, {}, plan)
            self.assertEqual(Path.cwd(), before)
            self.assertEqual(manifest.read_text(), original)
            self.assertNotIn('"custom-protocol"', (native / 'apps/gui/src-tauri/Cargo.toml').read_text())
            css.write_text('updated CSS')
            self.assertEqual(materialize(config, home, {}, plan), native)
            (root / 'lib.rs').write_text('fn native_changed() {}')
            self.assertNotEqual(materialize(config, home, {}, plan), native)


if __name__ == '__main__':
    unittest.main()
