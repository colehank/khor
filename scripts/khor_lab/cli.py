import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
from urllib.error import URLError
from urllib.request import Request, urlopen
import webbrowser

from .config import SCENES, load
from .artifacts import write_json


def register(sub):
    parser = sub.add_parser('lab', help='Complete production App Lab, with independent mode and builder choices')
    parser.add_argument('operation', choices=['start', 'update', 'status', 'open', 'stop', 'reset', 'scene', 'restart', '_serve'])
    parser.add_argument('--mode', choices=['frontend', 'full'])
    parser.add_argument('--builder', choices=['local', 'remote'])
    parser.add_argument('--scene', choices=SCENES)
    parser.add_argument('--port', type=int)
    parser.add_argument('--builder-repo')
    parser.add_argument('--cache-root')
    parser.add_argument('--resume', action='store_true', help='Acquire the previous request again without rebuilding')
    parser.add_argument('--settings', help=argparse.SUPPRESS)


def request(home, action=None, body=None):
    receipt = json.loads((home / 'manager.json').read_text())
    url = receipt['url'].replace('/app/', '/lab/control/' + action if action else '/lab/status')
    req = Request(url, headers={'authorization': 'Bearer ' + receipt['token']},
                  data=json.dumps(body or {}).encode() if action else None)
    with urlopen(req, timeout=10) as response:
        result = json.load(response)
    if result['pid'] != receipt['pid'] or result['root'] != receipt['root']:
        raise ValueError('Another Lab owns this port; choose a different --port')
    return result


def main(args, root):
    if args.operation == '_serve':
        from .server import serve
        settings = json.loads(Path(args.settings).read_text())
        serve(settings['config'], Path(settings['home']))
        return
    options = {name: getattr(args, name) for name in ('mode', 'builder', 'scene', 'port', 'builder_repo', 'cache_root')}
    if options['mode'] == 'full' and options['scene'] is None:
        options['scene'] = 'live'
    config = load(root, options)
    key = hashlib.sha256(str(root.resolve()).encode()).hexdigest()[:16] + '-' + str(config['port'])
    home = Path(config['cache_root']) / key
    if args.operation == 'start':
        try:
            current = request(home)
        except (URLError, OSError, ValueError):
            current = None
        if current:
            if any(current[name] != config[name] for name in ('mode', 'builder')):
                raise ValueError('Stop the existing Lab before changing mode/builder')
            print(json.dumps(current, ensure_ascii=False, indent=2))
            return
        home.mkdir(parents=True, exist_ok=True)
        home.chmod(0o700)
        if args.resume:
            config['resume'] = True
        settings = home / 'settings.json'
        write_json(settings, {'config': config, 'home': str(home)})
        with (home / 'manager.log').open('ab') as log:
            subprocess.Popen([sys.executable, str(Path(__file__).resolve().parents[1] / 'dev.py'), 'lab', '_serve', '--settings', str(settings)],
                             cwd=root, stdout=log, stderr=log, start_new_session=True)
        for _ in range(30):
            try:
                result = request(home)
                print(json.dumps(result, ensure_ascii=False, indent=2))
                return
            except (URLError, OSError, ValueError):
                time.sleep(0.1)
        raise ValueError(f'Lab manager did not start. Inspect {home / "manager.log"}')
    if args.operation == 'open':
        status = request(home)
        if status['mode'] == 'full':
            if not status.get('native_pid'):
                raise ValueError('Native host is still building; inspect lab status')
            subprocess.run(['open', '-a', status['native_app']], check=True)
        else:
            webbrowser.open(status['url'])
        return
    action = args.operation if args.operation != 'status' else None
    result = request(home, action, {'scene': config['scene']} if action == 'scene' else {})
    print(json.dumps(result, ensure_ascii=False, indent=2))
    if action == 'stop':
        # Only the manager authenticated above owns this process group and its children.
        try:
            if os.name == 'nt':
                subprocess.run(['taskkill', '/PID', str(result['pid']), '/T', '/F'], timeout=20, capture_output=True, check=False)
            elif os.getpgid(result['pid']) == result['pid']:
                os.killpg(result['pid'], signal.SIGTERM)
        except ProcessLookupError:
            pass
        if result.get('building') and result.get('run') and result['builder'] == 'remote':
            subprocess.run(['gh', 'run', 'cancel', str(result['run']['databaseId']), '--repo', config['builder_repo']],
                           timeout=30, capture_output=True, check=False)
