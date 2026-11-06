"""Conservative native/generated contract. Frontend-only edits leave it unchanged."""
import hashlib
from pathlib import Path
import subprocess


def _digest(root, native):
    names = subprocess.check_output(['git', '-C', str(root), 'ls-files', '-z']).decode().split('\0')
    result = hashlib.sha256()
    for name in sorted(filter(None, names)):
        path = Path(name)
        included = path.suffix in ('.rs', '.toml', '.lock', '.swift', '.kt', '.c', '.h') or name.startswith('vendor/')
        if native:
            included |= name.startswith(('apps/agent/', 'apps/gui/src-tauri/')) or name in ('scripts/khor_lab/native.py', 'apps/gui/package.json', 'apps/gui/package-lock.json')
        if not included or not (root / name).is_file():
            continue
        result.update(name.encode() + b'\0' + (root / name).read_bytes() + b'\0')
    return result.hexdigest()


def digest(root):
    return _digest(root, native=True)


def generated_digest(root):
    return _digest(root, native=False)
