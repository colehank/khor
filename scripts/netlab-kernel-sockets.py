"""Verify read-only TCP socket observations separately from physical acceptance."""
from collections import Counter
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

spec = importlib.util.spec_from_file_location('kernel_high', Path(__file__).with_name('netlab-kernel-high.py'))
high = importlib.util.module_from_spec(spec); spec.loader.exec_module(high)
REVISION = 'workload_kernel_tcp_sockets_v1'


def sockets(path, planned_seconds=(3, 8, 15, 25)):
    marker = '[netlab-workload-calibration] '
    rows = [json.loads(s.split(marker, 1)[1]) for s in path.read_text().splitlines() if marker in s]
    manifest = high.one(rows, 'manifest')
    assert manifest['socket_diagnostic'] is True
    if 'socket_plan_seconds' in manifest:
        assert manifest['socket_plan_seconds'] == list(planned_seconds)
    samples = [r for r in rows if r['event'] == 'tcp_snapshot']
    groups = {}
    for side in ['client', 'server']:
        for operation in ['upload', 'download']:
            rs = [r for r in samples if r['side'] == side and r['operation'] == operation]
            assert rs and len({r['planned_after_us'] for r in rs}) == len(rs)
            counts = Counter(r['planned_after_us'] for r in rs)
            assert counts[3000000] == counts[8000000] == 1
            assert set(counts) <= {v*1000000 for v in planned_seconds}
            assert len({(r['local'], r['peer']) for r in rs}) == 1
            for r in rs:
                assert r['clock_origin'] == 'local_payload_start'
                assert r['ended_after_us'] >= r['started_after_us'] >= r['planned_after_us']
                c = r['command']; assert c['ok'] is True and c['timed_out'] is False
                raw = c['stdout']
                assert r['local'] in raw and r['peer'] in raw
                assert all(marker in raw for marker in ['cwnd:', 'rtt:', 'skmem:'])
            assert all(a['started_after_us'] < b['started_after_us'] for a, b in zip(rs, rs[1:]))
            groups[side+'/'+operation] = rs
    assert len(samples) == sum(len(rs) for rs in groups.values())
    for operation in ['upload', 'download']:
        client, server = groups['client/'+operation][0], groups['server/'+operation][0]
        assert client['local'] == server['peer'] and client['peer'] == server['local']
    return groups


def verify(report, revision=REVISION, bulk_bytes=high.BYTES, planned_seconds=(3, 8, 15, 25), repeats=3):
    report = Path(report)
    result = dict(schema=1, revision=revision, source=json.loads((report/'source.json').read_text()),
                  results=[], observations_verified=False, physical_calibration_accepted=False, slo=False, full_N3_accepted=False,
                  limits=['Read-only diagnostic overhead retained, not a policy benchmark',
                          'Valid socket observations do not make failed frozen physical bounds pass',
                          'Missing optional kernel fields remain unknown; rcv_space is not a peer advertised window'])
    paths = sorted((report/'netlab').glob('*/scenarios.log'))
    for path in paths:
        row = dict(log=str(path.relative_to(report)), sha256=hashlib.sha256(path.read_bytes()).hexdigest(), valid=False)
        try:
            row['calibration'] = high.case(path, revision=revision, allow_bounds_failure=True, bulk_bytes=bulk_bytes)
            row['sockets'] = sockets(path, planned_seconds)
            row['valid'] = True
        except (AssertionError, ValueError, KeyError, TypeError, IndexError, ZeroDivisionError) as error:
            row['error'] = type(error).__name__+': '+str(error)
        result['results'].append(row)
    result['observations_verified'] = len(paths) == repeats and all(r['valid'] for r in result['results'])
    result['physical_bounds_met'] = result['observations_verified'] and all(r['calibration']['functional_ok'] for r in result['results'])
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1]); result = verify(report)
    (report/'workload-kernel-sockets-contracts.json').write_text(json.dumps(result, indent=2)+'\n')
    sys.exit(0 if result['observations_verified'] else 1)
