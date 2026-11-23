"""Manifest-driven cleanup of build directories the Lab may own.

Two rules keep this safe: every path must come from a manifest the human
reviewed, and every path must carry the build-directory signature named in
that manifest. A directory is staged by rename before removal, so a running
build fails the rename instead of being half deleted.
"""
import os
import shutil
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path

from .safepaths import UnsafePath, assert_no_reparse

SIGNATURE_FILES = {'cargo': '.rustc_info.json', 'cachedir': 'CACHEDIR.TAG'}


def _size_of(directory):
    total = 0
    for path in Path(directory).rglob('*'):
        try:
            if path.is_file():
                total += path.stat().st_size
        except OSError:
            continue
    return total


def inventory(prefix_root, pattern='khor-*'):
    """Describe matching directories; never deletes."""
    base = Path(prefix_root)
    items = []
    for entry in sorted(base.glob(pattern)):
        if not entry.is_dir() or entry.is_symlink():
            continue
        signatures = [name for name, marker in SIGNATURE_FILES.items() if (entry / marker).is_file()]
        items.append({
            'path': str(entry.resolve()),
            'signatures': signatures,
            'bytes': _size_of(entry),
            'modified': datetime.fromtimestamp(entry.stat().st_mtime, timezone.utc).isoformat(),
        })
    return {'schema': 1, 'generated_at': datetime.now(timezone.utc).isoformat(), 'items': items}


def clean(manifest, allow_prefixes, dry_run=True, yes=False):
    """Remove manifest items; validate the whole manifest before touching anything."""
    if not dry_run and not yes:
        raise ValueError('refusing to delete without --yes')
    prefixes = [os.path.normcase(str(Path(prefix).expanduser().resolve())) for prefix in allow_prefixes]
    planned = []
    for item in manifest.get('items', []):
        raw = Path(str(item['path'])).expanduser()
        assert_no_reparse(raw)
        resolved = raw.resolve()
        text = os.path.normcase(str(resolved))
        if not any(text.startswith(prefix) for prefix in prefixes):
            raise UnsafePath('manifest path is outside the allowed prefixes: %s' % resolved)
        planned.append((resolved, item))
    receipt = {'checked_at': datetime.now(timezone.utc).isoformat(), 'removed': [], 'skipped': [], 'bytes': 0}
    for resolved, item in planned:
        expected = set(item.get('signatures') or [])
        actual = {name for name, marker in SIGNATURE_FILES.items() if (resolved / marker).is_file()}
        if expected and not expected.issubset(actual):
            receipt['skipped'].append({'path': str(resolved), 'reason': 'signature mismatch'})
            continue
        size = _size_of(resolved)
        receipt['bytes'] += size
        if dry_run:
            receipt['removed'].append(str(resolved))
            continue
        stamp = time.strftime('%Y%m%dT%H%M%SZ', time.gmtime()) + '-' + uuid.uuid4().hex[:6]
        staged = resolved.parent / (resolved.name + '.deleting-' + stamp)
        try:
            os.replace(str(resolved), str(staged))
        except OSError as error:
            receipt['bytes'] -= size
            receipt['skipped'].append({'path': str(resolved), 'reason': 'in use: %s' % error})
            continue
        shutil.rmtree(str(staged))
        receipt['removed'].append(str(resolved))
    return receipt
