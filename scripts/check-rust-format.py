#!/usr/bin/env python3
"""First-party Rust formatting; check by default, write only with --write."""
import argparse
import hashlib
import os
import shutil
from pathlib import Path
import stat
import subprocess
import tempfile
import tomllib

def fail(message):
    raise SystemExit(message)


environment = {**os.environ, "RUSTUP_AUTO_INSTALL": "0"}


def diagnostic(stderr):
    lines = (stderr or b"").decode(errors="replace").splitlines()
    return next((line.strip()[:240] for line in lines if line.strip()), "no diagnostic")


def invoke(args, label=None, **kwargs):
    context = label or Path(args[0]).name
    try:
        return subprocess.run(args, capture_output=True, timeout=30, env=environment, **kwargs)
    except subprocess.TimeoutExpired as error:
        fail(f"{context}: {Path(args[0]).name} timed out: {diagnostic(error.stderr)}")
    except OSError as error:
        fail(f"{context}: cannot run {Path(args[0]).name}: {str(error)[:240]}")


def run(args, label=None, **kwargs):
    result = invoke(args, label, **kwargs)
    if result.returncode:
        fail(f"{label or Path(args[0]).name}: exit {result.returncode}: {diagnostic(result.stderr)}")
    return result.stdout


def resolve_formatter():
    rustup = shutil.which("rustup")
    explicit = os.environ.get("RUSTFMT")
    if explicit:
        candidate = shutil.which(explicit)
        if not candidate or (rustup and os.path.samefile(candidate, rustup)):
            fail("RUSTFMT must name an executable formatter, not a rustup proxy")
        return str(Path(candidate).resolve())
    why = "rustfmt not found"
    if rustup:
        result = invoke([rustup, "which", "rustfmt"])
        candidate = Path(os.fsdecode(result.stdout).strip())
        if result.returncode == 0 and candidate.is_file() and os.access(candidate, os.X_OK):
            if not os.path.samefile(candidate, rustup):
                return str(candidate.resolve())
        why = diagnostic(result.stderr)
    candidate = shutil.which("rustfmt")
    if candidate and (not rustup or not os.path.samefile(candidate, rustup)):
        return str(Path(candidate).resolve())
    fail(f"set RUSTFMT to the installed real formatter; no install attempted: {why}")


def inventory(root):
    output = run(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", "*.rs"], cwd=root)
    return sorted(set(os.fsdecode(p) for p in output.split(b"\0") if p))


def read(root, relative):
    path = root / relative
    if not path.resolve().is_relative_to(root) or any(p.is_symlink() for p in (path, *path.parents) if p != root and root in p.parents):
        fail(f"unsafe source path: {relative}")
    if not path.is_file():
        fail(f"missing or non-file input: {relative}")
    return path.read_bytes()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--write", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    formatter = resolve_formatter()
    if run([formatter, "--version"]).decode().split()[:2] != ["rustfmt", "1.9.0-stable"]:
        fail("requires rustfmt 1.9.0-stable (Rust 1.95.0); no automatic install")

    config = "rustfmt.toml"
    inputs = {"Cargo.toml": read(root, "Cargo.toml"), config: read(root, config)}
    workspace = tomllib.loads(inputs["Cargo.toml"].decode())["workspace"]
    harness = "crates/node/tests/windows-process-harness"
    editions = {}
    for member in [*workspace["members"], harness]:
        manifest = member + "/Cargo.toml"
        inputs[manifest] = read(root, manifest)
        edition = tomllib.loads(inputs[manifest].decode())["package"]["edition"]
        editions[member] = str(workspace["package"]["edition"] if isinstance(edition, dict) else edition)
    standalone = {"apps/gui/lab/terminal-color-answers.rs", "apps/gui/scripts/editor-history-probe/lib.rs", "apps/gui/scripts/native-menu-probe/lib.rs", "scripts/fixtures/native-io.rs", "scripts/fixtures/native-paste.rs"}
    thirdparty = "apps/gui/scripts/fixtures/native-events/tauri-2.11.5/event-mod.rs"
    expected = "dfcb7cf11d7674903f71196e64ae0fa95286757ac816bfbc3139ebdedcb25dea"
    paths = inventory(root)
    if thirdparty not in paths or hashlib.sha256(read(root, thirdparty)).hexdigest() != expected:
        fail("third-party snapshot missing or SHA contract changed")
    formatted = {}
    for relative in paths:
        inputs[relative] = read(root, relative)
        if relative.startswith("vendor/") or relative == thirdparty:
            continue
        owners = [p for p in editions if relative.startswith(p + "/")]
        edition = editions[max(owners, key=len)] if owners else "2024" if relative in standalone else None
        if edition is None:
            fail(f"unclassified Rust input: {relative}")
        # Stdin formatting does not load external modules or resolve template paths.
        formatted[relative] = run([formatter, "--emit", "stdout", "--edition", edition, "--config", "style_edition=" + edition, "--config-path", str(root / config)], label=relative, input=inputs[relative], cwd=root)
    if inventory(root) != paths or any(read(root, p) != original for p, original in inputs.items()):
        fail("inputs changed during formatting; nothing written")
    changed = [p for p, output in formatted.items() if output != inputs[p]]
    if args.write:
        for relative in changed:
            if read(root, relative) != inputs[relative]:
                fail(f"input changed before write: {relative}; earlier files may already be formatted")
            path = root / relative
            pending = None
            try:
                with tempfile.NamedTemporaryFile(dir=path.parent, prefix="." + path.name + ".fmt-", delete=False) as tmp:
                    pending = Path(tmp.name)
                    tmp.write(formatted[relative])
                os.chmod(pending, stat.S_IMODE(path.stat().st_mode))
                os.replace(pending, path)
            finally:
                if pending is not None:
                    pending.unlink(missing_ok=True)
        if any(read(root, p) != inputs[p] for p in paths if p not in formatted):
            fail("excluded third-party input changed")
    print(f"Rust format: {len(formatted)} first-party files, {len(paths) - len(formatted)} excluded, {len(changed)} {'written' if args.write else 'differ'}")
    if not args.write and changed:
        print("\n".join(changed[:20]))
        raise SystemExit(1)


if __name__ == "__main__":
    main()
