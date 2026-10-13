#!/usr/bin/env python3
"""Optional installed-Codex contract check, independent of the normal GUI gate.

python3 apps/gui/scripts/check-codex-paging.py [--codex /path/to/codex]

Resumes generated history in an isolated tmux server and CODEX_HOME. The model
provider is an unreachable localhost fixture, and no prompt is ever submitted.
Only this test pane receives keys; the user's config/sessions are untouched.
"""
import argparse
import datetime
import json
import os
import re
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import time
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--codex", default=shutil.which("codex"))
    parser.add_argument("--tmux", default=shutil.which("tmux"))
    parser.add_argument("--mouse", action="store_true", help="require native mouse capture and verify fine-grained scrolling")
    args = parser.parse_args()
    if not args.codex or not args.tmux:
        parser.error("this optional check needs installed codex and tmux")
    version = subprocess.check_output([args.codex, "--version"], text=True).strip()
    with tempfile.TemporaryDirectory(prefix="khor-codex-paging-") as temp:
        root = Path(temp)
        home = root / "codex"
        now = datetime.datetime.now(datetime.timezone.utc)
        sessions = home / "sessions" / now.strftime("%Y/%m/%d")
        sessions.mkdir(parents=True)
        session = str(uuid.uuid4())
        stamp = now.strftime("%Y-%m-%dT%H:%M:%SZ")
        rollout = sessions / f"rollout-{now.strftime('%Y-%m-%dT%H-%M-%S')}-{session}.jsonl"
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
                    "type": "user_message" if role == "user" else "agent_message",
                    "message": message,
                    **({"images": [], "local_images": []} if role == "user" else {"phase": "final_answer"}),
                }})
        rollout.write_text("".join(json.dumps(row) + "\n" for row in rows))
        (home / "config.toml").write_text(f'''model="fixture"
model_provider="fixture"
[tui]
screen_reader_detection_done=true
[model_providers.fixture]
name="fixture"
base_url="http://127.0.0.1:9/v1"
wire_api="responses"
env_key="KHOR_PAGING_FIXTURE_KEY"
[projects.{json.dumps(str(root))}]
trust_level="trusted"
''')
        # Some Unix sockaddr paths are limited to 104 bytes. mkdtemp's
        # path is short enough on both macOS and Linux.
        socket = str(root / "tmux.sock")
        env = os.environ.copy()
        for key in tuple(env):
            if key.startswith(("CODEX_", "KHOR_")) or key in ("ZDOTDIR", "FORCE_COLOR", "CLICOLOR_FORCE"):
                env.pop(key, None)
        env.update(HOME=str(root), USERPROFILE=str(root), CODEX_HOME=str(home), KHOR_PAGING_FIXTURE_KEY="fixture",
                   TERM="xterm-256color", COLORTERM="truecolor")
        for key in ("TMUX", "TMUX_PANE", "NO_COLOR", "OPENAI_API_KEY", "OPENAI_BASE_URL"):
            env.pop(key, None)

        def tmux(*argv):
            return subprocess.check_output([args.tmux, "-S", socket, *argv], env=env, text=True)

        def capture():
            return tmux("capture-pane", "-p", "-t", "probe")

        def keys(*argv):
            tmux("send-keys", "-t", "probe", *argv)

        def wait_screen(predicate, label):
            deadline = time.monotonic() + 15
            screen = ""
            while time.monotonic() < deadline:
                screen = capture()
                if predicate(screen):
                    return screen
                time.sleep(0.05)
            raise AssertionError(label + "\n" + screen)

        draft = "UNSENT_DRAFT_719"
        try:
            # Configure this isolated tmux connection before starting Codex, as
            # khor grouped attach does. Never wrap or rewrite the Codex command.
            config = root / "tmux.conf"
            config.write_text("set -g default-shell /bin/sh\n" + ("set -g mouse on\n" if args.mouse else ""))
            command = shlex.join([args.codex, "resume", session])
            tmux("-f", str(config), "new-session", "-d", "-s", "probe", "-x", "100", "-y", "30", "-c", str(root), command)
            wait_screen(lambda s: "ANSWER_060" in s, "fixture did not resume")
            if args.mouse:
                mode = tmux("display-message", "-p", "-t", "probe", "#{mouse} #{mouse_any_flag}").strip()
                assert mode == "1 1", f"native mouse scrolling unavailable: {mode}"
            # There is deliberately no Enter anywhere in this test.
            keys("-l", draft)
            bottom = wait_screen(lambda s: draft in s, "draft missing")
            if args.mouse:
                def markers(screen):
                    return {line.strip(): i for i, line in enumerate(screen.splitlines())
                            if re.search(r"(?:QUESTION|ANSWER)_\d{3}", line)}
                previous = markers(capture())
                # One native wheel notch at a time, as in the touch scroller.
                # Ignore the pinned question header when measuring motion.
                # Entry also adds a one-row follow control; later notches
                # must move exactly one row and never touch the draft.
                for _ in range(4):
                    keys("-l", "\x1b[<64;5;5M")
                    current = wait_screen(lambda s: markers(s) != previous, "wheel notch did not move transcript")
                    positions = markers(current)
                    shifts = {positions[k] - previous[k] for k in positions.keys() & previous.keys() if positions[k] > 0 and previous[k] > 0}
                    assert shifts == {1} or (_ == 0 and shifts == {2}), f"wheel must move one row (plus first follow-control row), got {shifts}:\n{current}"
                    assert draft in current
                    previous = positions
                keys("-l", "\x1b[<65;5;5M" * 4)
                wait_screen(lambda s: markers(s) == markers(bottom), "mouse reversal did not return to bottom")
            keys("PageUp")
            older = wait_screen(lambda s: "ANSWER_057" in s and "ANSWER_060" not in s, "PageUp did not scroll")
            assert draft in older
            keys("-N", "80", "PageUp")
            assert draft in wait_screen(lambda s: "QUESTION_001" in s, "top not reached")
            keys("-N", "5", "PageUp")
            time.sleep(0.15)
            assert "QUESTION_001" in capture() and draft in capture()
            keys("PageDown")
            assert draft in wait_screen(lambda s: "QUESTION_001" not in s, "direction did not reverse")
            keys("-N", "80", "PageDown")
            assert draft in wait_screen(lambda s: "ANSWER_060" in s, "bottom not reached")
            keys("-N", "5", "PageDown")
            time.sleep(0.15)
            assert draft in capture()
            print(f"PASS {version}: native mouse={args.mouse}, paging, reverse, top/bottom boundaries; unsent draft retained; no prompt submitted")
        finally:
            subprocess.run([args.codex, "app-server", "daemon", "stop"], env=env, capture_output=True, timeout=20)
            subprocess.run([args.tmux, "-S", socket, "kill-server"], env=env, capture_output=True)


if __name__ == "__main__":
    main()
