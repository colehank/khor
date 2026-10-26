"""Run the actual deploy command with scratch-only SSH/rsync/build adapters."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

import test_release_mirror as mirror_fixture

ROOT = mirror_fixture.ROOT
ASSETS = mirror_fixture.ASSETS

ADAPTER = r'''#!/usr/bin/env python3
import json, os, shlex, shutil, subprocess, sys
from pathlib import Path
kind = Path(sys.argv[0]).name
args = sys.argv[1:]
with open(os.environ['DEPLOY_CALLS'], 'a') as log: log.write(json.dumps([kind, args])+'\n')
if kind == 'npm': sys.exit(0)
if kind == 'ssh':
    if args[0] == '--': args.pop(0)
    assert args.pop(0) == 'scratch-peer'
    command = ' '.join(args)
    # All remote commands target only this test's private temporary filesystem.
    root = os.environ['KHOR_SITE_ROOT']
    assert any(word.startswith(root) for word in shlex.split(command)), command
    if command.startswith('rm ') and os.environ.get('FAIL_REMOTE_CLEANUP'): sys.exit(255)
    sys.exit(subprocess.run(['/bin/sh','-c', command], input=sys.stdin.buffer.read()).returncode)
if args == ['--help']:
    print('--protect-args --exclude --delete'); sys.exit(0)
paths = [arg for arg in args if not arg.startswith('-')]
source, destination = paths[-2:]
assert destination.startswith('scratch-peer:')
destination = destination.split(':',1)[1]
assert destination.startswith(os.environ['KHOR_SITE_ROOT']+'/')
if source.endswith('.mirror/') and '--delete' in args and os.environ.get('FAIL_MIRROR_COPY'):
    sys.exit(23)
if '.incoming.' in destination and os.environ.get('FAIL_MIRROR_COPY'): sys.exit(23)
source, destination = Path(source), Path(destination)
destination.mkdir(parents=True, exist_ok=True)
def copy_tree(source, destination, top=False):
    destination.mkdir(parents=True, exist_ok=True)
    if '--delete' in args:
        for item in destination.iterdir():
            if top and item.name == 'dl' and ('--exclude=/dl/' in args or '/dl/' in args): continue
            if not (source/item.name).exists():
                if item.is_dir(): shutil.rmtree(item)
                else: item.unlink()
    for item in source.iterdir():
        if item.is_dir(): copy_tree(item, destination/item.name)
        else: shutil.copy2(item,destination/item.name)
copy_tree(source, destination, True)
if '.incoming.' in str(destination) and os.environ.get('CORRUPT_MIRROR_COPY'):
    (destination/'khor-linux-x86_64.gz').write_bytes(b'corrupt transfer')
'''


class SitePublicationTest(unittest.TestCase):
    def setUp(self):
        # Reuse only network fixture setup; every operation still invokes the
        # actual checked-in production deploy and mirror scripts.
        self.fixture = mirror_fixture.ReleaseMirrorTest()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.base = self.fixture.base
        self.repo = self.base / 'repo'
        (self.repo / 'scripts').mkdir(parents=True)
        for name in ['mirror-sync.sh', 'mirror-lib.sh', 'install.sh', 'onset-lib.sh', 'install-components.sh']:
            source = ROOT / 'scripts' / name
            if source.exists(): shutil.copy2(source, self.repo / 'scripts' / name)
        deploy = self.repo / 'apps/site/deploy'
        deploy.mkdir(parents=True)
        shutil.copy2(ROOT / 'apps/site/deploy/push.sh', deploy / 'push.sh')
        dist = self.repo / 'apps/site/dist'
        dist.mkdir()
        (dist / 'index.html').write_text('new page')
        self.remote = self.base / 'remote'
        old = self.remote / 'dl/releases/v1.2.2'
        old.mkdir(parents=True)
        (old / 'pinned-reader').write_bytes(b'old archive stays readable')
        (self.remote / 'dl/VERSION').write_text('v1.2.2\n')
        self.calls = self.base / 'deploy-calls'
        self.env = dict(self.fixture.env, KHOR_SITE_ROOT=str(self.remote), DEPLOY_CALLS=str(self.calls))
        for tool in ['ssh', 'rsync', 'npm']:
            file = self.fixture.tools / tool
            file.write_text(ADAPTER)
            file.chmod(0o755)

    def run_deploy(self):
        self.fixture.route_file.write_text(json.dumps(self.fixture.routes))
        return subprocess.run(['/bin/sh', str(self.repo/'apps/site/deploy/push.sh'), 'scratch-peer'],
                              env=self.env, capture_output=True, text=True, timeout=40)

    def test_complete_generation_precedes_pointer_and_retains_old_readers(self):
        result = self.run_deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.remote/'dl/VERSION').read_text(), 'v1.2.3\n')
        self.assertEqual((self.remote/'dl/releases/v1.2.2/pinned-reader').read_bytes(), b'old archive stays readable')
        for asset in ASSETS:
            self.assertTrue((self.remote/'dl/releases/v1.2.3'/f'{asset}.gz').is_file())
        self.assertEqual((self.remote/'index.html').read_text(), 'new page')

    def test_failed_copy_keeps_old_pointer_and_generation(self):
        self.env['FAIL_MIRROR_COPY'] = '1'
        result = self.run_deploy()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.remote/'dl/VERSION').read_text(), 'v1.2.2\n')
        self.assertEqual((self.remote/'dl/releases/v1.2.2/pinned-reader').read_bytes(), b'old archive stays readable')
        self.assertFalse((self.remote/'dl/releases/v1.2.3').exists())

    def test_failed_copy_retains_failure_when_remote_cleanup_is_unconfirmed(self):
        self.env['FAIL_MIRROR_COPY'] = '1'
        self.env['FAIL_REMOTE_CLEANUP'] = '1'
        result = self.run_deploy()
        self.assertEqual(result.returncode, 23, 'cleanup refusal must not replace the original copy failure')
        self.assertIn('remote upload stage cleanup could not be confirmed', result.stderr)
        self.assertEqual((self.remote/'dl/VERSION').read_text(), 'v1.2.2\n')
        self.assertEqual((self.remote/'dl/releases/v1.2.2/pinned-reader').read_bytes(), b'old archive stays readable')
        self.assertFalse((self.remote/'dl/releases/v1.2.3').exists())
        self.assertEqual(len(list((self.remote/'dl').glob('.incoming.*'))), 1,
                         'unconfirmed cleanup must retain the actual stage in this failure fixture')
        calls = [json.loads(line) for line in self.calls.read_text().splitlines()]
        self.assertTrue(any(kind == 'ssh' and args[-1].startswith('rm ') for kind, args in calls))

    def test_corrupt_transfer_never_changes_the_remote_pointer(self):
        self.env['CORRUPT_MIRROR_COPY'] = '1'
        result = self.run_deploy()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('checksum mismatch', result.stderr)
        self.assertEqual((self.remote/'dl/VERSION').read_text(), 'v1.2.2\n')
        self.assertFalse((self.remote/'dl/releases/v1.2.3').exists())
        self.assertEqual(list((self.remote/'dl').glob('.incoming.*')), [])

    def test_remote_existing_generation_cannot_be_replaced(self):
        existing = self.remote/'dl/releases/v1.2.3'
        existing.mkdir()
        (existing/'install.sh').write_text('existing immutable bytes')
        result = self.run_deploy()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.remote/'dl/VERSION').read_text(), 'v1.2.2\n')
        self.assertEqual((existing/'install.sh').read_text(), 'existing immutable bytes')
        self.assertEqual(list((self.remote/'dl').glob('.incoming.*')), [])

    def test_literal_paths_with_spaces_are_supported(self):
        moved = self.base/'remote with spaces'
        self.remote.rename(moved)
        self.remote = moved
        self.env['KHOR_SITE_ROOT'] = str(moved)
        result = self.run_deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((moved/'dl/VERSION').read_text(), 'v1.2.3\n')

    def test_literal_paths_with_quotes_are_supported(self):
        moved = self.base/"remote with 'quoted' path"
        self.remote.rename(moved)
        self.remote = moved
        self.env['KHOR_SITE_ROOT'] = str(moved)
        result = self.run_deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((moved/'dl/VERSION').read_text(), 'v1.2.3\n')


if __name__ == '__main__':
    unittest.main()
