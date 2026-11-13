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

    def test_api_bundle_roundtrip_preserves_the_snapshot_without_untracked_secrets(self):
        Path("private.key").write_text("must not leave the workstation\n")
        info = dict(dev.snapshot(), request_id="f" * 32, repository="private/build")
        original = dev.command
        captured = []
        def command(*args, **kwargs):
            if args[0] == "git" and "push" in args:
                self.assertEqual(args[-2], "https://github.com/private/build.git")
                commit, ref = args[-1].split(":", 1)
                self.assertEqual(ref, "refs/heads/dev-bundles/" + info["request_id"])
                captured.append(subprocess.check_output(["git", "-C", args[2], "show", commit + ":source.bundle"]))
                return ""
            return original(*args, **kwargs)
        with patch.object(dev, "command", side_effect=command):
            receipt = dev._upload_source_bundle(info)
        self.assertEqual(len(captured), 1)
        bundle = Path("source.bundle")
        bundle.write_bytes(captured[0])
        self.assertEqual(receipt["bundle_bytes"], len(captured[0]))
        self.assertEqual(receipt["bundle_sha256"], dev.hashlib.sha256(captured[0]).hexdigest())
        self.git("clone", "--quiet", str(bundle.resolve()), "restored")
        self.assertEqual(self.git("-C", "restored", "rev-parse", "HEAD"), info["snapshot_commit"])
        self.assertEqual(self.git("-C", "restored", "rev-parse", "HEAD:source"), info["tree"])
        self.assertEqual(len(self.git("-C", "restored", "rev-list", "HEAD").splitlines()), 1)
        self.assertFalse(Path("restored/source/private.key").exists())

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

    def test_submit_dispatches_the_selected_control_branch_and_records_it(self):
        request_id = "a" * 32
        run = dict(databaseId=123, displayTitle="dev net-unit " + request_id, url="https://example.invalid/run")
        def command(*args):
            if args[:2] == ("gh", "api"):
                return json.dumps({"private": True})
            if args[:3] == ("gh", "run", "list"):
                return json.dumps([run])
            return ""
        info = dict(base_commit="b" * 40, tree="c" * 40, snapshot_commit="d" * 40, excluded_untracked_count=0)
        args = SimpleNamespace(builder_repo="private/build", builder_ref="feat/net", task="net-unit")
        with patch.object(dev, "command", side_effect=command) as calls, patch.object(dev, "snapshot", return_value=info), patch.object(dev, "upload_snapshot") as upload, patch.object(dev, "state_dir", return_value=Path("receipts")), patch.object(dev.uuid, "uuid4", return_value=SimpleNamespace(hex=request_id)), contextlib.redirect_stdout(io.StringIO()):
            dev.submit(args)
        calls.assert_any_call("git", "check-ref-format", "refs/heads/feat/net")
        dispatch = next(c.args for c in calls.call_args_list if c.args[:3] == ("gh", "workflow", "run"))
        self.assertEqual(dispatch[dispatch.index("--ref") + 1], "feat/net")
        self.assertEqual(upload.call_count, 1)
        receipt = json.loads((Path("receipts/runs") / (request_id + ".json")).read_text())
        self.assertEqual(receipt["builder_ref"], "feat/net")
        self.assertEqual(receipt["databaseId"], 123)
        self.assertEqual(receipt["executor"], "standard")
        self.assertNotIn("executor=mac", dispatch)

    def test_mac_dispatch_keeps_the_source_receipt_and_selects_the_existing_worker(self):
        self.assert_mac_dispatch("net-vendor")

    def test_mac_vz_probe_keeps_the_exact_source_and_api_bundle_receipt(self):
        self.assert_mac_dispatch("netlab-vz-probe")

    def assert_mac_dispatch(self, task):
        request_id = "e" * 32
        run = dict(databaseId=456, displayTitle="dev " + task + " " + request_id,
                   url="https://example.invalid/mac-run")
        def command(*args):
            if args[:2] == ("gh", "api"):
                return json.dumps({"private": True})
            if args[:3] == ("gh", "run", "list"):
                return json.dumps([run])
            return ""
        info = dict(base_commit="b" * 40, tree="c" * 40, snapshot_commit="d" * 40,
                    excluded_untracked_count=0)
        args = SimpleNamespace(builder_repo="private/build", builder_ref="feat/net",
                               task=task, executor="mac")
        bundle = dict(bundle_blob_sha="1" * 40, bundle_sha256="2" * 64,
                      bundle_ref="dev-bundles/" + request_id)
        with patch.object(dev, "command", side_effect=command) as calls, patch.object(dev, "snapshot", return_value=info), patch.object(dev, "upload_snapshot") as upload, patch.object(dev, "upload_source_bundle", return_value=bundle) as transport, patch.object(dev, "state_dir", return_value=Path("receipts")), patch.object(dev.uuid, "uuid4", return_value=SimpleNamespace(hex=request_id)), contextlib.redirect_stdout(io.StringIO()):
            dev.submit(args)
        dispatch = next(c.args for c in calls.call_args_list if c.args[:3] == ("gh", "workflow", "run"))
        self.assertIn("executor=mac", dispatch)
        self.assertIn("development.yml", dispatch)
        self.assertEqual(upload.call_count, 1)
        self.assertEqual(transport.call_count, 1)
        self.assertIn("bundle_blob_sha=" + bundle["bundle_blob_sha"], dispatch)
        self.assertIn("bundle_sha256=" + bundle["bundle_sha256"], dispatch)
        receipt = json.loads((Path("receipts/runs") / (request_id + ".json")).read_text())
        self.assertEqual(receipt["executor"], "mac")
        self.assertEqual(receipt["task"], task)
        self.assertEqual(receipt["snapshot_commit"], "d" * 40)
        self.assertEqual(receipt["databaseId"], 456)
        self.assertEqual(receipt["bundle_ref"], bundle["bundle_ref"])

    def test_mac_rejects_linux_and_native_tasks_before_upload_or_dispatch(self):
        with patch.object(dev, "command") as command, patch.object(dev, "snapshot") as snapshot:
            for task in ["netlab", "netlab-recovery", "native", "lab", "fast"]:
                with self.subTest(task=task), self.assertRaisesRegex(ValueError, "supports portable"):
                    dev.submit(SimpleNamespace(task=task, executor="mac"))
        command.assert_not_called()
        snapshot.assert_not_called()

    def test_vz_probe_requires_mac_before_upload_or_dispatch(self):
        with patch.object(dev, "command") as command, patch.object(dev, "snapshot") as snapshot:
            with self.assertRaisesRegex(ValueError, "requires --executor mac"):
                dev.submit(SimpleNamespace(task="netlab-vz-probe", executor="standard"))
        command.assert_not_called()
        snapshot.assert_not_called()

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
        unavailable = subprocess.CalledProcessError(1, ["gh", "run", "download"])
        with patch.object(dev, "command", side_effect=[json.dumps(run), unavailable, ""]) as fetch, patch.object(dev, "submitted_source", return_value=None), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(ValueError, "did not pass"):
                dev.result(args)
        self.assertIn("dev-portable-evidence", fetch.call_args.args)


if __name__ == "__main__":
    unittest.main()
