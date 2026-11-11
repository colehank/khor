"""Serve the icon studio, or generate/check installed icons from approved appearances.

No app build is needed. Native catalog compilation belongs to mac_icon.py / CI.
"""
import argparse
import hashlib
from functools import lru_cache
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
APPEARANCES = ICONS / "src/app-icon-appearances.json"
IOS = Path("apps/gui/src-tauri/gen/apple/Assets.xcassets/AppIcon.appiconset")
RESVG_VERSION = "0.47.0"
PILLOW_VERSION = "12.2.0"
# Compatibility mask from the original app SVG. Native Apple assets stay square.
MASK = ("M330 0 L694 0 C894 0 1024 130 1024 330 L1024 694 "
        "C1024 894 894 1024 694 1024 L330 1024 C130 1024 0 894 0 694 "
        "L0 330 C0 130 130 0 330 0 Z")
INPUTS = [LAB / "renderer.ts", LAB / "render.ts", LAB / "package.json",
          LAB / "package-lock.json", LAB / "tsconfig.json", PRESET,
          APPEARANCES, ICONS / "src/app-icon-light.svg",
          ICONS / "src/app-icon-light-approval.json",
          Path("scripts/app_icon.py"), Path("scripts/requirements-app-icon.txt")]


def digest(data):
    return hashlib.sha256(data).hexdigest()


@lru_cache(maxsize=1)
def check_node():
    version = subprocess.check_output(["node", "--version"], text=True).strip()
    parts = tuple(map(int, version.removeprefix("v").split(".")[:2]))
    if parts < (22, 19):
        raise ValueError(f"Node 22.19+ required for the TypeScript studio (found {version})")


@lru_cache(maxsize=8)
def browser_javascript(source):
    """Derive browser JavaScript from the sole TS source, without a build tree."""
    check_node()
    return subprocess.check_output([
        "node", "--input-type=module", "--eval",
        "import { readFileSync } from 'node:fs';"
        "import { stripTypeScriptTypes } from 'node:module';"
        "process.stdout.write(stripTypeScriptTypes(readFileSync(0, 'utf8')));",
    ], input=source)


def pillow():
    # Serving and the general scripts test suite do not require imaging packages.
    import PIL
    from PIL import Image, PngImagePlugin
    if PIL.__version__ != PILLOW_VERSION:
        raise ValueError(f"Pillow {PILLOW_VERSION} required; install scripts/requirements-app-icon.txt")
    return Image, PngImagePlugin


def render(preset):
    check_node()
    return subprocess.check_output(["node", str(ROOT / LAB / "render.ts")], input=preset)


def appearance_policy(root=ROOT):
    policy = json.loads((root / APPEARANCES).read_text())
    if (policy.get("version") != 1 or policy.get("selection") != "system-icon-appearance"
            or set(policy.get("appearances", {})) != {"light", "dark"}
            or policy.get("default") != "light"
            or policy.get("fallback") not in {"light", "dark"}):
        raise ValueError("Invalid app icon appearance policy")
    for variant in policy["appearances"].values():
        for key, suffix in [("artwork", ".svg"), ("asset", ".png")]:
            name = variant[key]
            if Path(name).name != name or not name.endswith(suffix):
                raise ValueError(f"Invalid icon {key}: {name}")
    for key in ["artwork", "asset"]:
        if len({v[key] for v in policy["appearances"].values()}) != 2:
            raise ValueError("Light and dark must have separate artwork and assets")
    if {key: value["artwork"] for key, value in policy["appearances"].items()} != {
            "light": "app-icon-light.svg", "dark": "mandala-glass.svg"}:
        raise ValueError("Artwork must use the approved light and generated dark source paths")
    return policy


