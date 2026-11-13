"""Fixed high-BDP resident preflight; preserves low regression and all outcomes."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys


def module(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(file))
    value = importlib.util.module_from_spec(spec); spec.loader.exec_module(value)
    return value


policy = module('workload_policy', 'netlab-workload-policy.py')
queues = module('queue_profile', 'netlab-kernel-queues.py')
observer = policy.observer
REVISION = 'resident_mixed_workload_high_v3'
BYTES = 192*1024*1024


def download_captures(rows, bulk_bytes):
    sides = ['x', 'y', 'gateway', 'right_gateway']
    assert len(rows) == 8 and {r['name'] for r in rows} == {s+'_'+t for s in sides for t in ['before', 'after']}
    reads = {r['name']: queues.read(r['capture'], 'gateway' in r['name']) for r in rows}
    result = {}
    for side in sides:
        a, b = reads[side+'_before'], reads[side+'_after']
        keys = ['rate_interface', 'delay_interface', 'actual_burst_bytes'] if 'gateway' in side else ['endpoint_interface']
        assert all(a[k] == b[k] for k in keys) and a['interfaces'].keys() == b['interfaces'].keys()
        delta = {}
        for iface in a['interfaces']:
            delta[iface] = {}
            for direction in ['rx', 'tx']:
                n = b['interfaces'][iface][direction]['bytes']-a['interfaces'][iface][direction]['bytes']
                assert isinstance(n, int) and n >= 0
                delta[iface][direction+'_bytes'] = n
        if 'gateway' in side:
            for key in ['rate_bytes', 'rate_drops', 'delay_bytes', 'delay_drops']:
                delta[key] = b[key]-a[key]; assert delta[key] >= 0
            incoming = a['rate_interface'] if side == 'gateway' else a['delay_interface']
            outgoing = a['delay_interface'] if side == 'gateway' else a['rate_interface']
            assert delta[incoming]['rx_bytes'] >= bulk_bytes and delta[outgoing]['tx_bytes'] >= bulk_bytes
            assert delta['delay_bytes' if side == 'gateway' else 'rate_bytes'] >= bulk_bytes
        else:
            direction = 'rx_bytes' if side == 'x' else 'tx_bytes'
            assert delta[a['endpoint_interface']][direction] >= bulk_bytes
        result[side] = dict(before=a, after=b, delta=delta)
    return result


def case(path, allow_incomplete_bulk=False):
    rows = observer.records(path, revision=REVISION, allow_failed=True)
    manifest = observer.one(rows, 'manifest'); bulk = observer.one(rows, 'bulk'); done = observer.one(rows, 'complete')
    reader = download_captures
    if allow_incomplete_bulk and bulk['outcome'] != 'ok':
        # Preserve geometry and monotonic counters for a failed attempt. It
        # cannot pass functional acceptance without a complete verified file.
        reader = lambda captures, expected: download_captures(captures, 0)
    value = observer.case(path, 'buffered', revision=REVISION, bulk_bytes=BYTES, period_us=400000,
                          profile_reader=reader, allow_failed=True)
    assert manifest['network_profile'] == 'forwarding_split_300ms_50mbit'
    assert manifest['impairment_installed_before_residents'] is True
    assert manifest['nominal_rtt_ms'] == 300 and manifest['mbit_per_direction'] == 50
    assert manifest['queue_ms'] == 40 and manifest['random_loss_pct'] == 0
    assert manifest['connection_layout'] == 'directory_and_fetch_Main__terminal_Tunnel'
    assert manifest['phases'] == observer.PHASES and manifest['operations'] == ['directory', 'terminal']
    assert manifest['seed'] == '6d697865646c6f61' and manifest['diagnostic_recording'] is True
    assert bulk['expected'] == BYTES and bulk['caller_ms'] == 240000 and bulk['ended_us'] > bulk['started_us']
    assert done['bulk'] == bulk and done['profile_readback_commands_ok'] is True
    for phase in observer.PHASES:
        for operation in ['directory', 'terminal']:
            samples = sorted([r for r in rows if r['event'] == 'sample' and r['phase'] == phase and r['operation'] == operation], key=lambda r:r['index'])
            assert all(b['started_us'] >= a['ended_us'] for a, b in zip(samples, samples[1:]))
    marker = '[netlab-workload-calibration] '
    installs = [json.loads(l.split(marker, 1)[1]) for l in path.read_text().splitlines() if marker in l]
    assert len(installs) == 2 and all(r['schema'] == 1 and r['revision'] == REVISION and r['event'] == 'rate_install' for r in installs)
    queues.installs(installs, value['profiles'])
    value['transfer_populations'] = policy.populations(path, bulk, revision=REVISION, allow_failed=True)
    value['bulk_seconds'] = (bulk['ended_us']-bulk['started_us'])/1000000
    value['verified_payload_mbit'] = BYTES*8/(bulk['ended_us']-bulk['started_us']) if done['exact_bytes'] else None
    return value


def verify(report, source_root=None):
    report = Path(report); source_root = source_root or Path(__file__).resolve().parents[1]
    result = dict(schema=1, revision=REVISION, source=json.loads((report/'source.json').read_text()),
        low_regression=[], results=[], passed=False, slo=False, full_N3_accepted=False, p99_claimed=False,
        limits=['High resident preflight only, not priority A/B comparison or N3 acceptance',
                'Layered capacity/latency admission is explicitly versioned in MODEL-DECISION; old TCP failures remain',
                'Single y-to-x file: reverse control/ACK bytes are recorded and not required to equal file size',
                'Guard and all three low regressions are required before the three high cases; failures retained'])
    try:
        result['configuration'] = policy.configuration(source_root)
        result['guard'] = observer.guard(report)
        lows = sorted((report/'low-regression').glob('*/scenarios.log'))
        for path in lows:
            row = dict(log=str(path.relative_to(report)),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),valid=False)
            try:
                row['workload'] = observer.case(path, 'buffered'); row['valid'] = row['workload']['functional_ok']
            except (AssertionError,KeyError,ValueError,TypeError,IndexError,OSError) as error:
                row['error'] = type(error).__name__+': '+str(error)
            result['low_regression'].append(row)
        logs = sorted((report/'netlab').glob('*/scenarios.log'))
        for path in logs:
            row = dict(log=str(path.relative_to(report)),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),valid=False)
            try:
                assert '[netlab-diagnostic]' not in path.read_text() and not list((path.parent/'diagnostics').glob('*'))
                row['workload'] = case(path); row['valid'] = row['workload']['functional_ok']
            except (AssertionError,KeyError,ValueError,TypeError,IndexError,OSError) as error:
                row['error'] = type(error).__name__+': '+str(error)
            result['results'].append(row)
        result['passed'] = len(lows) == len(logs) == 3 and all(r['valid'] for r in result['low_regression']+result['results'])
    except (AssertionError,KeyError,ValueError,TypeError,IndexError,OSError) as error:
        result['error'] = type(error).__name__+': '+str(error)
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1]); result = verify(report)
    (report/'workload-high-contracts.json').write_text(json.dumps(result, indent=2)+'\n')
    sys.exit(0 if result['passed'] else 1)
