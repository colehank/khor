"""Validate complete logical-request records; retain non-successes and incomplete attempts."""
import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path
import sys

MARKER = "[netlab-attempt] "
REVISION = "attempt_outcomes_v4"
OUTCOMES = {"ok", "error", "timeout", "cancelled", "aborted"}
CONTEXT = ("case", "from", "to", "operation", "scope", "within_ms")


def inspect(logs, calibration=False):
    records = []
    problems = []
    pairs = defaultdict(dict)
    decoder = json.JSONDecoder()
    for path in logs:
        with path.open(errors="replace") as source:
            for number, line in enumerate(source, 1):
                position = 0
                while (position := line.find(MARKER, position)) >= 0:
                    start = position + len(MARKER)
                    try:
                        event, consumed = decoder.raw_decode(line[start:])
                    except ValueError as error:
                        problems.append(f"{path}:{number}: malformed attempt: {error}")
                        break
                    position = start + consumed
                    record = dict(log=str(path), line=number, column=start - len(MARKER), event=event)
                    records.append(record)
                    if not isinstance(event, dict):
                        problems.append(f"{path}:{number}: attempt is not an object")
                        continue
                    integers = ("pid", "attempt", "at_ms", "within_ms", "elapsed_us")
                    valid = (
                        type(event.get("schema")) is int and event["schema"] == 1
                        and event.get("measurement_revision") == REVISION
                        and all(type(event.get(key)) is int and event[key] >= 0 for key in integers)
                        and event["pid"] > 0 and event["attempt"] > 0 and event["within_ms"] > 0
                        and all(isinstance(event.get(key), str) and event[key]
                                for key in ("case", "from", "to", "operation", "scope"))
                        and event["scope"] == "logical_request"
                    )
                    if not valid:
                        problems.append(f"{path}:{number}: invalid schema/context")
                        continue
                    phase = event.get("phase")
                    if phase not in {"started", "finished"}:
                        problems.append(f"{path}:{number}: unknown phase")
                        continue
                    key = (str(path), event["pid"], event["attempt"])
                    if phase in pairs[key]:
                        problems.append(f"{path}:{number}: duplicate {phase} for {key}")
                    else:
                        pairs[key][phase] = record
                    if phase == "started":
                        if event.get("outcome") is not None or event.get("error") is not None or event["elapsed_us"] != 0:
                            problems.append(f"{path}:{number}: start already has an outcome")
                    else:
                        outcome = event.get("outcome")
                        if outcome not in OUTCOMES:
                            problems.append(f"{path}:{number}: unknown outcome")
                        elif outcome == "ok" and event.get("error") is not None:
                            problems.append(f"{path}:{number}: success contains an error")
                        elif outcome != "ok" and not isinstance(event.get("error"), str):
                            problems.append(f"{path}:{number}: non-success lost its reason")
                        if outcome == "timeout" and event["elapsed_us"] < event["within_ms"] * 1000:
                            problems.append(f"{path}:{number}: timeout before its deadline")

    incomplete = []
    outcomes = Counter()
    by_case = defaultdict(lambda: dict(started=0, finished=0, outcomes=Counter(), elapsed_us=[]))
    for key, phases in pairs.items():
        begin = phases.get("started")
        end = phases.get("finished")
        if begin:
            by_case[begin["event"]["case"]]["started"] += 1
        if end:
            event = end["event"]
            group = by_case[event["case"]]
            group["finished"] += 1
            group["outcomes"][event.get("outcome")] += 1
            group["elapsed_us"].append(event["elapsed_us"])
            outcomes[event.get("outcome")] += 1
        if not begin or not end:
            incomplete.append(dict(key=key, phases=phases))
        elif any(begin["event"][field] != end["event"][field] for field in CONTEXT):
            problems.append(f"{key}: request context changed between start and finish")
        elif (end["line"], end["column"]) <= (begin["line"], begin["column"]):
            problems.append(f"{key}: finish precedes start")
    if not records:
        problems.append("no attempt records: this source was not instrumented")
    if incomplete:
        problems.append("incomplete/orphan attempts are retained; distribution is not complete")
    if calibration:
        expected = {"m-ok": "ok", "m-error": "error", "m-timeout": "timeout",
                    "m-cancel": "cancelled", "m-resumed": "ok"}
        if set(by_case) != set(expected):
            problems.append("calibration did not produce exactly the five real request cases")
        for case, outcome in expected.items():
            group = by_case.get(case)
            if not group or group["started"] != 1 or group["finished"] != 1 or group["outcomes"] != {outcome: 1}:
                problems.append(f"calibration case {case}: wrong count or outcome")
    return dict(schema=1, measurement_revision=REVISION, valid=not problems,
                log_files=[str(path) for path in logs], events=len(records),
                started=sum("started" in phases for phases in pairs.values()),
                finished=sum("finished" in phases for phases in pairs.values()),
                outcomes=dict(outcomes), by_case=dict(by_case), incomplete=incomplete,
                problems=problems, records=records,
                limits=["Elapsed values include all finished outcomes; no success-only filtering",
                        "Cancellation describes the caller future, not proof that remote execution stopped",
                        "Hard process termination can leave unmatched starts; these cannot support a complete SLO"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("--calibration", action="store_true")
    args = parser.parse_args()
    logs = sorted(args.input.rglob("scenarios.log")) if args.input.is_dir() else [args.input]
    result = inspect(logs, args.calibration)
    print(json.dumps(result, indent=2))
    return 0 if result["valid"] else 1


if __name__ == "__main__":
    sys.exit(main())
