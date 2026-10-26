"""Check actual bundle entry orchestration with an inert Bun subprocess."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
NODE = shutil.which('node')
BUN = r'''#!/usr/bin/env python3
import json,os,sys
from pathlib import Path
Path(os.environ['BUNDLE_CALLS']).write_text(json.dumps(sys.argv[1:]))
status = int(os.environ.get('BUNDLE_EXIT','0'))
if not status:
    output = Path(sys.argv[sys.argv.index('--outfile')+1])
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_bytes(b'fixture compiled component')
sys.exit(status)
'''


@unittest.skipUnless(NODE,'bundle entry requires Node')
class AgentBundleBuildTest(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix='khor-bundle-entry-')
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        shutil.copy2(ROOT/'apps/agent/build.mjs',self.root/'build.mjs')
        (self.root/'scripts').mkdir()
        (self.root/'scripts/providers.ts').write_text("process.exit(Number(process.env.CATALOG_EXIT || '0'));\n")
        tools=self.root/'tools';tools.mkdir()
        (tools/'bun').write_text(BUN);(tools/'bun').chmod(0o755)
        self.calls=self.root/'calls.json'
        self.output=self.root/'owned output/runtime'
        self.env=dict(os.environ,PATH=f"{tools}:{os.environ['PATH']}",BUNDLE_CALLS=str(self.calls))

    def build(self,target,explicit=True):
        args=[NODE,str(self.root/'build.mjs'),target]
        if explicit:args.append(str(self.output))
        return subprocess.run(args,env=self.env,cwd=self.root,text=True,capture_output=True,timeout=10)

    def test_explicit_output_is_owned_without_writing_shared_dist(self):
        result=self.build('x86_64-unknown-linux-gnu')
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(self.output.read_bytes(),b'fixture compiled component')
        self.assertFalse((self.root/'dist').exists())
        args=json.loads(self.calls.read_text())
        self.assertIn('--target=bun-linux-x64-baseline',args)
        self.assertIn('--no-compile-autoload-bunfig',args)
        self.assertIn('--no-compile-autoload-dotenv',args)

    def test_musl_architectures_select_their_matching_runtime(self):
        for target,bun in [('x86_64-unknown-linux-musl','bun-linux-x64-baseline-musl'),
                           ('aarch64-unknown-linux-musl','bun-linux-arm64-musl')]:
            with self.subTest(target=target):
                result=self.build(target)
                self.assertEqual(result.returncode,0,result.stderr)
                self.assertIn('--target='+bun,json.loads(self.calls.read_text()))

    def test_provider_catalog_failure_never_invokes_bun(self):
        self.env['CATALOG_EXIT']='7'
        result=self.build('x86_64-unknown-linux-gnu')
        self.assertNotEqual(result.returncode,0)
        self.assertFalse(self.calls.exists())
        self.assertFalse(self.output.exists())

    def test_bun_failure_is_returned_without_a_component(self):
        self.env['BUNDLE_EXIT']='9'
        result=self.build('x86_64-unknown-linux-gnu')
        self.assertEqual(result.returncode,9)
        self.assertFalse(self.output.exists())

    def test_default_output_remains_in_project_dist(self):
        result=self.build('aarch64-unknown-linux-gnu',explicit=False)
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertTrue((self.root/'dist/khor-agent').is_file())


if __name__=='__main__':
    unittest.main()
