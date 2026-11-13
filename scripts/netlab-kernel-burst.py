"""Fixed ABBA burst/derived-queue diagnostic; data-only, no model acceptance."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

spec = importlib.util.spec_from_file_location('kernel_high', Path(__file__).with_name('netlab-kernel-high.py'))
high = importlib.util.module_from_spec(spec); spec.loader.exec_module(high)
REVISION = 'workload_kernel_tcp_burst_v1'
ORDER = (4096, 65536, 65536, 4096)


def case(path, expected):
    calibrated = high.case(path, revision=REVISION, allow_bounds_failure=True)
    marker = '[netlab-workload-calibration] '
    rows = [json.loads(line.split(marker, 1)[1]) for line in path.read_text().splitlines() if marker in line]
    manifest = high.one(rows, 'manifest')
    assert manifest['requested_burst_bytes'] == expected and manifest['socket_diagnostic'] is False
    assert not any(r['event'] == 'tcp_snapshot' for r in rows)
    changes = [r for r in rows if r['event'] == 'shaper_change']
    assert len(changes) == 2 and {r['side'] for r in changes} == {'x', 'gateway'}
    for change in changes:
        assert change['requested_burst_bytes'] == expected and change['latency_us'] == 40000
        assert change['derived_fifo_limit_changes'] is True
        assert change['before']['ok'] is True and change['before']['timed_out'] is False
        initial = json.loads(change['before']['stdout'])
        rates = [r for r in initial if r['kind'] == 'tbf']; nets = [r for r in initial if r['kind'] == 'netem']
        assert len(rates) == len(nets) == 1
        rate, net = rates[0], nets[0]
        assert rate['dev'] == net['dev'] == change['device'] and rate['parent'] == '1:1' and rate['handle'] == '10:'
        assert net['root'] is True and net['handle'] == '1:' and net['options']['limit'] == 1000
        assert net['options']['delay'] == dict(delay=.15, jitter=0, correlation=0)
        assert rate['options']['rate'] == 6250000 and rate['options']['lat'] == 40000
        assert 4080 <= rate['options']['burst'] <= 4096
        assert change['args'] == ['qdisc', 'change', 'dev', change['device'], 'parent', '1:1', 'handle', '10:',
                                  'tbf', 'rate', '50mbit', 'burst', str(expected), 'latency', '40ms']
        assert change['command']['ok'] is True and change['command']['timed_out'] is False
        counters = calibrated['profile_counters'][change['side']]
        assert counters['before']['interface'] == change['device']
        for moment in ['before', 'after']:
            assert expected-16 <= counters[moment]['actual_burst_bytes'] <= expected
            assert counters[moment]['actual_netem_limit'] == 1000
    return calibrated


def verify(report):
    report = Path(report)
    result = dict(schema=1, revision=REVISION, source=json.loads((report/'source.json').read_text()),
                  requested_burst_order=list(ORDER), results=[], observations_verified=False,
                  physical_calibration_accepted=False, full_N3_accepted=False, slo=False,
                  limits=['ABBA diagnostic only; not adequate repeated model calibration or Node policy acceptance',
                          'One burst input knob with fixed latency: TBF also derives a different FIFO limit',
                          'Does not isolate token timing from changed queue capacity; preserve all physical failures'])
    paths = sorted((report/'netlab').glob('*/scenarios.log'))
    for index, path in enumerate(paths):
        row = dict(log=str(path.relative_to(report)), sha256=hashlib.sha256(path.read_bytes()).hexdigest(), valid=False)
        try:
            assert index < len(ORDER)
            row['requested_burst_bytes'] = ORDER[index]
            row['calibration'] = case(path, ORDER[index])
            row['valid'] = True
        except (AssertionError, ValueError, KeyError, TypeError, IndexError, ZeroDivisionError) as error:
            row['error'] = type(error).__name__+': '+str(error)
        result['results'].append(row)
    result['observations_verified'] = len(paths) == len(ORDER) and all(r['valid'] for r in result['results'])
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1]); result = verify(report)
    (report/'workload-kernel-burst-contracts.json').write_text(json.dumps(result, indent=2)+'\n')
    sys.exit(0 if result['observations_verified'] else 1)
