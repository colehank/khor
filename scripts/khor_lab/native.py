"""Build an isolated debug Tauri host which loads the managed frontend, never the production store."""
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import shutil
import subprocess
import sys
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from khor_lab.artifacts import write_json
from khor_lab.contract import digest


def build(root, output, source):
    if platform.system() != 'Darwin' or platform.machine() != 'arm64':
        raise ValueError('The initial full Lab host supports macOS arm64; frontend Lab is portable')
    port = int(os.environ.get('KHOR_LAB_PORT', '23910'))
    if not 1024 <= port <= 65534:
        raise ValueError('Invalid Lab port')
    output.mkdir(parents=True, exist_ok=True)
    info = json.loads(source.read_text())
    config = {'identifier': 'io.github.colehank.khor.developer-lab', 'productName': 'KhorDeveloperLab',
              'build': {'devUrl': f'http://127.0.0.1:{port}/app/'}}
    gui = root / 'apps/gui'
    if not (gui / 'dist/index.html').is_file():
        subprocess.run(['npm', 'ci'], cwd=gui, check=True)
        subprocess.run(['npm', 'run', 'gen'], cwd=gui, check=True)
        subprocess.run(['npm', 'run', 'build'], cwd=gui, check=True)
    agent = output / 'khor-agent-native'
    target = 'aarch64-apple-darwin'
    subprocess.run(['npm', 'ci'], cwd=root / 'apps/agent', check=True)
    subprocess.run(['node', 'build.mjs', target, str(agent)], cwd=root / 'apps/agent', check=True)
    environment = dict(os.environ, TAURI_CONFIG=json.dumps(config), KHOR_AGENT_BUNDLE=str(agent),
                       CARGO_INCREMENTAL='0', CARGO_PROFILE_DEV_DEBUG='0')
    subprocess.run(['cargo', 'build', '--locked', '-p', 'khor-gui'], cwd=root, env=environment, check=True)
    app = output / 'KhorDeveloperLab.app'
    programs = app / 'Contents/MacOS'
    programs.mkdir(parents=True, exist_ok=True)
    target_dir = Path(os.environ.get('CARGO_TARGET_DIR', root / 'target'))
    binary = programs / 'KhorDeveloperLab'
    shutil.copyfile(target_dir / 'debug/khor-gui', binary)
    binary.chmod(0o755)
    plist = dict(CFBundleName='KhorDeveloperLab', CFBundleIdentifier=config['identifier'],
                 CFBundleExecutable='KhorDeveloperLab', CFBundlePackageType='APPL', CFBundleVersion='1',
                 CFBundleShortVersionString='0.0.0', NSHighResolutionCapable=True, LSMinimumSystemVersion='13.0')
    (app / 'Contents/Info.plist').write_bytes(plistlib.dumps(plist))
    subprocess.run(['codesign', '--force', '--sign', '-', str(app)], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
    archive = output / 'native.zip'
    with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED) as bundle:
        for path in app.rglob('*'):
            if path.is_file():
                bundle.write(path, path.relative_to(output))
    write_json(output / 'native-manifest.json', {**info, 'native_contract': digest(root),
               'platform': 'Darwin', 'arch': 'arm64', 'port': port, 'identifier': config['identifier'],
               'archive_sha256': hashlib.sha256(archive.read_bytes()).hexdigest(), 'host': 'KhorDeveloperLab.app/Contents/MacOS/KhorDeveloperLab'})
    shutil.rmtree(app)
    agent.unlink()


if __name__ == '__main__':
    build(*map(lambda s: Path(s).resolve(), sys.argv[1:]))
