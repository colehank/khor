"""Serve the icon studio, or generate/check every installed app icon from one preset.

No app build is needed. Native catalog compilation belongs to mac_icon.py / CI.
"""
import argparse
import hashlib
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[1]
LAB = Path("apps/gui/dev/icon-lab")
ICONS = Path("apps/gui/src-tauri/icons")
PRESET = ICONS / "src/app-icon.json"
MANIFEST = ICONS / "src/generated.json"
IOS = Path("apps/gui/src-tauri/gen/apple/Assets.xcassets/AppIcon.appiconset")
RESVG_VERSION = "0.47.0"
PILLOW_VERSION = "12.2.0"
# Compatibility mask from the original app SVG. Native Apple assets stay square.
MASK = ("M330 0 L694 0 C894 0 1024 130 1024 330 L1024 694 "
        "C1024 894 894 1024 694 1024 L330 1024 C130 1024 0 894 0 694 "
        "L0 330 C0 130 130 0 330 0 Z")
INPUTS = [LAB / "renderer.mjs", LAB / "render.mjs", PRESET,
          ICONS / "Icon.icon/icon.json", IOS / "Contents.json",
          Path("scripts/app_icon.py"), Path("scripts/requirements-app-icon.txt")]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def pillow():
    # Serving and the general scripts test suite do not require imaging packages.
    import PIL
    from PIL import Image, PngImagePlugin
    if PIL.__version__ != PILLOW_VERSION:
        raise ValueError(f"Pillow {PILLOW_VERSION} required; install scripts/requirements-app-icon.txt")
    return Image, PngImagePlugin


def render(preset):
    return subprocess.check_output(["node", str(ROOT / LAB / "render.mjs")], input=preset)


def ios_sizes(root=ROOT):
    entries = json.loads((root / IOS / "Contents.json").read_text())["images"]
    sizes = {}
    for entry in entries:
        filename = entry["filename"]
        if Path(filename).name != filename or not filename.endswith(".png"):
            raise ValueError(f"Invalid iOS icon filename: {filename}")
        width, height = map(float, entry["size"].split("x"))
        pixels = width * float(entry["scale"].removesuffix("x"))
        if width != height or not pixels.is_integer() or pixels <= 0:
            raise ValueError(f"Invalid iOS icon size: {entry}")
        size = int(pixels)
        if filename in sizes and sizes[filename] != size:
            raise ValueError(f"Conflicting iOS icon sizes: {filename}")
        sizes[filename] = size
    return sizes


def asset_spec():
    spec = {
        str(ICONS / "src/mandala-glass.svg"): None,
        str(ICONS / "Icon.icon/Assets/app-icon.png"): (1024, "RGB"),
        str(ICONS / "icon.png"): (512, "RGBA"),
        str(ICONS / "icon.icns"): (1024, "RGBA"),
        str(ICONS / "icon.ico"): (256, "RGBA"),
        str(LAB / "native-macos.png"): (1024, "RGBA"),
        str(LAB / "native-ios.png"): (1024, "RGBA"),
    }
    spec.update({str(IOS / name): (size, "RGB") for name, size in ios_sizes().items()})
    return spec


def check():
    Image, _ = pillow()
    manifest = json.loads((ROOT / MANIFEST).read_text())
    expected_inputs = {str(path): digest((ROOT / path).read_bytes()) for path in INPUTS}
    if manifest.get("version") != 1 or manifest.get("inputs") != expected_inputs:
        raise ValueError("Icon inputs changed; run app_icon.py generate")
    if render((ROOT / PRESET).read_bytes()) != (ROOT / ICONS / "src/mandala-glass.svg").read_bytes():
        raise ValueError("SVG does not match the current preset and renderer")
    spec = asset_spec()
    if set(manifest["outputs"]) != set(spec):
        raise ValueError("Generated icon inventory is incomplete or contains unexpected files")
    for filename, image_spec in spec.items():
        path = ROOT / filename
        if digest(path.read_bytes()) != manifest["outputs"][filename]:
            raise ValueError(f"Generated icon changed: {filename}")
        if image_spec:
            with Image.open(path) as image:
                image.load()
                size, mode = image_spec
                if image.size != (size, size) or image.mode != mode:
                    raise ValueError(f"Wrong icon dimensions or color mode: {filename}")
                if path.suffix == ".ico" and image.info["sizes"] != {(s, s) for s in [16, 24, 32, 48, 64, 128, 256]}:
                    raise ValueError("ICO is missing required sizes")
    master = (ROOT / ICONS / "Icon.icon/Assets/app-icon.png").read_bytes()
    if master != (ROOT / IOS / "AppIcon-512@2x.png").read_bytes():
        raise ValueError("macOS and iOS must share the exact same 1024px artwork")
    icon = json.loads((ROOT / ICONS / "Icon.icon/icon.json").read_text())
    layer = icon["groups"][0]["layers"][0]
    if layer["image-name"] != "app-icon.png" or layer["glass"] or layer["position"] != {"scale": 1, "translation-in-points": [0, 0]}:
        raise ValueError("Native icon must use the shared artwork at full size without an extra glass layer")
    print(f"PASS: shared preset/SVG, {len(spec)} generated assets, iOS/macOS source equality")


def find_ictool():
    developer = Path(subprocess.check_output(["xcode-select", "-p"], text=True).strip())
    executable = developer.parent / "Applications/Icon Composer.app/Contents/Executables/ictool"
    if not executable.is_file():
        raise ValueError("Xcode with Icon Composer is required to export native previews")
    return executable


