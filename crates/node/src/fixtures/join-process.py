"""Owned, local-only replacement for ssh in Q045 subprocess regressions."""

import os
import signal
import sys


def write_all(fd, data):
    view = memoryview(data)
    while view:
        view = view[os.write(fd, view):]


# A failed parent assertion must not leave a fixture waiting forever. No mode
# starts descendants or connects to a host. The Rust parent has a second bound.
signal.signal(signal.SIGALRM, signal.SIG_DFL)
signal.setitimer(signal.ITIMER_REAL, 3.0)
with open(os.environ["Q045_FIXTURE_PID"], "x", encoding="ascii") as receipt:
    receipt.write(str(os.getpid()))

mode = os.environ["Q045_MODE"]
if mode in ("blocked-input", "cancel", "existing-master"):
    while True:
        signal.pause()
elif mode == "dual-output":
    for _ in range(16):
        write_all(1, b"x" * 8192)
        write_all(2, b"y" * 8192)
    sys.stdin.buffer.read()
elif mode == "normal":
    data = sys.stdin.buffer.read()
    write_all(1, ("count=" + str(len(data)) + "\n").encode("ascii"))
    write_all(2, b"fixture diagnostic\n")
elif mode == "stdout-limit":
    for _ in range(129):
        write_all(1, b"x" * 8192)
elif mode == "stderr-tail":
    for _ in range(40):
        write_all(2, b"y" * 8192)
    write_all(2, b"LAST-DIAGNOSTIC\n")
    write_all(1, b"complete\n")
    sys.stdin.buffer.read()
elif mode == "stderr-flood":
    while True:
        write_all(2, b"y" * 8192)
elif mode in ("early-refusal", "early-success"):
    os.close(0)
    write_all(2, b"Permission denied (owned fixture)\n")
    sys.exit(7 if mode == "early-refusal" else 0)
elif mode == "closed-io":
    import time
    for fd in (0, 1, 2):
        os.close(fd)
    time.sleep(0.35)
else:
    raise RuntimeError("unknown owned fixture mode")
