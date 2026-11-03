import os
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import apple_release as apple
import mac_notarization as mac
import release_manifest as manifest

SHA = "a" * 40
SUBMISSION = "11111111-2222-4333-8444-555555555555"
ASSET = "khor-macos-arm64"


class SigningBoundaryTest(unittest.TestCase):
    def test_mac_build_signs_without_submitting_or_waiting_for_notarization(self):
        credentials = {name: "test" for name in [
            "APPLE_CERTIFICATE", "APPLE_CERTIFICATE_PASSWORD", "APPLE_SIGNING_IDENTITY",
            "APPLE_API_KEY", "APPLE_API_ISSUER", "APPLE_API_KEY_CONTENT", "APPLE_API_KEY_PATH",
            "APPLE_ID", "APPLE_PASSWORD", "APPLE_TEAM_ID",
        ]}
        with patch.dict(os.environ, credentials, clear=True), patch.object(apple, "run") as run, \
                patch.object(apple, "verify_app") as verify:
            apple.mac_build("aarch64-apple-darwin", True)
        verify.assert_called_once_with(Path("target/aarch64-apple-darwin/release/bundle/macos/Khor.app"))
        environment = run.call_args_list[0].kwargs["env"]
        self.assertEqual(environment["APPLE_SIGNING_IDENTITY"], "test")
        self.assertFalse("APPLE_API_KEY" in environment)
        self.assertFalse("APPLE_ID" in environment)
        self.assertFalse(any("stapler" in call.args[0] for call in run.call_args_list))


