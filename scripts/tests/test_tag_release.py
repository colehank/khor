import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import apple_release
import distribute_candidate as distribute
import release_candidate
import release_manifest
import release_store as store
from release_policy import compiler_identity, matching_candidate, parse_tag

SHA = "a" * 40
RC = "v1.2.3-rc.1"


class CompilerIdentityTests(unittest.TestCase):
    def test_tagged_build_records_complete_identity_and_retry_keeps_its_build_id(self):
        env = {"GITHUB_REF": "refs/tags/" + RC, "GITHUB_SHA": SHA, "GITHUB_RUN_ID": "42", "GITHUB_RUN_ATTEMPT": "1"}
        first = compiler_identity(SHA, "1.2.3", env)
        self.assertEqual(first, {"KHOR_BUILD_SOURCE": SHA, "KHOR_RELEASE_TAG": RC, "KHOR_RELEASE_BUILD": "42"})
        env["GITHUB_RUN_ATTEMPT"] = "2"
        self.assertEqual(compiler_identity(SHA, "1.2.3", env), first)

    def test_development_checkout_never_borrows_the_private_control_repository_identity(self):
        self.assertEqual(compiler_identity(SHA, "1.2.3", {"GITHUB_REF": "refs/heads/main", "GITHUB_SHA": "b" * 40, "GITHUB_RUN_ID": "42"}),
                         {"KHOR_BUILD_SOURCE": SHA, "KHOR_RELEASE_TAG": "", "KHOR_RELEASE_BUILD": ""})

    def test_source_tag_and_build_disagreement_fail_before_native_compilation(self):
        valid = {"GITHUB_REF": "refs/tags/" + RC, "GITHUB_SHA": SHA, "GITHUB_RUN_ID": "42"}
        for field, value in [("GITHUB_SHA", "b" * 40), ("GITHUB_REF", "refs/tags/v1.2.4-rc.1"), ("GITHUB_RUN_ID", ""), ("GITHUB_RUN_ID", "42\ninjected=1")]:
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                compiler_identity(SHA, "1.2.3", {**valid, field: value})
        with self.assertRaises(ValueError):
            compiler_identity("short", "1.2.3", valid)


def candidate_at(root):
    build = {"version": "1.2.3", "build": "1042.1", "signed": True}
    for platform in release_candidate.PLATFORMS:
        folder = root / platform
        folder.mkdir(parents=True)
        if platform == "ios-archive":
            (folder / "Khor.xcarchive.tar.gz").write_bytes(b"signed ios archive")
            (folder / "build.json").write_text(json.dumps(build))
        elif platform.endswith("-app"):
            (folder / (platform.removesuffix("-app") + ".app.tar.gz")).write_bytes(platform.encode())
            (folder / "signed.txt").write_text("true\n")
            (folder / "notarization.json").write_text(json.dumps({
                "status": "Accepted", "signed": True, "source_sha": SHA, "run_id": "42",
                "asset": platform.removesuffix("-app"), "request_id": "11111111-2222-4333-8444-555555555555",
                "archive_sha256": "e" * 64,
            }))
        else:
            (folder / (platform + ".gz")).write_bytes(platform.encode())
        receipt = {"schema": 1, "kind": platform, "version": "1.2.3", "source_sha": SHA,
                   "ui_revision": "same-ui", "run_id": "42", "attempt": "1", "files": release_manifest.files(folder)}
        (folder / "receipt.json").write_text(json.dumps(receipt))
    (root / "install.sh").write_text("original verified installer")
    with patch.object(release_candidate, "version", return_value="1.2.3"), patch.object(release_candidate, "source_sha", return_value=SHA), patch.dict(os.environ, {"GITHUB_RUN_ID": "42", "GITHUB_REF_TYPE": "tag", "GITHUB_REF": "refs/tags/" + RC}):
        record = release_candidate.assemble(root)
    (root / "candidate-manifest").mkdir()
    (root / "install.sh").rename(root / "candidate-manifest/install.sh")
    (root / "candidate-manifest/manifest.json").write_text(json.dumps(record))
    uploaded = {"version": "1.2.3", "build": "1042.1", "apple_build_id": "apple-42", "processing_state": "VALID",
                "source_sha": SHA, "run_id": "42"}
    return record, uploaded


