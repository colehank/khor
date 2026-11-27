"""Exercise detached terminals from a staged or standalone CLI in an isolated home."""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile


def check(cli, sidecar, acp):
    with tempfile.TemporaryDirectory(prefix="khor-session-server-", ignore_cleanup_errors=os.name == "nt") as tmp:
        scratch = Path(tmp).resolve()
        programs = scratch / "bin"
        programs.mkdir()
        program = programs / cli.name
        shutil.copy2(cli, program)
        if sidecar is not None:
            shutil.copy2(sidecar, programs / sidecar.name)
        home = scratch / "home"
        (home / ".khor").mkdir(parents=True)
        (home / ".khor/relays.json").write_text('{"relays":[],"only_mine":true}', encoding="utf-8")
        runtime = scratch / "runtime"
        if os.name != "nt":
            runtime.mkdir(mode=0o700)
            (runtime / "owner").write_bytes(b"khor-runtime-v1\n")
            with (runtime / "fixture-legacy-cache").open("wb") as fixture:
                fixture.truncate(4 * 1024**3)
        environment = dict(os.environ, KHOR_HOME=str(home), KHOR_RUNTIME_DIR=str(runtime),
                           KHOR_NO_MDNS="1", KHOR_NAME="session-server-check")
        for key in ("KHOR_RUNTIME_INSTALL_SOURCE", "KHOR_INSTALL_SOURCE", "KHOR_SHIELD_FROM",
                    "KHOR_SESSION", "KHOR_SHELL_LOGIN", "ENV", "ZDOTDIR"):
            environment.pop(key, None)
        environment["KHOR_SHELL"] = (str(Path(os.environ["SystemRoot"]) / "System32/cmd.exe")
                                     if os.name == "nt" else "/bin/sh")
        sessions = []

        def invoke(*args, check=True):
            result = subprocess.run([str(program), *args], cwd=scratch, env=environment,
                                    capture_output=True, text=True, timeout=90)
            if check and result.returncode:
                raise RuntimeError(f"{args[0]} failed: {result.stderr}")
            return result.stdout.strip()

        try:
            for title in ("session-server one", "session-server two"):
                session = invoke("open", "-d", "--title", title)
                if not re.fullmatch(r"shell/[0-9a-f]+", session):
                    raise ValueError(f"Unexpected detached session ID: {session}")
                sessions.append(session)
            if acp is not None:
                session = invoke("open", "--gui", "--title", "session-server ACP", "--", str(acp))
                if not re.fullmatch(r"tui/[A-Za-z0-9._-]+", session):
                    raise ValueError(f"Unexpected ACP session ID: {session}")
                sessions.append(session)
            store = scratch / "runtime-sessions"
            catalog = json.loads((store / "index.json").read_text())
            generations = catalog["roles"]["session-host"]
            assert len(generations) == 1, generations
            assert len(list(store.glob("*/manifest.json"))) == 1
            name = "khor-server.exe" if os.name == "nt" else "khor-server"
            cached = (store / generations[0] / name).resolve(strict=True)
            if sys.platform == "darwin":
                subprocess.run(["codesign", "--verify", "--strict", str(cached)], check=True)
            for session in sessions:
                host = home / ".khor/sessions" / session.replace("/", "-") / "host.json"
                pid = json.loads(host.read_text())["host_pid"]
                if os.name == "nt":
                    exe = subprocess.check_output(["powershell.exe", "-NoProfile", "-Command",
                                                   f"(Get-Process -Id {int(pid)}).Path"], text=True).strip()
                elif sys.platform == "darwin":
                    exe = subprocess.check_output(["ps", "-ww", "-p", str(pid), "-o", "comm="],
                                                  text=True).strip()
                else:
                    exe = os.readlink(f"/proc/{pid}/exe")
                assert Path(exe).resolve(strict=True) == cached, (pid, exe, cached)
            print(f"{len(sessions)} live sessions share one {cached.stat().st_size:,}-byte khor-server image.")
        finally:
            for session in sessions:
                invoke("close", session, check=False)
            invoke("quit", check=False)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cli", type=Path)
    parser.add_argument("--sidecar", type=Path)
    parser.add_argument("--acp", type=Path)
    args = parser.parse_args()
    check(args.cli.resolve(strict=True), args.sidecar and args.sidecar.resolve(strict=True),
          args.acp and args.acp.resolve(strict=True))