class NotarizationRecoveryTest(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.base = Path(temporary.name)
        self.root, self.state, self.output = [self.base / name for name in ["build", "state", "final"]]
        self.root.mkdir()
        self.addCleanup(patch.stopall)
        patch.dict(os.environ, {"GITHUB_RUN_ID": "42", "GITHUB_REPOSITORY": "owner/repo",
                               "GITHUB_OUTPUT": str(self.base / "outputs")}, clear=True).start()
        patch.object(mac, "source_sha", return_value=SHA).start()
        self.archive = self.root / "payload.zip"
        self.archive.write_bytes(b"signed Mac zip fixture")
        self.archive = self.archive.rename(self.root / f"{ASSET}-42-{manifest.digest(self.archive)}.zip")
        (self.root / "signed.txt").write_text("true\n")
        self.receipt = {"schema": 1, "kind": f"{ASSET}-app-build", "source_sha": SHA, "run_id": "42",
                        "version": "1.2.3", "ui_revision": "the-built-ui", "agent_sha256": "c" * 64,
                        "files": manifest.files(self.root)}
        mac.write_json(self.root / "receipt.json", self.receipt)
        mac.prepare(self.root, ASSET, self.state)
        self.client = Mock()
        self.client.call.side_effect = self.api
        self.calls = []
        self.history, self.status = [], "Accepted"

    def api(self, action, *args):
        self.calls.append((action, args))
        if action == "history":
            return {"history": self.history}
        if action == "submit":
            return {"id": SUBMISSION}
        if action == "info":
            return {"id": SUBMISSION, "name": self.archive.name, "status": self.status}
        if action == "log":
            return {"issues": [{"message": "fixture rejection"}]}
        self.fail(f"Unexpected notary operation {action}")

    def submit(self, allow=True):
        return mac.submit(self.root, ASSET, self.state, allow, self.client)

    def fake_package(self, command, **_):
        if command[:3] == ["ditto", "-x", "-k"]:
            (Path(command[-1]) / "Khor.app").mkdir()
        if command[:2] == ["tar", "-czf"]:
            Path(command[2]).write_bytes(b"stapled final archive")

    def test_submit_persists_id_before_wait_and_reuses_it(self):
        self.assertEqual(self.submit()["request_id"], SUBMISSION)
        self.assertEqual([name for name, _ in self.calls], ["history", "submit"])
        self.assertIn("--no-wait", self.calls[1][1])
        self.calls.clear()
        self.submit(False)
        self.assertEqual(self.calls, [])

    def test_lost_response_recovers_only_exact_archive_from_history(self):
        self.history = [{"name": "other-build.zip", "id": "other"},
                        {"name": self.archive.name, "id": SUBMISSION}]
        self.assertEqual(self.submit(False)["request_id"], SUBMISSION)
        self.assertEqual([name for name, _ in self.calls], ["history"])

    def test_ambiguous_or_unknown_submission_never_reuploads(self):
        with self.assertRaisesRegex(ValueError, "outcome is unknown"):
            self.submit(False)
        self.history = [{"name": self.archive.name, "id": SUBMISSION}] * 2
        with self.assertRaisesRegex(ValueError, "Multiple"):
            self.submit(False)
        self.assertFalse(any(name == "submit" for name, _ in self.calls))

    def test_changed_bytes_or_wrong_source_fail_before_apple_calls(self):
        self.archive.write_bytes(b"tampered")
        with self.assertRaisesRegex(ValueError, "bytes changed"):
            self.submit()
        self.assertEqual(self.calls, [])

    def test_wrong_run_or_request_binding_is_rejected(self):
        with patch.dict(os.environ, {"GITHUB_RUN_ID": "43"}):
            with self.assertRaisesRegex(ValueError, "different platform or run"):
                self.submit()
        record = self.submit()
        record["build_receipt_sha256"] = "0" * 64
        mac.write_json(self.state / "request.json", record)
        with self.assertRaisesRegex(ValueError, "different build"):
            self.submit(False)

    def test_timeout_keeps_request_and_retry_uses_same_id(self):
        record = self.submit()
        self.calls.clear()
        self.status = "In Progress"
        with self.assertRaisesRegex(TimeoutError, "Re-run this failed job"):
            mac.wait_for_acceptance(record, self.state, self.client, 0)
        self.assertEqual(json.loads((self.state / "status.json").read_text())["status"], "In Progress")
        self.status = "Accepted"
        mac.wait_for_acceptance(record, self.state, self.client, 0)
        self.assertEqual(self.calls, [("info", (SUBMISSION,)), ("info", (SUBMISSION,))])

    def test_temporary_query_failure_retries_without_resubmitting(self):
        record = self.submit()
        self.client.call.side_effect = [RuntimeError("temporary network failure"), self.api("info", SUBMISSION)]
        with patch.object(mac.time, "sleep"):
            mac.wait_for_acceptance(record, self.state, self.client, 60)
        self.assertEqual(self.client.call.call_args.args, ("info", SUBMISSION))

    def test_rejected_or_mismatched_request_cannot_be_packaged(self):
        record = self.submit()
        self.status = "Invalid"
        with patch.object(mac, "run") as run:
            with self.assertRaisesRegex(ValueError, "Apple rejected"):
                mac.finalize(self.root, ASSET, self.state, self.output, self.client, 0)
            run.assert_not_called()
        self.assertTrue((self.state / "rejection.json").is_file())
        self.client.call.side_effect = None
        self.client.call.return_value = {"id": SUBMISSION, "name": "wrong.zip", "status": "Accepted"}
        with self.assertRaisesRegex(ValueError, "does not match"):
            mac.wait_for_acceptance(record, self.state, self.client, 0)

    def test_stapling_failure_keeps_original_build_and_request(self):
        record = self.submit()
        def fail_staple(command, **kwargs):
            if command[:3] == ["xcrun", "stapler", "staple"]:
                raise RuntimeError("stapling unavailable")
            self.fake_package(command, **kwargs)
        with patch.object(mac, "run", side_effect=fail_staple):
            with self.assertRaisesRegex(RuntimeError, "stapling unavailable"):
                mac.finalize(self.root, ASSET, self.state, self.output, self.client, 0)
        self.assertFalse((self.output / "receipt.json").exists())
        self.assertEqual(manifest.verify(self.root, SHA), self.receipt)
        self.assertEqual(json.loads((self.state / "request.json").read_text()), record)

    def test_finalized_artifact_preserves_provenance_and_is_reusable(self):
        self.submit()
        with patch.object(mac, "run", side_effect=self.fake_package) as run:
            mac.finalize(self.root, ASSET, self.state, self.output, self.client, 0)
            self.assertIn(["xcrun", "stapler", "validate"], [call.args[0][:3] for call in run.call_args_list])
        receipt = manifest.verify(self.output, SHA)
        self.assertEqual(receipt["kind"], f"{ASSET}-app")
        self.assertEqual(receipt["ui_revision"], self.receipt["ui_revision"])
        self.assertEqual(receipt["agent_sha256"], self.receipt["agent_sha256"])
        self.assertEqual(json.loads((self.output / "notarization.json").read_text())["status"], "Accepted")
        self.client.reset_mock()
        with patch.object(mac, "run") as run:
            mac.finalize(self.root, ASSET, self.state, self.output, self.client, 0)
            run.assert_not_called()
            self.client.call.assert_not_called()

    def test_unsigned_diagnostic_never_contacts_apple_or_claims_acceptance(self):
        (self.root / "signed.txt").write_text("false\n")
        self.receipt["files"] = manifest.files(self.root)
        mac.write_json(self.root / "receipt.json", self.receipt)
        (self.state / "intent.json").unlink()
        mac.prepare(self.root, ASSET, self.state)
        mac.submit(self.root, ASSET, self.state, False, None)
        with patch.object(mac, "run", side_effect=self.fake_package) as run:
            mac.finalize(self.root, ASSET, self.state, self.output, None, 0)
            self.assertFalse(any("stapler" in call.args[0] for call in run.call_args_list))
        self.assertEqual(json.loads((self.output / "notarization.json").read_text())["status"], "Unsigned")

    def test_restore_distinguishes_missing_from_expired_and_network_failure(self):
        with patch.object(mac, "gh", return_value=json.dumps([{"artifacts": []}])):
            mac.restore_artifact("request", self.state)
        self.assertEqual((self.base / "outputs").read_text(), "found=false\n")
        with patch.object(mac, "gh", return_value=json.dumps([{"artifacts": [{"name": "request", "expired": True}]}])):
            with self.assertRaisesRegex(ValueError, "expired"):
                mac.restore_artifact("request", self.state)
        with patch.object(mac, "gh", side_effect=RuntimeError("network unavailable")):
            with self.assertRaisesRegex(RuntimeError, "network unavailable"):
                mac.restore_artifact("request", self.state)

    def test_notary_errors_do_not_include_credentials_or_command(self):
        with patch.dict(os.environ, {"APPLE_API_KEY": "test-key", "APPLE_API_ISSUER": "test-issuer"}):
            client = mac.Notary(Path("test-private-key.p8"))
        with patch.object(mac.subprocess, "run", side_effect=subprocess.TimeoutExpired(["private command"], 60)):
            with self.assertRaisesRegex(RuntimeError, "^notarytool info timed out$"):
                client.call("info", SUBMISSION)


if __name__ == "__main__":
    unittest.main()
