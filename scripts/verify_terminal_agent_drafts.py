import base64
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

binary = Path(sys.argv[1]).resolve()
evidence = Path(__file__).resolve().parent
result = []


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def until(description, check, timeout=20):
    deadline = time.monotonic() + timeout
    last_error = None
    while time.monotonic() < deadline:
        try:
            value = check()
            if value:
                return value
        except (urllib.error.URLError, FileNotFoundError) as error:
            last_error = type(error).__name__
        time.sleep(0.2)
    raise AssertionError(f"{description} timed out ({last_error or 'condition false'})")


class App:
    def __init__(self, root, relay_port, name):
        self.root = root
        self.port = free_port()
        self.work = root / "work"
        self.process = None
        self.face_at = 0.0
        for folder in (self.work, root / ".khor", root / "tmux", root / ".claude",
                       root / ".codex", root / "shell"):
            folder.mkdir(parents=True, mode=0o700)
        (root / ".khor" / "relays.json").write_text(json.dumps(dict(
            only_mine=True, relays=[dict(url=f"http://127.0.0.1:{relay_port}")])))
        self.env = {k:v for k,v in os.environ.items()
                    if not k.startswith(("KHOR_", "CLAUDE_", "CODEX_", "ANTHROPIC_", "OPENAI_"))
                    and k not in ("TMUX", "TMUX_PANE", "ENV", "BASH_ENV", "NO_COLOR")}
        self.env.update({
            "KHOR_HOME":str(root), "KHOR_NAME":name, "KHOR_RUNTIME_DIR":str(root / "runtime"),
            "KHOR_RUNTIME_INSTALL_SOURCE":str(binary), "KHOR_WEB_PORT":str(self.port),
            "KHOR_NO_MDNS":"1", "KHOR_NETLAB":"1", "KHOR_BIND_IP":"127.0.0.1",
            "KHOR_SHELL":"/bin/zsh", "KHOR_SHELL_LOGIN":"never", "KHOR_PATH":os.environ["PATH"],
            "TMUX_TMPDIR":str(root / "tmux"), "KHOR_TMUX_SOCKET":name, "ZDOTDIR":str(root / "shell"),
            "TERM":"xterm-256color", "COLORTERM":"truecolor", "LANG":"en_US.UTF-8", "LC_ALL":"en_US.UTF-8",
            "OPENAI_API_KEY":"synthetic-test-key", "OPENAI_BASE_URL":"http://127.0.0.1:9/v1",
            "ANTHROPIC_API_KEY":"synthetic-test-key", "ANTHROPIC_BASE_URL":"http://127.0.0.1:9",
            "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC":"1",
        })
        (root / ".codex" / "config.toml").write_text(
            'model_provider = "offline-test"\nmodel = "gpt-5.4"\n'
            '[model_providers.offline-test]\nname = "Offline test"\n'
            'base_url = "http://127.0.0.1:9/v1"\nwire_api = "responses"\nenv_key = "OPENAI_API_KEY"\n'
            f'[projects."{self.work}"]\ntrust_level = "trusted"\n'
        )
        (root / ".claude" / ".claude.json").write_text(json.dumps({
            "hasCompletedOnboarding":True, "theme":"dark",
            "projects":{str(self.work.resolve()):{"hasTrustDialogAccepted":True,"hasCompletedProjectOnboarding":True}}
        }))

    def run(self, *args, input=None, check=True, timeout=35):
        job = subprocess.run([binary, *args], cwd=self.work, env=self.env, input=input,
                             text=True, capture_output=True, timeout=timeout)
        if check and job.returncode:
            raise RuntimeError(f"khor {args[0]} exited {job.returncode}: {job.stderr[-400:]}")
        return job.stdout

    def start(self):
        with (self.root / "serve.log").open("wb") as log:
            self.process = subprocess.Popen([binary, "serve"], cwd=self.work, env=self.env,
                                            stdout=log, stderr=log)
        until("web server", lambda: isinstance(self.api("sessions", by="recent"), list), timeout=35)

    def stop(self):
        if self.process is None:
            return
        try:
            self.run("quit", check=False, timeout=10)
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            if self.process.poll() is None:
                self.process.terminate()
                self.process.wait(timeout=5)

    def api(self, operation, **args):
        key = (self.root / ".khor" / "web.key").read_text().strip()
        req = urllib.request.Request(f"http://127.0.0.1:{self.port}/api/{operation}",
                                     data=json.dumps(args).encode(),
                                     headers={"Authorization": "Bearer " + key,
                                              "Content-Type":"application/json"})
        try:
            with urllib.request.urlopen(req, timeout=12) as stream:
                return json.load(stream)
        except urllib.error.HTTPError as error:
            raise RuntimeError(f"{operation} HTTP {error.code}: {error.read().decode()[-450:]}") from None

    def row(self, title):
        if time.monotonic() - self.face_at >= 5:
            self.api("face_here", visible=True)
            self.face_at = time.monotonic()
        return next((r for r in self.api("sessions", by="recent") if r["title"] == title), None)


