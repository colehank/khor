"""Audit completed workload streams and opt-in server stages; data only."""
from collections import Counter
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys

spec = importlib.util.spec_from_file_location('observer', Path(__file__).with_name('netlab-observer.py'))
observer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(observer)
MARKER = '[khor-carried-lifecycle] '
STAGES = ['permit_asked_us', 'seated_us', 'request_read_us', 'prepared_us', 'encoded_us', 'written_us']


def records(path):
    lines = [s for s in path.read_text().splitlines() if MARKER in s]
    assert lines and all(s.count(MARKER) == 1 for s in lines)
    rows = [json.loads(s.split(MARKER, 1)[1]) for s in lines]
    assert all(r['schema'] == 1 for r in rows)
    return rows


def wire_evidence(client, server, continued_priority=None):
    replies = [r for r in client if r['phase'] in ['list_response_received', 'path_slice_received']]
    sent = [r for r in server if r['phase'] in ['list_response_sent', 'path_slice_response_sent']]
    assert Counter(r['details']['operation'] for r in replies) == {'directory': 360, 'fetch_path': 64}
    assert Counter(r['details']['operation'] for r in sent) == {'directory': 360, 'fetch_path': 64}
    for rows in [replies, sent]:
        assert len({r['pid'] for r in rows}) == len({r['peer'] for r in rows}) == 1
        assert len({(r['details']['wire'], r['details']['stream']) for r in rows}) == 424
        assert all(r['details']['alpn'] == 'khor/0' for r in rows)
        offsets = [r['details']['offset'] for r in rows if r['details']['operation'] == 'fetch_path']
        assert sorted(offsets) == list(range(0, 16*1024*1024, 256*1024))
    assert next(iter({r['peer'] for r in replies})) != next(iter({r['peer'] for r in sent}))
    for r in replies:
        d = r['details']
        assert d['elapsed_us'] >= 0 and d['timing_origin'] == ('early_dial_started' if d['early'] else 'exchange_started')
        if d['operation'] == 'fetch_path': assert d['payload_bytes'] == 256*1024
    for r in sent:
        d = r['details']
        assert d['timing_origin'] == 'stream_accepted' and d['frame_bytes'] > 0
        stages = [d[k] for k in STAGES]
        assert all(isinstance(v, int) and v >= 0 for v in stages) and stages == sorted(stages)
        if continued_priority is not None:
            expected = continued_priority if d['operation'] == 'fetch_path' and d['offset'] > 0 else 0
            assert d['priority_applied'] is True and d['response_priority'] == expected
    pairs = []
    for op in ['directory', 'fetch_path']:
        a = [r for r in replies if r['details']['operation'] == op]
        b = [r for r in sent if r['details']['operation'] == op]
        if op == 'fetch_path':
            a.sort(key=lambda r:r['details']['offset'])
            b.sort(key=lambda r:r['details']['offset'])
        for x, y in zip(a, b):
            assert x['details']['stream'] == y['details']['stream']
            assert x['details']['offset'] == y['details']['offset']
            pairs.append((x['details']['wire'], y['details']['wire']))
    mapping = set(pairs)
    assert len({x for x, y in mapping}) == len({y for x, y in mapping}) == len(mapping)
    ready = [r for r in client if r['phase'] == 'tunnel_stream_ready']
    accepted = [r for r in server if r['phase'] == 'tunnel_target_accepted']
    assert len(ready) == len(accepted) == 1
    assert ready[0]['details']['stream'] == accepted[0]['details']['stream']
    assert accepted[0]['details']['paired'] is True
    assert ready[0]['details']['alpn'] == accepted[0]['details']['alpn'] == 'khor-tunnel/0'
    assert ready[0]['details']['wire'] not in {x for x, y in mapping}
    assert accepted[0]['details']['wire'] not in {y for x, y in mapping}
    stage_groups = {}
    for op in ['directory', 'fetch_path']:
        rs = [r['details'] for r in sent if r['details']['operation'] == op]
        groups = {op: rs} if op == 'fetch_path' else {phase:rs[i*120:(i+1)*120] for i, phase in enumerate(observer.PHASES)}
        for name, group in groups.items():
            previous = [0]*len(group)
            stages = {}
            for key in STAGES:
                values = [d[key] for d in group]
                stages[key] = observer.distribution([v-p for v, p in zip(values, previous)])
                previous = values
            stage_groups[op+'/'+name] = stages
    directory_wires = {r['details']['wire'] for r in replies if r['details']['operation'] == 'directory'}
    fetch_wires = {r['details']['wire'] for r in replies if r['details']['operation'] == 'fetch_path'}
    return dict(directory_replies=360, fetch_replies=64, matched_streams=424,
                actual_main_wire_pairs=sorted(mapping), directory_client_wires=sorted(directory_wires),
                fetch_client_wires=sorted(fetch_wires), all_business_on_one_Main=len(mapping)==1,
                separate_Tunnel=True, tunnel_client_wire=ready[0]['details']['wire'],
                tunnel_server_wire=accepted[0]['details']['wire'], server_stage_durations_us=stage_groups,
                directory_phase_assignment='serial response order, 120 per phase',
                limits=['Completed replies only; logical failure population is in the workload log',
                        'Stages start after accept_bi; QUIC pre-accept wait is not observed',
                        'written_us measures local write_all completion; it is not a delivery or ACK time',
                        'Server clocks are not subtracted from client clocks; no CPU/I/O causal claim'])


def verify(report):
    report = Path(report)
    result = dict(schema=1, revision='workload_wire_stages_v1', source=json.loads((report/'source.json').read_text()),
                  results=[], passed=False, slo=False, full_N3_accepted=False, p99_claimed=False)
    try:
        result['guard'] = observer.guard(report)
        logs = sorted((report/'netlab').glob('*/scenarios.log'))
        assert len(logs) == 3
        for log in logs:
            row = dict(log=str(log.relative_to(report)), sha256=hashlib.sha256(log.read_bytes()).hexdigest(), valid=False)
            try:
                row['workload'] = observer.case(log, 'buffered')
                paths = [log.parent/'diagnostics'/f'w2-buffer-{name}.log' for name in ['x', 'y']]
                assert all('mode=lifecycle samples=off' in p.read_text() for p in paths)
                row['diagnostic_sha256'] = {str(p.relative_to(report)):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
                text = (Path(__file__).resolve().parents[1]/'crates/node/src/link.rs').read_text()
                priority = re.findall(r'^const CONTINUED_FILE_RESPONSE_PRIORITY: i32 = (-?\d+);$', text, re.M)
                assert len(priority) <= 1
                expected = int(priority[0]) if priority else None
                row['wires'] = wire_evidence(*(records(p) for p in paths), continued_priority=expected)
                row['wires']['expected_continued_priority'] = expected
                row['valid'] = row['workload']['functional_ok']
            except (AssertionError, KeyError, ValueError, TypeError, IndexError, OSError) as error:
                row['error'] = type(error).__name__+': '+str(error)
            result['results'].append(row)
        result['passed'] = all(r['valid'] for r in result['results'])
    except (AssertionError, KeyError, ValueError, TypeError, IndexError, OSError) as error:
        result['error'] = type(error).__name__+': '+str(error)
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1])
    result = verify(report)
    (report/'workload-wire-contracts.json').write_text(json.dumps(result, indent=2)+'\n')
    sys.exit(0 if result['passed'] else 1)
