"""Owned process cleanup checks; these do not compile or test the product."""

import importlib.util
import os
from pathlib import Path
import signal
import sys
import tempfile
import time
import unittest
from unittest.mock import patch


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
spec = importlib.util.spec_from_file_location("node_acceptance", SCRIPTS / "run-node-acceptance.py")
entry = importlib.util.module_from_spec(spec)
spec.loader.exec_module(entry)


class ProcessCleanupTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="khor-runner-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.marker = self.root / "parent.pid"
        self.script = self.root / "owned-tree"
        self.script.write_text(
            '#!/bin/sh\nprintf "%s\\n" "$$" > "$1"\n'
            'for child in 1 2 3 4; do /bin/sleep "$2" & done\nwait\n'
        )
        self.script.chmod(0o700)

    def tearDown(self):
        if not self.marker.exists():
            return
        pid = int(self.marker.read_text())
        if entry.group_exists(pid):
            os.killpg(pid, signal.SIGKILL)
        try:
            os.waitpid(pid, 0)
        except ChildProcessError:
            pass
        deadline = time.monotonic() + 5
        while entry.group_exists(pid) and time.monotonic() < deadline:
            time.sleep(0.025)
        self.assertFalse(entry.group_exists(pid), "owned control group must be gone")

    def run_tree(self, limit, duration):
        with patch.object(entry, "MAX_OBSERVED_PROCESSES", limit), patch.object(
            entry, "TIMEOUT_SECONDS", 3
        ), patch.dict(os.environ, {"KHOR_TEST_MCP_EXECUTABLE": ""}):
            return entry.run(
                self.script, [str(self.marker), duration], self.root / "run.log", self.root
            )

    def test_inventory_failure_still_drains_the_owned_group(self):
        with self.assertRaisesRegex(RuntimeError, "ownership inventory exceeded"):
            self.run_tree(2, "20")
        self.assertTrue(self.marker.exists())
        self.assertFalse(entry.group_exists(int(self.marker.read_text())))

    def test_configured_inventory_keeps_the_normal_exit_checks(self):
        result, _ = self.run_tree(16, "0.2")
        self.assertEqual(result["exit"], 0)
        self.assertGreaterEqual(len(result["observed_owners"]), 5)
        self.assertFalse(result["forced_cleanup"])
        self.assertTrue(result["group_absent"])
        self.assertEqual(result["remaining_owners"], [])


if __name__ == "__main__":
    unittest.main()
