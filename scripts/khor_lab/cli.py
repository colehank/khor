import argparse
from datetime import timedelta
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
    parser.add_argument('operation', choices=['dev', 'start', 'update', 'status', 'open', 'stop', 'reset', 'scene',
                                              'restart', '_serve', 'probe', 'clean', 'doctor'])
    parser.add_argument('sub_operation', nargs='?', help='new|gc for probe')
    parser.add_argument('--kind', default='probe', help='probe kind recorded in its marker')
    parser.add_argument('--older-than', default='7d', help='probe gc age, for example 7d or 12h')
    parser.add_argument('--inventory', help='directory to inventory for lab clean')
    parser.add_argument('--pattern', default='khor-*', help='inventory glob for lab clean')
    parser.add_argument('--manifest', help='reviewed manifest file for lab clean')
    parser.add_argument('--prefix', default='F:\\khor-', help='allowed cleanup path prefix for lab clean')
    parser.add_argument('--output', help='write the inventory or receipt to this file')
    parser.add_argument('--yes', action='store_true', help='actually delete instead of planning')
    parser.add_argument('--mode', choices=['frontend', 'full'])
    parser.add_argument('--builder', choices=['local', 'remote'])
    parser.add_argument('--scene', choices=SCENES)
    parser.add_argument('--port', type=int)
    parser.add_argument('--builder-repo')
    parser.add_argument('--cache-root')
    parser.add_argument('--resume', action='store_true', help='Acquire the previous request again without rebuilding')
    parser.add_argument('--target', choices=['desktop', 'ios'], default='desktop', help='Native development target for lab dev')
    parser.add_argument('--device', help='Available iOS simulator name or UDID for lab dev')
    parser.add_argument('--prepare-only', action='store_true', help='Prepare the source development target without launching it')
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


def _older_than(text):
    units = {'s': 1, 'm': 60, 'h': 3600, 'd': 86400}
    text = str(text).strip().lower()
    if len(text) < 2 or text[-1] not in units or not text[:-1].isdigit():
        raise ValueError('expected an age like 7d, 12h or 30m')
    return timedelta(seconds=int(text[:-1]) * units[text[-1]])


def _safety(root, config, args):
    from .clean import clean, inventory
    from .doctor import doctor
    from .probe import gc_probes, new_probe
    if args.operation == 'doctor':
        rows = doctor(config['root'], config)
        for name, ok, hint in rows:
            print('%-12s %-4s %s' % (name, 'ok' if ok else 'FAIL', hint))
        failed = [name for name, ok, _ in rows if not ok]
        if failed:
            raise ValueError('lab doctor: %s outside the workspace' % ', '.join(failed))
        return
    if args.operation == 'probe':
        if args.sub_operation == 'new':
            print(new_probe(config['root'], args.kind))
            return
        if args.sub_operation == 'gc':
            removed = gc_probes(config['root'], _older_than(args.older_than), dry_run=not args.yes)
            for path in removed:
                print(path)
            print('%d probe director%s %s' % (len(removed), 'y' if len(removed) == 1 else 'ies',
                                               'removed' if args.yes else 'would be removed; pass --yes'))
            return
        raise ValueError('lab probe expects new or gc')
    if args.inventory:
        payload = json.dumps(inventory(Path(args.inventory), args.pattern), indent=2)
        if args.output:
            Path(args.output).write_text(payload + '\n', encoding='utf-8')
            print(args.output)
        else:
            print(payload)
        return
    if not args.manifest:
        raise ValueError('lab clean needs --inventory <dir> or --manifest <file>')
    prefix = args.prefix
    resolved_prefix = Path(prefix).resolve()
    if resolved_prefix.drive.upper() == 'C:' or str(resolved_prefix) == str(Path(resolved_prefix.anchor)):
        raise ValueError('refusing a cleanup prefix on C: or a drive root: %s' % prefix)
    manifest = json.loads(Path(args.manifest).read_text(encoding='utf-8'))
    receipt = clean(manifest, [prefix], dry_run=not args.yes, yes=args.yes)
    payload = json.dumps(receipt, indent=2)
    if args.output:
        Path(args.output).write_text(payload + '\n', encoding='utf-8')
        print(args.output)
    else:
        print(payload)
    if receipt['skipped']:
        print('skipped: %d' % len(receipt['skipped']), file=sys.stderr)


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
    if args.operation in ('probe', 'clean', 'doctor'):
        _safety(root, config, args)
        return
    if args.operation == 'dev':
        from .hot import develop
        develop(config, target=args.target, device=args.device, prepare_only=args.prepare_only)
        return
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
