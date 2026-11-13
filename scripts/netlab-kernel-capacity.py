"""Independent fixed UDP capacity evidence; never rewrites the TCP/model gates."""
import collections
import hashlib
import importlib.util
import ipaddress
import json
import math
from pathlib import Path
import re
import sys

spec = importlib.util.spec_from_file_location('kernel_queues', Path(__file__).with_name('netlab-kernel-queues.py'))
queues = importlib.util.module_from_spec(spec); spec.loader.exec_module(queues)
REVISION = 'workload_kernel_udp_capacity_v2'
SIZE = 1200
RATE = 70000000
PACKETS = 20*RATE//(SIZE*8)
WORDS = (PACKETS+63)//64
TAGS = dict(upload=0x7564705f75705f31, download=0x7564705f646e5f31)


def bitmap(value):
    assert isinstance(value, list) and len(value) == WORDS
    assert all(type(w) is int and 0 <= w < 1 << 64 for w in value)
    assert value[-1] >> (PACKETS % 64) == 0
    return sum(bin(w).count('1') for w in value)


def buckets(value, count):
    assert len(value) == count and all(type(n) is int and n >= 0 for n in value)
    return sum(value)


def address(value):
    ip, port = value.rsplit(':', 1)
    addr = ipaddress.IPv4Address(ip)
    assert not addr.is_unspecified and not addr.is_loopback and 0 < int(port) <= 65535
    return ip


def direction(row):
    assert row['tag'] == TAGS[row['direction']]
    s, r = row['sender'], row['receiver']
    sent, failed, received = bitmap(s['sent']), bitmap(s['failed']), bitmap(r['received'])
    assert not any(a & b for a, b in zip(s['sent'], s['failed']))
    assert not any(a & ~b for a, b in zip(r['received'], s['sent']))
    assert failed == 0 and s['first_error'] is None and sent*100 >= PACKETS*99
    assert 19000000 <= s['elapsed_us'] <= 21000000 and 0 < s['ticks'] <= 4000
    assert sent <= s['ticks']*37 and 0 <= s['max_lateness_us'] <= 20000
    assert 0 <= s['late_over_10ms']*100 <= s['ticks']
    assert buckets(s['by_second'], 21) == sent
    sender_rate = sum(s['by_second'][5:15])*SIZE*8/10000000
    assert 65 <= sender_rate <= 75
    assert 0 <= r['first_after_us'] <= 2000000 and 24000000 <= r['elapsed_us'] <= 26000000
    assert buckets(r['by_second'], 25) == received and r['datagrams'] == received
    assert all(r[k] == 0 for k in ['duplicates', 'malformed', 'foreign', 'out_of_range'])
    assert s['local'] == r['peer'] and s['peer'] == r['local']
    source, dest = address(s['local']), address(s['peer']); assert source != dest
    assert row['sent_packets'] == sent and row['unique_packets'] == received
    assert row['missing_packets'] == sent-received and row['unique_payload_bytes'] == received*SIZE
    rate = sum(r['by_second'][5:15])*SIZE*8/10000000
    assert math.isclose(row['receiver_steady_mbit'], rate, rel_tol=1e-12)
    capacity = 40 <= rate <= 60
    assert row['capacity_bounds_ok'] == capacity and row['observations_ok'] is True
    return dict(direction=row['direction'], source_ip=source, destination_ip=dest,
                sender_local=s['local'], receiver_local=r['local'], sent_packets=sent,
                failed_packets=failed, pacer_skipped_packets=PACKETS-sent-failed,
                unique_packets=received, missing_packets=sent-received,
                unique_payload_bytes=received*SIZE, sender_steady_mbit=sender_rate,
                receiver_steady_mbit=rate, capacity_bounds_ok=capacity,
                sender_elapsed_us=s['elapsed_us'], receiver_elapsed_us=r['elapsed_us'],
                sender_max_lateness_us=s['max_lateness_us'], sender_late_over_10ms=s['late_over_10ms'],
                receiver_first_after_us=r['first_after_us'])


