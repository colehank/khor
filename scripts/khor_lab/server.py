"""One loopback Lab manager, owning its builds and optional native process."""
from functools import partial
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import mimetypes
import os
from pathlib import Path
import platform
import secrets
import shutil
import subprocess
import threading
import time
from urllib.parse import unquote, urlsplit

from . import builders
from .artifacts import prune, unpack, write_json, validate


class Lab:
    def __init__(self, config, home):
        self.config, self.home = config, home
        prior_file = home / 'manager.json'
        prior = json.loads(prior_file.read_text()) if prior_file.exists() else {}
        self.lock = threading.RLock()
        self.cancel = threading.Event()
        self.native = None
        self.build = None
        self.last_progress = 0.0
        if config.pop('resume', False):
            run = prior.get('run')
            if not run:
                raise ValueError('No prior Lab run to acquire')
            records = [json.loads(path.read_text()) for path in (home / 'versions').glob('*/request.json')]
            info = next((record for record in records if record['request_id'] in run['displayTitle']), None)
            if not info:
                raise ValueError('Prior source receipt is missing; cannot validate its artifacts')
            self.config['resume_request'] = {'source': info, 'run': run}
        self.config['native_required'] = True
        self.value = dict(schema=1, pid=os.getpid(), root=config['root'], token=secrets.token_hex(32),
                          mode=config['mode'], builder=config['builder'], scene=config['scene'],
                          revision=secrets.token_hex(8), available=False, checks='not-run', user_accepted=False,
                          url=f'http://127.0.0.1:{config["port"]}/app/', building=False)
        if prior.get('available') and prior.get('mode') == config['mode']:
            previous = home / 'versions' / prior['version']
            try:
                manifest = validate(previous, prior['source'])
                self.value.update(version=prior['version'], available=True, source=prior['source'], manifest=manifest,
                                  snapshot_url=self.value['url'].replace('/app/', '/snapshots/' + prior['version'] + '/app/'))
            except (OSError, ValueError, KeyError):
                pass
        self.save()

    def save(self):
        write_json(self.home / 'manager.json', self.value)

    def status(self):
        with self.lock:
            return {k: v for k, v in self.value.items() if k != 'token'}

    def update(self):
        with self.lock:
            if self.value['building']:
                return self.status()
            if self.config['mode'] == 'full':
                from .contract import digest
                host = self.host_directory(digest(Path(self.config['root'])))
                self.config['native_required'] = not (host / 'native-manifest.json').is_file()
            self.value.update(building=True, error=None, checks='pending')
            self.save()
            self.build = threading.Thread(target=self.build_latest, daemon=True)
            self.build.start()
            return self.status()

    def build_latest(self):
        identifier = secrets.token_hex(16)
        directory = self.home / 'versions' / identifier
        directory.mkdir(parents=True)
        try:
            result = getattr(builders, self.config['builder'])(self.config, directory, partial(self.publish, identifier), self.cancel.is_set)
            with self.lock:
                self.value['checks'] = result
        except Exception as error:
            with self.lock:
                self.value['error'] = str(error)
                self.value['checks'] = 'failed'
        finally:
            with self.lock:
                self.value['building'] = False
                self.save()
                prune(self.home / 'versions', self.config['keep_versions'], [self.value.get('version', '')])

    def publish(self, identifier, kind, manifest, info):
        if self.cancel.is_set():
            return
        with self.lock:
            if kind == 'progress':
                self.value['progress'] = manifest
                if time.monotonic() - self.last_progress < 2:
                    return
                self.last_progress = time.monotonic()
            elif kind == 'requested':
                self.value['run'] = manifest
            elif kind == 'frontend':
                previous = self.value.get('source', {}).get('native_contract')
                if self.config['mode'] == 'full' and previous != info['native_contract']:
                    self.stop_native()
                self.value.update(version=identifier, available=True, source=info, manifest=manifest, snapshot_url=self.value['url'].replace('/app/', '/snapshots/' + identifier + '/app/'))
                cached = self.host_directory(info['native_contract'])
                if self.config['mode'] == 'full' and (cached / 'native-manifest.json').is_file():
                    self.launch_native(cached, json.loads((cached / 'native-manifest.json').read_text()))
            elif kind == 'native':
                cached = self.host_directory(manifest['native_contract'])
                if not cached.exists():
                    shutil.copytree(self.home / 'versions' / identifier / 'native', cached)
                retained = json.loads((cached / 'native-manifest.json').read_text())
                self.launch_native(cached, retained)
                prune(self.home / 'hosts', 2, [cached.name])
            self.save()

    def host_directory(self, contract):
        return self.home / 'hosts' / (contract + '-' + str(self.config['port']))

    def launch_native(self, directory, manifest):
        if manifest.get('managed_frontend') is not True or manifest.get('real_backend_verified') is not True:
            raise ValueError('Native Lab has not verified the managed frontend and real backend')
        if manifest['platform'] != platform.system() or manifest['arch'] != platform.machine():
            raise ValueError('Native Lab architecture differs from this computer')
        if manifest['port'] != self.config['port'] or manifest['native_contract'] != self.value['source']['native_contract']:
            raise ValueError('Native Lab contract/port differs from the active frontend')
        if self.native and self.native.poll() is None:
            current = self.value.get('native_source', {})
            if current.get('native_contract') == manifest['native_contract']:
                return
        archive = directory / 'native.zip'
        with archive.open('rb') as stream:
            actual_digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        if actual_digest != manifest['archive_sha256']:
            raise ValueError('Native archive digest differs')
        unpack(archive, directory)
        binary = (directory / manifest['host']).resolve()
        if not binary.is_relative_to(directory.resolve()) or not binary.is_file():
            raise ValueError('Native host path is invalid')
        self.stop_native()
        environment = dict(os.environ, KHOR_HOME=str(self.home / 'store'), KHOR_NAME='Khor Developer Lab', KHOR_WEB_PORT='0')
        environment.pop('KHOR_SESSION', None)
        with (self.home / 'native.log').open('ab') as log:
            self.native = subprocess.Popen([str(binary)], env=environment, stdout=log, stderr=log)
        time.sleep(1)
        if self.native.poll() is not None:
            raise ValueError('Native host exited during startup; inspect native.log')
        self.value.update(native_pid=self.native.pid, native_source=manifest)

    def stop_native(self):
        if self.native and self.native.poll() is None:
            self.native.terminate()
            try:
                self.native.wait(timeout=8)
            except subprocess.TimeoutExpired:
                self.native.kill()
                self.native.wait()
        self.native = None
        self.value.pop('native_pid', None)

    def control(self, action, body):
        if action == 'update':
            return self.update()
        with self.lock:
            if action in ('scene', 'reset'):
                if self.config['mode'] == 'full':
                    raise ValueError('Full Lab keeps real isolated data; stop it before removing its store')
                from .config import SCENES
                scene = body.get('scene', self.value['scene'])
                if scene not in SCENES or scene == 'live':
                    raise ValueError('Unknown frontend scenario')
                self.value.update(scene=scene, revision=secrets.token_hex(8))
                self.save()
            elif action not in ('scene', 'reset'):
                raise ValueError('Unknown Lab control operation')
            return self.status()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def respond(self, code, data, content_type='application/json'):
        body = data if isinstance(data, bytes) else json.dumps(data, ensure_ascii=False).encode()
        self.send_response(code)
        self.send_header('content-type', content_type)
        self.send_header('content-length', str(len(body)))
        self.send_header('cache-control', 'no-store')
        self.send_header('x-content-type-options', 'nosniff')
        self.end_headers()
        self.wfile.write(body)

    @property
    def lab(self):
        return self.server.lab

    def do_GET(self):
        path = unquote(urlsplit(self.path).path)
        if path == '/lab/example':
            return self.respond(200, b'<!doctype html><meta name="viewport" content="width=device-width"><title>Lab page</title><h1>Khor Lab page</h1><p>Isolated test page inside the real browser surface.</p><label>Text <input placeholder="Keyboard test"></label>', 'text/html; charset=utf-8')
        if path == '/lab/status':
            return self.respond(200, self.lab.status())
        if path == '/lab/runtime':
            from urllib.parse import parse_qs
            status = self.lab.status()
            version = parse_qs(urlsplit(self.path).query).get('version', [status.get('version', '')])[0]
            manifest = self.lab.home / 'versions' / version / 'lab-manifest.json'
            if not version.isalnum() or not manifest.is_file():
                return self.respond(404, 'Lab snapshot does not exist')
            source = json.loads(manifest.read_text())
            return self.respond(200, {k: status.get(k) for k in ('mode', 'scene', 'revision')} | {'tree': source['tree']})
        if path == '/':
            # This launch page is deliberately outside the App; the App receives no substitute shell.
            html = b'<!doctype html><meta charset="utf-8"><title>Khor developer Lab</title><h1>Khor developer Lab</h1><p><a href="/app/">Open the complete App</a> | <a href="/lab/status">Build/source status</a></p><p>Refresh the App after an update. Native verification and user acceptance are recorded separately.</p>'
            return self.respond(200, html, 'text/html; charset=utf-8')
        status = self.lab.status()
        if path in ('/app', '/app/', '/app/index.html') or path.startswith('/snapshots/'):
            if not status.get('available'):
                return self.respond(503, 'Lab is building. Inspect /lab/status and refresh when available.')
            version = status['version']
            if path.startswith('/snapshots/'):
                parts = path.strip('/').split('/')
                if len(parts) != 3 or parts[2] != 'app':
                    return self.respond(404, 'Unknown snapshot route')
                version = parts[1]
                if not version.isalnum() or not (self.lab.home / 'versions' / version / 'index.html').is_file():
                    return self.respond(404, 'Snapshot was removed by retention')
            html = (self.lab.home / 'versions' / version / 'index.html').read_text()
            html = html.replace('./assets/', f'/versions/{version}/assets/')
            html = html.replace('<head>', '<head><meta name="khor-lab-version" content="' + version + '">')
            return self.respond(200, html.encode(), 'text/html; charset=utf-8')
        if not path.startswith('/versions/'):
            return self.respond(404, 'Unknown Lab route')
        directory = self.lab.home / 'versions'
        file = (self.lab.home / path.lstrip('/')).resolve()
        # Serve only built frontend files, never store, manager token, native archives or requests.
        relative = Path(path).parts
        asset_root = directory / (relative[2] if len(relative) > 2 else '') / 'assets'
        if len(relative) < 5 or not relative[2].isalnum() or relative[3] != 'assets' or not file.is_relative_to(asset_root.resolve()) or not file.is_file():
            return self.respond(404, 'Missing frontend asset')
        self.respond(200, file.read_bytes(), mimetypes.guess_type(file.name)[0] or 'application/octet-stream')

    def do_POST(self):
        token = self.headers.get('authorization', '')
        if not secrets.compare_digest(token, 'Bearer ' + self.lab.value['token']):
            return self.respond(403, 'Lab control requires its local CLI token')
        try:
            length = int(self.headers.get('content-length', '0'))
            if not 0 <= length <= 4096:
                raise ValueError('Lab control request is too large')
            body = json.loads(self.rfile.read(length) or '{}')
            action = urlsplit(self.path).path.removeprefix('/lab/control/')
            if action == 'stop':
                self.respond(200, self.lab.status())
                threading.Thread(target=self.server.shutdown, daemon=True).start()
            else:
                self.respond(200, self.lab.control(action, body))
        except (ValueError, OSError) as error:
            self.respond(400, str(error))


def serve(config, home):
    home.mkdir(parents=True, exist_ok=True)
    home.chmod(0o700)
    server = ThreadingHTTPServer(('127.0.0.1', config['port']), Handler)
    server.daemon_threads = True
    lab = Lab(config, home)
    (home / 'manager.json').chmod(0o600)
    server.lab = lab
    lab.update()
    try:
        server.serve_forever(poll_interval=0.2)
    finally:
        lab.cancel.set()
        lab.stop_native()
        server.server_close()
