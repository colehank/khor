"""Owned test adapter: delay this fixture's real metadata Persist startup only."""
import json
import os
import sys
import time


def main():
    if len(sys.argv) != 6 or sys.argv[5] != "_mcp_worker":
        raise ValueError("invalid adapter arguments")
    executable, selected_root, events, delay = sys.argv[1:5]
    delay_ms = int(delay)
    if delay_ms not in (0, 2000) or not os.path.isabs(executable):
        raise ValueError("invalid controlled startup")
    payload = sys.stdin.buffer.read(4097)
    if len(payload) > 4096:
        raise ValueError("fixture request exceeds adapter budget")
    request = json.loads(payload)
    operation = request.get("operation", {})
    metadata = operation.get("operation", {})
    selected = (
        os.path.realpath(request.get("root", "")) == os.path.realpath(selected_root)
        and operation.get("kind") == "cancellation_metadata"
        and isinstance(metadata, dict)
        and isinstance(metadata.get("Persist"), dict)
    )

    def event(phase):
        data = json.dumps({
            "phase": phase,
            "pid": os.getpid(),
            "monotonic_ns": time.monotonic_ns(),
            "delay_ms": delay_ms,
        }).encode() + b"\n"
        fd = os.open(events, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o600)
        try:
            if os.fstat(fd).st_size + len(data) > 2048:
                raise ValueError("fixture event budget exceeded")
            if os.write(fd, data) != len(data):
                raise ValueError("incomplete fixture event")
        finally:
            os.close(fd)

    if selected:
        event("gate_enter")
        time.sleep(delay_ms / 1000)
        event("pre_exec")
    # The fixture request is at most 4 KiB. Restore its exact bytes to an empty
    # pipe, then exec: the real CLI retains this owned Session's PID and PGID.
    read_fd, write_fd = os.pipe()
    written = 0
    while written < len(payload):
        written += os.write(write_fd, payload[written:])
    os.close(write_fd)
    os.dup2(read_fd, 0)
    os.close(read_fd)
    os.execv(executable, [executable, "_mcp_worker"])


try:
    main()
except BaseException:
    # Never print the request, an exception containing it, or a protocol reply.
    sys.stderr.write("metadata startup adapter failed\n")
    sys.exit(1)