def native_document(policy):
    variants = policy["appearances"]
    return {
        "fill": {"solid": "srgb:0.95686,0.95294,0.92941,1.00000"},
        "groups": [{"layers": [{
            "blend-mode": "normal", "fill": "automatic", "glass": False, "hidden": False,
            # Composer uses specializations INSTEAD OF a fixed image-name.
            "image-name-specializations": [
                {"value": variants[policy["default"]]["asset"]},
                {"appearance": "dark", "value": variants["dark"]["asset"]},
            ],
            "name": "辉心", "position": {"scale": 1, "translation-in-points": [0, 0]},
        }], "shadow": {"kind": "neutral", "opacity": 0},
            "translucency": {"enabled": False, "value": 0}}],
        "supported-platforms": {"squares": "shared"},
    }


def ios_document(policy):
    # Apple's single-size catalog generates legacy sizes too. Dark icons are
    # selected by iOS 18+ Home Screen appearance, independently of app UI theme.
    return {"images": [
        {"filename": "AppIcon-512@2x.png", "idiom": "universal", "platform": "ios", "size": "1024x1024"},
        {"filename": "AppIcon-dark.png", "idiom": "universal", "platform": "ios", "size": "1024x1024",
         "appearances": [{"appearance": "luminosity", "value": "dark"}]},
    ], "info": {"author": "xcode", "version": 1}}


def ios_assets(root=ROOT):
    entries = json.loads((root / IOS / "Contents.json").read_text())["images"]
    sizes = {}
    for entry in entries:
        filename = entry["filename"]
        if Path(filename).name != filename or not filename.endswith(".png"):
            raise ValueError(f"Invalid iOS icon filename: {filename}")
        width, height = map(float, entry["size"].split("x"))
        pixels = width * float(entry.get("scale", "1x").removesuffix("x"))
        if width != height or not pixels.is_integer() or pixels <= 0:
            raise ValueError(f"Invalid iOS icon size: {entry}")
        appearance = "light"
        if entry.get("appearances"):
            if entry["appearances"] != [{"appearance": "luminosity", "value": "dark"}]:
                raise ValueError(f"Unsupported iOS icon appearance: {entry}")
            appearance = "dark"
        item = (int(pixels), appearance)
        if filename in sizes and sizes[filename] != item:
            raise ValueError(f"Conflicting iOS icon sizes: {filename}")
        sizes[filename] = item
    return sizes


def ios_sizes(root=ROOT):
    return {name: item[0] for name, item in ios_assets(root).items()}


def asset_spec(root=ROOT):
    policy = appearance_policy(root)
    spec = {
        str(ICONS / "src/mandala-glass.svg"): None,
        str(ICONS / "Icon.icon/icon.json"): None,
        str(IOS / "Contents.json"): None,
        str(ICONS / "icon.png"): (512, "RGBA"),
        str(ICONS / "icon.icns"): (1024, "RGBA"),
        str(ICONS / "icon.ico"): (256, "RGBA"),
        str(LAB / "native-macos.png"): (1024, "RGBA"),
        str(LAB / "native-ios.png"): (1024, "RGBA"),
        str(LAB / "native-macos-dark.png"): (1024, "RGBA"),
        str(LAB / "native-ios-dark.png"): (1024, "RGBA"),
    }
    spec.update({str(ICONS / "Icon.icon/Assets" / v["asset"]): (1024, "RGB")
                 for v in policy["appearances"].values()})
    spec.update({str(IOS / name): (size, "RGB") for name, size in ios_sizes(root).items()})
    return spec


