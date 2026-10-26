"""Cooperative Pi only; Rust owns real Prompt/Stop and cancellation IPC."""
import json
import os
from pathlib import Path
import signal
import sys

signal.signal(signal.SIGALRM, signal.SIG_DFL)
signal.alarm(25)
directory = Path(__file__).parent
start = json.loads(sys.stdin.readline())
assert start["type"] == "start" and start["protocol"] == 1
with (directory / "pi-pids").open("a", encoding="ascii") as log:
    log.write(str(os.getpid()) + "\n")
    log.flush()
(directory / "ready").write_text(str(os.getpid()), encoding="ascii")
for line in sys.stdin:
    if json.loads(line).get("type") == "abort":
        with (directory / "pi-aborts").open("a", encoding="ascii") as log:
            log.write(str(os.getpid()) + "\n")
        print(json.dumps({"type": "done", "cancelled": True}), flush=True)
        sys.exit(0)
