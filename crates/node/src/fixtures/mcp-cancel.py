"""Local private-pipe fixture; Rust still performs the actual MCP dispatch."""

import json
import os
from pathlib import Path
import signal
import sys


signal.signal(signal.SIGALRM, signal.SIG_DFL)
signal.alarm(30)
directory = Path(__file__).parent


def emit(value):
    sys.stdout.write(json.dumps(value) + "\n")
    sys.stdout.flush()


def request(identifier, method, params):
    emit({"type": "mcp", "message": {
        "jsonrpc": "2.0", "id": identifier, "method": method, "params": params,
    }})


start = json.loads(sys.stdin.readline())
assert start["type"] == "start" and start["protocol"] == 1
(directory / "sidecar.pid").write_text(str(os.getpid()), encoding="ascii")
if (directory / "sidecar.mode").read_text(encoding="ascii") == "finish":
    emit({"type": "done"})
    sys.exit(0)

request(1, "initialize", {"protocolVersion": "2024-11-05", "capabilities": {},
    "clientInfo": {"name": "owned-mcp-fixture", "version": "1"}})
initialized = json.loads(sys.stdin.readline())
assert initialized["type"] == "mcp" and initialized["message"]["id"] == 1
assert "result" in initialized["message"]
emit({"type": "mcp", "message": {
    "jsonrpc": "2.0", "method": "notifications/initialized",
}})
request(2, "tools/list", {})
listed = json.loads(sys.stdin.readline())
assert listed["type"] == "mcp" and listed["message"]["id"] == 2
assert any(tool["name"] == "task_status" for tool in listed["message"]["result"]["tools"])
request(3, "tools/call", {"name": "task_status", "arguments": {
    "agent": "fixture", "task": "owned-mcp-task",
}})

for line in sys.stdin:
    value = json.loads(line)
    if value.get("type") == "abort":
        (directory / "sidecar.abort").write_text("received", encoding="ascii")
        emit({"type": "mcp", "message": {
            "jsonrpc": "2.0", "method": "notifications/cancelled",
            "params": {"requestId": 3},
        }})
        emit({"type": "done", "cancelled": True})
        sys.exit(0)