def check_assets(root=ROOT):
    Image, _ = pillow()
    policy = appearance_policy(root)
    manifest = json.loads((root / MANIFEST).read_text())
    expected_inputs = {str(path): digest((root / path).read_bytes()) for path in INPUTS}
    if manifest.get("version") != 2 or manifest.get("inputs") != expected_inputs:
        raise ValueError("Icon inputs changed; run app_icon.py generate")
    if json.loads((root / IOS / "Contents.json").read_text()) != ios_document(policy):
        raise ValueError("iOS appearance mapping differs from the shared policy")
    if json.loads((root / ICONS / "Icon.icon/icon.json").read_text()) != native_document(policy):
        raise ValueError("Native appearance mapping differs from the shared policy")
    approval = json.loads((root / ICONS / "src/app-icon-light-approval.json").read_text())
    light = root / ICONS / "src" / policy["appearances"]["light"]["artwork"]
    if digest(light.read_bytes()) != approval["sha256"]["artwork.svg"]:
        raise ValueError("Light artwork differs from the approved final")
    spec = asset_spec(root)
    if set(manifest["outputs"]) != set(spec):
        raise ValueError("Generated icon inventory is incomplete or contains unexpected files")
    for filename, image_spec in spec.items():
        path = root / filename
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
    for name, (size, appearance) in ios_assets(root).items():
        if size == 1024:
            master = root / ICONS / "Icon.icon/Assets" / policy["appearances"][appearance]["asset"]
            if master.read_bytes() != (root / IOS / name).read_bytes():
                raise ValueError(f"macOS and iOS {appearance} artwork must be identical")
    expected_ios = {name for name, _ in ios_assets(root).items()} | {"Contents.json"}
    if {p.name for p in (root / IOS).iterdir() if p.is_file()} != expected_ios:
        raise ValueError("iOS catalog contains stale or untracked icon assets")
    return len(spec)


def check():
    count = check_assets()
    if render((ROOT / PRESET).read_bytes()) != (ROOT / ICONS / "src/mandala-glass.svg").read_bytes():
        raise ValueError("SVG does not match the current preset and renderer")
    print(f"PASS: approved light, dark preset/SVG, {count} assets, both iOS/macOS appearances equal")


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
    policy = appearance_policy()
    previous = json.loads((ROOT / MANIFEST).read_text())
    previous_ios = {Path(name).name for name in previous["outputs"] if Path(name).parent == IOS}
    if {p.name for p in (ROOT / IOS).iterdir() if p.is_file()} - previous_ios - {"Contents.json"}:
        raise ValueError("Unexpected files in iOS catalog; preserve them before regeneration")
    preset = (preset_path or ROOT / PRESET).read_bytes()
    if preset_path:
        svg = render(preset)  # Validate before changing any tracked file.
    else:
        # Repackage accepted artwork without running a TS toolchain. If its
        # preset/renderer changed, require an explicit re-render instead.
        for path in [p for p in INPUTS if p == PRESET or p.parts[:len(LAB.parts)] == LAB.parts]:
            if previous["inputs"].get(str(path)) != digest((ROOT / path).read_bytes()):
                raise ValueError("Dark rendering inputs changed; generate with --preset")
        path = ICONS / "src/mandala-glass.svg"
        svg = (ROOT / path).read_bytes()
        if previous["outputs"].get(str(path)) != digest(svg):
            raise ValueError("Dark SVG changed; generate with --preset")
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

        for path in INPUTS:
            shutil.copyfile(ROOT / path, target(path))
        variants = {appearance: svg if appearance == "dark" else (ROOT / ICONS / "src" / v["artwork"]).read_bytes()
                    for appearance, v in policy["appearances"].items()}
        masters = {}
        for appearance, source in variants.items():
            full = raster(source, appearance)
            if full.size != (1024, 1024) or full.getchannel("A").getextrema() != (255, 255):
                raise ValueError("Installed artwork must be opaque and 1024px square")
            masters[appearance] = full.convert("RGB")
        srgb = PngImagePlugin.PngInfo()
        srgb.add(b"sRGB", b"\x00")

        def png(image, path):
            image.save(target(path), pnginfo=srgb, compress_level=9)

        target(PRESET).write_bytes(preset)
        target(ICONS / "src/mandala-glass.svg").write_bytes(svg)
        target(ICONS / "Icon.icon/icon.json").write_text(json.dumps(native_document(policy), indent=2, ensure_ascii=False) + "\n")
        target(IOS / "Contents.json").write_text(json.dumps(ios_document(policy), indent=2) + "\n")
        for appearance, master in masters.items():
            png(master, ICONS / "Icon.icon/Assets" / policy["appearances"][appearance]["asset"])
        for name, (size, appearance) in ios_assets(stage).items():
            png(masters[appearance].resize((size, size), Image.Resampling.LANCZOS), IOS / name)
        # Clip only compatibility assets. No hand-painted outer glass treatment.
        body = variants[policy["fallback"]].decode().split(">", 1)[1].rsplit("</svg>", 1)[0]
        clipped = ('<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024">'
                   f'<defs><clipPath id="legacy-mask"><path d="{MASK}"/></clipPath></defs>'
                   f'<g clip-path="url(#legacy-mask)">{body}</g></svg>')
        legacy = raster(clipped.encode(), "legacy")
        png(legacy.resize((512, 512), Image.Resampling.LANCZOS), ICONS / "icon.png")
        legacy.save(target(ICONS / "icon.icns"), format="ICNS")
        legacy.save(target(ICONS / "icon.ico"), format="ICO",
                    sizes=[(size, size) for size in [16, 24, 32, 48, 64, 128, 256]])
        for platform, name in [("macOS", "macos"), ("iOS", "ios")]:
            for suffix, rendition in [("", "Default"), ("-dark", "Dark")]:
                output = target(LAB / f"native-{name}{suffix}.png")
                subprocess.run([str(ictool), str(stage / ICONS / "Icon.icon"), "--export-image",
                                "--output-file", str(output), "--platform", platform,
                                "--rendition", rendition, "--width", "1024", "--height", "1024", "--scale", "1"],
                               check=True, stdout=subprocess.DEVNULL)
        inputs = {str(path): digest((stage / path).read_bytes()) for path in INPUTS}
        outputs = {path: digest((stage / path).read_bytes()) for path in asset_spec(stage)}
        manifest = {"version": 2, "tools": {"resvg": RESVG_VERSION, "Pillow": PILLOW_VERSION},
                    "inputs": inputs, "outputs": outputs}
        target(MANIFEST).write_text(json.dumps(manifest, indent=2) + "\n")
        check_assets(stage)
        # All rendering/exports must succeed before publishing the staged files.
        for path in [PRESET, *map(Path, outputs), MANIFEST]:
            source, destination = stage / path, ROOT / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)
        for name in previous["outputs"]:
            path = Path(name)
            if path.parent == IOS and path.suffix == ".png" and name not in outputs:
                (ROOT / path).unlink(missing_ok=True)
    count = check_assets()
    print(f"Generated {count} assets from approved light/dark artwork; run check for renderer verification")