class View:
    def __init__(self, app, sid, cols):
        self.app = app
        self.sid = sid
        self.lease = app.api("attachment_begin", id=sid, kind="terminal")
        assert self.call("term_open", cols=cols, rows=30, history=40) is None
        initial = self.call("term_attach")
        self.seq = initial["seq"]
        self.output = base64.b64decode(initial["bytes"])
        self.call("term_ready")

    def call(self, method, **args):
        return self.app.api(method, id=self.sid, lease=self.lease, **args)

    def type(self, keys):
        self.call("term_key", keys=keys)

    def read(self):
        data = self.call("term_stream", since=self.seq)
        self.seq = data["seq"]
        self.output += base64.b64decode(data["bytes"])
        return self.output


def check_agent(app, peer, name, args, direct, cols):
    program = shutil.which(name)
    assert program, f"installed {name} not found"
    title = f"Draft lab {name} {cols}"
    mode = ["--tui"] if direct else []
    for attempt in range(3):
        try:
            sid = app.run("open", "-d", "--title", title, *mode, "--", program, *args).strip().splitlines()[-1]
            break
        except RuntimeError as error:
            if "session row is busy" not in str(error) or attempt == 2:
                raise
            row = app.row(title)
            if row and row.get("attachable"):
                sid = row["id"]
                break
            time.sleep(0.6)
    try:
        view = View(app, sid, cols)
        claude_prompt = "❯\u00a0".encode()
        if name == "claude":
            until("Claude startup screen", lambda: claude_prompt in view.read()
                  or b"Quick" in view.output or b"synthetic-test-key" in view.output)
            if b"Quick" in view.output and claude_prompt not in view.output:
                assert str(app.work.resolve()).encode() in view.output
                assert b"Security" in view.output and b"folder" in view.output
                view.type("\x1b[B")
                time.sleep(0.4)
                view.type("\r")
            until("Claude editor or synthetic API key dialog",
                  lambda: claude_prompt in view.read() or b"synthetic-test-key" in view.output)
            if b"synthetic-test-key" in view.output and claude_prompt not in view.output:
                assert b"recommended" in view.output
                view.type("\x1b[A")
                time.sleep(0.4)
                view.type("\r")
        marker = ("›" if name == "codex" else "❯").encode()
        composer = b"gpt-5.4 default" if name == "codex" else claude_prompt
        until(f"{name} editor", lambda: marker in view.read() and composer in view.output, timeout=35)
        initial = until("host row", lambda: app.row(title))
        assert not initial.get("draft"), "empty editor must have no preview"
        text = f"{name} draft verification"
        baseline = "--baseline" in sys.argv
        snapshots = evidence / "pty-replay"
        if baseline:
            snapshots.mkdir(exist_ok=True)
            (snapshots / f"{name}-initial.bin").write_bytes(view.read())
        view.type(text + "X")
        if baseline:
            until(f"{name} renders typed characters", lambda: b"verification" in view.read(), timeout=12)
            (snapshots / f"{name}-type.bin").write_bytes(view.read())
            view.type("\x7f")
            time.sleep(0.8)
            (snapshots / f"{name}-backspace.bin").write_bytes(view.read())
            view.type("\x1b[200~ 中文\x1b[201~")
            time.sleep(0.8)
            (snapshots / f"{name}-paste.bin").write_bytes(view.read())
            view.type("\x15")
            time.sleep(0.8)
            (snapshots / f"{name}-clear.bin").write_bytes(view.read())
            row = app.row(title)
            print(f"{name} input replay collected; baseline draft={row.get('draft')!r}", flush=True)
            return
        wait_draft(app, title, text + "X")
        if peer is not None:
            wait_draft(peer, title, text + "X")
        view.type("\x7f")
        wait_draft(app, title, text)
        if peer is not None:
            wait_draft(peer, title, text)
        view.type("\x1b[200~ 中文\x1b[201~")
        wait_draft(app, title, text + " 中文")
        if peer is not None:
            wait_draft(peer, title, text + " 中文")
        view.type("\x15")
        wait_draft(app, title, None)
        if peer is not None:
            wait_draft(peer, title, None)
        result.append(dict(agent=name, columns=cols, direct=direct, peer=peer is not None,
                           passed=["empty", "typing", "backspace", "paste", "clear"]))
        print(json.dumps(result[-1]), flush=True)
    except Exception:
        if "view" in locals():
            view.read()
            (evidence / f"{name}-{cols}-terminal.bin").write_bytes(view.output)
        row = app.row(title) or {}
        print(json.dumps({field: row.get(field) for field in (
            "kind", "category", "draft", "doing", "word", "attachable")}), flush=True)
        for ledger in (app.root / ".khor").rglob("commands.json"):
            data = json.loads(ledger.read_text())
            print("scratch ledger", json.dumps(data.get("doing")), flush=True)
        raise
    finally:
        app.run("close", sid, check=False, timeout=12)


