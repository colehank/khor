"""The real receiver dispatcher admits only the trusted, correctly named stop."""
import json
from pathlib import Path
import re
import sys


def verify(report):
    mode = json.loads((report / 'measurement-mode.json').read_text())
    assert mode == dict(layer='node', slo=False, suite='pull_stop_rpc_v1',
                        diagnostic=True, full_regression=False, test_threads=1, expected_tests=1)
    text = (report / 'pull-stop-rpc.log').read_text()
    assert re.findall(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', text) == [('ok', '1', '0', '0')]
    events = []
    issued = []
    opened = []
    decoded = []
    for line in text.splitlines():
        for prefix, target in [('[pull-stop-rpc] ', events), ('[pull-window-wire] ', issued)]:
            if prefix in line:
                target.append(json.loads(line.split(prefix, 1)[1]))
        prefix = '[khor-carried-lifecycle] '
        if prefix in line:
            row = json.loads(line.split(prefix, 1)[1])
            if row['phase'] == 'request_stream_opened':
                opened.append(row)
            if row['phase'] == 'path_slice_received':
                decoded.append(row)
    assert len(events) == 2
    negative, complete = events
    assert negative['event'] == 'negative_controls' and complete['event'] == 'complete'
    assert negative['receiver'] == complete['receiver'] and negative['controller'] == complete['controller']
    assert all(negative[k] for k in ['unpaired_refused', 'malformed_token_refused', 'wrong_token_did_not_stop_target'])
    assert all(complete[k] for k in ['cancelled_without_caller_drop', 'landing_clean', 'connections_healthy', 'stop_idempotent'])
    assert len(issued) == 9 and all(row['event'] == 'issued' for row in issued)
    assert sorted(row['index'] for row in issued) == negative['issued'] == list(range(9))
    source = {row['peer'] for row in issued}
    assert len(source) == 1
    source = source.pop()
    assert len({source, negative['receiver'], negative['controller'], negative['stranger']}) == 4
    assert complete['source_stop_count'] == 8 and complete['source_wire'] != complete['control_wire']
    fetch = [row for row in opened if row['details']['operation'] == 'fetch_path']
    control = [row for row in opened if row['details']['operation'] == 'other']
    assert len(fetch) == 9 and len(control) == 5 and len(opened) == 14
    assert all(row['peer'] == source and row['details']['wire'] == complete['source_wire'] for row in fetch)
    assert all(row['peer'] == complete['receiver'] for row in control)
    assert sum(row['details']['wire'] == complete['control_wire'] for row in control) == 4
    assert len(decoded) == 1 and decoded[0]['peer'] == source
    assert decoded[0]['details']['offset'] == 0 and decoded[0]['details']['payload_bytes'] == 262144
    return dict(schema=1, revision='pull_stop_rpc_v1', passed=True, events=events,
                actual_fetch_rpcs=9, actual_control_rpcs=5, real_receiver_dispatch=True,
                device_table_pre_admitted=True, pairing_protocol_tested=False,
                production_file_source=False, host_child_cancel=False,
                mobility_tested=False, ordinary_latency_samples=0, full_n3=False)


if __name__ == '__main__':
    print(json.dumps(verify(Path(sys.argv[1])), indent=2))
