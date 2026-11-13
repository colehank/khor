"""Strict high-BDP kernel TCP calibration; reads data only."""
import hashlib
import json
from pathlib import Path
import re
import sys

REVISION = 'workload_kernel_tcp_calibration_high_v1'
BYTES = 96 * 1024 * 1024


def one(rows, event):
    found = [r for r in rows if r['event'] == event]
    assert len(found) == 1
    return found[0]


def capture(value):
    assert all(value[k]['ok'] is True for k in ['qdisc_json', 'qdisc_text', 'interfaces'])
    rows = json.loads(value['qdisc_json']['stdout'])
    nets = [r for r in rows if r['kind'] == 'netem']
    rates = [r for r in rows if r['kind'] == 'tbf']
    assert len(nets) == len(rates) == 1
    n, t = nets[0], rates[0]
    assert n['root'] is True and n['handle'] == '1:' and t['parent'] == '1:1' and n['dev'] == t['dev']
    assert n['options']['delay'] == dict(delay=.15, jitter=0, correlation=0)
    assert t['options']['rate'] == 6250000 and t['options']['lat'] == 40000
    assert isinstance(t['options']['burst'], int) and t['options']['burst'] > 0
    text = value['qdisc_text']['stdout']
    assert re.search(r'\bdelay 150(?:\.0)?ms\b', text)
    assert re.search(r'\brate 50(?:\.0)?Mbit\b', text) and re.search(r'\blat 40(?:\.0)?ms\b', text)
    interfaces = json.loads(value['interfaces']['stdout'])
    iface = [r for r in interfaces if r['ifname'] == n['dev']]
    assert len(iface) == 1
    for direction in ['rx', 'tx']:
        assert isinstance(iface[0]['stats64'][direction]['bytes'], int)
    return dict(interface=n['dev'], netem_bytes=n['bytes'], netem_drops=n['drops'],
                tbf_bytes=t['bytes'], tbf_drops=t['drops'], actual_burst_bytes=t['options']['burst'],
                actual_netem_limit=n['options']['limit'],
                interface_rx_bytes=iface[0]['stats64']['rx']['bytes'], interface_tx_bytes=iface[0]['stats64']['tx']['bytes'])