def case(path):
    raw = path.read_text(); marker = '[netlab-workload-calibration] '
    lines = [l for l in raw.splitlines() if marker in l]
    assert lines and all(l.count(marker) == 1 for l in lines)
    rows = [json.loads(l.split(marker, 1)[1]) for l in lines]
    assert all(r['schema'] == 1 and r['revision'] == REVISION for r in rows)
    assert collections.Counter(r['event'] for r in rows) == dict(rate_install=2,udp_manifest=1,capture=8,udp_direction=2,udp_complete=1)
    m = queues.high.one(rows, 'udp_manifest'); done = queues.high.one(rows, 'udp_complete')
    expected = dict(layout='forwarding_split',payload_bytes=1200,offered_bit_per_sec=RATE,planned_packets=PACKETS,
        send_seconds=20,receive_seconds=25,ready_limit_seconds=2,direction_limit_seconds=28,steady_seconds=[5,15],
        rtt_ms=300,mbit_each_direction=50,queue_ms=40,burst_bytes=65536,capacity_mbit_bounds=[40,60],
        sender_steady_mbit_bounds=[65,75],receiver_first_limit_us=2000000,pacer_tick_us=5000,
        pacer_max_lateness_us=20000,pacer_late_10ms_percent_limit=1,pacer_skip_percent_limit=1,
        layer='kernel_UDP',residents=False,slo=False,physical_calibration_accepted=False)
    assert all(m[k] == v for k, v in expected.items())
    measured = [direction(r) for r in rows if r['event'] == 'udp_direction']
    assert [d['direction'] for d in measured] == ['upload', 'download']
    assert measured[0]['source_ip'] == measured[1]['destination_ip'] and measured[0]['destination_ip'] == measured[1]['source_ip']
    captures = [r for r in rows if r['event'] == 'capture']
    minimum = min(d['unique_payload_bytes'] for d in measured); assert minimum > 0
    counters = queues.captures(captures, minimum)
    queues.installs(rows, counters)
    for cap in captures:
        ifaces = json.loads(cap['capture']['interfaces']['stdout'])
        assert all(i['mtu'] >= 1280 for i in ifaces if i['ifname'] in counters[cap['name'].rsplit('_',1)[0]]['before']['interfaces'])
    bounds = all(d['capacity_bounds_ok'] for d in measured)
    assert done['observations_ok'] is True and done['capacity_bounds_ok'] == bounds
    assert done['physical_calibration_accepted'] is False and done['full_N3_accepted'] is False and done['slo'] is False
    footer = re.findall(r'^test result: (.*)$', raw, re.M)
    expected_footer = 'ok. 1 passed; 0 failed; 0 ignored;' if bounds else 'FAILED. 0 passed; 1 failed; 0 ignored;'
    assert len(footer) == 1 and footer[0].startswith(expected_footer)
    return dict(directions=measured, profile_counters=counters, capacity_bounds_ok=bounds)


def verify(report):
    report = Path(report)
    result = dict(schema=1,revision=REVISION,source=json.loads((report/'source.json').read_text()),results=[],
        observations_verified=False,capacity_bounds_met=False,physical_calibration_accepted=False,full_N3_accepted=False,slo=False,
        limits=['Independent UDP capacity diagnosis only; original TCP failures and gates remain unchanged',
                'Overload losses and pacer skips retain their complete denominators; no Noq/Node/phone/energy acceptance'])
    paths = sorted((report/'netlab').glob('*/scenarios.log'))
    for path in paths:
        row = dict(log=str(path.relative_to(report)),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),valid=False)
        try:
            row['measurement'] = case(path); row['valid'] = True
        except (AssertionError,ValueError,KeyError,TypeError,IndexError,ZeroDivisionError) as error:
            row['error'] = type(error).__name__+': '+str(error)
        result['results'].append(row)
    result['observations_verified'] = len(paths) == 3 and all(r['valid'] for r in result['results'])
    result['capacity_bounds_met'] = result['observations_verified'] and all(r['measurement']['capacity_bounds_ok'] for r in result['results'])
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1]); result = verify(report)
    (report/'workload-kernel-capacity-contracts.json').write_text(json.dumps(result,indent=2)+'\n')
    sys.exit(0 if result['observations_verified'] else 1)
