#!/usr/bin/env python3
"""Run actual installer components in scratch; never run its main entry point."""
import contextlib
import gzip
import hashlib
import http.server
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest


ROOT = Path(__file__).resolve().parents[1]
INSTALL = ROOT / "scripts/install.sh"
BEGIN = "# KHOR_INSTALL_DECLARATIONS_BEGIN\n"
END = "# KHOR_INSTALL_DECLARATIONS_END\n"
MAIN = 'khor_install_main "$@"\n'


def component_declarations(source):
    if source.count(BEGIN) != 1 or source.count(END) != 1:
        raise ValueError("installer declaration markers must be unique")
    prefix, rest = source.split(BEGIN)
    block, suffix = rest.split(END)
    if any(line.strip() and not line.startswith("#") for line in prefix.splitlines()):
        raise ValueError("unexpected executable installer prefix")
    if suffix != MAIN or source.count("\n" + MAIN) != 1:
        raise ValueError("installer must end in its unique main invocation")
    main_header = "khor_install_main() {\n"
    if block.count(main_header) != 1 or not block.endswith("}\n"):
        raise ValueError("installer main declaration boundary changed")
    helpers, _ = block.split(main_header)
    canonical = (ROOT / "scripts/install-components.sh").read_bytes().decode("utf-8")
    if helpers != canonical:
        raise ValueError("embedded installer components differ from their canonical source")
    syntax = subprocess.run(["/bin/sh", "-n"], input=helpers, text=True, capture_output=True)
    if syntax.returncode:
        raise ValueError("installer component syntax is invalid")
    return helpers


@contextlib.contextmanager
def loopback(contents, requests=None):
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            self.respond()

        def do_HEAD(self):
            self.respond()

        def respond(self):
            if requests is not None:
                requests.append(self.path)
            payload = contents.get(self.path)
            if callable(payload):
                payload = payload()
            if isinstance(payload, tuple) and payload[0] == "raw":
                self.connection.sendall(payload[1])
                self.close_connection = True
                return
            if isinstance(payload, tuple):
                status, headers, payload = payload
            else:
                status, headers = (200 if payload is not None else 404), []
            if isinstance(status, tuple):
                for interim in status[:-1]:
                    self.send_response_only(interim)
                    self.end_headers()
                status = status[-1]
            self.send_response(status)
            for key, value in headers:
                self.send_header(key, value)
            self.end_headers()
            if payload is not None and self.command != "HEAD":
                self.wfile.write(payload)

        def log_message(self, *_):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}"
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


class InstallComponentsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = INSTALL.read_bytes().decode("utf-8")
        cls.helpers = component_declarations(cls.source)

    def setUp(self):
        self.scratch = tempfile.TemporaryDirectory(prefix="release-install-")
        self.addCleanup(self.scratch.cleanup)
        self.directory = Path(self.scratch.name)
        self.destination = self.directory / "bin"
        self.destination.mkdir()
        self.binary = self.destination / "khor"
        self.binary.write_bytes(b"original binary")
        self.binary.chmod(0o755)
        self.payload = Path("/usr/bin/true").read_bytes()
        self.archive = gzip.compress(self.payload, mtime=0)

    def run_component(self, url, preplaced=None, digest=None, version="v1.2.3", mirror=None):
        # The wrapper confines curl to this exact loopback fixture and ignores
        # user curl configuration. No HOME override or installer product switch.
        wrapper = self.directory / "tools"
        wrapper.mkdir(exist_ok=True)
        endpoint = url.rsplit("/", 1)[0] + "/"
        (wrapper / "curl").write_text(
            "#!/bin/sh\nfor arg do\ncase \"$arg\" in\n"
            f"http://*|https://*) case \"$arg\" in '{endpoint}'*) : ;; *) exit 66 ;; esac ;;\n"
            "esac\ndone\nexec /usr/bin/curl --disable --noproxy '*' \"$@\"\n"
        )
        (wrapper / "curl").chmod(0o755)
        environment = {key: value for key, value in os.environ.items() if not key.startswith("KHOR_")}
        environment["PATH"] = str(wrapper) + os.pathsep + os.environ.get("PATH", os.defpath)
        environment["TMPDIR"] = str(self.directory)
        if preplaced:
            environment["KHOR_ASSET_FILE"] = str(preplaced)
        if digest is not None:
            environment["KHOR_ASSET_SHA256"] = digest
        script = self.helpers
        if mirror is not None:
            script += '\nkhor_fetch_release "$5" "$4" "$6" "$3" "$2"\n'

        else:
            script += '\nkhor_fetch_verify_place "$1" "$2" "$3" "$4"\n'
        return subprocess.run(
            ["/bin/sh", "-s", "--", url, str(self.destination), "fixture", version, "owner/repo", mirror or ""],
            input=script, text=True, capture_output=True, env=environment,
        )

    def assert_no_stage(self):
        self.assertEqual(list(self.destination.glob(".khor.download.*")), [])
        self.assertEqual(list(self.directory.glob("khor-release.*")), [])

    def with_digest(self, payload, line=None):
        digest = hashlib.sha256(payload).hexdigest()
        return {"/fixture.gz": payload, "/fixture.gz.sha256": (line if line is not None else digest + "  fixture.gz\n").encode()}

    def test_load_declares_only_components(self):
        result = subprocess.run(["/bin/sh", "-s"], input=self.helpers + "\nprintf loaded\n", text=True, capture_output=True)
        self.assertEqual((result.returncode, result.stdout), (0, "loaded"))
        self.assertNotIn("$HOME", self.helpers)

    def test_loader_refuses_changed_entry_boundaries(self):
        mutations = [
            self.source.replace(BEGIN, BEGIN + BEGIN), self.source.replace(END, ""),
            self.source.replace("\n", "\r\n"),
            "printf unexpected\n" + self.source, self.source + "printf unexpected\n",
            self.source.replace(MAIN, MAIN + MAIN), self.source.replace("khor_install_main() {\n", "printf unexpected\n"),
        ]
        for source in mutations:
            with self.subTest(source_sha256=hashlib.sha256(source.encode()).hexdigest()):
                with self.assertRaises(ValueError):
                    component_declarations(source)

    def test_loader_refuses_top_level_commands_between_helpers(self):
        marker = self.directory / "unexpected"
        extra = "khor_fixture_extra() (\n    :\n)\n\n"
        for command in [f"printf unexpected > '{marker}'\n", "khor_install_main\n"]:
            source = self.source.replace("khor_install_main() {\n", command + extra + "khor_install_main() {\n")
            with self.subTest(command=command):
                with self.assertRaises(ValueError):
                    component_declarations(source)
                self.assertFalse(marker.exists())

    def shell_components(self, source):
        installer = self.directory / "installer.sh"
        installer.write_text(source)
        library = (ROOT / "scripts/onset-lib.sh").read_text()
        return subprocess.run(["/bin/sh", "-s", "--", str(installer), str(ROOT / "scripts/install-components.sh")],
            input=library + '\nkhor_install_components "$1" "$2"\n', text=True, capture_output=True)

    def test_onset_shell_loader_matches_and_rejects_interhelper_commands(self):
        result = self.shell_components(self.source)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, self.helpers)
        marker = self.directory / "unexpected"
        for command in [f"printf unexpected > '{marker}'\n", "khor_install_main\n"]:
            source = self.source.replace("khor_install_main() {\n", command + "khor_install_main() {\n")
            result = self.shell_components(source)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "")
            self.assertFalse(marker.exists())

    def test_loaders_refuse_compound_closures_and_quoted_boundary_escape(self):
        variants = {
            "compound-close": "\n) ; printf LOADER_TOPLEVEL_RAN; khor_shadow() (\n    :\n)\n",
            "quoted-boundary": "\n    _quoted='\n)\nkhor_shadow() (\n    ' ; ) ; printf LOADER_TOPLEVEL_RAN; khor_second() (\n    :\n)\n",
        }
        for name, replacement in variants.items():
            source = self.source.replace("\n)\n", replacement, 1)
            with self.subTest(variant=name, loader="python"):
                with self.assertRaises(ValueError):
                    component_declarations(source)
            with self.subTest(variant=name, loader="onset-shell"):
                result = self.shell_components(source)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, "")

    def test_fetch_place_preserves_open_old_inode(self):
        with self.binary.open("rb") as old, loopback(self.with_digest(self.archive)) as base:
            result = self.run_component(base + "/fixture.gz")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(old.read(), b"original binary")
        self.assertEqual(self.binary.read_bytes(), self.payload)
        self.assertTrue(os.access(self.binary, os.X_OK))
        self.assert_no_stage()

    def test_failed_fetch_preserves_old_binary(self):
        with loopback({}) as base:
            result = self.run_component(base + "/missing.gz")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.binary.read_bytes(), b"original binary")
        self.assert_no_stage()

    def test_bad_gzip_preserves_old_binary(self):
        with loopback(self.with_digest(b"not gzip")) as base:
            result = self.run_component(base + "/fixture.gz")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.binary.read_bytes(), b"original binary")
        self.assert_no_stage()

    def test_nonprogram_preserves_old_binary(self):
        with loopback(self.with_digest(gzip.compress(b"not executable", mtime=0))) as base:
            result = self.run_component(base + "/fixture.gz")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.binary.read_bytes(), b"original binary")
        self.assert_no_stage()

    def test_preplaced_archive_preserves_existing_consumption(self):
        archive = self.directory / "provided.gz"
        archive.write_bytes(self.archive)
        result = self.run_component("http://127.0.0.1:1/unused", archive, hashlib.sha256(self.archive).hexdigest())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.binary.read_bytes(), self.payload)
        self.assertFalse(archive.exists())
        self.assert_no_stage()

    def test_missing_checksum_preserves_old_binary(self):
        with loopback({"/fixture.gz": self.archive}) as base:
            result = self.run_component(base + "/fixture.gz")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.binary.read_bytes(), b"original binary")
        self.assert_no_stage()

    def test_wrong_checksum_preserves_old_binary(self):
        with loopback(self.with_digest(self.archive, "0" * 64 + "  fixture.gz\n")) as base:
            result = self.run_component(base + "/fixture.gz")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.binary.read_bytes(), b"original binary")
        self.assert_no_stage()

    def test_checksum_exact_basename_and_single_record(self):
        digest = hashlib.sha256(self.archive).hexdigest()
        invalid = ["", digest + "  other.gz\n", digest + "  ./fixture.gz\n", digest + " *fixture.gz\n",
                   digest + "  fixture.gz\n\n", digest + "  fixture.gz\n" + digest + "  fixture.gz\n",
                   digest + "  fixture.gz\r\n", digest.upper() + "  fixture.gz\n"]
        for line in invalid:
            with self.subTest(line=line), loopback(self.with_digest(self.archive, line)) as base:
                result = self.run_component(base + "/fixture.gz")
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.binary.read_bytes(), b"original binary")
                self.assert_no_stage()

    def test_checksum_single_record_without_final_lf(self):
        line = hashlib.sha256(self.archive).hexdigest() + "  fixture.gz"
        with loopback(self.with_digest(self.archive, line)) as base:
            result = self.run_component(base + "/fixture.gz")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.binary.read_bytes(), self.payload)

    def test_digest_without_preplaced_file_never_fetches(self):
        requests = []
        with loopback(self.with_digest(self.archive), requests) as base:
            result = self.run_component(base + "/fixture.gz", digest="0" * 64)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(requests, [])
        self.assertEqual(self.binary.read_bytes(), b"original binary")

    def test_preplaced_latest_does_not_resolve_network_metadata(self):
        archive = self.directory / "provided.gz"
        archive.write_bytes(self.archive)
        result = self.run_component("http://127.0.0.1:1/unused", archive,
            hashlib.sha256(self.archive).hexdigest(), version="latest", mirror="")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.binary.read_bytes(), self.payload)

    def test_preplaced_requires_matching_digest_without_consuming_failure(self):
        for digest in [None, "0" * 64, "not-a-digest"]:
            with self.subTest(digest=digest):
                archive = self.directory / "provided.gz"
                archive.write_bytes(self.archive)
                result = self.run_component("http://127.0.0.1:1/unused", archive, digest)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.binary.read_bytes(), b"original binary")
                self.assertEqual(archive.read_bytes(), self.archive)
                self.assert_no_stage()

    def test_preplaced_destination_alias_does_not_remove_new_binary(self):
        self.binary.write_bytes(self.archive)
        alias = self.destination / ".." / "bin" / "khor"
        result = self.run_component("http://127.0.0.1:1/unused", alias, hashlib.sha256(self.archive).hexdigest())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.binary.read_bytes(), self.payload)
        self.assert_no_stage()

    def test_oversized_checksum_is_bounded_and_rejected(self):
        with loopback(self.with_digest(self.archive, "x" * 100000)) as base:
            result = self.run_component(base + "/fixture.gz")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.binary.read_bytes(), b"original binary")
        self.assert_no_stage()

    def mirror_contents(self):
        older = self.archive
        newer = gzip.compress(self.payload + b"new generation", mtime=0)
        contents = {"/VERSION": b"v1.2.3\n", "/fixture.gz": newer,
                    "/fixture.gz.sha256": (hashlib.sha256(newer).hexdigest() + "  fixture.gz\n").encode()}
        for tag, archive in [("v1.2.3", older), ("v1.2.4", newer)]:
            contents[f"/releases/{tag}/fixture.gz"] = archive
            contents[f"/releases/{tag}/fixture.gz.sha256"] = (hashlib.sha256(archive).hexdigest() + "  fixture.gz\n").encode()
        return contents

    def test_explicit_mirror_version_never_reads_current(self):
        requests = []
        with loopback(self.mirror_contents(), requests) as base:
            result = self.run_component(base + "/unused", version="v1.2.3", mirror=base)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.binary.read_bytes(), self.payload)
        self.assertEqual(requests, ["/releases/v1.2.3/fixture.gz", "/releases/v1.2.3/fixture.gz.sha256"])

    def test_latest_mirror_pins_version_once(self):
        requests = []
        contents = self.mirror_contents()
        def select_then_move():
            contents["/VERSION"] = b"v1.2.4\n"
            return b"v1.2.3\n"
        contents["/VERSION"] = select_then_move
        with loopback(contents, requests) as base:
            result = self.run_component(base + "/unused", version="latest", mirror=base)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.binary.read_bytes(), self.payload)
        self.assertEqual(requests, ["/VERSION", "/releases/v1.2.3/fixture.gz", "/releases/v1.2.3/fixture.gz.sha256"])

    def test_descriptor_is_one_bounded_safe_semver_tag(self):
        descriptor = self.directory / "VERSION"
        valid = [b"v0.1.0", b"v1.2.3\n", b"v1.2.3-beta.1+build.02\n"]
        invalid = [b"", b"latest\n", b"v1.2.3\n\n", b"v1.2.3\r\n", b" v1.2.3\n", b"v1.2.3\0ignored",
                   b"v01.2.3", b"v1.2.3-", b"v1.2.3+", b"v1.2.3-beta..1", b"v1.2.3-01", b"v1.2.3/other", b"v1.2.3%2fother", b"v1.2.3+" + b"x" * 256]
        for value in valid + invalid:
            with self.subTest(value=value):
                descriptor.write_bytes(value)
                result = subprocess.run(["/bin/sh", "-s", "--", str(descriptor)],
                    input=self.helpers + '\nkhor_descriptor_tag "$1"\n', text=True, capture_output=True)
                if value in valid:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(result.stdout.encode(), value.rstrip(b"\n") + b"\n")
                else:
                    self.assertNotEqual(result.returncode, 0)

    def test_legacy_flat_mirror_fails_without_fallback(self):
        requests = []
        contents = self.with_digest(self.archive)
        contents["/VERSION"] = b"v1.2.3\n"
        with loopback(contents, requests) as base:
            result = self.run_component(base + "/unused", version="latest", mirror=base)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.binary.read_bytes(), b"original binary")
        self.assertEqual(requests, ["/VERSION", "/releases/v1.2.3/fixture.gz"])
        self.assertIn("must migrate", result.stderr)

    def run_github(self, tool, contents, version="latest", fail_after_tag=False,
                   help_override=None, drop_redirect_notice=False):
        executable = shutil.which(tool)
        if executable is None:
            self.skipTest(f"{tool} is unavailable")
        tools = self.directory / ("github-tools-" + tool)
        tools.mkdir(exist_ok=True)
        for name in ["awk", "grep", "dd", "wc", "tr", "cat", "head", "od", "mktemp", "mkfifo", "rm", "gzip", "chmod", "mv", "cp", "sed", "sha256sum", "shasum"]:
            original = shutil.which(name)
            if original and not (tools / name).exists():
                (tools / name).symlink_to(original)
        log = self.directory / ("github-network-" + tool + ".jsonl")
        log.write_text("")
        requests = []
        with loopback(contents, requests) as base:
            wrapper = tools / "network.py"
            wrapper.write_text(
                "import os,sys,json,subprocess\nfrom pathlib import Path\n"
                f"base={base!r}; log=Path({str(log)!r})\nargs=[]\n"
                f"help_override={help_override!r}\n"
                "if '--help' in sys.argv and help_override is not None:\n print(help_override);sys.exit(0)\n"
                "for arg in sys.argv[1:]:\n"
                " if arg.startswith(('http://','https://')):\n"
                "  with log.open('a') as stream: stream.write(json.dumps(arg)+'\\n')\n"
                "  if not arg.startswith('https://github.com/owner/repo/'): sys.exit(66)\n"
                "  arg=base+arg.removeprefix('https://github.com')\n"
                " args.append(arg)\n"
                + (f"command=[{executable!r},'--disable','--noproxy','*',*args]\n" if tool == "curl" else
                   f"command=[{executable!r},'--no-config','--no-hsts','--no-netrc','--no-proxy',*args]\n")
                + f"if {drop_redirect_notice!r} and '--spider' in args:\n"
                " result=subprocess.run(command,capture_output=True)\n sys.stdout.buffer.write(result.stdout)\n"
                " sys.stderr.buffer.write(result.stderr.replace(b'0 redirections exceeded.\\n',b''))\n sys.exit(result.returncode)\n"
                + f"if {fail_after_tag!r} and any('/releases/tag/' in arg for arg in args):\n"
                " result=subprocess.call(command)\n sys.exit(7 if result==0 else result)\n"
                f"os.execv({executable!r},command)\n"
            )
            (tools / tool).write_text(f"#!/bin/sh\nexec {shlex.quote(sys.executable)} {shlex.quote(str(wrapper))} \"$@\"\n")
            (tools / tool).chmod(0o755)
            environment = {key: value for key, value in os.environ.items() if not key.startswith("KHOR_")}
            environment["PATH"] = str(tools)
            environment["TMPDIR"] = str(self.directory)
            result = subprocess.run(["/bin/sh", "-s", "--", version, str(self.destination)],
                input=self.helpers + '\nkhor_fetch_release owner/repo "$1" "" fixture "$2"\n',
                text=True, capture_output=True, env=environment, timeout=30)
        return result, requests, [__import__("json").loads(line) for line in log.read_text().splitlines()]

    def github_contents(self):
        archive = self.archive
        return {
            "/owner/repo/releases/latest": (302, [("Location", "/owner/repo/releases/tag/v1.2.3")], b""),
            "/owner/repo/releases/tag/v1.2.3": b"",
            "/owner/repo/releases/download/v1.2.3/fixture.gz": archive,
            "/owner/repo/releases/download/v1.2.3/fixture.gz.sha256": (hashlib.sha256(archive).hexdigest() + "  fixture.gz\n").encode(),
        }

    def test_github_latest_once_with_real_curl_and_wget_only(self):
        for tool in ["curl", "wget"]:
            with self.subTest(tool=tool):
                contents = self.github_contents()
                def select_then_move():
                    contents["/owner/repo/releases/latest"] = (302, [("Location", "/owner/repo/releases/tag/v1.2.4")], b"")
                    return (302, [("Location", "/owner/repo/releases/tag/v1.2.3")], b"")
                contents["/owner/repo/releases/latest"] = select_then_move
                result, requests, _ = self.run_github(tool, contents)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(self.binary.read_bytes(), self.payload)
                self.assertEqual(requests, ["/owner/repo/releases/latest", "/owner/repo/releases/tag/v1.2.3",
                    "/owner/repo/releases/download/v1.2.3/fixture.gz", "/owner/repo/releases/download/v1.2.3/fixture.gz.sha256"])
                self.assert_no_stage()

    def test_github_explicit_tag_never_reads_latest(self):
        for tool in ["curl", "wget"]:
            with self.subTest(tool=tool):
                result, requests, _ = self.run_github(tool, self.github_contents(), "v1.2.3")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(self.binary.read_bytes(), self.payload)
                self.assertEqual(requests, ["/owner/repo/releases/download/v1.2.3/fixture.gz", "/owner/repo/releases/download/v1.2.3/fixture.gz.sha256"])

    def test_github_relative_redirect_and_encoded_build_metadata(self):
        for tool in ["curl", "wget"]:
            for encoded in [False, True]:
                with self.subTest(tool=tool, encoded=encoded):
                    tag = "v1.2.3+build.1" if encoded else "v1.2.3"
                    location = "/owner/repo/releases/tag/v1.2.3%2Bbuild.1" if encoded else "tag/v1.2.3"
                    contents = self.github_contents()
                    contents["/owner/repo/releases/latest"] = (302, [("Location", location)], b"")
                    contents["/owner/repo/releases/tag/" + tag.replace("+", "%2B")] = b""
                    contents[f"/owner/repo/releases/download/{tag}/fixture.gz"] = self.archive
                    contents[f"/owner/repo/releases/download/{tag}/fixture.gz.sha256"] = (hashlib.sha256(self.archive).hexdigest() + "  fixture.gz\n").encode()
                    result, _, _ = self.run_github(tool, contents)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(self.binary.read_bytes(), self.payload)

    def test_github_informational_response_does_not_replace_final_block(self):
        for tool in ["curl", "wget"]:
            with self.subTest(tool=tool):
                contents = self.github_contents()
                contents["/owner/repo/releases/latest"] = ((100, 302), [("Location", "/owner/repo/releases/tag/v1.2.3")], b"")
                result, _, _ = self.run_github(tool, contents)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(self.binary.read_bytes(), self.payload)

    def test_metadata_process_failure_cannot_publish_valid_headers(self):
        for tool in ["curl", "wget"]:
            with self.subTest(tool=tool):
                result, requests, _ = self.run_github(tool, self.github_contents(), fail_after_tag=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(self.binary.read_bytes(), b"original binary")
                self.assertFalse(any("/download/" in path for path in requests))
                self.assert_no_stage()
        result, requests, _ = self.run_github("wget", self.github_contents(), drop_redirect_notice=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(requests, ["/owner/repo/releases/latest"])

    def test_wget_missing_required_capabilities_fails_before_network(self):
        for help_text in ["BusyBox fixture wget capabilities", "GNU Wget fixture\n--max-redirect --server-response --spider"]:
            with self.subTest(help_text=help_text):
                result, requests, logical = self.run_github("wget", self.github_contents(), help_override=help_text)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(requests, [])
                self.assertEqual(logical, [])
                self.assertEqual(self.binary.read_bytes(), b"original binary")
                self.assertIn("cannot safely resolve latest", result.stderr)
                self.assert_no_stage()

    def test_wget_raw_block_contract_rejects_ambiguity(self):
        executable = shutil.which("wget")
        if executable is None:
            self.skipTest("GNU wget is unavailable")
        environment = os.environ.copy()
        environment["LC_ALL"] = "C"
        with loopback({"/probe": (302, [("Location", "/owner/repo/releases/tag/v1.2.3")], b"")}) as base:
            result = subprocess.run([executable, "--no-config", "--no-hsts", "--no-netrc", "--no-proxy",
                "--debug", "--server-response", "--spider", "--max-redirect=0", "--tries=1", "-T", "5", base + "/probe"],
                env=environment, capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 8)
        raw = result.stderr
        self.assertIn(b"---response begin---\n", raw)
        start, end = b"---response begin---\n", b"---response end---\n"
        cases = {
            "valid": raw,
            "nested-start": raw.replace(start, start + start, 1),
            "missing-end": raw.replace(end, b"", 1),
            "stray-end": end + raw,
            "empty-block": raw + start + b"\n" + end,
            "repeated-final": raw + start + b"HTTP/1.1 302 Found\r\nLocation: /other\r\n\r\n" + end,
        }
        for name, value in cases.items():
            with self.subTest(case=name):
                headers = self.directory / "headers"
                headers.write_bytes(value)
                result = subprocess.run(["/bin/sh", "-s", "--", str(headers)],
                    input=self.helpers + '\nkhor_header_summary "$1" wget\n', text=True, capture_output=True)
                self.assertEqual(result.returncode == 0, name == "valid")

    def test_github_rejects_ambiguous_or_untrusted_metadata_before_placement(self):
        responses = [
            (302, [("Location", "/owner/repo/releases/tag/v1.2.3"), ("Location", "/owner/repo/releases/tag/v1.2.3")], b""),
            (302, [("Location", "https://evil.test/owner/repo/releases/tag/v1.2.3")], b""),
            (302, [("Location", "https://github.com/other/repo/releases/tag/v1.2.3")], b""),
            (302, [("Location", "http://github.com/owner/repo/releases/tag/v1.2.3")], b""),
            (302, [("Location", "/owner/repo/releases/tag/v1.2.3%2fother")], b""),
            (302, [("Location", "/owner/repo/releases/tag/v1.2.3?latest")], b""),
            (302, [("Location", "/owner/repo/releases/latest")], b""),
            (302, [("Location", "/owner/repo/releases/tag/v1.2.3"), ("X-Long", "a" * 9000)], b""),
            (302, [("Location", "/owner/repo/releases/tag/v1.2.3")] + [(f"X-Flood-{i}", "a" * 7000) for i in range(11)], b""),
            ("raw", b"HTTP/1.1 302 Found\r\nLocation: /owner/repo/releases/tag/v1.2.3\r\n", b""),
            ("raw", b"HTTP/1.1 302 Found\r\nLocation: /owner/repo/releases/tag/v1.2.3\r\n---response begin---\r\n\r\n", b""),
            ("raw", b"HTTP/1.1 302 Found\r\nLocation: /owner/repo/releases/tag/v1.2.3\r\n---response end---\r\n\r\n", b""),
            (503, [], b""),
        ]
        for tool in ["curl", "wget"]:
            for response in responses:
                with self.subTest(tool=tool, response=response[:2]):
                    self.binary.write_bytes(b"original binary")
                    contents = self.github_contents()
                    contents["/owner/repo/releases/latest"] = response
                    result, requests, logical = self.run_github(tool, contents)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(self.binary.read_bytes(), b"original binary")
                    self.assertFalse(any("/download/" in path for path in requests))
                    self.assertTrue(all(url.startswith("https://github.com/owner/repo/") for url in logical))
                    self.assert_no_stage()


def run_suite():
    return 0 if unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(InstallComponentsTest)).wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(run_suite())
