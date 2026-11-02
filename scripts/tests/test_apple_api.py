import json
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from apple_api import AppleClient, raw_signature


class AppleBuildTest(unittest.TestCase):
    def setUp(self):
        self.client = AppleClient()
        self.client.app = "app"

    def test_filter_matches_build_and_marketing_version(self):
        response = {"data": [
            {"id": "wanted", "relationships": {"preReleaseVersion": {"data": {"id": "one"}}}},
            {"id": "other", "relationships": {"preReleaseVersion": {"data": {"id": "two"}}}},
        ], "included": [
            {"id": "one", "type": "preReleaseVersions", "attributes": {"version": "1.2.3", "platform": "IOS"}},
            {"id": "two", "type": "preReleaseVersions", "attributes": {"version": "1.2.4", "platform": "IOS"}},
        ]}
        with patch.object(self.client, "get", return_value=response) as get:
            self.assertEqual(self.client.find_build("1.2.3", "1042.1")["id"], "wanted")
            self.assertEqual(get.call_args.kwargs["filter[version]"], "1042.1")
            self.assertIsNone(self.client.find_build("2.0.0", "1042.1"))

    def test_pending_rejected_and_expired_are_not_success(self):
        for state in ["PROCESSING", "FAILED", "INVALID"]:
            build = {"id": "one", "attributes": {"processingState": state}}
            with self.subTest(state=state), patch.object(self.client, "find_build", return_value=build):
                with self.assertRaises((ValueError, TimeoutError)):
                    self.client.processed_build("1.2.3", "1042.1", timeout=0)
        with patch.object(self.client, "find_build", return_value={"attributes": {"processingState": "VALID", "expired": True}}):
            with self.assertRaises(ValueError):
                self.client.processed_build("1.2.3", "1042.1", timeout=0)

    def test_submission_retry_does_not_replace_a_different_build(self):
        response = {"data": [{"attributes": {"appStoreState": "IN_REVIEW"}, "relationships": {"build": {"data": {"id": "one"}}}}]}
        with patch.object(self.client, "get", return_value=response):
            self.assertTrue(self.client.already_submitted("1.2.3", "one"))
            with self.assertRaisesRegex(ValueError, "Another build"):
                self.client.already_submitted("1.2.3", "two")

    def test_es256_signature_conversion_handles_leading_zero(self):
        r, s = b"\0" + b"\x80" * 32, b"\x01" * 31
        sequence = b"\x02" + bytes([len(r)]) + r + b"\x02" + bytes([len(s)]) + s
        self.assertEqual(raw_signature(b"\x30" + bytes([len(sequence)]) + sequence), r[1:] + b"\0" + s)
        for bad in [b"", b"\x30\x01\x00", b"\x30\x06\x02\x01\x80\x02\x01\x01", b"\x30\x06\x02\x01\x01\x02\x00\x01"]:
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                raw_signature(bad)


if __name__ == "__main__":
    unittest.main()
