"""Run the terminal Lab in isolated iOS WKWebView with actual UIKit keyboard events."""
import argparse
from http.server import ThreadingHTTPServer
import json
import os
import shutil
from pathlib import Path
from contextlib import suppress
import plistlib
import subprocess
import sys
import threading
import time

from .config import load
from .ios_probe import dedicated_simulator, run
from .mobile import keyboard_probe
from .server import Handler, Lab


IDENTIFIER = 'dev.khor.terminal-keyboard-probe'
CLIPBOARD_SOURCE = IDENTIFIER + '.clipboard'
CLIPBOARD_TEXT = 'khor 剪贴板测试'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--out', required=True, type=Path)
    parser.add_argument('--simulator', help='Available CI device UUID; defaults to an isolated local Lab device')
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    lab_bundle = args.bundle.resolve()
    manifest = json.loads((lab_bundle / 'lab-manifest.json').read_text())
    revision = run('git', 'rev-parse', 'HEAD')
    if manifest['snapshot_commit'] != revision:
        raise ValueError('The Lab artifact must come from this exact revision')

    root = Path.cwd()
    home = args.out.resolve()
    # The preview serves a verified, immutable CI bundle. Only the fixture's
    # data is synthetic; WebKit, UIKit and the production terminal are real.
    version = home / 'versions' / 'pinned'
    version.parent.mkdir(parents=True)
    version.symlink_to(lab_bundle, target_is_directory=True)
    config = load(root, {'port':23928, 'scene':'terminal', 'mode':'frontend', 'builder':'local'})
    lab = Lab(config, home)
    lab.value.update(version='pinned', available=True, source=manifest, manifest=manifest)
    lab.save()

    app = home / 'KhorTerminalKeyboardProbe.app'
    app.mkdir()
    entry = 'http://127.0.0.1:23928/app/?scene=terminal'
    (app / 'Info.plist').write_bytes(plistlib.dumps({
        'CFBundleIdentifier':IDENTIFIER, 'CFBundleName':'KhorTerminalKeyboardProbe',
        'CFBundleExecutable':'Probe', 'CFBundleVersion':'1', 'CFBundleShortVersionString':'1.0',
        'CFBundlePackageType':'APPL', 'LSRequiresIPhoneOS':True, 'UILaunchScreen':{},
        'NSAppTransportSecurity':{'NSAllowsLocalNetworking':True},
        'KhorProbeURL':entry,
    }))
    paste_library = os.environ.get('KHOR_PASTE_PROBE_LIB')
    paste_flags = ['-D', 'PASTE_PROBE', paste_library, '-framework', 'UIKit'] if paste_library else []
    subprocess.run(['xcrun','--sdk','iphonesimulator','swiftc','-parse-as-library','-swift-version','5',
                    '-target','arm64-apple-ios17.0-simulator',
                    '-sdk',run('xcrun','--sdk','iphonesimulator','--show-sdk-path'),
                    'apps/gui/scripts/native-keyboard-probe/Probe.swift',*paste_flags,'-o',str(app/'Probe')],
                   timeout=120, check=True)
    subprocess.run(['codesign','--force','--sign','-',str(app)], timeout=30, check=True)
    simulator = args.simulator or dedicated_simulator()
    installed = []
    with ThreadingHTTPServer(('127.0.0.1', 23928), Handler) as server:
        server.lab = lab
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        try:
            subprocess.run(['xcrun','simctl','bootstatus',simulator,'-b'], timeout=240, check=True)
            subprocess.run(['xcrun','simctl','install',simulator,str(app)], timeout=150, check=True)
            installed.append(IDENTIFIER)
            if paste_library:
                source = home / 'KhorClipboardSource.app'
                shutil.copytree(app, source)
                plist = source / 'Info.plist'
                info = plistlib.loads(plist.read_bytes())
                info.update(CFBundleIdentifier=CLIPBOARD_SOURCE, CFBundleName='KhorClipboardSource',
                            KhorClipboardSeed=CLIPBOARD_TEXT)
                plist.write_bytes(plistlib.dumps(info))
                subprocess.run(['codesign','--force','--sign','-',str(source)], timeout=30, check=True)
                subprocess.run(['xcrun','simctl','install',simulator,str(source)], timeout=150, check=True)
                installed.append(CLIPBOARD_SOURCE)
                source_data = Path(run('xcrun','simctl','get_app_container',simulator,CLIPBOARD_SOURCE,'data'))
                ready = source_data / 'Documents' / 'clipboard-seeded'
                ready.parent.mkdir(exist_ok=True)
                ready.unlink(missing_ok=True)
                subprocess.run(['xcrun','simctl','launch',simulator,CLIPBOARD_SOURCE], timeout=120, check=True)
                deadline = time.monotonic() + 15
                while not ready.exists() and time.monotonic() < deadline:
                    time.sleep(.1)
                if not ready.exists():
                    raise ValueError('The companion app did not seed the iOS clipboard')
            data = Path(run('xcrun','simctl','get_app_container',simulator,IDENTIFIER,'data'))
            documents = data / 'Documents'
            documents.mkdir(exist_ok=True)
            receipt = keyboard_probe(home, simulator, IDENTIFIER, entry, home, terminal_documents=documents,
                                     paste_text=CLIPBOARD_TEXT if paste_library else None)
            receipt.update(simulator=simulator, revision=revision)
            (home / 'terminal-keyboard-result.json').write_text(json.dumps(receipt, indent=2))
        finally:
            for identifier in reversed(installed):
                with suppress(subprocess.TimeoutExpired):
                    subprocess.run(['xcrun','simctl','uninstall',simulator,identifier],
                                   capture_output=True, timeout=30)
            server.shutdown()
            worker.join(timeout=10)


if __name__ == '__main__':
    main()
