#!/usr/bin/env python3
"""Exercise the production SSH script and isolated production Rust rename code.

--shell-only is safe on source-only machines. The Rust harness compiles verbatim
production functions with tiny diagnostic stubs, avoiding unrelated node builds.
Full-crate compilation remains the ordinary node CI's responsibility.
"""
import argparse
from pathlib import Path
import re
import shlex
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def shell_checks():
    source = (ROOT / 'crates/node/src/satellite.rs').read_text()
    script = re.search(r'const RENAME_SH: &str = r#"(.*?)"#;', source, re.S)[1]
    with tempfile.TemporaryDirectory(prefix='khor-rename-check-') as scratch:
        root = Path(scratch)

        def move(a, b):
            command = f'KHOR_FROM={shlex.quote(str(a))}\nKHOR_TO={shlex.quote(str(b))}\n{script}'
            return subprocess.run(['sh', '-c', command], text=True, capture_output=True, check=True).stdout.strip()

        old, new = root / 'Methods.md', root / 'methods.md'
        old.write_text('keeper')
        insensitive = new.exists()
        assert move(old, new) == 'done'
        assert [p.name for p in root.iterdir()] == ['methods.md']
        assert new.read_text() == 'keeper'
        occupied = root / 'occupied'
        occupied.write_text('untouched')
        assert move(new, occupied) == 'taken'
        assert occupied.read_text() == 'untouched' and new.read_text() == 'keeper'
        assert move(new, new) == 'taken'
        if not insensitive:
            old.hardlink_to(new)
            assert move(old, new) == 'taken'
            assert old.read_text() == new.read_text() == 'keeper'
            old.unlink()
        link, renamed = root / 'Link', root / 'link'
        link.symlink_to('missing')
        assert move(link, renamed) == 'done'
        assert renamed.is_symlink() and renamed.readlink() == Path('missing')
        assert move(new, renamed) == 'taken'
        alias = root / 'alias'
        alias.symlink_to(root, target_is_directory=True)
        assert move(new, alias / 'METHODS.md') == 'done'
        assert (root / 'METHODS.md').read_text() == 'keeper'
        source, target = root / 'Source', root / 'target'
        source.mkdir()
        (source / 'draft').write_text('draft')
        assert move(source, root / 'source') == 'done'
        source = root / 'source'
        target.mkdir()
        assert move(source, target) == 'taken'
        assert move(source, source / 'child') == 'no'
        assert move(source, target / "source ' $x") == 'done'
        assert not source.exists() and (target / "source ' $x/draft").read_text() == 'draft'
        assert not list(root.glob('.khor-rename-*'))
        print(f'PASS production SSH rename: case_insensitive={insensitive}; files, directories, links, physical parent aliases, conflicts, descendants, quoting, cleanup')


def rust_checks():
    files = (ROOT / 'crates/node/src/files.rs').read_text()
    functions = files[files.index('pub fn rename_path('):files.index('/// What this machine opens')]
    tests = files[files.index('#[cfg(test)]\nmod rename_tests {'):]
    tree = (ROOT / 'crates/node/src/tree.rs').read_text()
    rename_new = tree[tree.index('pub(crate) fn rename_new('):tree.index('\npub fn permissions(')]
    with tempfile.TemporaryDirectory(prefix='khor-rename-rust-') as scratch:
        root = Path(scratch)
        (root / 'src').mkdir()
        (root / 'Cargo.toml').write_text('[package]\nname="khor-rename-check"\nversion="0.0.0"\nedition="2024"\n[dependencies]\nlibc="0.2"\ntempfile="3"\n')
        (root / 'src/lib.rs').write_text('mod msg {\npub fn path_not_absolute(p: &str) -> String { p.into() }\npub fn name_taken(p: &str) -> String { p.into() }\npub fn cant_move_name(p: &str, e: impl std::fmt::Display) -> String { format!("{p}: {e}") }\n}\nmod tree { use std::path::Path;\n' + rename_new + '\n}\n' + functions + tests)
        subprocess.run(['cargo', 'test', '--manifest-path', str(root / 'Cargo.toml')], check=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--shell-only', action='store_true')
    parser.add_argument('--rust-only', action='store_true')
    args = parser.parse_args()
    if not args.rust_only:
        shell_checks()
    if not args.shell_only:
        rust_checks()
