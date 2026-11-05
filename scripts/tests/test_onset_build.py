"""Exercise signed SSH deployment with isolated homes and a local release mirror."""
import gzip
import hashlib
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import unittest

ROOT = Path(__file__).resolve().parents[2]
TOOL = r'''#!/usr/bin/env python3
import json, os, shlex, shutil, subprocess, sys
from pathlib import Path
args = sys.argv[1:]
root = Path(os.environ['ONSET_FIXTURE_ROOT'])
with (root/'commands.jsonl').open('a') as log:
    log.write(json.dumps(dict(tool=Path(sys.argv[0]).name,args=args))+'\n')
if Path(sys.argv[0]).name == 'scp':
    target = shlex.split(args[-1].split(':',1)[1])[0]
    assert Path(target).is_relative_to(root/'remote')
    shutil.copyfile(args[-2],target)
    sys.exit(0)
if '-O' in args: sys.exit(0)
command = args[-1]
if command == 'sh -s':
    script = sys.stdin.read()
    transfer = "export KHOR_ASSET_FILE=" in script
    (root/('transfer.sh' if transfer else 'direct.sh')).write_text(script)
    if not transfer and os.environ.get('ONSET_DIRECT_STATUS'):
        sys.exit(int(os.environ['ONSET_DIRECT_STATUS']))
    environment = dict(os.environ,HOME=str(root/'remote'))
    sys.exit(subprocess.run(['/bin/sh','-s'],input=script,text=True,env=environment).returncode)
if command.startswith('printf "%s-%s"'): print('Linux-x86_64'); sys.exit(0)
if command.startswith('mktemp -d '):
    landing=root/'remote'/"khor-onset.A1b2C3d4"
    landing.mkdir();print(landing);sys.exit(0)
if command.startswith('rm -rf -- '):
    path=Path(shlex.split(command)[-1]);assert path.is_relative_to(root/'remote')
    shutil.rmtree(path);sys.exit(0)
raise SystemExit('unexpected remote command: '+command)
'''
MAIN = r'''khor_install_main() {
    [ -z "${KHOR_RUNTIME_INSTALL_SOURCE:-}" ] || exit 89
    [ -z "${KHOR_INSTALL_SOURCE:-}" ] || exit 89
    printf '%s\n%s\n%s\n' "$KHOR_VERSION" "${KHOR_NO_SERVE:-}" "${KHOR_ASSET_SHA256:-}" > "$HOME/result"
    if [ -n "${KHOR_ASSET_FILE:-}" ]; then
        [ -f "$KHOR_ASSET_FILE" ] && [ -f "$KHOR_RELEASE_FILE" ] && [ -f "$KHOR_RELEASE_SIGNATURE_FILE" ] || exit 91
        [ "$(khor_file_digest "$KHOR_ASSET_FILE")" = "$KHOR_ASSET_SHA256" ] || exit 92
        cp "$KHOR_RELEASE_FILE" "$HOME/manifest"
        cp "$KHOR_RELEASE_SIGNATURE_FILE" "$HOME/signature"
    fi
}
'''


class OnsetBuildTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="khor onset 'signed-")
        self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)
        scripts=self.root/'repo/scripts';scripts.mkdir(parents=True)
        for name in ['onset.sh','onset-lib.sh','install-components.sh']:
            shutil.copyfile(ROOT/'scripts'/name,scripts/name)
        components=(scripts/'install-components.sh').read_text()
        (scripts/'install.sh').write_text('#!/bin/sh\n# KHOR_INSTALL_DECLARATIONS_BEGIN\n'+components+MAIN+'# KHOR_INSTALL_DECLARATIONS_END\nkhor_install_main "$@"\n')
        self.script=scripts/'onset.sh'
        tools=self.root/'tools';tools.mkdir()
        (self.root/'remote').mkdir()
        for name in ['ssh','scp']:
            (tools/name).write_text(TOOL);(tools/name).chmod(0o755)
        self.env=dict(os.environ,PATH=str(tools)+':'+os.environ['PATH'],ONSET_FIXTURE_ROOT=str(self.root),TMPDIR=str(self.root))
        for name in ['KHOR_SSH_OPTS','KHOR_MIRROR','KHOR_ASSET_FILE','KHOR_ASSET_SHA256','KHOR_REPO']:
            self.env.pop(name,None)
        for name in ['HTTP_PROXY','http_proxy','HTTPS_PROXY','https_proxy','ALL_PROXY','all_proxy']:
            self.env.pop(name,None)

    def run_onset(self,*options):
        return subprocess.run(['/bin/sh',str(self.script),'fixture-host',*options],env=self.env,text=True,capture_output=True,timeout=20)

    def calls(self):
        path=self.root/'commands.jsonl'
        return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []

    def mirror(self):
        assets=self.root/'mirror/releases/v1.2.3';assets.mkdir(parents=True)
        archive=gzip.compress(b'\x7fELFtransport fixture, never executed')
        (assets/'khor-linux-x86_64.gz').write_bytes(archive)
        self.digest=hashlib.sha256(archive).hexdigest()
        (assets/'khor-linux-x86_64.gz.sha256').write_text(self.digest+'  khor-linux-x86_64.gz\n')
        (assets/'khor-release.json').write_text('{"fixture":"signed metadata transported verbatim"}')
        (assets/'khor-release.json.sig').write_text('fixture-signature')
        directory=str(self.root/'mirror')
        class Handler(SimpleHTTPRequestHandler):
            def __init__(self,*args,**kwargs):super().__init__(*args,directory=directory,**kwargs)
            def log_message(self,*args):pass
        server=ThreadingHTTPServer(('127.0.0.1',0),Handler)
        thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
        self.addCleanup(server.server_close);self.addCleanup(server.shutdown)
        self.env['KHOR_MIRROR']=f'http://127.0.0.1:{server.server_port}'
        return assets

    def test_default_uses_target_installer_without_upload_or_local_build(self):
        result=self.run_onset()
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual((self.root/'remote/result').read_text().splitlines()[0],'latest')
        self.assertFalse(any(call['tool']=='scp' for call in self.calls()))

    def test_fetch_failure_transfers_exact_archive_and_metadata_then_cleans_owned_landing(self):
        self.mirror();self.env['ONSET_DIRECT_STATUS']='75'
        result=self.run_onset('--release=v1.2.3','--no-serve')
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual((self.root/'remote/result').read_text().splitlines(),['v1.2.3','1',self.digest])
        self.assertEqual((self.root/'remote/signature').read_text(),'fixture-signature')
        self.assertEqual(len([call for call in self.calls() if call['tool']=='scp']),3)
        self.assertFalse((self.root/'remote/khor-onset.A1b2C3d4').exists())

    def test_non_download_failure_never_retries_or_uploads(self):
        for code in [1,78,255]:
            with self.subTest(code=code):
                self.env['ONSET_DIRECT_STATUS']=str(code)
                result=self.run_onset('--release=v1.2.3')
                self.assertEqual(result.returncode,code,result.stderr)
                self.assertFalse(any(call['tool']=='scp' for call in self.calls()))

    def test_bad_archive_stops_before_upload_or_any_replacement(self):
        assets=self.mirror();self.env['ONSET_DIRECT_STATUS']='75'
        (assets/'khor-linux-x86_64.gz').write_bytes(b'wrong archive')
        result=self.run_onset('--release=v1.2.3')
        self.assertNotEqual(result.returncode,0)
        self.assertFalse(any(call['tool']=='scp' for call in self.calls()))
        self.assertFalse((self.root/'remote/result').exists())

    def test_session_runtime_hints_cannot_select_an_old_installer(self):
        self.env.update(KHOR_RUNTIME_INSTALL_SOURCE='/retired/khor',KHOR_INSTALL_SOURCE='/retired/khor')
        result=self.run_onset()
        self.assertEqual(result.returncode,0,result.stderr)

    def test_failed_metadata_transfer_cleans_only_its_owned_directory(self):
        self.mirror();self.env['ONSET_DIRECT_STATUS']='75'
        tool=self.root/'tools/scp'
        source=tool.read_text().replace("shutil.copyfile(args[-2],target)","\n    if target.endswith('.sig'): sys.exit(17)\n    shutil.copyfile(args[-2],target)")
        tool.write_text(source)
        result=self.run_onset('--release=v1.2.3')
        self.assertEqual(result.returncode,17,result.stderr)
        self.assertFalse((self.root/'remote/khor-onset.A1b2C3d4').exists())
        self.assertFalse((self.root/'transfer.sh').exists())

    def test_raw_binary_install_modes_fail_before_contacting_target(self):
        for option in ['--musl','--bin-on-target=$HOME/.local/bin/raw']:
            with self.subTest(option=option):
                result=self.run_onset(option)
                self.assertEqual(result.returncode,2)
                self.assertEqual(self.calls(),[])

    def test_dry_run_does_not_contact_target_or_compile(self):
        result=self.run_onset('--dry-run')
        self.assertEqual(result.returncode,0,result.stderr)
        self.assertEqual(self.calls(),[])

    def test_out_of_sync_installer_is_rejected_before_remote_contact(self):
        (self.script.parent/'install.sh').write_text('# invalid generated installer\n')
        result=self.run_onset()
        self.assertNotEqual(result.returncode,0)
        self.assertFalse((self.root/'direct.sh').exists())


if __name__=='__main__':unittest.main()