class FakeReleases:
    def __init__(self):
        self.releases, self.contents, self.commands = {}, {}, []
        self.fail_upload = None

    def info(self, tag):
        return copy.deepcopy(self.releases.get(tag))

    def gh(self, *args, data=None):
        self.commands.append(args)
        if args[:2] == ("api", "repos/owner/repo/commits/v1.2.3") or args[:2] == ("api", "repos/owner/repo/commits/" + RC):
            return json.dumps({"sha": SHA})
        if args[:2] == ("release", "create"):
            tag = args[2]
            self.releases[tag] = {"id": tag, "tag_name": tag, "draft": True, "prerelease": "--prerelease" in args, "immutable": False, "assets": []}
        elif args[:2] == ("release", "upload"):
            tag, path = args[2], Path(args[3])
            if path.name == self.fail_upload:
                self.fail_upload = None
                raise RuntimeError("temporary upload outage")
            contents = path.read_bytes()
            self.contents[tag, path.name] = contents
            self.releases[tag]["assets"].append({"name": path.name, "size": len(contents), "digest": "sha256:" + hashlib.sha256(contents).hexdigest()})
        else:
            raise AssertionError(args)
        return ""

    def api(self, path, method="GET", data=None):
        if path.endswith("/immutable-releases"):
            return {"enabled": True}
        assert method == "PATCH"
        release = self.releases[path.rsplit("/", 1)[1]]
        release.update(data, immutable=True)
        return copy.deepcopy(release)

    def download(self, tag, name, target):
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(self.contents[tag, name])


class TagPolicyTest(unittest.TestCase):
    def test_release_stages_keep_the_same_numeric_application_version(self):
        self.assertTrue(parse_tag(RC).prerelease)
        self.assertFalse(parse_tag("v1.2.3").prerelease)
        self.assertEqual(parse_tag(RC).version, "1.2.3")
        for bad in ["main", "v01.2.3", "v1.2", "v1.2.3-beta.1", "v1.2.3-rc.0", "v1.2.3-rc.01", "v1.2.3+meta", "v1.2.3\n"]:
            with self.subTest(tag=bad), self.assertRaises(ValueError):
                parse_tag(bad)

    def test_promotion_never_guesses_between_candidates(self):
        self.assertEqual(matching_candidate("v1.2.3", [RC]), RC)
        for candidates in [[], [RC, "v1.2.3-rc.2"], ["v1.2.4-rc.1"]]:
            with self.assertRaises(ValueError):
                matching_candidate("v1.2.3", candidates)
        self.assertEqual(matching_candidate("v1.2.3", [RC, "v1.2.3-rc.2"], RC), RC)
        with self.assertRaises(ValueError):
            matching_candidate("v1.2.3", [RC], "v1.2.3-rc.2")

    def test_manifest_version_accepts_rc_and_rejects_wrong_base(self):
        cargo = '[workspace.package]\nversion="1.2.3"\n'
        with patch.object(Path, "read_text", side_effect=[cargo, '{"version":"1.2.3"}']), patch.dict(os.environ, {"GITHUB_REF": "refs/tags/" + RC}):
            self.assertEqual(release_manifest.version(), "1.2.3")
        with patch.object(Path, "read_text", side_effect=[cargo, '{"version":"1.2.3"}']), patch.dict(os.environ, {"GITHUB_REF": "refs/tags/v1.2.4-rc.1"}):
            with self.assertRaises(ValueError):
                release_manifest.version()


class ReleaseStorageTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.root = self.base / "candidate"
        self.record, self.uploaded = candidate_at(self.root)
        self.remote = FakeReleases()
        for name, value in [("gh", self.remote.gh), ("api", self.remote.api), ("release_info", self.remote.info), ("download_asset", self.remote.download)]:
            self.enterContext(patch.object(store, name, value))
        self.enterContext(patch.dict(os.environ, {"GITHUB_REPOSITORY": "owner/repo"}))

    def test_candidate_survives_actions_expiry_and_promotes_identical_bytes(self):
        store.publish(RC, self.root, self.uploaded)
        original = dict(self.remote.contents)
        restored = self.base / "restored"
        record, descriptor = store.fetch_release(RC, restored)
        self.assertEqual(record, self.record)
        self.assertEqual(descriptor["testflight"], self.uploaded)
        store.publish("v1.2.3", restored, descriptor["testflight"])
        for (tag, name), contents in original.items():
            if name != "release.json":
                self.assertEqual(self.remote.contents["v1.2.3", name], contents)
        self.assertTrue(self.remote.releases[RC]["prerelease"])
        self.assertFalse(self.remote.releases["v1.2.3"]["prerelease"])
        self.assertTrue(self.remote.releases["v1.2.3"]["immutable"])
        self.assertFalse(any("build" in command for command in self.remote.commands))

    def test_interrupted_upload_resumes_without_overwriting_existing_assets(self):
        self.remote.fail_upload = "manifest.json"
        with self.assertRaisesRegex(RuntimeError, "outage"):
            store.publish(RC, self.root, self.uploaded)
        self.assertTrue(self.remote.releases[RC]["draft"])
        previous = dict(self.remote.contents)
        store.publish(RC, self.root, self.uploaded)
        self.assertFalse(self.remote.releases[RC]["draft"])
        self.assertTrue(all(self.remote.contents[key] == value for key, value in previous.items()))
        count = len(self.remote.commands)
        store.publish(RC, self.root, self.uploaded)
        self.assertFalse(any(c[:2] == ("release", "upload") for c in self.remote.commands[count:]))

    def test_existing_different_asset_is_never_overwritten(self):
        store.publish(RC, self.root, self.uploaded)
        self.remote.releases[RC]["assets"][0]["digest"] = "sha256:" + "0" * 64
        with self.assertRaisesRegex(ValueError, "differs"):
            store.publish(RC, self.root, self.uploaded)

    def test_metadata_fetch_verifies_digests_without_downloading_binaries(self):
        store.publish(RC, self.root, self.uploaded)
        target = self.base / "metadata"
        store.fetch_release(RC, target, full=False)
        self.assertFalse(list(target.rglob("*.gz")))
        self.assertEqual(len(list(target.rglob("notarization.json"))), 2)
        self.remote.releases[RC]["immutable"] = False
        with self.assertRaisesRegex(ValueError, "immutable"):
            store.fetch_release(RC, self.base / "bad", full=False)

    def test_signed_but_unnotarized_or_mismatched_mac_cannot_distribute(self):
        path = self.root / "khor-macos-arm64-app/notarization.json"
        original = json.loads(path.read_text())
        for field, value in [("status", "In Progress"), ("status", "Invalid"), ("signed", False),
                             ("run_id", "43"), ("source_sha", "b" * 40), ("asset", "khor-macos-x86_64"),
                             ("request_id", "-" * 36), ("request_id", ""), ("archive_sha256", "")]:
            with self.subTest(field=field, value=value):
                path.write_text(json.dumps({**original, field: value}))
                with self.assertRaisesRegex(ValueError, "accepted notarization"):
                    store.signed_build(self.root, self.record)

    def test_metadata_fetch_rejects_changed_notarization_proof(self):
        store.publish(RC, self.root, self.uploaded)
        name = "khor-macos-arm64-app-notarization.json"
        self.remote.contents[RC, name] = b'{"status":"In Progress"}'
        with self.assertRaisesRegex(ValueError, "Downloaded release asset changed"):
            store.fetch_release(RC, self.base / "metadata", full=False)

    def test_tampering_unsigned_and_mixed_provenance_fail_closed(self):
        for field, value in [("source_sha", "b" * 40), ("run_id", "43"), ("version", "1.2.4")]:
            changed = copy.deepcopy(self.record)
            changed["artifacts"]["ios-archive"][field] = value
            with self.assertRaisesRegex(ValueError, "Mixed"):
                store.validate_manifest(changed)
        changed = copy.deepcopy(self.record)
        changed["artifacts"]["ios-archive"]["files"]["../outside"] = {"sha256": "0" * 64, "bytes": 1}
        with self.assertRaisesRegex(ValueError, "safe filenames"):
            store.asset_map(changed)
        (self.root / "khor-macos-arm64-app/signed.txt").write_text("false\n")
        with self.assertRaisesRegex(ValueError, "signed"):
            store.signed_build(self.root, self.record)
        with self.assertRaisesRegex(ValueError, "bytes changed"):
            store.verify_candidate(self.root)

    def test_wrong_or_unprocessed_apple_build_cannot_publish(self):
        for field, value in [("build", "1042.2"), ("run_id", "43"), ("processing_state", "PROCESSING")]:
            wrong = {**self.uploaded, field: value}
            with self.assertRaises(ValueError):
                store.publish(RC, self.root, wrong)
        self.assertEqual(self.remote.releases, {})