def generate(preset_path):
    Image, PngImagePlugin = pillow()
    version = subprocess.check_output(["resvg", "--version"], text=True).strip()
    if version.split()[-1] != RESVG_VERSION:
        raise ValueError(f"resvg {RESVG_VERSION} required (found {version})")
    ictool = find_ictool()
    preset = preset_path.read_bytes()
    svg = render(preset)  # Validate before changing any tracked file.
    with tempfile.TemporaryDirectory(prefix="khor-icon-") as directory:
        stage = Path(directory)

        def target(path):
            result = stage / path
            result.parent.mkdir(parents=True, exist_ok=True)
            return result

        def raster(source, name):
            source_path = stage / f"{name}.svg"
            source_path.write_bytes(source)
            output = stage / f"{name}.png"
            subprocess.run(["resvg", str(source_path), str(output)], check=True)
            with Image.open(output) as image:
                return image.convert("RGBA")

        full = raster(svg, "full")
        if full.getchannel("A").getextrema() != (255, 255):
            raise ValueError("Installed Apple artwork must have an opaque background")
        master = full.convert("RGB")
        srgb = PngImagePlugin.PngInfo()
        srgb.add(b"sRGB", b"\x00")

        def png(image, path):
            image.save(target(path), pnginfo=srgb, compress_level=9)

        target(PRESET).write_bytes(preset)
        target(ICONS / "src/mandala-glass.svg").write_bytes(svg)
        png(master, ICONS / "Icon.icon/Assets/app-icon.png")
        for name, size in ios_sizes().items():
            png(master.resize((size, size), Image.Resampling.LANCZOS), IOS / name)
        # Clip only compatibility assets. No hand-painted outer glass treatment.
        body = svg.decode().split(">", 1)[1].rsplit("</svg>", 1)[0]
        clipped = ('<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024">'
                   f'<defs><clipPath id="legacy-mask"><path d="{MASK}"/></clipPath></defs>'
                   f'<g clip-path="url(#legacy-mask)">{body}</g></svg>')
        legacy = raster(clipped.encode(), "legacy")
        png(legacy.resize((512, 512), Image.Resampling.LANCZOS), ICONS / "icon.png")
        legacy.save(target(ICONS / "icon.icns"), format="ICNS")
        legacy.save(target(ICONS / "icon.ico"), format="ICO",
                    sizes=[(size, size) for size in [16, 24, 32, 48, 64, 128, 256]])
        shutil.copyfile(ROOT / ICONS / "Icon.icon/icon.json", target(ICONS / "Icon.icon/icon.json"))
        for platform, name in [("macOS", "macos"), ("iOS", "ios")]:
            output = target(LAB / f"native-{name}.png")
            subprocess.run([str(ictool), str(stage / ICONS / "Icon.icon"), "--export-image",
                            "--output-file", str(output), "--platform", platform,
                            "--rendition", "Default", "--width", "1024", "--height", "1024", "--scale", "1"],
                           check=True, stdout=subprocess.DEVNULL)
        inputs = {str(path): digest((stage / path if path == PRESET else ROOT / path).read_bytes()) for path in INPUTS}
        outputs = {path: digest((stage / path).read_bytes()) for path in asset_spec()}
        manifest = {"version": 1, "tools": {"resvg": RESVG_VERSION, "Pillow": PILLOW_VERSION},
                    "inputs": inputs, "outputs": outputs}
        target(MANIFEST).write_text(json.dumps(manifest, indent=2) + "\n")
        # All rendering/exports must succeed before publishing the staged files.
        for path in [PRESET, *map(Path, outputs), MANIFEST]:
            source, destination = stage / path, ROOT / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)
    check()


def served_files():
    files = {"/" + name: ROOT / LAB / name for name in [
        "index.html", "styles.css", "app.mjs", "renderer.mjs", "reference-07.png",
        "native.html", "native-macos.png", "native-ios.png",
    ]}
    files["/"] = files["/index.html"]
    files["/icon-lab.html"] = files["/index.html"]
    files["/current-preset.json"] = ROOT / PRESET
    files["/tokens.css"] = ROOT / "apps/gui/src/styles/tokens.css"
    return files


class StudioHandler(SimpleHTTPRequestHandler):
    def do_GET(self):
        self.serve_file(False)

    def do_HEAD(self):
        self.serve_file(True)

    def serve_file(self, head):
        path = served_files().get(unquote(urlsplit(self.path).path))
        if path is None or not path.is_file():
            self.send_error(404)
            return
        data = path.read_bytes()
        if path.name == "tokens.css":
            # This zero-build tool consumes the authoritative product tokens.
            # Tailwind's theme containers become ordinary CSS variables; its
            # package imports are unnecessary for the studio's plain HTML.
            css = re.sub(r'^@import [^;]+;\s*', '', data.decode(), flags=re.MULTILINE)
            css = re.sub(r'@theme (?:inline|static)\s*\{', ':root {', css)
            data = css.encode()
        self.send_response(200)
        mime = "text/javascript" if path.suffix == ".mjs" else self.guess_type(str(path))
        self.send_header("Content-Type", mime)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        self.end_headers()
        if not head:
            self.wfile.write(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="action", required=True)
    server = commands.add_parser("serve", help="Serve only icon studio files")
    server.add_argument("--host", default="127.0.0.1")
    server.add_argument("--port", type=int, default=23962)
    commands.add_parser("check", help="Read-only source and generated asset validation")
    generator = commands.add_parser("generate", help="Regenerate assets and native previews on macOS")
    generator.add_argument("--preset", type=Path, default=ROOT / PRESET)
    args = parser.parse_args()
    try:
        if args.action == "generate":
            generate(args.preset)
        elif args.action == "check":
            check()
        else:
            with ThreadingHTTPServer((args.host, args.port), StudioHandler) as http:
                print(f"Icon studio: http://{args.host}:{args.port}/", flush=True)
                http.serve_forever()
    except KeyboardInterrupt:
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"app-icon: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
