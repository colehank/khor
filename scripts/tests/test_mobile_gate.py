"""Check gate orchestration without starting Rust or requiring the iOS SDK."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


REPO = Path(__file__).resolve().parents[2]


class MobileGateTest(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="khor-mobile-runner-")
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        scripts = self.root / "apps/gui/scripts"
        scripts.mkdir(parents=True)
        self.script = scripts / "check-mobile.mjs"
        shutil.copy2(REPO / "apps/gui/scripts/check-mobile.mjs", self.script)
        (scripts / "check-mobile-static.mjs").write_text(
            'export const checkMobileSources = () => process.env.STATIC_FAIL === "1" ? ["fixture source failure"] : [];\n'
        )
        dist = self.root / "apps/gui/dist"
        dist.mkdir()
        (dist / "index.html").write_text("fixture")
        self.bin = self.root / "bin"
        self.bin.mkdir()
        for name, body in {
            "rustup": 'echo rustup >> "$MOBILE_CALLS"; echo aarch64-apple-ios',
            "cargo": 'echo cargo >> "$MOBILE_CALLS"; exit "${CARGO_STATUS:-0}"',
        }.items():
            path = self.bin / name
            path.write_text("#!/bin/sh\n" + body + "\n")
            path.chmod(0o755)

    def run_gate(self, *args, **overrides):
        self.result = subprocess.run(
            ["node", str(self.script), *args],
            env={**os.environ, "PATH": f"{self.bin}:{os.environ['PATH']}",
                 "MOBILE_CALLS": str(self.root / "calls"), **overrides},
            text=True, capture_output=True, timeout=5,
        )
        calls = self.root / "calls"
        self.calls = calls.read_text().splitlines() if calls.exists() else []
        return self.result.returncode

    def test_static_only_never_probes_or_compiles(self):
        self.assertEqual(self.run_gate("--static-only"), 0, self.result.stderr)
        self.assertEqual(self.calls, [])

    def test_static_failure_stops_before_toolchain(self):
        self.assertEqual(self.run_gate(STATIC_FAIL="1"), 1, self.result.stderr)
        self.assertEqual(self.calls, [])

    def test_static_failure_is_reported(self):
        self.assertEqual(self.run_gate("--static-only", STATIC_FAIL="1"), 1)
        self.assertIn("fixture source failure", self.result.stderr)
        self.assertEqual(self.calls, [])

    def test_conflicting_modes_are_rejected(self):
        self.assertEqual(self.run_gate("--static-only", "--compile-only"), 2)
        self.assertEqual(self.calls, [])

    @unittest.skipUnless(os.uname().sysname == "Darwin", "compiler control needs macOS")
    def test_compile_failure_preserves_exit_code(self):
        self.assertEqual(self.run_gate("--compile-only", CARGO_STATUS="23"), 23)
        self.assertEqual(self.calls, ["rustup", "cargo"])

    @unittest.skipUnless(os.uname().sysname != "Darwin", "non-macOS behavior")
    def test_non_macos_compile_refuses_instead_of_claiming_a_pass(self):
        self.assertEqual(self.run_gate("--compile-only"), 2)
        self.assertEqual(self.calls, [])


if __name__ == "__main__":
    unittest.main()
