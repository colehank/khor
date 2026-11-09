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

    def test_lab_domain_cache_is_covered_by_the_frontend_receipt(self):
        cache = self.frontend / "lab-world.generated.json"
        cache.write_text('{"schema":1}')
        receipt = manifest.create(self.frontend, "frontend")
        self.assertIn(cache.name, receipt["files"])
        manifest.verify(self.frontend, "a" * 40)
        cache.write_text('{"schema":2}')
        with self.assertRaisesRegex(ValueError, "bytes changed"):
            manifest.verify(self.frontend, "a" * 40)

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

    def test_shared_runtime_rejects_another_target_run_or_changed_bytes(self):
        runtime = self.root / "runtime"
        runtime.mkdir()
        agent = runtime / "khor-agent"
        agent.write_bytes(b"shared target runtime")
        kind = "agent-runtime-aarch64-apple-darwin"
        with patch.dict(manifest.os.environ, {"GITHUB_RUN_ID": "123", "GITHUB_RUN_ATTEMPT": "1"}):
            receipt = manifest.create(runtime, kind, self.frontend, agent=agent)
        self.assertEqual(receipt["agent_sha256"], manifest.digest(agent))
        # Failed downstream jobs may reuse this artifact on a later attempt of the same run.
        with patch.dict(manifest.os.environ, {"GITHUB_RUN_ATTEMPT": "2"}):
            self.assertEqual(manifest.verify(runtime, "a" * 40, kind=kind, run_id="123"), receipt)
        with self.assertRaisesRegex(ValueError, "different source"):
            manifest.verify(runtime, "b" * 40, kind=kind, run_id="123")
        with self.assertRaisesRegex(ValueError, "different kind"):
            manifest.verify(runtime, "a" * 40, kind="agent-runtime-x86_64-apple-darwin", run_id="123")
        with self.assertRaisesRegex(ValueError, "different workflow run"):
            manifest.verify(runtime, "a" * 40, kind=kind, run_id="456")
        agent.write_bytes(b"another runtime")
        with self.assertRaisesRegex(ValueError, "bytes changed"):
            manifest.verify(runtime, "a" * 40, kind=kind, run_id="123")

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

    def test_report_must_belong_to_the_exact_compiled_source_tag_and_build(self):
        product = self.root / "product"
        product.mkdir()
        (product / "archive.gz").write_bytes(b"immutable archive")
        release = dict(tag="v1.2.3-rc.1", source="a" * 40, build=42)
        info = dict(schema=1, build=dict(version="1.2.3", source="a" * 40, release=release))
        environment = dict(GITHUB_REF="refs/tags/v1.2.3-rc.1", GITHUB_SHA="a" * 40, GITHUB_RUN_ID="42")
        with patch.dict(manifest.os.environ, environment):
            self.assertEqual(manifest.create(product, "product", self.frontend, program_info=info)["program"], info)
            for changed in [dict(info, build={}), dict(info, schema=2),
                            dict(info, build=dict(info["build"], release=dict(release, build=43)))]:
                with self.assertRaisesRegex(ValueError, "release identity"):
                    manifest.create(product, "product", self.frontend, program_info=changed)

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

    def test_unsigned_archive_does_not_require_or_mutate_a_developer_team(self):
        with patch.dict(apple.os.environ, {"APPLE_DEVELOPMENT_TEAM": ""}), patch.object(
            apple, "build_number", return_value="1004.1"
        ), patch.object(apple, "run") as run:
            apple.archive_ios(False)
            command = run.call_args.args[0]
            self.assertIn("--no-sign", command)
            self.assertIn("--archive-only", command)
            self.assertEqual(run.call_args.kwargs["env"]["APPLE_DEVELOPMENT_TEAM"], "0000000000")
            self.assertEqual(apple.os.environ["APPLE_DEVELOPMENT_TEAM"], "")


if __name__ == "__main__":
    unittest.main()
