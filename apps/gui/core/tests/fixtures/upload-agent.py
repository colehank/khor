#!/usr/bin/env python3
"""Deterministic ACP peer; requests are durable evidence of actual prompts."""
import hashlib
import json
import os
import sys
from pathlib import Path

log = Path(sys.argv[1])
redacted = "--redacted" in sys.argv[2:]
mode = "careful"
model = "one"
prompt = None
cancelled = False


def emit(value):
    print(json.dumps(value), flush=True)


def result(request_id, value):
    emit({"jsonrpc": "2.0", "id": request_id, "result": value})


def options():
    return [{"id": "model", "name": "Model", "category": "model", "type": "select",
             "currentValue": model, "options": [{"value": value, "name": value} for value in ["one", "two"]]}]


for line in sys.stdin:
    request = json.loads(line)
    with log.open("a") as out:
        evidence = request
        if redacted:
            evidence = {key: request[key] for key in ["jsonrpc", "id", "method"] if key in request}
            evidence["pid"] = os.getpid()
            if request.get("method") == "session/prompt":
                evidence["content"] = [{"type": block.get("type"),
                    "json_bytes": len(json.dumps(block).encode()),
                    "sha256": hashlib.sha256(json.dumps(block, sort_keys=True).encode()).hexdigest()}
                    for block in request.get("params", {}).get("prompt", [])]
        out.write(json.dumps(evidence) + "\n")
        out.flush()
        os.fsync(out.fileno())
    method = request.get("method")
    request_id = request.get("id")
    if method == "initialize":
        result(request_id, {"protocolVersion": 1, "agentCapabilities": ({"promptCapabilities": {"image": True, "embeddedContext": True}} if redacted else {}),
                            "agentInfo": {"name": "upload-fixture", "version": "1"}})
    elif method == "session/new":
        result(request_id, {"sessionId": "upload-fixture", "modes": {"currentModeId": mode,
                            "availableModes": [{"id": value, "name": value} for value in ["careful", "bold"]]},
                            "configOptions": options()})
    elif method == "session/set_mode":
        mode = request["params"]["modeId"]
        result(request_id, {})
    elif method == "session/set_config_option":
        model = request["params"]["value"]
        result(request_id, {"configOptions": options()})
    elif method == "session/prompt":
        prompt = request_id
        cancelled = False
        emit({"jsonrpc": "2.0", "id": "permission", "method": "session/request_permission",
              "params": {"sessionId": "upload-fixture", "toolCall": {"toolCallId": "tool-1"},
                         "options": [{"optionId": "go", "name": "Allow", "kind": "allow_once"}]}})
    elif method == "session/cancel":
        cancelled = True
        if prompt is not None:
            result(prompt, {"stopReason": "cancelled"})
            prompt = None
    elif request_id == "permission" and prompt is not None:
        result(prompt, {"stopReason": "cancelled" if cancelled else "end_turn"})
        prompt = None
    elif request_id is not None:
        result(request_id, {})
