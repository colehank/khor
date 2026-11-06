import json
from pathlib import Path, PurePosixPath
import shutil
import stat
import zipfile


def write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')
    temporary.chmod(0o600)
    temporary.replace(path)


def unpack(archive, destination, limit=1024 * 1024 * 1024):
    with zipfile.ZipFile(archive) as bundle:
        if len(bundle.infolist()) > 20000:
            raise ValueError('Lab archive contains too many files')
        if sum(item.file_size for item in bundle.infolist()) > limit:
            raise ValueError('Lab archive exceeds its extraction budget')
        for item in bundle.infolist():
            name = PurePosixPath(item.filename)
            kind = (item.external_attr >> 16) & 0o170000
            if name.is_absolute() or (name.parts and ':' in name.parts[0]) or '..' in name.parts or '\\' in item.filename or kind == stat.S_IFLNK:
                raise ValueError('Lab archive contains an unsafe path')
        bundle.extractall(destination)
        for item in bundle.infolist():
            if item.external_attr >> 16 & 0o111:
                (destination / item.filename).chmod(0o755)


def validate(directory, expected):
    value = json.loads((directory / 'lab-manifest.json').read_text())
    if value.get('schema') != 1 or value.get('entry') != 'production-app':
        raise ValueError('Not a complete App Lab bundle')
    for key in ('tree', 'base_commit', 'snapshot_commit', 'native_contract'):
        if value.get(key) != expected.get(key):
            raise ValueError(f'Lab source identity differs: {key}')
    if not (directory / 'index.html').is_file():
        raise ValueError('Lab App entry is missing')
    return value


def prune(directory, keep, protected=()):
    if not directory.exists():
        return
    versions = sorted((p for p in directory.iterdir() if p.is_dir()), key=lambda p: p.stat().st_mtime, reverse=True)
    retain = set(p.name for p in versions[:keep]) | set(protected)
    for path in versions:
        if path.name not in retain:
            shutil.rmtree(path)
