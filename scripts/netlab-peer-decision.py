"""Supplement the sampled-request gate with both inner recovery decision times.

The original physical fixture and sampled-request checker remain unchanged.
This controlled case must retain each endpoint's complete diagnostic stream;
missing decisions fail rather than treating a late reverse read as early proof.
"""
import datetime
import hashlib
import json
from pathlib import Path
import sys


def check(folder):
    folder = Path(folder)
    calibration = json.loads((folder / 'peer-mobility-calibration.json').read_text())
    requests = json.loads((folder / 'peer-selection-contract.json').read_text())
    source = json.loads((folder / 'report/source.json').read_text())
    pivot = requests['pivot_at_ms']
    decisions = []
    problems = []
    for role in ['x', 'y']:
        files = list((folder / 'report/netlab').glob('*/diagnostics/peer-middle-v4-' + role + '.log'))
        if len(files) != 1:
            problems.append(dict(role=role, reason='missing unique complete endpoint log'))
            continue
        path = files[0]
        rows = []
        for line in path.read_text().splitlines():
            if 'move decision' not in line or 'from=Relay' not in line or 'to=Custom' not in line or 'step=Go' not in line:
                continue
            at = datetime.datetime.fromisoformat(line.split()[0].replace('Z', '+00:00'))
            delta_us = round(at.timestamp() * 1000000) - pivot * 1000
            if delta_us >= 0:
                rows.append(delta_us)
        first = min(rows) if rows else None
        ok = first is not None and first <= 4000000
        decisions.append(dict(role=role, first_go_after_replug_us=first, passed=ok,
                              log_sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
        if not ok:
            problems.append(dict(role=role, reason='no observed inner Relay-to-Carried recovery Go by four seconds',
                                 first_go_after_replug_us=first))
    if not calibration['availability_verified']:
        problems.append(dict(reason='physical availability calibration failed'))
    if not requests['passed']:
        problems.append(dict(reason='original sampled-request selection gate failed'))
    return dict(schema=1, contract='peer_relay_replug_both_recovery_decisions_within_four_seconds_v1',
                run_id=folder.name, source=source, pivot_phase='middle_replug_finished',
                pivot_at_ms=pivot, after_ms=4000, sampled_request_gate_passed=requests['passed'],
                decisions=decisions, passed=not problems, problems=problems,
                limits=['Decision-time contract for the fixed replug case, combined with actual sampled streams.',
                        'Missing recovery decisions require additional state evidence; absence is never automatic success.',
                        'Does not claim arbitrary WAN hard deadlines or per-packet final carriage.'])


if __name__ == '__main__':
    result = check(sys.argv[1])
    print(json.dumps(result, indent=2))
    raise SystemExit(0 if result['passed'] else 1)
