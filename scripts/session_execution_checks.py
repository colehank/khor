#!/usr/bin/env python3
"""Run the same execution lifecycle contracts on each native desktop platform."""
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import time


def main():
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is not None:
            reconfigure(encoding="utf-8", errors="replace")
    output = Path(os.environ.get("SESSION_EXECUTION_EVIDENCE", "session-execution-evidence"))
    output.mkdir(parents=True, exist_ok=True)
    source = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    filters = ["command_task", "user_environment", "shell_start::", "integration::", "answers::", "host::ring_file_tests", "host::handshake_tests", "once::shell_claim_tests", "live::tests::startup_failure", "host::startup_failure_tests", "platform_path_tests", "mcp::tests", "link::execution::tests", "link::legacy_shell_tests", "runtime_image::compatibility_tests"]
    if sys.platform == "win32":
        # This bounded login probe is Unix-only; Windows uses its native
        # inherited environment, covered by the path and real worker checks.
        filters.remove("user_environment")
        filters.extend(["live::tests::windows_close", "win_process::tests",
                        "commands::tests::history_is_searched_by_words_directory_and_failure"])
    # Keep native transcript and file-lock coverage on the same desktop matrix.
    filters.extend(["adaptor::codex", "host::agent_draft::tests"])
    commands = [
        ["cargo", "test", "--locked", "-p", "khor-node", "--lib", name,
         "--", "--nocapture", "--test-threads=1"]
        for name in filters
    ]
    commands.append(["cargo", "test", "--locked", "-p", "khor-node", "--test", "execution_wire", "--", "--nocapture", "--test-threads=1"])
    commands.append(["cargo", "test", "--locked", "--no-fail-fast", "-p", "khor-cli", "--test", "command_tasks",
                     "--test", "mcp_terminals", "--test", "run", "--test", "open",
                     "--", "--nocapture", "--test-threads=1"])
    report = {"source": source, "platform": platform.platform(), "checks": []}
    code = 0
    try:
        for index, command in enumerate(commands):
            started = time.monotonic()
            log = output / f"{index:02d}.log"
            print("Running " + " ".join(command), flush=True)
            with log.open("wb") as stream:
                completed = subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT, check=False)
            report["checks"].append({"command": command, "exit": completed.returncode,
                                      "seconds": time.monotonic() - started, "log": log.name})
            if completed.returncode:
                print(log.read_text(encoding="utf-8", errors="replace")[-24000:])
                # These contracts run in separate processes. Retain the first
                # failure while still collecting worker, MCP and CLI evidence.
                if not code:
                    code = completed.returncode
    finally:
        (output / "result.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    return code


if __name__ == "__main__":
    sys.exit(main())
