import copy
import gzip
import io
import json
import os
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import release_manifest
import release_store as store
import release_updates as updates
from test_tag_release import candidate_at, FakeReleases, RC, SHA


def update_candidate(root):
    record, uploaded = candidate_at(root)
    for name, (component, system, arch) in updates.TARGETS.items():
        folder = root / name
        filename = name.removesuffix("-app") + (".app.tar.gz" if component == "desktop-app" else ".gz")
        if component == "cli":
            with gzip.open(folder / filename, "wb") as stream:
                stream.write(b"fixture CLI, never executed")
        else:
            with tarfile.open(folder / filename, "w:gz") as archive:
                member = tarfile.TarInfo("Khor.app/Contents/MacOS/khor-gui")
                member.size = len(b"fixture App, never executed")
                archive.addfile(member, io.BytesIO(b"fixture App, never executed"))
        receipt = record["artifacts"][name]
        receipt["program"] = dict(schema=1, platform=dict(os=system, arch=arch), component=component,
                                  build=dict(version="1.2.3", source=SHA, release=dict(tag=RC, source=SHA, build=42)),
                                  contract=dict(terminal=5, gui="fixture", node="fixture", store=1, journal=1, agent=1))
        receipt["files"] = release_manifest.files(folder)
        (folder / "receipt.json").write_text(json.dumps(receipt))
    (root / "candidate-manifest/manifest.json").write_text(json.dumps(record))
    return record, uploaded


class UpdatePublicationTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.root = self.base / "candidate"
        self.record, self.uploaded = update_candidate(self.root)
        self.output = self.base / "signed-updates"
        self.enterContext(patch.dict(os.environ, {"GITHUB_REPOSITORY": "owner/repo"}))
        self.sign = self.enterContext(patch.object(updates, "sign", side_effect=self.fixture_sign))
        self.verify = self.enterContext(patch.object(updates, "verify_native"))

    def fixture_sign(self, path, signer, version):
        signature = "fixture-signature-" + release_manifest.digest(path)
        Path(str(path) + ".sig").write_text(signature)
        return signature

    def create(self, tag=RC, output=None):
        return updates.create(self.root, output or self.output, tag, Path("unused-signer"), Path("unused-verifier"))

    def test_original_archives_are_bound_to_both_native_and_tauri_metadata(self):
        receipt = self.create()
        self.assertEqual(self.sign.call_count, 7)
        self.assertEqual(self.verify.call_count, 1)
        self.assertEqual(receipt["identity"], dict(tag=RC, source=SHA, build=42))
        native = json.loads((self.output / updates.MANIFEST).read_text())
        tauri = json.loads((self.output / updates.TAURI).read_text())
        self.assertEqual(tauri["version"], RC[1:])
        self.assertEqual(set(tauri["platforms"]), {"darwin-aarch64", "darwin-x86_64"})
        self.assertEqual(len(native["artifacts"]), 6)
        for asset in native["artifacts"]:
            self.assertGreater(asset["unpacked_bytes"], 0)
            self.assertIn("/" + RC + "/", asset["url"])
        self.create()
        self.assertEqual(self.sign.call_count, 7, "retry must reuse the original signatures")

    def test_stable_promotion_reuses_every_original_archive_and_its_signature(self):
        self.create()
        self.output.rename(self.root / "update-publication")
        old = json.loads((self.root / "update-publication" / updates.MANIFEST).read_text())
        self.sign.reset_mock()
        self.create("v1.2.3")
        self.assertEqual(self.sign.call_count, 1, "only publication metadata is signed again")
        new = json.loads((self.output / updates.MANIFEST).read_text())
        self.assertEqual(old["artifacts"], new["artifacts"])
        self.assertEqual(new["built_from"], RC)
        self.assertEqual(new["identity"]["tag"], "v1.2.3")
        self.assertEqual(json.loads((self.output / updates.TAURI).read_text())["version"], "1.2.3")

    def test_wrong_binary_source_build_platform_or_contract_never_reaches_the_signer(self):
        for field, value in [("build", {}), ("platform", {}), ("component", "desktop-app"), ("contract", None)]:
            with self.subTest(field=field):
                record = copy.deepcopy(self.record)
                record["artifacts"]["khor-linux-x86_64"]["program"][field] = value
                path = self.root / "khor-linux-x86_64/receipt.json"
                path.write_text(json.dumps(record["artifacts"]["khor-linux-x86_64"]))
                (self.root / "candidate-manifest/manifest.json").write_text(json.dumps(record))
                with self.assertRaisesRegex(ValueError, "executable identity"):
                    self.create()
                self.assertFalse(self.output.exists())
                self.sign.assert_not_called()
        path.write_text(json.dumps(self.record["artifacts"]["khor-linux-x86_64"]))
        (self.root / "candidate-manifest/manifest.json").write_text(json.dumps(self.record))

    def test_changed_original_or_tauri_metadata_is_rejected_and_never_repaired_silently(self):
        self.create()
        (self.output / updates.TAURI).write_text('{"version":"9.0.0"}')
        receipt = json.loads((self.output / "receipt.json").read_text())
        receipt["files"] = release_manifest.files(self.output)
        (self.output / "receipt.json").write_text(json.dumps(receipt))
        with self.assertRaisesRegex(ValueError, "Tauri metadata"):
            self.create()
        original = self.root / "khor-linux-x86_64/khor-linux-x86_64.gz"
        original.write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "bytes changed"):
            self.create()
        self.assertEqual(self.sign.call_count, 7)

    def test_native_signature_rejection_never_publishes_a_ready_directory(self):
        self.verify.side_effect = ValueError("Native trust root rejected the update publication")
        with self.assertRaisesRegex(ValueError, "Native trust root"):
            self.create()
        self.assertFalse(self.output.exists())
        self.assertFalse(list(self.base.glob("khor-sign-publication-*")))

    def test_published_signatures_survive_actions_expiry_and_stable_promotion(self):
        self.create()
        remote = FakeReleases()
        for name, value in [("gh", remote.gh), ("api", remote.api), ("release_info", remote.info),
                            ("download_asset", remote.download)]:
            self.enterContext(patch.object(store, name, value))
        store.publish(RC, self.root, self.uploaded, self.output)
        metadata = self.base / "metadata"
        _, descriptor = store.fetch_release(RC, metadata, full=False)
        self.assertEqual(descriptor["updates"]["identity"]["tag"], RC)
        self.assertFalse(list(metadata.glob("khor-*/*.gz")))
        restored = self.base / "restored"
        store.fetch_release(RC, restored)
        previous = self.root
        self.root = restored
        promoted = self.base / "stable-updates"
        self.create("v1.2.3", promoted)
        store.publish("v1.2.3", restored, self.uploaded, promoted)
        for name, filename, _, _ in updates.products(self.record):
            self.assertEqual(remote.contents[RC, filename], remote.contents["v1.2.3", filename])
            self.assertEqual((previous / name / filename).read_bytes(), remote.contents["v1.2.3", filename])
        self.assertNotEqual(remote.contents[RC, updates.MANIFEST], remote.contents["v1.2.3", updates.MANIFEST])


class OfficialSignerTests(unittest.TestCase):
    def test_signer_failure_does_not_expose_credentials_in_argv_or_error(self):
        with patch.object(updates.subprocess, "run") as run:
            run.return_value.returncode = 1
            run.return_value.stderr = b"private key and passphrase"
            with self.assertRaisesRegex(RuntimeError, "not logged") as caught:
                updates.sign(Path("archive.gz"), Path("tauri"), "1.2.3")
            self.assertNotIn("private key", str(caught.exception))
            self.assertEqual(run.call_args.args[0], ["tauri", "signer", "sign", "--app-version", "1.2.3", "archive.gz"])



if __name__ == "__main__":
    unittest.main()
