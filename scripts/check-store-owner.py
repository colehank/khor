#!/usr/bin/env python3
"""Exercise the production, std-only ownership rules and scratch SSH boundary.

Requires rustc, but no Cargo dependencies, network, or installed khor. The helper
uses a supplied scratch machine fingerprint; Node's hardware lookup and the CLI
dispatch are separately covered by their Cargo tests.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    repo = Path(__file__).resolve().parent.parent
    module = repo / "crates/node/src/store_owner.rs"
    output = Path(os.environ.get("STORE_OWNER_OUT") or tempfile.mkdtemp(prefix="khor-owner-evidence-"))
    output.mkdir(parents=True, exist_ok=True)
    report = {"module_sha256": hashlib.sha256(module.read_bytes()).hexdigest(), "checks": []}

    def run(name, args, env=None):
        result = subprocess.run(args, env=env, text=True, capture_output=True, timeout=180)
        (output / (name + ".log")).write_text(result.stdout + result.stderr)
        report["checks"].append({"name": name, "code": result.returncode})
        (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")
        print(name, "PASS" if result.returncode == 0 else "FAIL")
        return result.returncode

    with tempfile.TemporaryDirectory(prefix="khor-owner-rust-") as directory:
        scratch = Path(directory).resolve()
        tests = scratch / "owner-tests"
        if run("compile-tests", ["rustc", "--edition=2024", "--test", str(module), "-o", str(tests)]):
            return 1
        if run("rust-rules", [str(tests)]):
            return 1
        probe = scratch / "probe.rs"
        # Path is escaped as a Rust string (JSON string escaping is compatible
        # for this UTF-8 path once ensure_ascii is disabled).
        probe.write_text("#[path = " + json.dumps(str(module), ensure_ascii=False) + "]\n" + '''mod store_owner;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 4);
    if let Err(error) = store_owner::verify(std::path::Path::new(&args[1]), &args[2], Some(&args[3])) {
        eprintln!("{error:?}");
        std::process::exit(1);
    }
}
''')
        binary = scratch / "owner-probe"
        if run("compile-probe", ["rustc", "--edition=2024", str(probe), "-o", str(binary)]):
            return 1
        environment = {**os.environ, "ONSET_TEST_OWNER_VERIFIER": str(binary),
                       "ONSET_TEST_OUT": str(output / "uninstall"), "PYTHONDONTWRITEBYTECODE": "1"}
        return run("uninstall-with-rust-owner", [sys.executable, str(repo / "scripts/check-onset-uninstall.py")], environment)


if __name__ == "__main__":
    sys.exit(main())
