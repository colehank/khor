"""One fixed prefetch value, two separate profiles; failed files remain failures."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

spec = importlib.util.spec_from_file_location('high_resident', Path(__file__).with_name('netlab-workload-high.py'))
high = importlib.util.module_from_spec(spec)
spec.loader.exec_module(high)
observer = high.observer


def low_captures(rows, required):
    names = {'x_before', 'gateway_before', 'x_after', 'gateway_after'}
    assert len(rows) == 4 and {r['name'] for r in rows} == names
    reads = {r['name']: observer.profile(r['capture']) for r in rows}
    for side in ['x', 'gateway']:
        a, b = reads[side+'_before'], reads[side+'_after']
        assert a['interface'] == b['interface'] and b['bytes'] >= a['bytes'] and b['drops'] >= a['drops']
    assert reads['x_after']['bytes'] > reads['x_before']['bytes']
    assert reads['gateway_after']['bytes']-reads['gateway_before']['bytes'] >= required
    return reads


def case(path, profile):
    revision = observer.REVISION if profile == 'low' else high.REVISION
    rows = observer.records(path, revision=revision, allow_failed=True)
    bulk = observer.one(rows, 'bulk'); complete = observer.one(rows, 'complete')
    assert bulk['outcome'] in ['ok', 'error', 'timeout']
    if profile == 'high':
        value = high.case(path, allow_incomplete_bulk=True)
        expected, caller_ms = high.BYTES, 240000
    else:
        expected, caller_ms = observer.BYTES, 120000
        required = expected if bulk['outcome'] == 'ok' else 0
        value = observer.case(path, 'buffered', profile_reader=lambda captures, size: low_captures(captures, required), allow_failed=True)
    assert bulk['expected'] == expected and bulk['caller_ms'] == caller_ms
    assert complete['bulk'] == bulk and complete['profile_readback_commands_ok'] is True
    probes = {r['phase']: r for r in rows if r['event'] == 'probe'}
    assert probes['bulk']['started_us'] <= bulk['started_us'] < bulk['ended_us'] <= probes['bulk']['ended_us']
    assert all(probes[b]['started_us'] >= probes[a]['ended_us'] for a, b in zip(observer.PHASES, observer.PHASES[1:]))
    for phase in observer.PHASES:
        for operation in ['directory', 'terminal']:
            samples = sorted([r for r in rows if r['event'] == 'sample' and r['phase'] == phase and r['operation'] == operation], key=lambda r:r['index'])
            assert all(b['started_us'] >= a['ended_us'] for a, b in zip(samples, samples[1:]))
    duration = bulk['ended_us']-bulk['started_us']
    if bulk['outcome'] == 'timeout':
        assert duration >= caller_ms*1000
    finished = bulk['outcome'] == 'ok' and complete['exact_bytes'] is True and bulk['moved'] == expected
    populations = high.policy.populations(path, bulk, revision=revision, allow_failed=True)
    # A failed caller has no verified actual transfer completion time.
    if not finished:
        for groups in populations.values():
            groups['started_during_attempt'] = groups.pop('started_during_transfer')
            groups['fully_during_attempt'] = groups.pop('fully_during_transfer')
    value.update(transfer_populations=populations, interval_kind='completed_transfer' if finished else 'caller_attempt',
        file_outcome=bulk['outcome'], bulk_seconds=duration/1000000,
        verified_payload_mbit=expected*8/duration if finished else None,
        completion_censored_at_caller_deadline=bulk['outcome'] == 'timeout',
        complete_file_verified=finished, full_N3_accepted=False)
    return value


def verify(report, source_root=None):
    report = Path(report)
    source_root = source_root or Path(__file__).resolve().parents[1]
    result = dict(schema=1, revision='workload_prefetch_v1', source=json.loads((report/'source.json').read_text()),
        results=[], passed=False, slo=False, full_N3_accepted=False, p99_claimed=False,
        limits=['One value in the predeclared 8,1,2,1,2,8,2,8,1 matrix; fixed priority must be recorded before launch',
                'Low and high populations remain separate; original deadlines and complete-file acceptance unchanged',
                'Failed bulk exposes attempt intervals and no verified payload rate; failures are never accepted',
                'No complete-preview, fairness, backpressure, device-scale or energy acceptance'])
    try:
        result['configuration'] = high.policy.configuration(source_root, allowed_ahead=(1, 2, 8))
        result['guard'] = observer.guard(report)
        for profile in ['low', 'high']:
            logs = sorted((report/(profile+'-prefetch')).glob('*/scenarios.log'))
            for path in logs:
                row = dict(profile=profile, log=str(path.relative_to(report)), sha256=hashlib.sha256(path.read_bytes()).hexdigest(), valid=False)
                try:
                    assert '[netlab-diagnostic]' not in path.read_text() and not list((path.parent/'diagnostics').glob('*'))
                    row['workload'] = case(path, profile)
                    row['valid'] = row['workload']['functional_ok']
                except (AssertionError, KeyError, ValueError, TypeError, IndexError, OSError) as error:
                    row['error'] = type(error).__name__+': '+str(error)
                result['results'].append(row)
        result['passed'] = len(result['results']) == 2 and sorted(r['profile'] for r in result['results']) == ['high', 'low'] and all(r['valid'] for r in result['results'])
    except (AssertionError, KeyError, ValueError, TypeError, IndexError, OSError) as error:
        result['error'] = type(error).__name__+': '+str(error)
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1]); result = verify(report)
    (report/'workload-prefetch-contracts.json').write_text(json.dumps(result, indent=2)+'\n')
    sys.exit(0 if result['passed'] else 1)