def wait_draft(app, title, expected):
    def match():
        row = app.row(title)
        if row is None:
            return False
        editor = (row.get("doing") or {}).get("editor")
        return row.get("draft") == expected and editor == expected
    return until(f"{title}, preview {expected!r}", match, timeout=22)


with tempfile.TemporaryDirectory(prefix="terminal-draft-lab-", ignore_cleanup_errors=True) as scratch:
    scratch = Path(scratch)
    relay_port = free_port()
    app = App(scratch / "host", relay_port, "terminal-draft-host")
    peer = App(scratch / "peer", relay_port, "terminal-draft-peer") if "--peer" in sys.argv else None
    with (scratch / "relay.log").open("wb") as log:
        relay = subprocess.Popen([binary, "relay", "--dev", "--bind", f"127.0.0.1:{relay_port}",
                                  "--state", str(scratch / "relay")], cwd=scratch, env=app.env,
                                 stdout=log, stderr=log)
    try:
        app.start()
        if peer is not None:
            ticket = app.run("invite")
            peer.run("pair", "-", input=ticket)
            peer.start()
        failure = None
        for name, options, direct, columns in [
            ("codex", ["--no-alt-screen"], False, 56),
            ("claude", [], True, 100),
        ]:
            try:
                check_agent(app, peer, name, options, direct, columns)
            except Exception as error:
                print(f"{name} verification failed: {error}", flush=True)
                failure = error
        if failure is not None:
            raise failure
    finally:
        if peer is not None:
            peer.stop()
        app.stop()
        if relay.poll() is None:
            relay.terminate()
            relay.wait(timeout=5)
        (evidence / "native-draft-checks.json").write_text(json.dumps(result, indent=2)+"\n")
