import json
from pathlib import Path
import plistlib
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from mac_icon import catalog_name, verify_app


class MacIconTest(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.app = Path(self.temporary.name) / "Fixture.app"
        self.contents = self.app / "Contents"
        self.resources = self.contents / "Resources"
        self.resources.mkdir(parents=True)
        (self.resources / "Assets.car").touch()
        (self.resources / "legacy.icns").touch()
        self.info = {"CFBundleIconName": "Icon", "CFBundleIconFile": "legacy.icns"}

    def write_info(self):
        (self.contents / "Info.plist").write_bytes(plistlib.dumps(self.info))

    def test_plain_or_ambiguous_catalog_cannot_pass_as_native_layered_icon(self):
        for entries in [[], [{"AssetType": "Icon Image", "Name": "Icon"}],
                        [{"AssetType": "IconImageStack"}, {"AssetType": "Icon Image", "Name": "a"},
                         {"AssetType": "Icon Image", "Name": "b"}]]:
            with self.subTest(entries=entries), patch("mac_icon.subprocess.check_output", return_value=json.dumps(entries)):
                with self.assertRaisesRegex(ValueError, "layered"):
                    catalog_name(self.resources / "Assets.car")

    def test_package_requires_catalog_selection_and_legacy_fallback(self):
        self.write_info()
        with patch("mac_icon.subprocess.check_output", return_value=json.dumps([
                {"AssetType": "IconImageStack", "Name": "Icon"}, {"AssetType": "Icon Image", "Name": "Icon"}])):
            verify_app(self.app)
            self.info.pop("CFBundleIconName")
            self.write_info()
            with self.assertRaisesRegex(ValueError, "select"):
                verify_app(self.app)
            self.info["CFBundleIconName"] = "Icon"
            self.write_info()
            (self.resources / "legacy.icns").unlink()
            with self.assertRaisesRegex(ValueError, "ICNS"):
                verify_app(self.app)
            (self.resources / "Assets.car").unlink()
            with self.assertRaisesRegex(ValueError, "Missing native"):
                verify_app(self.app)


if __name__ == "__main__":
    unittest.main()
