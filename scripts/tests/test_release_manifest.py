import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import release_manifest as manifest
import release_candidate as candidate
import apple_release as apple


class ArtifactReceiptTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.addCleanup(patch.stopall)
        patch.object(manifest, "source_sha", return_value="a" * 40).start()
        patch.object(candidate, "source_sha", return_value="a" * 40).start()
        patch.object(manifest, "version", return_value="1.2.3").start()
        self.frontend = self.root / "frontend"
        (self.frontend / "dist/assets").mkdir(parents=True)
        (self.frontend / "dist/index.html").write_text('<script src="/assets/index-one.js"></script>')
        (self.frontend / "dist/assets/index-one.js").write_text("a frontend")
        manifest.create(self.frontend, "frontend")

    def test_rejects_mutation_extra_files_and_wrong_source(self):
        manifest.verify(self.frontend, "a" * 40)
        with self.assertRaisesRegex(ValueError, "different source"):
            manifest.verify(self.frontend, "b" * 40)
        entry = self.frontend / "dist/assets/index-one.js"
        entry.write_text("different frontend")
        with self.assertRaisesRegex(ValueError, "bytes changed"):
            manifest.verify(self.frontend)
        entry.write_text("a frontend")
        (self.frontend / "unexpected.js").write_text("extra")
        with self.assertRaisesRegex(ValueError, "bytes changed"):
            manifest.verify(self.frontend)

    def test_product_must_embed_the_verified_frontend_and_pi(self):
        product = self.root / "product"
        product.mkdir()
        binary = product / "khor"
        agent = self.root / "agent"
        agent.write_bytes(b"actual target runtime")
        binary.write_bytes(b"khor-ui-assets:/assets/index-other.js")
        with self.assertRaisesRegex(ValueError, "does not embed this UI"):
            manifest.create(product, "linux", self.frontend, binary, agent)
        binary.write_bytes(b"khor-ui-assets:/assets/index-one.js")
        with self.assertRaisesRegex(ValueError, "does not embed this Pi"):
            manifest.create(product, "linux", self.frontend, binary, agent)
        binary.write_bytes(binary.read_bytes() + agent.read_bytes())
        manifest.create(product, "linux", self.frontend, binary, agent)
        manifest.verify(product, "a" * 40)

    def test_refuses_symlinks_and_missing_frontend_entry(self):
        (self.frontend / "link").symlink_to(self.root / "private-key")
        with self.assertRaisesRegex(ValueError, "regular files"):
            manifest.files(self.frontend)
        (self.frontend / "link").unlink()
        (self.frontend / "dist/assets/index-one.js").unlink()
        with self.assertRaisesRegex(ValueError, "no built entrypoint"):
            manifest.create(self.frontend, "frontend")

    def test_incomplete_candidate_does_not_become_publishable(self):
        with self.assertRaisesRegex(ValueError, "platform set"):
            candidate.assemble(self.root)

    def test_ios_build_numbers_are_distinct_bounded_and_above_existing_builds(self):
        with patch.dict(apple.os.environ, {"GITHUB_RUN_NUMBER": "20", "GITHUB_RUN_ATTEMPT": "1"}):
            self.assertEqual(apple.build_number(), "1020.1")
        with patch.dict(apple.os.environ, {"GITHUB_RUN_NUMBER": "20", "GITHUB_RUN_ATTEMPT": "2"}):
            self.assertEqual(apple.build_number(), "1020.2")
        with patch.dict(apple.os.environ, {"GITHUB_RUN_NUMBER": "9000", "GITHUB_RUN_ATTEMPT": "1"}):
            with self.assertRaisesRegex(ValueError, "limits"):
                apple.build_number()

    def test_unsigned_ios_is_not_uploaded(self):
        (self.root / "build.json").write_text(json.dumps({"signed": False}))
        with self.assertRaisesRegex(ValueError, "unsigned"):
            apple.upload_ios(self.root)


if __name__ == "__main__":
    unittest.main()
