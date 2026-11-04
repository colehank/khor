import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import dev
import source_hook


class RemoteDevelopmentTest(unittest.TestCase):
    def setUp(self):
        # Git hooks export repository/index selectors; a fixture must never inherit them.
        environment = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
        self.environment = patch.dict(os.environ, environment, clear=True)
        self.environment.start()
        self.addCleanup(self.environment.stop)
        self.directory = tempfile.TemporaryDirectory()
        self.previous = Path.cwd()
        os.chdir(self.directory.name)
        self.git("init", "-q")
        self.assertEqual(Path(self.git("rev-parse", "--show-toplevel")).resolve(), Path(self.directory.name).resolve())
        self.git("config", "user.name", "Test")
        self.git("config", "user.email", "test@localhost")
        self.git("config", "commit.gpgsign", "false")
        Path("source.json").write_text('{"state": "base"}\n')
        self.git("add", "source.json")
        self.git("commit", "-qm", "base")

    def tearDown(self):
        os.chdir(self.previous)
        self.directory.cleanup()

    def git(self, *args):
        return subprocess.check_output(["git", *args], text=True).strip()

    def test_snapshot_captures_worktree_without_stashing_or_including_untracked_secrets(self):
        Path("source.json").write_text('{"state": "staged"}\n')
        self.git("add", "source.json")
        Path("source.json").write_text('{"state": "working"}\n')
        Path("new.py").write_text("pass\n")
        self.git("add", "new.py")
        Path("private.key").write_text("not source\n")
        before = self.git("status", "--porcelain")
        original_index = self.git("show", ":source.json")
        info = dev.snapshot()
        self.assertEqual(self.git("show", info["snapshot_commit"] + ":source/source.json"), '{"state": "working"}')
        self.assertEqual(self.git("show", info["snapshot_commit"] + ":source/new.py"), "pass")
        self.assertNotIn("private.key", self.git("ls-tree", "-r", "--name-only", info["snapshot_commit"]))
        self.assertEqual(info["excluded_untracked_count"], 1)
        self.assertEqual(self.git("status", "--porcelain"), before)
        self.assertEqual(self.git("show", ":source.json"), original_index)
        self.assertEqual(self.git("stash", "list"), "")
        self.assertEqual(len(self.git("rev-list", info["snapshot_commit"]).splitlines()), 1)

    def test_source_workflows_cannot_become_control_repository_workflows(self):
        workflow = Path(".github/workflows/uncommitted.yml")
        workflow.parent.mkdir(parents=True)
        workflow.write_text("on: push\n")
        self.git("add", str(workflow))
        info = dev.snapshot()
        self.assertEqual(self.git("ls-tree", "--name-only", info["snapshot_commit"]), "source")
        self.assertEqual(self.git("rev-parse", info["snapshot_commit"] + ":source"), info["tree"])

    def test_offline_hook_checks_the_index_even_when_the_working_file_is_valid(self):
        Path("source.json").write_text('{"state":}\n')
        self.git("add", "source.json")
        Path("source.json").write_text('{"state": "valid"}\n')
        with self.assertRaisesRegex(ValueError, "source.json"):
            source_hook.check_staged()
        self.assertEqual(json.loads(Path("source.json").read_text())["state"], "valid")

    def test_fixture_is_safe_when_started_from_a_git_hook_environment(self):
        before = self.git("rev-parse", "HEAD")
        env = dict(os.environ, GIT_DIR=str(Path.cwd() / ".git"), GIT_WORK_TREE=str(Path.cwd()),
                   GIT_INDEX_FILE=str(Path.cwd() / ".git/index"))
        result = subprocess.run([sys.executable, str(Path(__file__).resolve()),
                                 "RemoteDevelopmentTest.test_snapshot_captures_worktree_without_stashing_or_including_untracked_secrets"],
                                env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.git("rev-parse", "HEAD"), before)
        self.assertEqual(self.git("status", "--porcelain"), "")

    def test_configure_and_restore_preserve_existing_hooks_and_reject_external_override(self):
        Path("old-hooks").mkdir()
        old = Path("old-hooks/pre-commit")
        old.write_text("existing graph and pre-commit\n")
        self.git("config", "core.hooksPath", "old-hooks")
        with contextlib.redirect_stdout(io.StringIO()):
            dev.configure()
            dev.configure()
        installed = self.git("config", "--get", "core.hooksPath")
        self.assertEqual(old.read_text(), "existing graph and pre-commit\n")
        self.git("config", "core.hooksPath", "other-hooks")
        with self.assertRaisesRegex(ValueError, "changed since"):
            dev.configure(restore=True)
        self.git("config", "core.hooksPath", installed)
        with contextlib.redirect_stdout(io.StringIO()):
            dev.configure(restore=True)
        self.assertEqual(self.git("config", "--get", "core.hooksPath"), "old-hooks")

    def test_installed_hooks_allow_a_real_commit_without_node_or_cargo(self):
        graph = Path(".git/hooks/pre-commit.legacy")
        graph.write_text('#!/bin/sh\n# Installed by code-review-graph\nprintf graph > graph-hook-ran\n')
        graph.chmod(0o755)
        with contextlib.redirect_stdout(io.StringIO()):
            dev.configure()
        Path("source.json").write_text('{"state": "changed"}\n')
        self.git("add", "source.json")
        minimal = Path("minimal-bin")
        minimal.mkdir()
        for program in ["git", "sh"]:
            import shutil
            (minimal / program).symlink_to(shutil.which(program))
        env = dict(os.environ, PATH=str(minimal.resolve()))
        result = subprocess.run(["git", "commit", "-qm", "offline"], env=env, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("NOT validated locally", result.stdout + result.stderr)
        self.assertEqual(Path("graph-hook-ran").read_text(), "graph")

    def test_result_rejects_an_artifact_for_other_source(self):
        output = Path("download")
        output.mkdir()
        (output / "source.json").write_text(json.dumps({"snapshot_commit": "other", "base_commit": "base", "tree": "tree"}))
        expected = {"snapshot_commit": "requested", "base_commit": "base", "tree": "tree"}
        args = SimpleNamespace(run_id="123", builder_repo="private/build", output=str(output), lab=False)
        run = {"status": "completed", "conclusion": "success", "url": "https://example.invalid/run"}
        with patch.object(dev, "command", return_value=json.dumps(run)), patch.object(dev, "submitted_source", return_value=expected):
            with self.assertRaisesRegex(ValueError, "does not match"):
                dev.result(args)

    def test_failed_result_retains_evidence_but_is_not_success(self):
        output = Path("download")
        output.mkdir()
        (output / "source.json").write_text("{}")
        args = SimpleNamespace(run_id="123", builder_repo="private/build", output=str(output), lab=False)
        run = {"status": "completed", "conclusion": "failure", "url": "https://example.invalid/run"}
        with patch.object(dev, "command", return_value=json.dumps(run)), patch.object(dev, "submitted_source", return_value=None), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(ValueError, "did not pass"):
                dev.result(args)


if __name__ == "__main__":
    unittest.main()