def case(path, revision=REVISION, allow_bounds_failure=False, bulk_bytes=BYTES, profile_reader=None):
    text = path.read_text()
    marker = '[netlab-workload-calibration] '
    lines = [line for line in text.splitlines() if marker in line]
    assert lines and all(line.count(marker) == 1 for line in lines)
    rows = [json.loads(line.split(marker, 1)[1]) for line in lines]
    assert all(r['schema'] == 1 and r['revision'] == revision for r in rows)
    manifest, complete, connection = (one(rows, e) for e in ['manifest', 'complete', 'connect'])
    assert manifest['warmup'] == 4 and manifest['echoes'] == 64 and manifest['echo_bytes'] == 64
    assert manifest['bulk_bytes'] == bulk_bytes and manifest['bulk_caller_us'] == 240000000
    assert manifest['rtt_ms'] == 300 and manifest['mbit_each_direction'] == 50 and manifest['queue_ms'] == 40
    assert manifest['min_rtt_us_bounds'] == [270000, 420000] and manifest['goodput_mbit_bounds'] == [40.0, 60.0]
    assert manifest['layer'] == 'kernel_TCP' and manifest['residents'] is False and manifest['slo'] is False
    assert manifest['qdisc_units_review_required'] is True
    echoes = [r for r in rows if r['event'] == 'echo']
    bulk = [r for r in rows if r['event'] == 'bulk']
    captures = [r for r in rows if r['event'] == 'capture']
    assert len(echoes) == 68 and {r['index'] for r in echoes} == set(range(68))
    assert all(r['warmup'] == (r['index'] < 4) and r['bytes'] == 64 for r in echoes)
    assert len(bulk) == 2 and {r['direction'] for r in bulk} == {'upload', 'download'}
    assert all(r['bytes'] == bulk_bytes for r in bulk)
    assert all(r['ended_us'] >= r['started_us'] and r['elapsed_us'] == r['ended_us']-r['started_us'] for r in echoes+bulk)
    assert all(r['elapsed_us'] > 0 for r in bulk)
    if profile_reader is not None:
        counters = profile_reader(captures, bulk_bytes)
    else:
        assert len(captures) == 4 and {r['name'] for r in captures} == {'x_before', 'gateway_before', 'x_after', 'gateway_after'}
        reads = {r['name']: capture(r['capture']) for r in captures}
        counters = {}
        for side in ['x', 'gateway']:
            before, after = reads[side+'_before'], reads[side+'_after']
            assert all(before[k] == after[k] for k in ['interface', 'actual_burst_bytes', 'actual_netem_limit'])
            delta = {k:after[k]-before[k] for k in ['netem_bytes', 'netem_drops', 'tbf_bytes', 'tbf_drops', 'interface_rx_bytes', 'interface_tx_bytes']}
            assert all(v >= 0 for v in delta.values()) and delta['netem_bytes'] >= bulk_bytes
            assert delta['interface_rx_bytes'] >= bulk_bytes and delta['interface_tx_bytes'] >= bulk_bytes
            counters[side] = dict(before=before, after=after, delta=delta)
    successful = [r['elapsed_us'] for r in echoes if not r['warmup'] and r['outcome'] == 'ok']
    minimum = min(successful) if successful else None
    rates = [bulk_bytes*8/r['elapsed_us'] for r in bulk]
    bounds = minimum is not None and 270000 <= minimum <= 420000 and all(40 <= v <= 60 for v in rates)
    assert complete['samples'] == 70 and complete['min_rtt_us'] == minimum and complete['goodput_mbit'] == rates
    assert complete['bounds_ok'] == bounds and complete['calibrated'] is False and complete['qdisc_units_review_required'] is True
    summaries = re.findall(r'^test result: (.*)$', text, re.M)
    transport_ok = connection['ok'] is True and all(r['outcome'] == 'ok' and r['error'] is None for r in echoes+bulk)
    transport_ok &= all(r['exact_bytes'] is True for r in bulk)
    transport_ok &= complete['server_ok'] is True and complete['qdiscs_present'] is True
    functional = bounds and transport_ok
    assert complete['functional_and_bounds_ok'] == functional
    expected = 'ok. 1 passed; 0 failed; 0 ignored;'
    if allow_bounds_failure:
        assert transport_ok
        if not bounds: expected = 'FAILED. 0 passed; 1 failed; 0 ignored;'
    assert len(summaries) == 1 and summaries[0].startswith(expected)
    return dict(functional_ok=functional, transport_ok=transport_ok, min_rtt_us=minimum, goodput_mbit=rates,
                warmup_population=4, steady_echo_population=64, bulk_bytes_each_direction=bulk_bytes,
                profile_counters=counters, profile_readback_confirmed=True)


def verify(report):
    report = Path(report)
    result = dict(schema=1, revision=REVISION, source=json.loads((report/'source.json').read_text()),
                  results=[], passed=False, calibrated=False, slo=False, full_N3_accepted=False,
                  limits=['Independent kernel TCP model only, not Noq/Node policy or throughput acceptance',
                          'All warmup/steady/transfer outcomes retained in raw logs; no p99, phone or energy acceptance'])
    paths = sorted((report/'netlab').glob('*/scenarios.log'))
    for path in paths:
        row = dict(log=str(path.relative_to(report)), sha256=hashlib.sha256(path.read_bytes()).hexdigest(), valid=False)
        try:
            row['calibration'] = case(path)
            row['valid'] = row['calibration']['functional_ok']
        except (AssertionError, ValueError, KeyError, TypeError, IndexError, ZeroDivisionError) as error:
            row['error'] = type(error).__name__+': '+str(error)
        result['results'].append(row)
    result['passed'] = len(paths) == 3 and all(r['valid'] for r in result['results'])
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1]); result = verify(report)
    (report/'workload-kernel-high-contracts.json').write_text(json.dumps(result, indent=2)+'\n')
    sys.exit(0 if result['passed'] else 1)
