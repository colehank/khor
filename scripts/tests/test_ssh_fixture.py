"""Fixture controls only; real signed SSH acceptance runs on the build workers."""
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("ssh_fixture", Path(__file__).resolve().parents[1] / "check-ssh-install.py")
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)


class SignedSshFixture(unittest.TestCase):
    def test_a_failed_native_build_restores_the_original_public_root(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            public = root / "updater.pub"
            public.write_bytes(b"original public root")

            def generate(*args, **kwargs):
                (root / "fixture.key.pub").write_bytes(b"temporary public root")

            with patch.object(fixture.subprocess, "run", side_effect=generate):
                with self.assertRaisesRegex(RuntimeError, "native build failed"):
                    with fixture.temporary_trust(public, root, Path("unused-signer"), {}) as key:
                        self.assertEqual(public.read_bytes(), b"temporary public root")
                        self.assertEqual(key, root / "fixture.key")
                        raise RuntimeError("native build failed")
            self.assertEqual(public.read_bytes(), b"original public root")

    def test_an_executable_from_another_source_tree_is_not_admitted(self):
        with tempfile.TemporaryDirectory() as temporary:
            program = Path(temporary) / "khor"
            program.write_bytes(b"controlled artifact; never executed")
            row = dict(reason="compiler-artifact", target=dict(name="khor", kind=["bin"]),
                       profile=dict(test=False), executable=str(program), manifest_path="/unrelated/Cargo.toml")
            with self.assertRaisesRegex(RuntimeError, "expected one actual CLI"):
                fixture.executable([row], "khor", "bin")


if __name__ == "__main__":
    unittest.main()
