"""Strict forwarding queue topology evidence, with the frozen combined control."""
import hashlib
import importlib.util
import ipaddress
import json
from pathlib import Path
import re
import sys

spec = importlib.util.spec_from_file_location('kernel_burst', Path(__file__).with_name('netlab-kernel-burst.py'))
burst = importlib.util.module_from_spec(spec); spec.loader.exec_module(burst)
high = burst.high
REVISION = 'workload_kernel_tcp_queues_v1'
ORDER = ('combined', 'separated', 'separated', 'combined')


def read(value, router):
    assert all(value[k]['ok'] is True for k in ['qdisc_json', 'qdisc_text', 'interfaces'])
    q = json.loads(value['qdisc_json']['stdout'])
    nets = [r for r in q if r['kind'] == 'netem']; rates = [r for r in q if r['kind'] == 'tbf']
    interfaces = json.loads(value['interfaces']['stdout'])
    if not router:
        assert not nets and not rates
        ifaces = [r for r in interfaces if r['ifname'] != 'lo']
        assert len(ifaces) == 1
        return dict(endpoint_interface=ifaces[0]['ifname'], interfaces={ifaces[0]['ifname']: ifaces[0]['stats64']})
    assert len(nets) == len(rates) == 1
    n, t = nets[0], rates[0]
    assert n['root'] is True and t['root'] is True and n['dev'] != t['dev']
    assert n['handle'] == '1:' and t['handle'] == '10:' and 'parent' not in t
    assert n['options']['delay'] == dict(delay=.15, jitter=0, correlation=0) and n['options']['limit'] == 1000
    assert not n['options'].get('loss') and not n['options'].get('gap') and not n['options'].get('ecn')
    assert t['options']['rate'] == 6250000 and t['options']['lat'] == 40000
    assert 65520 <= t['options']['burst'] <= 65536
    lines = value['qdisc_text']['stdout'].splitlines()
    netline = [l for l in lines if l.startswith('qdisc netem 1: dev '+n['dev']+' root ')]
    rateline = [l for l in lines if l.startswith('qdisc tbf 10: dev '+t['dev']+' root ')]
    assert len(netline) == len(rateline) == 1
    assert re.search(r'\bdelay 150(?:\.0)?ms\b', netline[0]) and re.search(r'\blimit 1000\b', netline[0])
    assert re.search(r'\brate 50(?:\.0)?Mbit\b', rateline[0]) and re.search(r'\blat 40(?:\.0)?ms\b', rateline[0])
    selected = {r['ifname']: r['stats64'] for r in interfaces if r['ifname'] in {n['dev'], t['dev']}}
    assert len(selected) == 2
    return dict(rate_interface=t['dev'], delay_interface=n['dev'], actual_burst_bytes=t['options']['burst'],
                rate_bytes=t['bytes'], rate_drops=t['drops'], delay_bytes=n['bytes'], delay_drops=n['drops'],
                interfaces=selected)


def captures(rows, bulk_bytes):
    sides = ['x', 'y', 'gateway', 'right_gateway']
    assert len(rows) == 8 and {r['name'] for r in rows} == {s+'_'+m for s in sides for m in ['before', 'after']}
    reads = {r['name']: read(r['capture'], r['name'].startswith(('gateway_', 'right_gateway_'))) for r in rows}
    counters = {}
    for side in sides:
        a, b = reads[side+'_before'], reads[side+'_after']
        immutable = ['rate_interface', 'delay_interface', 'actual_burst_bytes'] if 'gateway' in side else ['endpoint_interface']
        assert all(a[k] == b[k] for k in immutable) and a['interfaces'].keys() == b['interfaces'].keys()
        delta = {}
        for iface in a['interfaces']:
            delta[iface] = {}
            for direction in ['rx', 'tx']:
                count = b['interfaces'][iface][direction]['bytes']-a['interfaces'][iface][direction]['bytes']
                assert isinstance(count, int) and count >= bulk_bytes
                delta[iface][direction+'_bytes'] = count
        if 'gateway' in side:
            for key in ['rate_bytes', 'rate_drops', 'delay_bytes', 'delay_drops']:
                delta[key] = b[key]-a[key]
                assert delta[key] >= (bulk_bytes if key.endswith('_bytes') else 0)
        counters[side] = dict(before=a, after=b, delta=delta)
    return counters


