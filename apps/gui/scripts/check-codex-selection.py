#!/usr/bin/env python3
"""Optional real Codex selection check; no prompt is submitted to a model.

python3 apps/gui/scripts/check-codex-selection.py [--khor PATH] [--codex PATH]

Uses scratch HOME/CODEX_HOME/KHOR_HOME and a uniquely named tmux server. Resumes
fixture history, imports its tmux row, then runs /new and /resume. Checks the
same CLI row's shell fallback or Codex binding and the native title's active UUID.
The CLI does not expose message previews; preview assertions live in Rust tests.
No hooks are installed and no hook approval dialog is accepted.
"""
import argparse
import datetime
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import tempfile
import time
import uuid


UUID = re.compile(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b", re.I)
ANSI = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")


def uuid7():
    # Python before 3.14 has no uuid.uuid7. The adaptor intentionally accepts
    # v7 rollout identities, so a convenient uuid4 fixture would test nothing.
    bits = (int(time.time() * 1000) << 80) | (uuid.uuid4().int & ((1 << 80) - 1))
    bits = (bits & ~(0xF << 76)) | (7 << 76)
    bits = (bits & ~(3 << 62)) | (2 << 62)
    value = uuid.UUID(int=bits)
    assert value.version == 7
    return str(value)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--khor", default=str(Path(__file__).resolve().parents[3] / "target/debug/khor"))
    parser.add_argument("--native-title", action="store_true", help="explicitly opt the fixture into native title metadata")
    parser.add_argument("--codex", default=shutil.which("codex"))
    args = parser.parse_args()
    tmux_bin = shutil.which("tmux")
    if not args.codex or not tmux_bin:
        parser.error("this optional check needs installed codex and tmux")
    khor, codex = Path(args.khor).expanduser().absolute(), Path(args.codex).expanduser().absolute()
    for binary in (khor, codex):
        if not binary.is_file() or not os.access(binary, os.X_OK):
            parser.error(f"not an executable: {binary}")

    with tempfile.TemporaryDirectory(prefix="khor-codex-selection-") as temp:
        root = Path(temp)
        home = root / ".codex"
        now = datetime.datetime.now(datetime.timezone.utc)
        session = uuid7()
        stamp = now.strftime("%Y-%m-%dT%H:%M:%SZ")
        folder = home / "sessions" / now.strftime("%Y/%m/%d")
        folder.mkdir(parents=True)
        rollout = folder / f"rollout-{now.strftime('%Y-%m-%dT%H-%M-%S')}-{session}.jsonl"
        socket = "khor-selection-" + uuid.uuid4().hex
        name = "selection-fixture"
        env = os.environ.copy()
        for key in tuple(env):
            if key.startswith(("CODEX_", "KHOR_")) or key in (
                "TMUX", "TMUX_PANE", "NO_COLOR", "FORCE_COLOR", "CLICOLOR_FORCE",
                "OPENAI_API_KEY", "OPENAI_BASE_URL", "ZDOTDIR",
            ):
                env.pop(key, None)
        env.update(
            HOME=str(root), USERPROFILE=str(root),
            XDG_CONFIG_HOME=str(root / "config"), XDG_DATA_HOME=str(root / "data"),
            XDG_CACHE_HOME=str(root / "cache"), XDG_STATE_HOME=str(root / "state"),
            CODEX_HOME=str(home), KHOR_HOME=str(root), KHOR_VENDOR_HOME=str(root),
            KHOR_TMUX_SOCKET=socket,
            KHOR_SELECTION_FIXTURE_KEY="fixture", TERM="xterm-256color", COLORTERM="truecolor",
            PATH=env.get("PATH", ""),
        )
        version = subprocess.check_output([str(codex), "--version"], env=env, text=True, timeout=15).strip()
        rows = [{"timestamp": stamp, "type": "session_meta", "payload": {
            "id": session, "timestamp": stamp, "cwd": str(root),
            "originator": "codex_cli_rs", "cli_version": version.split()[-1],
            "source": "cli", "model_provider": "fixture",
            "base_instructions": {"text": "Test fixture. Do not call tools."},
        }}]
        for n in range(1, 61):
            for role, kind, message in [
                ("user", "input_text", f"QUESTION_{n:03d} fixture question"),
                ("assistant", "output_text", f"ANSWER_{n:03d} fixture response line alpha\nSecond line beta\nThird line gamma"),
            ]:
                rows.append({"timestamp": stamp, "type": "response_item", "payload": {
                    "type": "message", "role": role, "content": [{"type": kind, "text": message}],
                }})
                rows.append({"timestamp": stamp, "type": "event_msg", "payload": {
                    "type": "user_message" if role == "user" else "agent_message", "message": message,
                    **({"images": [], "local_images": []} if role == "user" else {"phase": "final_answer"}),
                }})
        rollout.write_text("".join(json.dumps(row) + "\n" for row in rows))
        # This project is only the disposable fixture directory. There are no
        # hooks or real credentials; the provider cannot make a model call.
        (home / "config.toml").write_text(f'''model="fixture"
model_provider="fixture"
[tui]
screen_reader_detection_done=true
[model_providers.fixture]
name="fixture"
base_url="http://127.0.0.1:9/v1"
wire_api="responses"
env_key="KHOR_SELECTION_FIXTURE_KEY"
[projects.{json.dumps(str(root))}]
trust_level="trusted"
''')

        def tmux(*argv):
            # Never omit -L: even cleanup must address only this test server.
            return subprocess.check_output([tmux_bin, "-L", socket, *argv], env=env, text=True, stderr=subprocess.STDOUT, timeout=15)

        def cli(*argv):
            return subprocess.check_output([str(khor), *argv], env=env, text=True, stderr=subprocess.STDOUT, timeout=20)

        def screen():
            result = tmux("capture-pane", "-p", "-t", name)
            assert "Hooks need review" not in result, "launcher unexpectedly requested hook trust"
            return result

        def native_id():
            title = tmux("display-message", "-p", "-t", name, "#{pane_title}").strip()
            if UUID.fullmatch(title):
                return title.lower()
            if len(title) == 32 and title.endswith("..."):
                prefix = title[:-3].lower()
                ids = [path.stem.lower() for path in (home / "thread-writer-locks").glob("*.lock")
                       if UUID.fullmatch(path.stem) and path.stem.lower().startswith(prefix)]
                if len(ids) == 1:
                    return ids[0]
            return None

        def wait_for(read, accepted, label):
            deadline = time.monotonic() + 30
            last = None
            while time.monotonic() < deadline:
                last = read()
                if accepted(last):
                    return last
                time.sleep(0.15)
            raise AssertionError(f"{label}\nLast observation: {last}\nScreen:\n{screen()}")

        def send_slash(command):
            # Only these local Codex commands receive Enter, never a prompt.
            assert command == "/new" or command == f"/resume {session}"
            tmux("send-keys", "-t", name, "-l", command)
            time.sleep(0.3)  # Let Codex parse the slash before its Enter arrives.
            tmux("send-keys", "-t", name, "Enter")

        def row_in_group(listing, wanted, category):
            group = ""
            matches = []
            for line in ANSI.sub("", listing).splitlines():
                fields = line.split("\t")
                if len(fields) == 1 and line.strip():
                    group = line.strip()
                elif fields[0] == wanted:
                    matches.append(group)
            # Check the exact imported row, not a separate archived Codex row.
            return len(matches) == 1 and re.search(r"\b" + category + r"\b", matches[0], re.I) is not None

        def expect_category(imported, phase, category=None):
            category = category or ("codex" if args.native_title else "shell")
            wait_for(lambda: cli("sessions", "--all", "--by", "category"),
                     lambda value: row_in_group(value, imported, category),
                     f"{phase}: imported row {imported} has the wrong terminal form")

        def expect_bound(imported, phase):
            def bound(listing):
                line = next((line for line in listing.splitlines() if line.startswith(imported + "\t")), "")
                return bool(line) and "识别受限" not in line and "Thread unavailable" not in line
            wait_for(lambda: cli("sessions", "--all"), bound,
                     f"{phase}: native selection became unavailable")

        try:
            if args.native_title:
                config = home / "config.toml"
                config.write_text(config.read_text().replace("[tui]", '[tui]\nterminal_title=["thread-id"]'))
            command = shlex.join([str(codex), "resume", session])
            tmux("-f", "/dev/null", "new-session", "-d", "-s", name, "-x", "100", "-y", "30", "-c", str(root), command)
            wait_for(screen, lambda value: "ANSWER_060" in value, "fixture did not resume")
            if args.native_title:
                wait_for(native_id, lambda value: value == session, "first resume did not publish its native UUID")

            def candidates():
                return [line.split("\t") for line in cli("import").splitlines() if len(line.split("\t")) == 7 and (args.native_title or line.split("\t")[-1].startswith("shell/"))]

            # Agent discovery can merge the tmux seat into a tui/UUID
            # candidate whose title is the fixture cwd rather than the tmux
            # name. This isolated vendor home/socket has exactly one seat.
            choices = wait_for(candidates, lambda value: len(value) == 1, "fixture import candidate not uniquely found")
            imported = cli("import", choices[0][-1]).strip()
            assert imported and "\n" not in imported and "\t" not in imported, f"unexpected import ID: {imported!r}"
            expect_category(imported, "initial resume")
            if args.native_title:
                expect_bound(imported, "initial resume")
                # Each CLI invocation starts a fresh reader. A later unrelated
                # config rewrite must preserve activation even across restarts.
                time.sleep(1.1)  # Process start clocks can have second precision.
                config.write_text(config.read_text().replace(
                    "[tui]", '[tui]\nstatus_line=["model-with-reasoning"]'))
                expect_category(imported, "status-line edit")
                expect_bound(imported, "status-line edit")
                # The running TUI retains its local title selection. Removing
                # the disk preference must not discard a verified client.
                config.write_text(config.read_text().replace('terminal_title=["thread-id"]\n', ""))
                expect_category(imported, "title preference removed")
                expect_bound(imported, "title preference removed")
                # /new reloads disk settings in real 0.162.1. Without the ID
                # title it must become a shell, not retain the previous thread.
                send_slash("/new")
                wait_for(screen, lambda value: "ANSWER_060" not in value, "disabled-title /new kept old content")
                expect_category(imported, "disabled-title /new", "shell")
                assert native_id() is None, "disabled-title /new invented a UUID title"
                # Re-enable and resume in this SAME process. Its verified client
                # evidence must survive, while thread selection is read afresh.
                time.sleep(1.1)
                config.write_text(config.read_text().replace("[tui]", '[tui]\nterminal_title=["thread-id"]'))
                send_slash(f"/resume {session}")
                wait_for(native_id, lambda value: value == session, "re-enabled /resume did not restore its UUID")
                expect_category(imported, "re-enabled /resume")
                expect_bound(imported, "re-enabled /resume")
            if not args.native_title:
                listing = cli("sessions", "--all")
                line = next(line for line in listing.splitlines() if line.startswith(imported + "\t"))
                assert "识别受限" not in line and "Thread unavailable" not in line, line

            send_slash("/new")
            new_id = None
            if args.native_title:
                new_id = wait_for(native_id, lambda value: value is not None and value != session, "/new retained the old native UUID")
                assert uuid.UUID(new_id).version == 7, f"/new supplied a non-v7 UUID: {new_id}"
            wait_for(screen, lambda value: "ANSWER_060" not in value, "/new retained the old transcript on screen")
            expect_category(imported, "/new")
            if args.native_title:
                expect_bound(imported, "/new after config edit")

            send_slash(f"/resume {session}")
            if args.native_title:
                wait_for(native_id, lambda value: value == session, "/resume did not restore the old native UUID")
            wait_for(screen, lambda value: "ANSWER_060" in value, "/resume did not restore the fixture transcript")
            expect_category(imported, "second resume")
            if args.native_title:
                expect_bound(imported, "second resume after config edit")
            print(f"PASS {version}: default shared daemon, same row {imported} stays {'Codex' if args.native_title else 'shell'}; native title opt-in={args.native_title}; no model prompt submitted")
            print("CLI exposes category/title, not preview; message-preview mapping is covered by the Rust selection tests.")
        finally:
            subprocess.run([str(codex), "app-server", "daemon", "stop"], env=env, capture_output=True, timeout=20)
            subprocess.run([tmux_bin, "-L", socket, "kill-server"], env=env, capture_output=True, timeout=15)


if __name__ == "__main__":
    main()
