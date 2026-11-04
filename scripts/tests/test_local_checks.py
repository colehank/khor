import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import local_checks


class LocalChecksTest(unittest.TestCase):
    def test_invalid_source_fails_without_rewriting_it(self):
        with tempfile.TemporaryDirectory() as directory:
            for name, source in [("bad.py", b"def incomplete("), ("bad.json", b'{"x":}'), ("bad.toml", b"x = [")]:
                path = Path(directory) / name
                path.write_bytes(source)
                with self.subTest(name=name), self.assertRaises((SyntaxError, ValueError)):
                    local_checks.syntax([str(path)])
                self.assertEqual(path.read_bytes(), source)

    def test_unprepared_dependency_is_an_error_not_a_skipped_check(self):
        with patch.object(Path, "is_file", return_value=False), patch.object(local_checks, "run") as run:
            with self.assertRaisesRegex(RuntimeError, "npm ci"):
                local_checks.npm("gui", ["test:types"])
            run.assert_not_called()

    def test_dirty_or_different_push_source_is_rejected(self):
        with patch.object(subprocess, "check_output", return_value=" M source.py\n"):
            with self.assertRaisesRegex(RuntimeError, "unfinished"):
                local_checks.push_tree()
        with patch.dict(os.environ, {"PRE_COMMIT_TO_REF": "refs/heads/other"}), patch.object(subprocess, "check_output", side_effect=["", "a\n", "b\n"]):
            with self.assertRaisesRegex(RuntimeError, "worktree"):
                local_checks.push_tree()

    def test_workflow_checks_require_the_same_shellcheck_version(self):
        with patch.object(local_checks, "executable", return_value="shellcheck"), patch.object(subprocess, "check_output", return_value="version: 0.10.0\n"), patch.object(local_checks, "run") as run:
            with self.assertRaisesRegex(RuntimeError, "0.11.0"):
                local_checks.workflows()
            run.assert_not_called()

    def test_script_fixtures_do_not_inherit_the_hooks_git_repository(self):
        with patch.dict(os.environ, {"GIT_DIR": "/developer/.git", "GIT_INDEX_FILE": "/developer/index",
                                    "GIT_CONFIG_GLOBAL": "/dev/null"}), patch.object(local_checks, "run") as run:
            local_checks.script_tests()
        for call in run.call_args_list:
            self.assertNotIn("GIT_DIR", call.kwargs["env"])
            self.assertNotIn("GIT_INDEX_FILE", call.kwargs["env"])
            self.assertEqual(call.kwargs["env"]["GIT_CONFIG_GLOBAL"], "/dev/null")


if __name__ == "__main__":
    unittest.main()