class ReleaseLookupTest(unittest.TestCase):
    def test_unpublished_draft_is_found_on_a_later_release_page(self):
        draft = {"id": 42, "tag_name": RC, "draft": True, "assets": []}
        missing = subprocess.CompletedProcess([], 1, stdout="", stderr="HTTP 404: Not Found")
        pages = [[{"id": 41, "tag_name": "v1.2.2"}], [draft]]
        with patch.dict(os.environ, {"GITHUB_REPOSITORY": "owner/repo"}), \
                patch.object(store.subprocess, "run", return_value=missing), \
                patch.object(store, "gh", return_value=json.dumps(pages)) as gh, \
                patch.object(store, "api", return_value=draft) as api:
            self.assertEqual(store.release_info(RC), draft)
            gh.assert_called_once_with("api", "repos/owner/repo/releases?per_page=100", "--paginate", "--slurp")
            api.assert_called_once_with("repos/owner/repo/releases/42")

    def test_lookup_errors_are_not_treated_as_missing_releases(self):
        denied = subprocess.CompletedProcess([], 1, stdout="", stderr="HTTP 403: Forbidden")
        with patch.dict(os.environ, {"GITHUB_REPOSITORY": "owner/repo"}), \
                patch.object(store.subprocess, "run", return_value=denied), patch.object(store, "gh") as gh:
            with self.assertRaisesRegex(RuntimeError, "403"):
                store.release_info(RC)
            gh.assert_not_called()


class DistributionGateTest(unittest.TestCase):
    def test_failed_quality_cannot_download_or_upload_candidate(self):
        result = {"path": ".github/workflows/release.yml", "event": "push", "head_sha": SHA}
        with patch.dict(os.environ, {"RELEASE_TAG": RC, "CANDIDATE_RUN_ID": "42", "GITHUB_REPOSITORY": "owner/repo"}), patch.object(distribute, "tag_source", return_value=SHA), patch.object(distribute, "api", return_value=result), patch.object(distribute, "gh", return_value='[{"jobs":[{"name":"candidate","conclusion":"skipped"}]}]') as gh:
            with self.assertRaisesRegex(ValueError, "required checks"):
                distribute.fetch()
            self.assertEqual(gh.call_count, 1)

    def test_stable_tag_never_rebuilds(self):
        with patch.dict(os.environ, {"GITHUB_REF": "refs/tags/v1.2.3"}), self.assertRaisesRegex(ValueError, "never build"):
            distribute.check_build()

    def test_rc_never_submits_app_store_review(self):
        with patch.dict(os.environ, {"RELEASE_TAG": RC}), patch.object(distribute, "run") as run:
            with self.assertRaisesRegex(ValueError, "Only stable"):
                distribute.appstore_review()
            run.assert_not_called()

    def test_upload_retry_reuses_processed_apple_build(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "build.json").write_text(json.dumps({"version": "1.2.3", "build": "1042.1", "signed": True}))
            with patch.object(apple_release, "AppleClient") as factory, patch.object(apple_release, "export_ios") as export:
                client = factory.return_value.__enter__.return_value
                client.find_build.return_value = {"id": "existing"}
                client.processed_build.return_value = {"id": "existing", "attributes": {"processingState": "VALID"}}
                result = apple_release.upload_ios(root)
                export.assert_not_called()
                self.assertEqual(result["apple_build_id"], "existing")
                client.find_build.return_value = None
                apple_release.upload_ios(root)
                export.assert_called_once_with(root)


if __name__ == "__main__":
    unittest.main()
