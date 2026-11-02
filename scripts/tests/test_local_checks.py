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


if __name__ == "__main__":
    unittest.main()
