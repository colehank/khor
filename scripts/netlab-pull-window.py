"""Validate actual wire scheduling; these diagnostics are not timing samples."""
import argparse
import hashlib
import json
from pathlib import Path
import re


def verify(report):
    mode = json.loads((report / 'measurement-mode.json').read_text())
    expected = mode['scheduler']
    assert expected in {'batch', 'rolling'}
    data = (report / 'pull-window-wire.log').read_bytes()
    text = data.decode()
    footer = re.findall(r'test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', text)
    assert footer == [('ok', '3', '0', '0')], footer
    peers = {}
    decoded = {}
    snapshots = {}
    for line in text.splitlines():
        marker = '[khor-carried-lifecycle] '
        if marker in line:
            item = json.loads(line.split(marker, 1)[1])
            if item['phase'] == 'path_slice_received':
                details = item['details']
                decoded.setdefault(item['peer'], []).append(details)
        marker = '[pull-window-wire] '
        if marker not in line:
            continue
        row = json.loads(line.split(marker, 1)[1])
        peer = row['peer']
        state = peers.setdefault(peer, {'kind': row['kind'], 'issued': []})
        assert state['kind'] == row['kind']
        if row['event'] == 'issued':
            assert row['index'] not in state['issued'], row
            state['issued'].append(row['index'])
        else:
            key = row['event']
            assert key not in snapshots
            assert sorted(state['issued']) == row['issued']
            completed = list(decoded.get(peer, []))
            indices = [r['offset'] // row['slice'] for r in completed]
            assert len(indices) == len(set(indices)), 'duplicate decoded slice'
            assert set(indices) <= set(row['issued'])
            assert row['slice'] == 262144 and row['total'] == 12 * row['slice'] + 1234
            assert all(0 <= r['offset'] < row['total'] and r['offset'] % row['slice'] == 0
                       and r['payload_bytes'] == min(row['slice'], row['total'] - r['offset'])
                       and r['wire'] == row['wire'] for r in completed)
            assert row['committed'] % row['slice'] == 0
            commits = row['committed'] // row['slice']
            assert set(range(commits)) <= set(indices), 'committed bytes require decoded replies'
            assert len(row['issued']) - commits <= row['width'], 'pending + uncommitted exceeds width'
            row['client_decoded_indices'] = indices
            snapshots[key] = row
    assert set(snapshots) == {'tail_before_release', 'head_before_release', 'before_drop', 'after_drop_control'}
    tail, head, before, after = [snapshots[k] for k in
                               ['tail_before_release', 'head_before_release', 'before_drop', 'after_drop_control']]
    width = head['width']
    assert width in {2, 8} and all(r['width'] == width for r in snapshots.values())
    assert (width + 1 in tail['issued']) == (expected == 'rolling')
    assert width not in tail['client_decoded_indices']
    assert set(range(width)) <= set(tail['client_decoded_indices'])
    assert head['issued'] == list(range(width + 1)) and head['committed'] == head['slice']
    assert set(head['client_decoded_indices']) == set(range(width + 1)) - {1}
    assert before['issued'] == after['issued'] == list(range(width + 1))
    assert before['client_decoded_indices'] == after['client_decoded_indices'] == [0]
    assert before['stopped'] == [] and after['stopped'] == list(range(1, width + 1))
    assert before['wire'] == after['wire'] and before['peer'] == after['peer']
    assert len(peers) == 3 and len({r['peer'] for r in snapshots.values()}) == 3
    assert all(sorted(s['issued']) == list(range(13)) for s in peers.values() if s['kind'] != 'All')
    return dict(schema=2, revision='pull_window_real_rpc_v2', scheduler=expected, width=width,
                passed=True, diagnostic=True, timing_samples=0, production_server_cancel=False,
                job_stop=False, host_child_cancel=False, snapshots=snapshots,
                source=json.loads((report / 'source.json').read_text()),
                log_sha256=hashlib.sha256(data).hexdigest())


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('report', type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(args.report), indent=2))
