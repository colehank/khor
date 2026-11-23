"""Read-only checks that the Lab keeps its writes inside the workspace."""
import os
from pathlib import Path


def _within(child, parent):
    try:
        child.relative_to(parent)
        return True
    except ValueError:
        return False


def doctor(root, values):
    """Return (name, ok, hint) rows for every path the Lab will write."""
    root = Path(root).resolve()
    checks = []
    cache = Path(values['cache_root']).expanduser().resolve()
    checks.append(('cache_root', _within(cache, root),
                   'set "cache_root": "<root>/.cache/lab" in .khor-lab.local.json'))
    target = os.environ.get('CARGO_TARGET_DIR')
    checks.append(('cargo_target', target is None or _within(Path(target).expanduser().resolve(), root),
                   'set CARGO_TARGET_DIR to "<root>/.cache/target" for builds'))
    for name in ('CARGO_HOME', 'RUSTUP_HOME', 'PNPM_HOME'):
        value = os.environ.get(name)
        inside = value is None or _within(Path(value).expanduser().resolve(), root)
        hints = 'inside the workspace' if inside else \
            'outside the workspace: adding or updating dependencies needs one approved full-access call'
        checks.append((name.lower(), True, hints))
    return checks