def installs(rows, counters):
    installs = [r for r in rows if r['event'] == 'rate_install']
    assert len(installs) == 2 and {r['side'] for r in installs} == {'gateway', 'right_gateway'}
    for install in installs:
        before = install['before']; assert before['ok'] is True and before['timed_out'] is False
        q = json.loads(before['stdout']); assert not any(r['kind'] == 'tbf' for r in q)
        roots = [r for r in q if r['dev'] == install['device'] and r.get('root') is True]
        assert len(roots) == 1 and roots[0]['kind'] == 'noqueue'
        n = [r for r in q if r['kind'] == 'netem']; assert len(n) == 1 and n[0]['dev'] != install['device']
        assert n[0]['root'] is True and n[0]['options']['delay'] == dict(delay=.15, jitter=0, correlation=0)
        assert n[0]['options']['limit'] == 1000
        assert ipaddress.IPv4Address(install['uplink_ip']).is_loopback is False
        assert install['device'] == counters[install['side']]['before']['rate_interface']
        assert install['args'] == ['qdisc', 'replace', 'dev', install['device'], 'root', 'handle', '10:', 'tbf',
                                   'rate', '50mbit', 'burst', '65536', 'latency', '40ms']
        assert install['command']['ok'] is True and install['command']['timed_out'] is False


def separated(path):
    calibrated = high.case(path, revision=REVISION, allow_bounds_failure=True, profile_reader=captures)
    marker = '[netlab-workload-calibration] '
    rows = [json.loads(l.split(marker, 1)[1]) for l in path.read_text().splitlines() if marker in l]
    manifest = high.one(rows, 'manifest')
    assert manifest['separate_queues'] is True and manifest['requested_burst_bytes'] == 65536
    assert manifest['socket_diagnostic'] is False and not any(r['event'] in ['tcp_snapshot', 'shaper_change'] for r in rows)
    installs(rows, calibrated['profile_counters'])
    return calibrated


def verify(report):
    report = Path(report)
    result = dict(schema=1, revision=REVISION, source=json.loads((report/'source.json').read_text()),
                  expected_order=list(ORDER), results=[], observations_verified=False,
                  physical_calibration_accepted=False, full_N3_accepted=False, slo=False,
                  limits=['ABBA model topology diagnostic; not three independent repetitions of the candidate model',
                          'Topology and queue position change together; not a single-parameter causal claim',
                          'All original physical failures preserved; does not accept Node policy or N3'])
    paths = sorted((report/'netlab').glob('*/scenarios.log'))
    for i, path in enumerate(paths):
        row = dict(log=str(path.relative_to(report)), sha256=hashlib.sha256(path.read_bytes()).hexdigest(), valid=False)
        try:
            assert i < len(ORDER)
            row['layout'] = ORDER[i]
            if ORDER[i] == 'combined':
                row['calibration'] = burst.case(path, 65536)
                marker = '[netlab-workload-calibration] '
                records = [json.loads(l.split(marker, 1)[1]) for l in path.read_text().splitlines() if marker in l]
                assert high.one(records, 'manifest')['separate_queues'] is False
            else:
                row['calibration'] = separated(path)
            row['valid'] = True
        except (AssertionError, ValueError, KeyError, TypeError, IndexError, ZeroDivisionError) as error:
            row['error'] = type(error).__name__+': '+str(error)
        result['results'].append(row)
    result['observations_verified'] = len(paths) == len(ORDER) and all(r['valid'] for r in result['results'])
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1]); result = verify(report)
    (report/'workload-kernel-queues-contracts.json').write_text(json.dumps(result, indent=2)+'\n')
    sys.exit(0 if result['observations_verified'] else 1)
