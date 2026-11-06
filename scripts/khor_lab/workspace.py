"""Disposable native project; platform generators never rewrite the developer checkout."""
import json
import hashlib
from pathlib import Path
import re
import plistlib
import shutil
import subprocess
import tarfile
import tomllib

from .artifacts import write_json, prune
from .contract import digest


def materialize(config, home, environment, plan):
    from .builders import source, run
    root = Path(config['root'])
    contract = digest(root)
    flavor = hashlib.sha256((contract + json.dumps(plan['tauri'], sort_keys=True) + plan['target']).encode() + Path(__file__).read_bytes()).hexdigest()
    workspace = home / 'native-sources' / flavor
    receipt = workspace / '.lab-source.json'
    if receipt.exists():
        return workspace
    if workspace.exists():
        if not (workspace / '.lab-preparing.json').exists():
            raise ValueError(f'Preserving an unowned native source directory: {workspace}')
        shutil.rmtree(workspace)
    workspace.mkdir(parents=True)
    write_json(workspace / '.lab-preparing.json', {'root': str(root), 'native_contract': contract})
    info = source(config)
    archive = subprocess.Popen(['git', '-C', str(root), 'archive', info['snapshot_commit'] + ':source'], stdout=subprocess.PIPE)
    try:
        with tarfile.open(fileobj=archive.stdout, mode='r|') as files:
            files.extractall(workspace, filter='data')
    finally:
        archive.stdout.close()
    if archive.wait():
        raise ValueError('Could not materialize the native development source')
    run('git', 'init', '-q', str(workspace))
    (workspace / '.git/info/exclude').write_text('.lab-preparing.json\n.lab-source.json\n')
    run('git', '-C', str(workspace), 'add', '--all')
    run('git', '-C', str(workspace), '-c', 'user.name=Khor Lab', '-c', 'user.email=lab@localhost',
        '-c', 'commit.gpgsign=false', 'commit', '-qm', 'Native development input ' + info['tree'])
    gui = workspace / 'apps/gui'
    (gui / 'node_modules').symlink_to((root / 'apps/gui/node_modules').resolve(), target_is_directory=True)
    shutil.copytree(root / 'apps/gui/src/gen', gui / 'src/gen', dirs_exist_ok=True)
    manifest = gui / 'src-tauri/Cargo.toml'
    text = manifest.read_text()
    defaults = tomllib.loads(text)['features'].get('default', [])
    if 'custom-protocol' in defaults:
        defaults.remove('custom-protocol')
        text, count = re.subn(r'^default\s*=\s*\[[^\]]*\]', 'default = ' + json.dumps(defaults), text, flags=re.M)
        if count != 1:
            raise ValueError('Cannot configure development features in the isolated native project')
        manifest.write_text(text)
    run('npx', '--no-install', 'vite', 'build', cwd=gui, env=environment, timeout=900)
    if plan['target'] == 'ios':
        run('rustup', 'target', 'add', 'aarch64-apple-ios-sim', env=environment, timeout=900)
        run('npx', '--no-install', 'tauri', 'ios', 'init', '--ci', '--skip-targets-install',
            '--config', str(home / 'tauri.json'), cwd=gui, env=environment, timeout=900)
        # Retain the repository's native frameworks while giving this build flavor its own container.
        apple = gui / 'src-tauri/gen/apple'
        for path in [apple / 'project.yml', *apple.glob('*.xcodeproj/project.pbxproj')]:
            if path.exists():
                path.write_text(re.sub(r'io\.github\.colehank\.khor(?![A-Za-z0-9_.-])', plan['tauri']['identifier'], path.read_text()))
        for path in apple.glob('*_iOS/Info.plist'):
            info_plist = plistlib.loads(path.read_bytes())
            info_plist.update(KhorLabNativeContract=contract, KhorLabEntryURL=plan['url'])
            path.write_bytes(plistlib.dumps(info_plist))
    write_json(receipt, dict(source=info, native_contract=contract, mutable_native=False))
    prune(workspace.parent, 2, [flavor])
    return workspace
