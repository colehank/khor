"""Owned process cleanup checks; these do not compile or test the product."""

import importlib.util
import json
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


class PersistentOwnershipTests(unittest.TestCase):
    def test_suite_growth_runs_every_isolated_case_without_a_stale_count(self):
        cases = [f"test-{index:02}" for index in range(13)]
        self.assertEqual(entry.isolated_tests(cases, cases), cases)
        self.assertEqual(entry.isolated_tests(cases, cases, {cases[-1]}), cases[:-1])
        with self.assertRaises(ValueError):
            entry.isolated_tests(["worker"], ["worker"], {"worker"})
        for all_tests, ignored in [([], []), (cases, cases[:-1]), (["a", "a"], ["a", "a"])]:
            with self.assertRaises(ValueError):
                entry.isolated_tests(all_tests, ignored)

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="khor-persistent-proof-")
        self.addCleanup(self.temp.cleanup)
        self.fixture = Path(self.temp.name).resolve()
        self.home = self.fixture / "store"
        self.directory = self.home / ".khor/sessions/shell-private"
        self.directory.mkdir(parents=True)
        self.session = "shell/private"
        self.meta = {"id": self.session, "kind": "shell", "pid": 10, "started_ms": 1}
        self.host = {"host_pid": 10, "child_pid": 11, "port": 1234, "cookie": "not-logged"}
        self.write_markers()
        def row(pid, parent, group=None):
            return {"pid": pid, "parent": parent, "group": pid if group is None else group,
                    "start": f"original-start-{pid}", "executable": "same-app-name"}
        self.observed = {r["pid"]: r for r in (row(1, 0), row(10, 1), row(11, 10),
                                                 row(12, 11), row(20, 1), row(21, 1, 11))}
        self.rows = {pid: dict(row) for pid, row in self.observed.items()}

    def write_markers(self):
        (self.directory / "meta.json").write_text(json.dumps(self.meta))
        (self.directory / "host.json").write_text(json.dumps(self.host))

    def classify(self):
        return entry.persistent_session_owners(
            self.rows, self.observed, fixture_root=self.fixture, home=self.home,
            session_dir=self.directory, session_id=self.session)

    def test_only_proven_session_ancestry_is_separated_from_app_cleanup(self):
        persistent = self.classify()
        self.assertEqual(set(persistent), {10, 11, 12})
        self.rows[22] = {"pid": 22, "parent": 999, "group": 11,
                         "start": "unobserved-group-member", "executable": "same-app-name"}
        after_quit = {pid: row for pid, row in self.rows.items() if pid != 1}
        self.assertEqual({r["pid"] for r in entry.remaining_temporary(after_quit, self.observed, persistent)}, {20, 21, 22})
        self.assertEqual({r["pid"] for r in entry.remaining(after_quit, self.observed)},
                         {10, 11, 12, 20, 21, 22}, "global remaining stays strict")

    def test_pid_reuse_for_host_or_child_is_not_a_persistent_proof(self):
        for pid in (10, 11):
            with self.subTest(pid=pid):
                self.rows[pid]["start"] = "reused-pid"
                with self.assertRaisesRegex(ValueError, "identity"):
                    self.classify()
                self.rows[pid]["start"] = self.observed[pid]["start"]

    def test_a_reused_pid_is_not_subtracted_from_the_strict_quit_gate(self):
        persistent = self.classify()
        self.rows[10]["start"] = "new-birth-after-proof"
        blockers = entry.remaining_temporary(self.rows, self.observed, persistent)
        self.assertIn(10, {row["pid"] for row in blockers})

    def test_wrong_session_marker_or_child_parent_cannot_exempt_processes(self):
        for mutation in (lambda: self.meta.update(id="shell/other"),
                         lambda: self.meta.update(pid=20),
                         lambda: self.rows[11].update(parent=20),
                         lambda: self.rows[10].update(group=1)):
            with self.subTest(mutation=mutation):
                old_meta, old_rows = dict(self.meta), {pid: dict(row) for pid, row in self.rows.items()}
                mutation()
                self.write_markers()
                with self.assertRaises(ValueError):
                    self.classify()
                self.meta, self.rows = old_meta, old_rows
                self.write_markers()

    def test_outside_home_and_symlinked_markers_are_rejected(self):
        home = self.home
        self.home = self.fixture.parent
        with self.assertRaisesRegex(ValueError, "fixture|session"):
            self.classify()
        self.home = home
        marker = self.directory / "host.json"
        marker.rename(self.directory / "real-host.json")
        marker.symlink_to("real-host.json")
        with self.assertRaisesRegex(ValueError, "symlink|regular"):
            self.classify()

    def test_detached_host_keeps_the_observed_birth_identity(self):
        self.rows[10]["parent"] = 999
        self.rows[12]["parent"] = 1
        self.assertEqual(set(self.classify()), {10, 11, 12})


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
