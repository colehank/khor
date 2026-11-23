"""Copy the DSH session store to a dated directory and prune old copies.

The store is the one thing on this machine that a deletion cannot regenerate,
so the copy is taken outside the workspace by a scheduled task, not by the
agent's shell.
"""
import argparse
from datetime import datetime, timezone
import shutil
import sys
from pathlib import Path

SOURCES = (
    'sessions',
    'storages',
    'states',
    'profiles/desktop/cordis.yml',
    'profiles/desktop/cordis.patch.yml',
    'profiles/desktop/package.json',
    'local-plugins/dsh-delete-guard/host.mjs',
    'local-plugins/dsh-delete-guard/guard.mjs',
    'local-plugins/dsh-delete-guard/package.json',
)


def plan(dsh_home):
    """Return (source, relative) pairs for the parts of the store that exist."""
    home = Path(dsh_home)
    return [(home / relative, relative) for relative in SOURCES if (home / relative).exists()]


def prune(destination, keep):
    """Remove the oldest dated copies beyond ``keep``; return their paths."""
    root = Path(destination)
    if not root.is_dir():
        return []
    copies = sorted((entry for entry in root.iterdir() if entry.is_dir()), reverse=True)
    removed = []
    for entry in copies[max(keep, 0):]:
        shutil.rmtree(str(entry))
        removed.append(str(entry))
    return removed


def run(dsh_home, destination, keep=7, now=None):
    """Copy the store, prune old copies, and return a receipt."""
    stamp = (now or datetime.now(timezone.utc)).strftime('%Y%m%dT%H%M%SZ')
    target = Path(destination) / stamp
    target.mkdir(parents=True, exist_ok=False)
    copied = []
    for source, relative in plan(dsh_home):
        destination_path = target / relative
        destination_path.parent.mkdir(parents=True, exist_ok=True)
        if source.is_dir():
            shutil.copytree(str(source), str(destination_path))
        else:
            shutil.copy2(str(source), str(destination_path))
        copied.append(relative)
    return {'target': str(target), 'copied': copied, 'pruned': prune(destination, keep), 'at': stamp}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dsh-home', default=str(Path.home() / '.dsh'))
    parser.add_argument('--destination', default='F:\\Backups\\dsh')
    parser.add_argument('--keep', type=int, default=7)
    args = parser.parse_args(argv)
    receipt = run(args.dsh_home, args.destination, args.keep)
    print(receipt['target'])
    print('copied %d entries; pruned %d old copies' % (len(receipt['copied']), len(receipt['pruned'])))
    return 0


if __name__ == '__main__':
    sys.exit(main())
