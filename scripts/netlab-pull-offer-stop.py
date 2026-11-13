"""Observe conversation stop and sender cut before fixture-owned caller cleanup."""
import json
from pathlib import Path
import re
import sys


def verify(report):
    mode = json.loads((report / 'measurement-mode.json').read_text())
    assert mode == dict(layer='node', slo=False, suite='pull_offer_stop_v1',
                        full_regression=False, diagnostic=True, test_threads=1, expected_tests=4)
    text = (report / 'pull-offer-stop.log').read_text()
    footers = re.findall(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
    assert len(footers) == 1
    status, good, failed, ignored = footers[0]
    assert int(good) + int(failed) == 4 and int(ignored) == 0
    assert (status == 'ok') == (int(failed) == 0)
    observations = {}
    issued = {}
    decoded = {}
    for line in text.splitlines():
        prefix = '[pull-window-wire] '
        if prefix in line:
            row = json.loads(line.split(prefix, 1)[1])
            assert row['event'] == 'issued'
            rows = issued.setdefault(row['peer'], [])
            assert row['index'] not in rows
            rows.append(row['index'])
        prefix = '[khor-carried-lifecycle] '
        if prefix in line:
            row = json.loads(line.split(prefix, 1)[1])
            if row['phase'] == 'path_slice_received':
                decoded.setdefault(row['peer'], []).append(row['details']['offset'])
        prefix = '[pull-offer-stop] '
        if prefix not in line:
            continue
        row = json.loads(line.split(prefix, 1)[1])
        key = ('cut-' if row['cut'] else 'stop-') + ('first' if row['first'] else 'window')
        cell = observations.setdefault(key, {})
        assert row['event'] not in cell
        cell[row['event']] = row
    assert set(observations) == {'stop-first', 'stop-window', 'cut-first', 'cut-window'}
    for key, cell in observations.items():
        assert set(cell) == {'caller_observed', 'cleanup_observed'}
        caller, cleanup = cell['caller_observed'], cell['cleanup_observed']
        assert caller['peer'] == cleanup['peer']
        pending = 1 if key.endswith('first') else 8
        expected = [0] if key.endswith('first') else list(range(9))
        assert sorted(issued[caller['peer']]) == caller['issued'] == expected
        assert decoded.get(caller['peer'], []) == []
        assert caller['bound_ms'] == 500 and caller['elapsed_us'] > 0
        assert caller['moved_before'] == caller['moved_after'] == (0 if key.endswith('first') else 262144)
        assert cleanup['same_wire_control'] and cleanup['job_table_cleanup_is_fixture_owned']
        assert cleanup['remote_stopped'] == cleanup['expected_stopped'] == pending
        assert cleanup['caller_returned_before_cleanup'] == caller['returned_without_caller_drop']
    passed = all(cell['caller_observed']['returned_without_caller_drop']
                 and cell['caller_observed']['cancelled']
                 and cell['caller_observed']['clean_at_return'] for cell in observations.values())
    assert passed == (status == 'ok')
    return dict(schema=1, revision='pull_offer_stop_v1', passed=passed, observations=observations,
                rust_passed=int(good), rust_failed=int(failed), ordinary_latency_samples=0,
                conversation_stop_to_real_offer=True, sender_cut_to_real_offer=True, remote_drop_stop_rpc=False,
                run_take_cleanup_tested=False, partial_left_to_run_take_owner=True,
                production_server_cancel=False, host_child_cancel=False, full_n3=False)


if __name__ == '__main__':
    result = verify(Path(sys.argv[1]))
    print(json.dumps(result, indent=2))
    sys.exit(0 if result['passed'] else 1)
