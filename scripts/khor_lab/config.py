import json
import re
import os
import sys
from pathlib import Path

DEFAULTS = dict(mode='frontend', builder='local', scene='gui', port=23910,
                builder_repo='colehank/khor-build', keep_versions=3)
SCENES = ('gui', 'codex', 'terminal', 'stream', 'content', 'page', 'list', 'workspace', 'workspaces', 'quota', 'devices', 'browser', 'live')


def default_cache():
    if os.name == 'nt':
        return Path(os.environ.get('LOCALAPPDATA', Path.home() / 'AppData/Local')) / 'KhorDeveloperLab'
    if sys.platform == 'darwin':
        return Path.home() / 'Library/Caches/KhorDeveloperLab'
    return Path(os.environ.get('XDG_CACHE_HOME', Path.home() / '.cache')) / 'KhorDeveloperLab'


def load(root, overrides=None):
    values = dict(DEFAULTS)
    for name in ('lab.config.json', '.khor-lab.local.json'):
        path = root / name
        if path.exists():
            data = json.loads(path.read_text())
            if not isinstance(data, dict) or data.keys() - (DEFAULTS.keys() | {'cache_root'}):
                raise ValueError(f'Unknown Lab configuration: {path}')
            values.update(data)
    values.update({k: v for k, v in (overrides or {}).items() if v is not None})
    if values['mode'] not in ('frontend', 'full') or values['builder'] not in ('local', 'remote'):
        raise ValueError('Lab mode is frontend/full; builder is local/remote')
    if values['scene'] not in SCENES or not isinstance(values['port'], int) or not 1024 <= values['port'] <= 65534:
        raise ValueError('Invalid Lab scene or port')
    if values['mode'] == 'full' and values['scene'] != 'live':
        raise ValueError('Full Lab uses scene live: real isolated backend, without browser fixture data')
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', values['builder_repo']):
        raise ValueError('Expected owner/repository')
    if not isinstance(values['keep_versions'], int) or not 1 <= values['keep_versions'] <= 10:
        raise ValueError('keep_versions must be 1..10')
    values['root'] = str(root.resolve())
    values['cache_root'] = str(Path(values.get('cache_root', default_cache())).expanduser().resolve())
    return values
