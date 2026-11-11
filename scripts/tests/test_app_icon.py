"""Contracts at the asset generator and the studio's HTTP boundary."""
import contextlib
from http.server import ThreadingHTTPServer
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import unittest
from urllib.error import HTTPError
from urllib.request import urlopen


MODULE = Path(__file__).resolve().parents[1] / "app_icon.py"
spec = importlib.util.spec_from_file_location("app_icon", MODULE)
app_icon = importlib.util.module_from_spec(spec)
spec.loader.exec_module(app_icon)


class IconInputTests(unittest.TestCase):
    def test_single_size_catalog_resolves_both_appearances_without_scale(self):
        self.assertEqual(set(app_icon.ios_assets().values()), {(1024, "light"), (1024, "dark")})

    def test_policy_rejects_shared_assets_and_paths_outside_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / app_icon.APPEARANCES
            path.parent.mkdir(parents=True)
            original = app_icon.appearance_policy()
            for field, value in [("asset", original["appearances"]["light"]["asset"]),
                                 ("artwork", "../outside.svg")]:
                policy = json.loads(json.dumps(original))
                policy["appearances"]["dark"][field] = value
                path.write_text(json.dumps(policy))
                with self.assertRaises(ValueError):
                    app_icon.appearance_policy(root)

    def test_ios_inventory_rejects_escaping_filename_and_fractional_pixels(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            catalog = root / app_icon.IOS / "Contents.json"
            catalog.parent.mkdir(parents=True)
            for entry in [
                {"filename": "../escape.png", "size": "20x20", "scale": "2x"},
                {"filename": "safe.png", "size": "20.1x20.1", "scale": "2x"},
            ]:
                catalog.write_text(json.dumps({"images": [entry]}))
                with self.assertRaises(ValueError):
                    app_icon.ios_sizes(root)

    @unittest.skipUnless(shutil.which("node"), "Node is required for the shared renderer")
    def test_cli_rejects_invalid_preset_before_producing_svg(self):
        result = subprocess.run(["node", str(app_icon.ROOT / app_icon.LAB / "render.ts")],
                                input=b'{"format":"unknown"}', capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, b"")


class AppearanceAssetTests(unittest.TestCase):
    """Exercise drift that a regenerated hash alone would otherwise conceal."""
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        paths = {*app_icon.INPUTS, *map(Path, app_icon.asset_spec()), app_icon.MANIFEST}
        for path in paths:
            target = self.root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(app_icon.ROOT / path, target)

    def resign(self, relative, section):
        manifest_path = self.root / app_icon.MANIFEST
        manifest = json.loads(manifest_path.read_text())
        manifest[section][str(relative)] = app_icon.digest((self.root / relative).read_bytes())
        manifest_path.write_text(json.dumps(manifest))

    def test_dark_ios_cannot_drift_from_mac_even_with_updated_hash(self):
        Image, _ = app_icon.pillow()
        path = app_icon.IOS / "AppIcon-dark.png"
        with Image.open(self.root / path) as source:
            image = source.copy()
        image.putpixel((0, 0), (255, 0, 0))
        image.save(self.root / path)
        self.resign(path, "outputs")
        with self.assertRaisesRegex(ValueError, "dark artwork must be identical"):
            app_icon.check_assets(self.root)

    def test_native_document_cannot_drop_dark_specialization(self):
        path = app_icon.ICONS / "Icon.icon/icon.json"
        document = json.loads((self.root / path).read_text())
        layer = document["groups"][0]["layers"][0]
        layer["image-name"] = layer.pop("image-name-specializations")[0]["value"]
        (self.root / path).write_text(json.dumps(document))
        self.resign(path, "outputs")
        with self.assertRaisesRegex(ValueError, "Native appearance mapping"):
            app_icon.check_assets(self.root)

    def test_light_final_cannot_change_by_only_regenerating_manifest(self):
        path = app_icon.ICONS / "src/app-icon-light.svg"
        with (self.root / path).open("ab") as output:
            output.write(b"\n")
        self.resign(path, "inputs")
        with self.assertRaisesRegex(ValueError, "approved final"):
            app_icon.check_assets(self.root)


class StudioServerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        class QuietHandler(app_icon.StudioHandler):
            def log_message(self, *args):
                pass
        cls.server = ThreadingHTTPServer(("127.0.0.1", 0), QuietHandler)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        cls.url = f"http://127.0.0.1:{cls.server.server_port}"

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(timeout=2)

    def test_serves_current_preset_and_module_without_build(self):
        for path, mime in [("/", "text/html"), ("/renderer.ts", "text/javascript"),
                           ("/current-preset.json", "application/json")]:
            with contextlib.closing(urlopen(self.url + path, timeout=2)) as response:
                self.assertIn(mime, response.headers["Content-Type"])
                self.assertEqual(response.headers["Cache-Control"], "no-store")
                self.assertTrue(response.read())

    def test_browser_modules_are_executable_javascript_from_the_shared_ts_source(self):
        for module in ["app", "renderer"]:
            with contextlib.closing(urlopen(self.url + f"/{module}.ts", timeout=3)) as response:
                javascript = response.read()
            result = subprocess.run(["node", "--input-type=module", "--check"],
                                    input=javascript, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            with contextlib.closing(urlopen(self.url + f"/{module}.mjs", timeout=3)) as response:
                self.assertEqual(response.read(), javascript)
        self.assertIn(b"./renderer.ts", app_icon.browser_javascript((app_icon.ROOT / app_icon.LAB / "app.ts").read_bytes()))

    def test_never_exposes_repository_files_or_directory_listings(self):
        for path in ["/.git/config", "/../../.git/config", "/%2e%2e/.git/config",
                     "/README.md", "/scripts/", "/reference-07.png/../app.mjs"]:
            with self.assertRaises(HTTPError) as error:
                urlopen(self.url + path, timeout=2)
            self.assertEqual(error.exception.code, 404)
            error.exception.close()

    def test_product_tokens_are_usable_without_tailwind_or_package_requests(self):
        with contextlib.closing(urlopen(self.url + "/tokens.css", timeout=2)) as response:
            css = response.read().decode()
        self.assertNotIn('@import "tailwindcss"', css)
        self.assertNotRegex(css, r'(?m)^@theme ')
        self.assertIn('--font-sans:', css)


if __name__ == "__main__":
    unittest.main()
