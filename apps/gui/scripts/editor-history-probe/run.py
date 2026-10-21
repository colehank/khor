#!/usr/bin/env python3
"""Run the production macOS undo adapter in an isolated AppKit/WK app."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.request

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out', required=True, type=Path, help='External evidence/build directory')
args = parser.parse_args()
if sys.platform != 'darwin':
    print('SKIP editor-history-native: requires macOS AppKit/WebKit and a GUI session')
    sys.exit(0)
here = Path(__file__).resolve().parent
gui = here.parents[1]
repo = gui.parents[1]
out = args.out.resolve()
if out == repo or repo in out.parents:
    parser.error('--out must be outside the repository')
marker = out / '.khor-editor-history-probe'
if out.exists() and any(out.iterdir()) and not marker.exists():
    parser.error('--out is nonempty and is not owned by this probe')
out.mkdir(parents=True, exist_ok=True)
marker.touch()
packages = tomllib.loads((repo / 'Cargo.lock').read_text())['package']
versions = {name: next(p['version'] for p in packages if p['name'] == name)
            for name in ('objc2', 'objc2-foundation', 'block2')}
sources = [gui / 'src-tauri/src/native_edit.rs', gui / 'src-tauri/src/native_edit.js']
digest = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
before = {str(p): digest(p) for p in sources}
(out / 'src').mkdir(exist_ok=True)
(out / 'src/lib.rs').write_text((here / 'lib.rs').read_text().replace('__KHOR_NATIVE_EDIT_SOURCE__', str(sources[0])))
(out / 'Cargo.toml').write_text('[package]\nname="khor-native-edit-probe"\nversion="0.0.0"\nedition="2024"\n[lib]\ncrate-type=["staticlib"]\n[dependencies]\n' + ''.join(f'{name}="={version}"\n' for name, version in versions.items()))

def run(command, log):
    with (out / log).open('w') as stream:
        subprocess.run(command, cwd=out, stdout=stream, stderr=subprocess.STDOUT, check=True, timeout=180)

fixture = Path(tempfile.mkdtemp(prefix='.editor-history-native-', dir=gui))
vite = None
try:
    (fixture / 'index.html').write_text('<html><body><div id="root" style="height:100vh;display:flex;flex-direction:column"></div><script type="module" src="./probe.tsx"></script></body></html>')
    (fixture / 'probe.tsx').write_text('''import React,{useState} from 'react';
import {createRoot} from 'react-dom/client';
import {CodeEditor} from '/src/components/CodeEditor';
import '/src/styles/tokens.css';import '/src/styles/app.css';
function Probe(){const [text,setText]=useState('hello');return <CodeEditor path='/probe.txt' text={text} editable onText={setText}/>}
createRoot(document.getElementById('root')).render(<Probe/>);
''')
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    env = dict(os.environ, KHOR_FACE_GATE='1')
    with (out / 'vite.log').open('w') as log:
        vite = subprocess.Popen([str(gui / 'node_modules/.bin/vite'), '--port', str(port), '--strictPort'], cwd=gui, env=env, stdout=log, stderr=subprocess.STDOUT)
    url = f'http://localhost:{port}/{fixture.name}/index.html'
    local_http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    for _ in range(100):
        if vite.poll() is not None:
            raise RuntimeError('Private Vite exited; see vite.log')
        try:
            with local_http.open(url, timeout=1) as response:
                if response.status == 200:
                    break
        except (OSError, TimeoutError):
            time.sleep(.1)
    else:
        raise RuntimeError('Private Vite did not start')
    # Pin the same dependency versions as the repository; no network or
    # full Tauri/node build is needed to execute the native adapter.
    run(['cargo', 'build', '--offline', '--manifest-path', str(out / 'Cargo.toml'), '--target-dir', str(out / 'target')], 'rust-build.log')
    run(['swiftc', '-import-objc-header', str(here / 'bridge.h'), str(here / 'Probe.swift'), str(out / 'target/debug/libkhor_native_edit_probe.a'), '-framework', 'AppKit', '-framework', 'WebKit', '-o', str(out / 'Probe')], 'swift-build.log')
    result = out / 'results.json'
    result.unlink(missing_ok=True)
    run([str(out / 'Probe'), url, str(result)], 'run.log')
    changed = [str(p) for p in sources if before[str(p)] != digest(p)]
    (out / 'manifest.json').write_text(json.dumps({'production_sources': before, 'sources_changed_during_probe': changed, 'dependency_versions': versions, 'tauri_schedule_facade_stubbed': True}, indent=2))
    results = json.loads(result.read_text())
    if changed or not results['passed']:
        raise RuntimeError('Native regression failed; see results.json and run.log')
    print(f"editor-history-native: {len(results['checks'])} checks passed; {out}")
finally:
    if vite is not None:
        vite.terminate()
        try:
            vite.wait(timeout=5)
        except subprocess.TimeoutExpired:
            vite.kill()
            vite.wait()
    shutil.rmtree(fixture)
