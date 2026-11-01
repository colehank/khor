"""Exercise onset build ordering without compiling or contacting a machine."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
TOOL = r'''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
name = Path(sys.argv[0]).name
args = sys.argv[1:]
root = Path(os.environ['ONSET_FIXTURE_ROOT'])
bundle = os.environ.get('KHOR_AGENT_BUNDLE')
record = {'tool':name, 'args':args}
if name == 'cargo':
    record['bundle'] = bundle
    record['bundle_exists'] = bool(bundle and Path(bundle).is_file() and Path(bundle).stat().st_size)
with open(os.environ['ONSET_FIXTURE_LOG'],'a') as log: log.write(json.dumps(record)+'\n')
if name == 'ssh':
    if '-O' in args: sys.exit(0)
    command = args[-1]
    if command == 'uname -m': print(os.environ.get('ONSET_FIXTURE_ARCH','x86_64')); sys.exit(0)
    if command == 'printf %s "$HOME"': print(root/'remote-home'); sys.exit(0)
    if command == 'command -v nvidia-smi >/dev/null 2>&1': sys.exit(1)
    if command.startswith('mkdir '): sys.exit(0)
    # No remote script is ever executed; unexpected installation stops here.
    sys.exit(79)
if name == 'scp': sys.exit(0)
if name == 'node':
    if os.environ.get('ONSET_BUNDLE_FAILURE'): sys.exit(34)
    if os.environ.get('ONSET_BUNDLE_EMPTY'): sys.exit(0)
    output = Path(args[2]) if len(args)>2 else root/'apps/agent/dist/khor-agent'
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_bytes(b'fixture standalone agent')
    sys.exit(0)
if name == 'npm': sys.exit(0)
if name == 'cargo':
    if os.environ.get('ONSET_CARGO_FAILURE'): sys.exit(17)
    target = args[args.index('--target')+1].split('.')[0]
    directory = Path(os.environ.get('CARGO_TARGET_DIR', str(root/'target')))
    binary = directory/target/'release/khor'
    binary.parent.mkdir(parents=True,exist_ok=True)
    binary.write_bytes(b'\x7fELFfixture executable bytes, never run')
    sys.exit(0)
if name == 'file': print(args[-1]+': ELF 64-bit dynamically linked'); sys.exit(0)
raise SystemExit('unexpected fixture tool')
'''


class OnsetBuildTest(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix='khor-onset-build-')
        self.addCleanup(self.scratch.cleanup)
        self.base = Path(self.scratch.name)
        self.repo = self.base/'repo'
        scripts = self.repo/'scripts'
        scripts.mkdir(parents=True)
        for name in ['onset.sh','onset-lib.sh','install.sh','install-components.sh']:
            shutil.copy2(ROOT/'scripts'/name,scripts/name)
        (self.repo/'apps/agent').mkdir(parents=True)
        shutil.copy2(ROOT/'apps/agent/build.mjs',self.repo/'apps/agent/build.mjs')
        self.tools = self.base/'tools'
        self.tools.mkdir()
        for name in ['ssh','scp','cargo','node','npm','file']:
            file = self.tools/name
            file.write_text(TOOL)
            file.chmod(0o755)
        self.log = self.base/'commands.jsonl'
        self.env = dict(os.environ,PATH=f"{self.tools}:{os.environ['PATH']}",
                        TMPDIR=str(self.base),ONSET_FIXTURE_ROOT=str(self.repo),ONSET_FIXTURE_LOG=str(self.log))
        for name in ['KHOR_AGENT_BUNDLE','KHOR_SSH_OPTS','CARGO_TARGET_DIR']:
            self.env.pop(name,None)

    def run_onset(self,*options):
        return subprocess.run(['/bin/sh',str(self.repo/'scripts/onset.sh'),'fixture-host',*options],
                              env=self.env,text=True,capture_output=True,timeout=10)

    def calls(self,tool=None):
        calls = [json.loads(line) for line in self.log.read_text().splitlines()] if self.log.exists() else []
        return [call for call in calls if not tool or call['tool']==tool]

    def assert_no_remote_mutation(self):
        self.assertEqual(self.calls('scp'),[])
        for call in self.calls('ssh'):
            if '-O' in call['args']:
                continue
            self.assertIn(call['args'][-1],['uname -m','printf %s "$HOME"',
                          'command -v nvidia-smi >/dev/null 2>&1'],call)

    def test_source_build_embeds_the_prepared_target_agent(self):
        result = self.run_onset('--dry-run')
        self.assertEqual(result.returncode,0,result.stderr)
        cargo = self.calls('cargo')
        self.assertEqual(len(cargo),1)
        self.assertTrue(cargo[0]['bundle_exists'],'compiler received no standalone Pi component')
        self.assertIn('--locked',cargo[0]['args'])
        node = self.calls('node')
        self.assertEqual(len(node),1)
        self.assertEqual(node[0]['args'][1],'x86_64-unknown-linux-gnu')
        self.assert_no_remote_mutation()

    def test_failed_agent_preparation_never_builds_or_modifies_remote_files(self):
        self.env['ONSET_BUNDLE_FAILURE']='1'
        result = self.run_onset('--dry-run')
        self.assertNotEqual(result.returncode,0)
        self.assertEqual(self.calls('cargo'),[])
        self.assert_no_remote_mutation()

    def test_failed_rust_build_never_modifies_remote_files(self):
        self.env['ONSET_CARGO_FAILURE']='1'
        result = self.run_onset('--dry-run')
        self.assertEqual(result.returncode,17,result.stderr)
        self.assert_no_remote_mutation()

    def test_existing_target_binary_dry_run_only_reads_target_information(self):
        result = self.run_onset('--bin-on-target=$HOME/.local/bin/ready-khor','--dry-run')
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(self.calls('cargo'),[])
        self.assert_no_remote_mutation()

    def test_build_uses_the_configured_cargo_target_directory(self):
        self.env['CARGO_TARGET_DIR']=str(self.base/'configured-target')
        result = self.run_onset('--dry-run')
        self.assertEqual(result.returncode,0,result.stderr)
        self.assert_no_remote_mutation()

    def test_empty_agent_output_cannot_reuse_a_previous_bundle(self):
        self.env['ONSET_BUNDLE_EMPTY']='1'
        result=self.run_onset('--dry-run')
        self.assertNotEqual(result.returncode,0)
        self.assertEqual(self.calls('cargo'),[])
        self.assert_no_remote_mutation()

    def test_arm64_build_uses_the_same_architecture_for_agent_and_host(self):
        self.env['ONSET_FIXTURE_ARCH']='aarch64'
        result=self.run_onset('--dry-run')
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(self.calls('node')[0]['args'][1],'aarch64-unknown-linux-gnu')
        cargo=self.calls('cargo')[0]['args']
        self.assertEqual(cargo[cargo.index('--target')+1],'aarch64-unknown-linux-gnu.2.31')
        self.assert_no_remote_mutation()

    def test_relative_target_directory_is_relative_to_the_workspace(self):
        self.env['CARGO_TARGET_DIR']='owned-target'
        result=self.run_onset('--dry-run')
        self.assertEqual(result.returncode,0,result.stderr)
        self.assert_no_remote_mutation()

    def assert_invalid_serve_up_is_rejected(self,template,dry):
        (self.repo/'scripts/install.sh').write_text(template)
        result=self.run_onset(*(['--dry-run'] if dry else []))
        self.assertNotEqual(result.returncode,0,'invalid serve-up must fail local preflight')
        self.assert_no_remote_mutation()

    def test_missing_serve_up_fails_dry_run_before_remote_mutation(self):
        self.assert_invalid_serve_up_is_rejected('#!/bin/sh\n# no serve-up template\n',True)

    def test_missing_serve_up_fails_live_before_remote_mutation(self):
        self.assert_invalid_serve_up_is_rejected('#!/bin/sh\n# no serve-up template\n',False)

    def test_malformed_serve_up_fails_dry_run_before_remote_mutation(self):
        self.assert_invalid_serve_up_is_rejected('khor_publish_file "$UP" 755 <<\'SERVEUP\'\nif\nSERVEUP\n',True)

    def test_malformed_serve_up_fails_live_before_remote_mutation(self):
        self.assert_invalid_serve_up_is_rejected('khor_publish_file "$UP" 755 <<\'SERVEUP\'\nif\nSERVEUP\n',False)


if __name__ == '__main__':
    unittest.main()
