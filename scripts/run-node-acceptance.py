#!/usr/bin/env python3
"""Run process-global Node acceptance tests in separate native processes.

Prepare and verify the build with native_provenance.py, then pass its receipt
with --provenance. See scripts/NATIVE-PROVENANCE.md. Cargo still runs in the
caller's existing guarded build phase; this entry only runs proven executables.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time

from native_provenance import key, read_messages, require_provenance


SUITES = {
    "runtime-close": ("khor_agent::runtime_close_tests::", 6),
    "doc-transfer": ("link::documents::flow_tests::", 5),
}
TIMEOUT_SECONDS = 300
MIN_FREE_BYTES = 5 * 1024 * 1024 * 1024


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def source_frame(root):
    names = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=root,
    ).decode().split("\0")
    return {name: digest(root / name) for name in names if name and (root / name).is_file()}


def group_exists(pgid):
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False


def check_space(*paths):
    for path in paths:
        usage = os.statvfs(path)
        if usage.f_bavail * usage.f_frsize < MIN_FREE_BYTES:
            raise RuntimeError(f"Disk below acceptance floor: {path}")


def processes():
    raw = subprocess.check_output(
        ["ps", "-ww", "-axo", "pid=,ppid=,pgid=,lstart=,args="], text=True, timeout=5)
    rows = {}
    for line in raw.splitlines():
        values = line.split(None, 8)
        if len(values) == 9:
            pid, parent, group = map(int, values[:3])
            rows[pid] = {"pid": pid, "parent": parent, "group": group,
                         "start": " ".join(values[3:8]),
                         "executable": values[8].split(None, 1)[0]}
    return rows


def observe(rows, parent, owned):
    selected = {parent}
    selected.update(pid for pid, old in owned.items()
                    if pid in rows and rows[pid]["start"] == old["start"])
    while True:
        children = {pid for pid, row in rows.items() if row["parent"] in selected}
        if children <= selected:
            break
        selected.update(children)
    for pid in selected & rows.keys():
        owned.setdefault(pid, rows[pid])
    if len(owned) > 512:
        raise RuntimeError("Native process ownership inventory exceeded its bound")


def remaining(rows, owned):
    groups = {pid: row for pid, row in owned.items() if pid == row["group"]}
    return [row for pid, row in rows.items()
            if (pid in owned and row["start"] == owned[pid]["start"])
            or row["group"] in groups]


def stop_observed_groups(process, owned):
    # A Worker can setsid after it is spawned. Once observed, its group remains
    # ours even after its leader exits; inspect all members, not just the leader.
    groups = {pid: row for pid, row in owned.items() if pid == row["group"]}
    for sig in (signal.SIGTERM, signal.SIGKILL):
        rows = processes()
        for group, original in groups.items():
            leader = rows.get(group)
            if leader and leader["start"] != original["start"]:
                continue  # Ambiguous reused identity: report, never signal it.
            if any(row["group"] == group for row in rows.values()):
                try:
                    os.killpg(group, sig)
                except ProcessLookupError:
                    pass
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            process.poll()
            if not remaining(processes(), owned):
                return
            time.sleep(0.05)


def run(binary, arguments, log_path, root):
    check_space(root, binary.parent)
    private = {str(binary)}
    worker = os.environ.get("KHOR_TEST_MCP_EXECUTABLE")
    if worker:
        private.add(str(Path(worker).resolve()))
    if any(row["executable"] in private for row in processes().values()):
        raise RuntimeError("A private test executable already has a live owner; preserve it")
    with log_path.open("wb") as log:
        process = subprocess.Popen(
            [str(binary), *arguments], cwd=root, stdout=log,
            stderr=subprocess.STDOUT, start_new_session=True,
        )
        timed_out = False
        interruption = None
        owned = {}
        began = time.monotonic()
        try:
            while process.poll() is None:
                observe(processes(), process.pid, owned)
                if time.monotonic() - began > TIMEOUT_SECONDS:
                    raise subprocess.TimeoutExpired(process.args, TIMEOUT_SECONDS)
                check_space(root, binary.parent)
                time.sleep(0.025)
        except BaseException as error:
            timed_out = isinstance(error, subprocess.TimeoutExpired)
            interruption = repr(error)
        observe(processes(), process.pid, owned)
        forced = group_exists(process.pid) or bool(remaining(processes(), owned))
        if forced:
            stop_observed_groups(process, owned)
            # The very first ancestry sample can fail. Popen still owns this
            # initial session; it does not authorize signaling unseen groups.
            try:
                if group_exists(process.pid):
                    os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                pass
            deadline = time.monotonic() + 5
            while group_exists(process.pid) and time.monotonic() < deadline:
                time.sleep(0.05)
        result = {
            "pid": process.pid, "exit": process.returncode,
            "timed_out": timed_out, "forced_cleanup": forced,
            "interruption": interruption,
            "group_absent": not group_exists(process.pid),
            "observed_owners": list(owned.values()),
            "remaining_owners": remaining(processes(), owned),
            "private_executable_footprint": [row for row in processes().values()
                                             if row["executable"] in private],
            "log": str(log_path), "log_sha256": digest(log_path),
        }
    if (interruption or forced or process.returncode != 0 or not result["group_absent"]
            or result["remaining_owners"] or result["private_executable_footprint"]):
        raise RuntimeError(f"Native process failed: {result}")
    return result, log_path.read_text(errors="replace")


def select_binary(artifacts, root):
    matches = []
    for line in artifacts.read_text().splitlines():
        try:
            item = json.loads(line)
        except ValueError:
            continue
        if (item.get("reason") == "compiler-artifact"
                and item.get("target", {}).get("name") == "khor_node"
                and item.get("profile", {}).get("test") and item.get("executable")):
            matches.append(item)
    if len(matches) != 1:
        raise ValueError("Expected exactly one actual khor-node libtest artifact")
    artifact = matches[0]
    if (artifact["manifest_path"] != str(root / "crates/node/Cargo.toml")
            or artifact["target"]["src_path"] != str(root / "crates/node/src/lib.rs")):
        raise ValueError("Artifact belongs to a different source tree")
    binary = Path(artifact["executable"])
    if binary.is_symlink() or not binary.is_file():
        raise ValueError("Actual native executable is absent or a symlink")
    return binary, artifact


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--provenance", type=Path, required=True)
    parser.add_argument("--worker-provenance", type=Path)
    parser.add_argument("--suite", choices=SUITES, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if os.name != "posix":
        parser.error("These native FIFO/process acceptance suites require Unix")
    root = Path(__file__).resolve().parent.parent
    output = args.output.resolve()
    if output.is_relative_to(root):
        parser.error("Keep acceptance output outside the source tree")
    output.mkdir(parents=True, exist_ok=False)
    receipt = {"suite": args.suite, "passes": [], "error": None}
    before = source_frame(root)
    (output / "source-before.json").write_text(json.dumps(before, indent=2) + "\n")
    try:
        binary, artifact = select_binary(args.artifacts, root)
        provenance = json.loads(args.provenance.read_text())
        require_provenance(provenance, root, artifact)
        if provenance["messages_sha256"] != key(read_messages(args.artifacts)):
            raise ValueError("Cargo artifact log differs from the proven build")
        worker_provenance = None
        if worker := os.environ.get("KHOR_TEST_MCP_EXECUTABLE"):
            if args.worker_provenance is None:
                raise ValueError("The real MCP worker requires --worker-provenance")
            worker_provenance = json.loads(args.worker_provenance.read_text())
            worker_row = worker_provenance["selected"]["row"]
            if (worker_row["executable"] != str(Path(worker).resolve())
                    or worker_row["profile"]["test"] or worker_row["target"]["kind"] != ["bin"]):
                raise ValueError("MCP worker is not the proven normal executable")
            require_provenance(worker_provenance, root, worker_row)
        binary_sha = digest(binary)
        receipt.update(artifact=artifact, binary_sha256=binary_sha,
                       artifacts_sha256=digest(args.artifacts),
                       provenance_sha256=digest(args.provenance),
                       worker_provenance_sha256=digest(args.worker_provenance) if worker_provenance else None)
        prefix, count = SUITES[args.suite]
        inventories = []
        for name, flags in (("all", []), ("ignored", ["--ignored"])):
            _, raw = run(binary, ["--list", "--format", "terse", *flags],
                         output / f"list-{name}.log", root)
            inventories.append(sorted(test for test in re.findall(r"^(.+): test$", raw, re.M)
                                      if test.startswith(prefix)))
        tests, ignored = inventories
        if len(tests) != count or tests != ignored:
            raise ValueError(f"Expected {count} isolated ignored tests; all={tests}, ignored={ignored}")
        receipt["selected_tests"] = tests
        for index, test in enumerate(tests, 1):
            result, raw = run(binary, ["--ignored", "--exact", test, "--nocapture"],
                              output / f"case-{index:02}.log", root)
            summaries = re.findall(
                r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", raw)
            if summaries != [("1", "0", "0")] or "skipped" in raw.lower():
                raise RuntimeError(f"{test}: expected one executed passing test")
            if digest(binary) != binary_sha or source_frame(root) != before:
                raise RuntimeError("Source or actual binary changed during acceptance")
            receipt["passes"].append({"test": test, **result})
            print(f"PASS {test}", flush=True)
        require_provenance(provenance, root, artifact)
        if worker_provenance:
            require_provenance(worker_provenance, root, worker_row)
    except Exception as error:
        receipt["error"] = str(error)
    finally:
        after = source_frame(root)
        (output / "source-after.json").write_text(json.dumps(after, indent=2) + "\n")
        if before != after:
            receipt["error"] = "Source changed during acceptance"
        (output / "result.json").write_text(json.dumps(receipt, indent=2) + "\n")
    if receipt["error"]:
        print(receipt["error"], file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
