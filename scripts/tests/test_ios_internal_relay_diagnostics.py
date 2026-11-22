import json
from pathlib import Path
import sys
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ios_internal_relay_diagnostics as diagnostics


class InternalRelayDiagnostics(unittest.TestCase):
    def test_only_one_internal_group_and_tester_are_allowed(self):
        client = Mock()
        group = {"id": "group-1", "attributes": {"isInternalGroup": True, "name": "khor-test"}}
        extra_group = {"id": "group-2", "attributes": {"isInternalGroup": True, "name": "another"}}
        tester = {"id": "tester-1"}
        client.get.side_effect = [{"data": [group]}, {"data": [tester]}]
        self.assertEqual(diagnostics.solo_audience(client), ("group-1", "tester-1"))
        client.get.side_effect = [{"data": [group, extra_group]}]
        with self.assertRaisesRegex(ValueError, "exactly the verified"):
            diagnostics.solo_audience(client)
        client.get.side_effect = [{"data": [group]}, {"data": [tester, {"id": "tester-2"}]}]
        with self.assertRaisesRegex(ValueError, "exactly one internal tester"):
            diagnostics.solo_audience(client)

    def test_upload_requests_internal_only_and_checks_apple_audience(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "build.json").write_text(json.dumps({
                "version": "0.2.0", "build": diagnostics.BUILD, "signed": True,
            }))
            with (
                patch.object(diagnostics, "ROOT", root),
                patch.object(diagnostics, "version", return_value="0.2.0"),
                patch.object(diagnostics, "AppleClient") as factory,
                patch.object(diagnostics, "solo_audience", return_value=("group-1", "tester-1")),
                patch.object(diagnostics, "export_ios") as export,
                patch.object(diagnostics, "attach_group") as attach,
            ):
                client = factory.return_value.__enter__.return_value
                client.find_build.return_value = None
                client.processed_build.return_value = {
                    "id": "build-1", "attributes": {"buildAudienceType": "APP_STORE_ELIGIBLE"},
                }
                with self.assertRaisesRegex(ValueError, "did not mark this build Internal Only"):
                    diagnostics.upload()
                export.assert_called_once_with(root, internal_only=True)
                attach.assert_not_called()


if __name__ == "__main__":
    unittest.main()
