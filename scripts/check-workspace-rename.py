#!/usr/bin/env python3
"""Exercise the production SSH script and isolated production Rust rename code.

--shell-only is safe on source-only machines. The Rust harness compiles verbatim
production functions with tiny diagnostic stubs, avoiding unrelated node builds.
Full-crate compilation remains the ordinary node CI's responsibility.
"""
import argparse
import os
import shutil
from pathlib import Path
import re
import shlex
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def shell_checks():
    source = (ROOT / 'crates/node/src/satellite.rs').read_text(encoding="utf-8")
    script = re.search(r'const RENAME_SH: &str = r#"(.*?)"#;', source, re.S)[1]
    with tempfile.TemporaryDirectory(prefix='khor-rename-check-') as scratch:
        root = Path(scratch)

        def move(a, b, extra_env=None):
            command = f'KHOR_FROM={shlex.quote(str(a))}\nKHOR_TO={shlex.quote(str(b))}\n{script}'
            return subprocess.run(['sh', '-c', command], text=True, capture_output=True, check=True, env={**os.environ, **(extra_env or {})}).stdout.strip()

        old, new = root / 'Methods.md', root / 'methods.md'
        old.write_text('keeper')
        insensitive = new.exists()
        assert move(old, new) == 'done'
        assert [p.name for p in root.iterdir()] == ['methods.md']
        assert new.read_text(encoding="utf-8") == 'keeper'
        occupied = root / 'occupied'
        occupied.write_text('untouched')
        assert move(new, occupied) == 'taken'
        assert occupied.read_text(encoding="utf-8") == 'untouched' and new.read_text(encoding="utf-8") == 'keeper'
        assert move(new, new) == 'taken'
        if not insensitive:
            old.hardlink_to(new)
            assert move(old, new) == 'taken'
            assert old.read_text(encoding="utf-8") == new.read_text(encoding="utf-8") == 'keeper'
            old.unlink()
        link, renamed = root / 'Link', root / 'link'
        link.symlink_to('missing')
        assert move(link, renamed) == 'done'
        assert renamed.is_symlink() and renamed.readlink() == Path('missing')
        assert move(new, renamed) == 'taken'
        alias = root / 'alias'
        alias.symlink_to(root, target_is_directory=True)
        assert move(new, alias / 'METHODS.md') == 'done'
        assert (root / 'METHODS.md').read_text(encoding="utf-8") == 'keeper'
        source, target = root / 'Source', root / 'target'
        source.mkdir()
        (source / 'draft').write_text('draft')
        assert move(source, root / 'source') == 'done'
        source = root / 'source'
        target.mkdir()
        assert move(source, target) == 'taken'
        assert move(source, source / 'child') == 'no'
        assert move(source, target / "source ' $x") == 'done'
        assert not source.exists() and (target / "source ' $x/draft").read_text(encoding="utf-8") == 'draft'
        assert not list(root.glob('.khor-rename-*'))
        if insensitive:
            # Fail the landing, then separately fail rollback. Exercise the
            # actual production shell, substituting only the OS move command.
            bin_dir = root / 'bin'
            bin_dir.mkdir()
            counter = root / 'counter'
            real_mv = shutil.which('mv')
            wrapper = bin_dir / 'mv'
            wrapper.write_text('#!/bin/sh\nn=0; [ ! -f "$KHOR_COUNTER" ] || n=$(cat "$KHOR_COUNTER"); n=$((n+1)); printf "%s" "$n" > "$KHOR_COUNTER"\nif [ "$n" = 2 ] || { [ "$n" = 3 ] && [ "$KHOR_FAIL_ROLLBACK" = yes ]; }; then exit 1; fi\nexec ' + shlex.quote(real_mv) + ' "$@"\n', encoding='utf-8')
            wrapper.chmod(0o755)
            env = {'PATH': str(bin_dir) + os.pathsep + os.environ['PATH'], 'KHOR_COUNTER': str(counter), 'KHOR_FAIL_ROLLBACK': 'no'}
            (root / 'Rollback').write_text('saved')
            assert move(root / 'Rollback', root / 'rollback', env) == 'no'
            assert (root / 'Rollback').read_text() == 'saved'
            assert not list(root.glob('.khor-rename-*'))
            counter.unlink()
            env['KHOR_FAIL_ROLLBACK'] = 'yes'
            result = move(root / 'Rollback', root / 'rollback', env)
            assert result.startswith('recovery:')
            parked = Path(result.removeprefix('recovery:'))
            assert parked.read_text() == 'saved' and not (root / 'Rollback').exists()
            print('PASS production SSH rename: landing failure rolls back; rollback failure preserves source at reported recovery path')
        print(f'PASS production SSH rename: case_insensitive={insensitive}; files, directories, links, physical parent aliases, conflicts, descendants, quoting, cleanup')


def rust_checks():
    files = (ROOT / 'crates/node/src/files.rs').read_text(encoding="utf-8")
    functions = files[files.index('pub fn rename_path('):files.index('/// What this machine opens')]
    tests = files[files.index('#[cfg(test)]\nmod rename_tests {'):]
    tree = (ROOT / 'crates/node/src/tree.rs').read_text(encoding="utf-8")
    rename_new = tree[tree.index('pub(crate) fn rename_new('):tree.index('\npub fn permissions(')]
    with tempfile.TemporaryDirectory(prefix='khor-rename-rust-') as scratch:
        root = Path(scratch)
        (root / 'src').mkdir()
        (root / 'Cargo.toml').write_text('[package]\nname="khor-rename-check"\nversion="0.0.0"\nedition="2024"\n[dependencies]\nlibc="0.2"\ntempfile="3"\n')
        (root / 'src/lib.rs').write_text('mod msg {\npub fn path_not_absolute(p: &str) -> String { p.into() }\npub fn name_taken(p: &str) -> String { p.into() }\npub fn cant_move_name(p: &str, e: impl std::fmt::Display) -> String { format!("{p}: {e}") }\n}\nmod tree { use std::path::Path;\n' + rename_new + '\n}\n' + functions + tests, encoding='utf-8')
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
