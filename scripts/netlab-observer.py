"""Validate paired observer evidence; parse data only, never execute a product."""
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import sys

REVISION = 'resident_mixed_workload_v4'
PHASES = ['idle_before', 'bulk', 'idle_after']
MODES = ['streaming', 'buffered', 'buffered', 'streaming', 'streaming', 'buffered']
BYTES = 16 * 1024 * 1024


def records(path, expected_passed=1, revision=REVISION, allow_failed=False):
    text = path.read_text()
    marker = '[netlab-workload] '
    lines = [line for line in text.splitlines() if marker in line]
    assert all(line.count(marker) == 1 for line in lines)
    rows = [json.loads(line.split(marker, 1)[1]) for line in lines]
    assert rows and all(r['schema'] == 1 and r['revision'] == revision for r in rows)
    summaries = re.findall(r'^test result: (.*)$', text, re.M)
    assert len(summaries) == 1
    passed = summaries[0].startswith(f'ok. {expected_passed} passed; 0 failed; 0 ignored;')
    failed = allow_failed and expected_passed == 1 and summaries[0].startswith('FAILED. 0 passed; 1 failed; 0 ignored;')
    assert passed or failed
    return rows


def one(rows, event):
    found = [r for r in rows if r['event'] == event]
    assert len(found) == 1, f'expected one {event}, found {len(found)}'
    return found[0]