def served_files():
    files = {"/" + name: ROOT / LAB / name for name in [
        "index.html", "styles.css", "app.ts", "renderer.ts", "reference-07.png",
        "native.html", "native-macos.png", "native-ios.png", "native-macos-dark.png", "native-ios-dark.png",
    ]}
    files["/"] = files["/index.html"]
    files["/icon-lab.html"] = files["/index.html"]
    files["/current-preset.json"] = ROOT / PRESET
    files["/tokens.css"] = ROOT / "apps/gui/src/styles/tokens.css"
    # Existing open tabs may still request the former URLs. Both aliases are
    # derived from the same TS source; there is no second JS implementation.
    files["/app.mjs"] = files["/app.ts"]
    files["/renderer.mjs"] = files["/renderer.ts"]
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
        if path.suffix == ".ts":
            try:
                data = browser_javascript(data)
            except (OSError, ValueError, subprocess.CalledProcessError) as error:
                self.log_error("Cannot prepare TypeScript module: %s", error)
                self.send_error(500, "TypeScript module unavailable")
                return
        if path.name == "tokens.css":
            # This zero-build tool consumes the authoritative product tokens.
            # Tailwind's theme containers become ordinary CSS variables; its
            # package imports are unnecessary for the studio's plain HTML.
            css = re.sub(r'^@import [^;]+;\s*', '', data.decode(), flags=re.MULTILINE)
            css = re.sub(r'@theme (?:inline|static)\s*\{', ':root {', css)
            data = css.encode()
        self.send_response(200)
        mime = "text/javascript" if path.suffix == ".ts" else self.guess_type(str(path))
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
    generator.add_argument("--preset", type=Path, help="Re-render dark artwork from a studio preset")
    args = parser.parse_args()
    try:
        if args.action == "generate":
            generate(args.preset)
        elif args.action == "check":
            check()
        else:
            check_node()
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
