"""Verify the actual Tauri host loads the managed App and can invoke its real backend."""
from http.server import ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
from urllib.parse import parse_qs, urlsplit

from .config import load
from .server import Handler, Lab
from .artifacts import write_json


def probe(app, frontend, root, port, output):
    ready = threading.Event()
    client = json.loads((frontend / 'lab-manifest.json').read_text())
    class ProbeHandler(Handler):
        def do_GET(self):
            url = urlsplit(self.path)
            if url.path == '/lab/native-probe':
                query = parse_qs(url.query)
                if all(query.get(key) == ['1'] for key in ('native', 'backend', 'rendered')):
                    ready.set()
                return self.respond(200, True)
            return super().do_GET()

        def respond(self, code, data, content_type='application/json'):
            if content_type.startswith('text/html'):
                script = b'''<script>
                let sent = false;
                const check = async () => {
                  if (sent || !document.querySelector('[data-list]') || !window.__TAURI_INTERNALS__) return;
                  sent = true;
                  try {
                    const devices = await window.__TAURI_INTERNALS__.invoke('devices');
                    if (Array.isArray(devices) && devices.some(d => d.me)) fetch('/lab/native-probe?native=1&backend=1&rendered=1');
                  } catch (error) { console.error('Native probe failed', error); }
                };
                new MutationObserver(check).observe(document.documentElement, {subtree:true,childList:true});
                check();
                </script>'''
                data += script
            super().respond(code, data, content_type)

    with tempfile.TemporaryDirectory(prefix='khor-native-probe-') as temporary:
        home = Path(temporary)
        shutil.copytree(frontend, home / 'versions/probe')
        lab = Lab(load(root, {'mode': 'full', 'builder': 'local', 'scene': 'live', 'port': port}), home)
        lab.value.update(version='probe', available=True, source=client)
        server = ThreadingHTTPServer(('127.0.0.1', port), ProbeHandler)
        server.lab = lab
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        environment = dict(os.environ, KHOR_HOME=str(home / 'store'), KHOR_NAME='Khor native probe', KHOR_WEB_PORT='0')
        environment.pop('KHOR_SESSION', None)
        with (output / 'native-probe.log').open('wb') as log:
            process = subprocess.Popen([str(app / 'Contents/MacOS/KhorDeveloperLab')], env=environment, stdout=log, stderr=log)
            try:
                if not ready.wait(timeout=40):
                    raise ValueError('Native host did not load the managed App and invoke its real backend; inspect native-probe.log')
            finally:
                process.terminate()
                try:
                    process.wait(timeout=8)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
                server.shutdown()
                server.server_close()
        result = {'managed_frontend': True, 'real_backend': True, 'tree': client['tree'], 'snapshot_commit': client['snapshot_commit']}
        write_json(output / 'native-probe.json', result)
        return result
