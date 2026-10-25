"""Exercise the netlab runner in a scratch checkout, without a network or Cargo."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "netlab.sh"


class NetlabRunnerTest(unittest.TestCase):
    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="khor-netlab-runner-")
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name)
        (self.root / "scripts").mkdir()
        shutil.copy2(SCRIPT, self.root / "scripts/netlab.sh")
        self.bin = self.root / "tools/bin"
        self.bin.mkdir(parents=True)
        for name, body in {
            "uname": "echo Linux",
            "nft": "exit 0",
            "tc": "exit 0",
            "cargo": '''
printf '%s\\n' "$*" >> "$NETLAB_CALLS"
case " $* " in
    *' --no-run '*)
        if [ "$NETLAB_FIXTURE" = compile-fail ]; then
            echo 'error: fixture compile failure' >&2
            exit 37
        fi
        echo '    Finished test profile'
        exit 0 ;;
esac
case "$NETLAB_FIXTURE" in
    test-fail)
        echo '[netlab-result] fixture | red | failed'
        echo 'test fixture ... FAILED'
        echo 'test result: FAILED. 0 passed; 1 failed'
        exit 42 ;;
    empty)
        echo 'test result: ok. 0 passed; 0 failed; 8 filtered out'
        exit 0 ;;
    no-report)
        echo 'test fixture ... ok'
        echo 'test result: ok. 1 passed; 0 failed'
        exit 0 ;;
    pass)
        echo '[netlab-result] fixture | green | connected'
        echo 'test fixture ... ok'
        echo 'test result: ok. 1 passed; 0 failed'
        exit 0 ;;
esac
exit 99
''',
        }.items():
            path = self.bin / name
            path.write_text("#!/bin/sh\nset -eu\n" + body + "\n")
            path.chmod(0o755)

    def run_fixture(self, fixture, *filters):
        result = subprocess.run(
            ["sh", str(self.root / "scripts/netlab.sh"), *filters],
            env={
                **os.environ,
                "PATH": f"{self.bin}:{os.environ['PATH']}",
                "NETLAB_TOOLS": str(self.bin.parent),
                "NETLAB_RUN_DIR": str(self.root / "logs"),
                "NETLAB_CALLS": str(self.root / "calls"),
                "NETLAB_FIXTURE": fixture,
            },
            text=True,
            capture_output=True,
            timeout=10,
        )
        self.output = result.stdout + result.stderr
        self.calls = (self.root / "calls").read_text().splitlines()
        return result.returncode

    def test_compile_failure_stops_before_scenarios(self):
        self.assertEqual(self.run_fixture("compile-fail"), 37, self.output)
        self.assertEqual(len(self.calls), 1)
        self.assertIn("fixture compile failure", self.output)

    def test_scenario_failure_retains_exit_code_and_report(self):
        self.assertEqual(self.run_fixture("test-fail"), 42, self.output)
        self.assertIn("fixture | red", self.output)
        self.assertIn("test result: FAILED", self.output)

    def test_empty_filter_is_not_a_passing_run(self):
        self.assertNotEqual(self.run_fixture("empty", "misspelled-filter"), 0, self.output)
        self.assertIn("no scenario results", self.output)

    def test_missing_scenario_report_is_not_a_passing_run(self):
        self.assertNotEqual(self.run_fixture("no-report"), 0, self.output)

    def test_success_preserves_filters_and_full_logs(self):
        self.assertEqual(self.run_fixture("pass", "nat", "relay"), 0, self.output)
        self.assertTrue(self.calls[1].endswith("--nocapture nat relay"), self.calls)
        logs = list((self.root / "logs").rglob("*.log"))
        self.assertEqual(len(logs), 2, self.output)
        self.assertTrue(any("Finished test profile" in p.read_text() for p in logs))
        self.assertTrue(any("[netlab-result] fixture" in p.read_text() for p in logs))


if __name__ == "__main__":
    unittest.main()
