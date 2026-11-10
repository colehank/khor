#!/usr/bin/env python3
"""Exercise the unchanged Rust retirement helper against a real WKWebView."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tomllib

HERE = Path(__file__).resolve().parent
GUI = HERE.parent.parent
REPO = GUI.parent.parent


def run(*args, **kwargs):
    print('+', ' '.join(map(str, args)), flush=True)
    return subprocess.run(list(map(str, args)), check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--remove-only', action='store_true', help='reproduce previous removal-only lifecycle')
    args = parser.parse_args()
    out = args.out.expanduser().resolve()
    if out == REPO or REPO in out.parents:
        raise RuntimeError('Probe artifacts belong outside the repository')
    out.mkdir(parents=True, exist_ok=True)
    marker = out / '.khor-page-retirement'
    if any(out.iterdir()) and not marker.exists():
        raise RuntimeError('Refusing an unowned nonempty probe directory')
    marker.write_text('Isolated page retirement probe\n')
    source = GUI / 'src-tauri/src/page_retire.rs'
    before = hashlib.sha256(source.read_bytes()).hexdigest()
    packages = tomllib.loads((REPO / 'Cargo.lock').read_text())['package']
    version = lambda name: next(p['version'] for p in packages if p['name'] == name)
    rust = out / 'rust'
    (rust / 'src').mkdir(parents=True, exist_ok=True)
    (rust / 'Cargo.toml').write_text(f'''[package]
name="khor-page-retirement-probe"
version="0.0.0"
edition="2024"
[lib]
crate-type=["cdylib"]
[dependencies]
objc2="={version('objc2')}"
objc2-foundation={{version="={version('objc2-foundation')}",default-features=false,features=["std","NSString"]}}
''')
    (rust / 'src/lib.rs').write_text(f'''#[path={json.dumps(str(source))}]
mod page_retire;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn khor_retire_page(view:*mut objc2::runtime::AnyObject) {{
    unsafe {{page_retire::stop(&*view)}}
}}
''')
    target = out / 'target'
    run('cargo', 'build', '--manifest-path', rust / 'Cargo.toml', env={**os.environ, 'CARGO_TARGET_DIR': str(target)})
    probe = out / 'probe'
    run('xcrun', 'swiftc', HERE / 'Probe.swift', '-o', probe, '-framework', 'WebKit', '-framework', 'AppKit')
    command = [probe, target / 'debug/libkhor_page_retirement_probe.dylib']
    if args.remove_only:
        command.append('--remove-only')
    result = subprocess.run(list(map(str, command)), capture_output=True, text=True, timeout=75)
    (out / ('remove-only.log' if args.remove_only else 'native.log')).write_text(result.stdout + result.stderr)
    after = hashlib.sha256(source.read_bytes()).hexdigest()
    if before != after:
        raise RuntimeError('Production source changed during probe')
    (out / ('remove-only.json' if args.remove_only else 'report.json')).write_text(json.dumps({
        'source': str(source), 'sha256': before, 'mode': 'remove-only' if args.remove_only else 'production',
        'exit_code': result.returncode, 'stdout': result.stdout, 'stderr': result.stderr,
    }, indent=2) + '\n')
    print(result.stdout, flush=True)
    if result.returncode:
        raise RuntimeError(f'Native page retirement failed: {result.stderr[-1800:]}')


if __name__ == '__main__':
    main()