def distribution(values):
    values = sorted(values)
    return dict(n=len(values), p50=values[(len(values)-1)//2] if values else None,
                p95=values[(len(values)*95+99)//100-1] if values else None,
                max=max(values) if values else None)


def profile(capture):
    assert all(capture[k]['ok'] is True for k in ['qdisc_json', 'qdisc_text', 'interfaces'])
    rows = json.loads(capture['qdisc_json']['stdout'])
    tbfs = [r for r in rows if r['kind'] == 'tbf']
    assert len(tbfs) == 1
    tbf = tbfs[0]
    netems = [r for r in rows if r['kind'] == 'netem' and r['dev'] == tbf['dev']]
    assert len(netems) == 1
    netem = netems[0]
    assert netem['root'] is True and netem['handle'] == '1:' and tbf['parent'] == '1:1'
    assert tbf['options'] == dict(rate=1000000, burst=4096, lat=40000)
    assert netem['options']['delay'] == dict(delay=.02, jitter=0, correlation=0)
    text = capture['qdisc_text']['stdout']
    assert re.search(r'\bdelay 20(?:\.0)?ms\b', text)
    assert re.search(r'\brate 8Mbit\b', text) and re.search(r'\blat 40(?:\.0)?ms\b', text)
    return dict(interface=tbf['dev'], bytes=netem['bytes'], drops=netem['drops'])


def guard(report):
    logs = sorted((report / 'observer-guards').glob('*/scenarios.log'))
    assert len(logs) == 1
    rows = records(logs[0], expected_passed=2)
    summary = one(rows, 'observer_guard_calibration')
    assert summary['passed'] is True and summary['modes'] == summary['cancelled'] == summary['completed'] == 2
    startup = one(rows, 'observer_resident_startup_guard')
    assert startup['passed'] is True and startup['unpolled_cancelled'] == startup['polled_cancelled'] == 1
    assert startup['remaining'] == 0
    samples = [r for r in rows if r['event'] == 'sample']
    assert len(samples) == 4
    for mode in ['streaming', 'buffered']:
        assert Counter(r['outcome'] for r in samples if r['mode'] == mode) == {'ok': 1, 'cancelled': 1}
    return dict(log=str(logs[0].relative_to(report)), sha256=hashlib.sha256(logs[0].read_bytes()).hexdigest(), passed=True,
                resident_startup_cancellation_verified=True)


def case(path, mode, revision=REVISION, bulk_bytes=BYTES, period_us=100000, profile_reader=None, allow_failed=False):
    rows = records(path, revision=revision, allow_failed=allow_failed)
    assert all(r.get("mode") == mode for r in rows)
    assert not any(r['event'] == 'logger_aborted' for r in rows)
    manifest, complete, bulk = (one(rows, event) for event in ['manifest', 'complete', 'bulk'])
    assert manifest['mode'] == complete['mode'] == bulk['mode'] == mode
    assert manifest['samples_per_operation_per_phase'] == 120 and manifest['period_us'] == period_us
    assert manifest['caller_us'] == 2000000 and manifest['probe_period_us'] == 20000 and manifest['logger_capacity'] == 512
    assert manifest['calibrated'] is False and manifest['slo'] is False and manifest['bulk_bytes'] == bulk_bytes
    samples = [r for r in rows if r['event'] == 'sample']
    expected = {(p, op) for p in PHASES for op in ['directory', 'terminal']}
    assert len(samples) == 720 and {(r['phase'], r['operation']) for r in samples} == expected
    probes = [r for r in rows if r['event'] == 'probe']
    finishes = [r for r in rows if r['event'] == 'phase_finished']
    starts = [r for r in rows if r['event'] == 'phase_started']
    assert all(len(rs) == 3 and {r['phase'] for r in rs} == set(PHASES) for rs in [probes, finishes, starts])
    scheduling, groups = {}, {}
    for phase in PHASES:
        probe = next(r for r in probes if r['phase'] == phase)
        finish = next(r for r in finishes if r['phase'] == phase)
        assert probe['mode'] == finish['mode'] == mode and probe['capped'] is False
        assert finish['logger_overflow'] is False and finish['probe_capped'] is False and finish['record_flush_us'] >= 0
        assert probe['period_us'] == 20000 and probe['capacity'] == 20000 and probe['missed_tick_behavior'] == 'skip'
        ticks = probe['ticks_us']
        assert ticks and len(ticks) < 20000 and all(probe['started_us'] <= p <= a <= probe['ended_us'] for p, a in ticks)
        assert all(b[0] > a[0] and b[1] >= a[1] and (b[0]-a[0]) % 20000 == 0 for a, b in zip(ticks, ticks[1:]))
        scheduling[phase] = dict(ticks=len(ticks), lag_us=distribution([a-p for p, a in ticks]),
                                 skipped_between_ticks=sum((b[0]-a[0])//20000-1 for a, b in zip(ticks, ticks[1:])),
                                 unobserved_tail_us=probe['ended_us']-ticks[-1][1], record_flush_us=finish['record_flush_us'])
        for op in ['directory', 'terminal']:
            group = sorted([r for r in samples if r['phase'] == phase and r['operation'] == op], key=lambda r:r['index'])
            assert len(group) == 120 and [r['index'] for r in group] == list(range(120))
            assert all(r['mode'] == mode and r['caller_us'] == 2000000 and r['scheduled_us'] == probe['started_us']+r['index']*period_us for r in group)
            assert all(r['ended_us'] >= r['started_us'] >= r['scheduled_us'] and r['ended_us'] <= probe['ended_us'] and r['elapsed_us'] == r['ended_us']-r['started_us'] for r in group)
            assert all(r['outcome'] in ['ok', 'error', 'timeout', 'blocked', 'cancelled', 'aborted'] for r in group)
            success = [r for r in group if r['outcome'] == 'ok']
            groups[phase+'/'+op] = dict(all_samples=120, outcomes=dict(Counter(r['outcome'] for r in group)),
                success_service_us=distribution([r['elapsed_us'] for r in success]),
                success_scheduled_to_end_us=distribution([r['ended_us']-r['scheduled_us'] for r in success]),
                schedule_lag_all_us=distribution([r['started_us']-r['scheduled_us'] for r in group]), p99_claimed=False)
    captures = [r for r in rows if r['event'] == 'capture']
    if profile_reader is not None:
        profiles = profile_reader(captures, bulk_bytes)
    else:
        assert len(captures) == 4 and {r['name'] for r in captures} == {'x_before', 'gateway_before', 'x_after', 'gateway_after'}
        profiles = {r['name']: profile(r['capture']) for r in captures}
        assert all(profiles[side+'_before']['interface'] == profiles[side+'_after']['interface'] for side in ['x', 'gateway'])
        assert profiles['x_after']['bytes'] > profiles['x_before']['bytes']
        assert profiles['gateway_after']['bytes']-profiles['gateway_before']['bytes'] >= bulk_bytes
    overlap = {op:sum(r['phase'] == 'bulk' and r['operation'] == op and bulk['started_us'] <= r['started_us'] <= r['ended_us'] <= bulk['ended_us'] for r in samples) for op in ['directory', 'terminal']}
    assert all(complete[op+'_overlap'] == n and n >= 30 for op, n in overlap.items()) and complete['samples'] == 720
    functional = all(r['outcome'] == 'ok' and r['error'] is None for r in samples) and bulk['outcome'] == 'ok' and bulk['moved'] == bulk_bytes
    functional &= all(complete[k] is True for k in ['functional_ok', 'all_answers', 'exact_bytes', 'closed', 'direct_after'])
    if allow_failed:
        summaries = re.findall(r'^test result: (.*)$', path.read_text(), re.M)
        expected_footer = 'ok. 1 passed; 0 failed; 0 ignored;' if functional else 'FAILED. 0 passed; 1 failed; 0 ignored;'
        assert summaries[0].startswith(expected_footer)
    return dict(valid=True, functional_ok=functional, mode=mode, groups=groups, scheduling=scheduling, overlap=overlap,
                bulk=bulk, profiles=profiles, profile_readback_confirmed=True, full_N3_accepted=False)


def verify(report):
    report = Path(report)
    result = dict(schema=1, revision=REVISION, source=json.loads((report/'source.json').read_text()),
                  expected_order=MODES, rounds=[], passed=False, full_N3_accepted=False, slo=False, p99_claimed=False,
                  limits=['Diagnostic observer comparison; odd paired order is only partially balanced',
                          'Actual tc readback verified, separate kernel calibration remains its own evidence',
                          'No Node queue policy, host/child exit, CPU/energy or phone acceptance'])
    try:
        result['guard'] = guard(report)
        paths = sorted((report/'netlab').glob('*/scenarios.log'))
        assert len(paths) == len(MODES)
        for path, mode in zip(paths, MODES):
            row = dict(log=str(path.relative_to(report)), sha256=hashlib.sha256(path.read_bytes()).hexdigest(), valid=False, functional_ok=False)
            try:
                row.update(case(path, mode))
            except (AssertionError, KeyError, ValueError, TypeError, IndexError) as error:
                row['error'] = type(error).__name__ + ': ' + str(error)
            result['rounds'].append(row)
        result['passed'] = all(r['valid'] and r['functional_ok'] for r in result['rounds'])
    except (AssertionError, KeyError, ValueError, TypeError, IndexError) as error:
        result['error'] = type(error).__name__ + ': ' + str(error)
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1])
    result = verify(report)
    (report/'workload-observer-contracts.json').write_text(json.dumps(result, indent=2)+'\n')
    sys.exit(0 if result['passed'] else 1)
