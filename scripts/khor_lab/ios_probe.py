"""Run only the isolated developer app on a dedicated simulator; receipts never imply human acceptance."""
import argparse
import json
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import threading
import time

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.artifacts import write_json
from khor_lab.config import load
from khor_lab.contract import digest
from khor_lab.hot import development_home


def run(*args):
    return subprocess.check_output(args, text=True, timeout=120).strip()


def dedicated_simulator():
    devices = json.loads(run('xcrun', 'simctl', 'list', 'devices', 'available', '--json'))['devices']
    for runtime, rows in devices.items():
        if '.iOS-' in runtime:
            for row in rows:
                if row['name'] == 'Khor Developer Lab' and row.get('isAvailable'):
                    return row['udid']
    runtime, rows = next((runtime, rows) for runtime, rows in reversed(list(devices.items()))
                         if '.iOS-' in runtime and any('iPhone' in row['name'] for row in rows))
    model = next(row['deviceTypeIdentifier'] for row in rows if 'iPhone' in row['name'])
    return run('xcrun', 'simctl', 'create', 'Khor Developer Lab', model, runtime)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('phase', choices=['prepare', 'verify', 'keyboard'])
    parser.add_argument('--report', type=Path, required=True)
    args = parser.parse_args()
    report = args.report.resolve()
    report.mkdir(parents=True, exist_ok=True)
    root = Path.cwd()
    cache = Path(os.environ.get('KHOR_LAB_CACHE_ROOT', str(Path.home() / 'khor-builder/cache/developer-lab')))
    config = load(root, dict(mode='full', scene='live', builder='local', port=23920, cache_root=str(cache)))
    home = development_home(config)
    udid = dedicated_simulator()
    receipt = dict(phase=args.phase, simulator=udid, passed=False, human_accepted=False,
                   native_contract=digest(root))
    write_json(report / 'ios.json', receipt)
    command = [sys.executable, 'scripts/dev.py', 'lab', 'dev', '--builder', 'local', '--mode', 'full',
               '--target', 'ios', '--device', udid, '--port', '23920', '--cache-root', str(cache)]
    if args.phase == 'prepare':
        subprocess.run([*command, '--prepare-only'], check=True)
        receipt.update(prepared=json.loads((home / 'development.json').read_text()), passed=True)
        write_json(report / 'ios.json', receipt)
        return
    states = json.loads(run('xcrun', 'simctl', 'list', 'devices', 'available', '--json'))['devices']
    selected = next(row for rows in states.values() for row in rows if row['udid'] == udid)
    if selected['state'] != 'Booted':
        subprocess.run(['xcrun', 'simctl', 'boot', udid], check=True, timeout=120)
    subprocess.run(['xcrun', 'simctl', 'bootstatus', udid, '-b'], check=True, timeout=180)
    child = None
    output = None
    fallback = threading.Event()
    log = (report / 'ios-development.log').open('w')
    try:
        child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
        def relay():
            for line in child.stdout:
                log.write(line)
                log.flush()
                print(line, end='', flush=True)
                if 'Opening Xcode' in line or 'Could not find an iOS Simulator matching' in line:
                    fallback.set()
        output = threading.Thread(target=relay, daemon=True)
        output.start()
        deadline = time.monotonic() + 5400
        app = None
        while time.monotonic() < deadline:
            if fallback.is_set():
                raise ValueError('Tauri did not start the requested simulator; inspect ios-development.log')
            if child.poll() is not None:
                raise ValueError(f'iOS development exited ({child.returncode}); inspect ios-development.log')
            if (home / 'development.json').exists():
                development = json.loads((home / 'development.json').read_text())
                try:
                    result = subprocess.run(['xcrun', 'simctl', 'get_app_container', udid, development['identifier'], 'app'], capture_output=True, text=True, timeout=30)
                except subprocess.TimeoutExpired:
                    print('Simulator service is still responding slowly; waiting within the installation deadline', flush=True)
                    continue
                if result.returncode == 0:
                    candidate = Path(result.stdout.strip())
                    info = plistlib.loads((candidate / 'Info.plist').read_bytes())
                    observed = {key:info.get(key) for key in ('CFBundleIdentifier','KhorLabNativeContract','KhorLabEntryURL')}
                    if receipt.get('observed_app') != observed:
                        receipt['observed_app'] = observed
                        print('Installed iOS app metadata:', json.dumps(observed), flush=True)
                        write_json(report / 'ios.json', receipt)
                    if info.get('KhorLabNativeContract') == receipt['native_contract'] and info.get('KhorLabEntryURL') == development['url']:
                        app = candidate
                        break
            time.sleep(3)
        if app is None:
            raise ValueError('Timed out waiting for the exact isolated iOS app installation')
        receipt.update(installed=True, app=str(app), identifier=development['identifier'], entry=development['url'])
        subprocess.run(['xcrun', 'simctl', 'io', udid, 'screenshot', str(report / 'ios-app.png')], check=True, timeout=30)
        receipt.update(keyboard_verified=False)
        if args.phase == 'keyboard':
            from khor_lab.mobile import keyboard_probe
            receipt.update(keyboard_probe(cache, udid, development['identifier'], development['url'], report))
        receipt.update(passed=True)
        write_json(report / 'ios.json', receipt)
    except Exception as error:
        receipt['error'] = str(error)
        raise
    finally:
        if child is not None and child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=30)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=10)
        if output:
            output.join(timeout=5)
        log.close()
        if receipt.get('identifier'):
            subprocess.run(['xcrun', 'simctl', 'terminate', udid, receipt['identifier']], capture_output=True, timeout=30)
        write_json(report / 'ios.json', receipt)


if __name__ == '__main__':
    main()
