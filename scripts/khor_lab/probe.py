"""Disposable probe directories: unique per run, never reused, never cleared in place."""
import json
import shutil
import time
import uuid
from datetime import datetime, timedelta, timezone
from pathlib import Path

from .safepaths import PROBE_MARKER, UnsafePath, require_marker, resolve_owned

MARKER = PROBE_MARKER


def probes_root(root):
    return Path(root).resolve() / '.probe'


def new_probe(root, kind):
    """Create a fresh marked probe directory and return its absolute path."""
    root = Path(root).resolve()
    base = probes_root(root)
    base.mkdir(parents=True, exist_ok=True)
    token = time.strftime('%Y%m%dT%H%M%SZ', time.gmtime()) + '-' + uuid.uuid4().hex[:8]
    directory = resolve_owned(base / token, [root])
    directory.mkdir()
    record = {'id': token, 'kind': kind, 'created_at': datetime.now(timezone.utc).isoformat()}
    (directory / MARKER).write_text(json.dumps(record, indent=2) + '\n', encoding='utf-8')
    return directory


def gc_probes(root, older_than, dry_run=True, now=None):
    """Remove marked probe directories older than ``older_than``; return the targets."""
    root = Path(root).resolve()
    base = probes_root(root)
    if not base.is_dir():
        return []
    moment = now or datetime.now(timezone.utc)
    targets = []
    for entry in sorted(base.iterdir()):
        if not entry.is_dir():
            continue
        try:
            record = require_marker(entry)
            created = datetime.fromisoformat(str(record['created_at']))
        except (UnsafePath, KeyError, TypeError, ValueError):
            continue
        if created.tzinfo is None:
            created = created.replace(tzinfo=timezone.utc)
        if moment - created < older_than:
            continue
        target = resolve_owned(entry, [root])
        targets.append(target)
        if not dry_run:
            shutil.rmtree(str(target))
    return targets
