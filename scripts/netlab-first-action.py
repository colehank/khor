"""Six first reads and per-interface cost intervals; no recovery or energy SLO."""
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys

REVISION = 'first_foreground_after_idle_v2'
SIDES = ['alpha', 'beta', 'second']
CASES = {f'p8-{side}-{phase}': (side, phase)
         for side in ['alpha', 'beta'] for phase in [0, 14, 5]}
MODE = dict(layer='node_first_action_measurement', measurement_revision=REVISION,
            diagnostic='off', test_threads=1, ignored=True, slo=False,
            guard_filter='paths::idle_action::p8g_', expected_guard_tests=2,
            action_filter='paths::first_action::p8_', expected_action_tests=6,
            cases=list(CASES), vantage_points=SIDES, full_N1_accepted=False)


def need(ok, message):
    if not ok:
        raise ValueError(message)


def integer(value):
    need(type(value) is int and value >= 0, 'nonnegative integer counter/time required')
    return value


def snapshot(value, side, slow):
    began, ended = (integer(value[k]) for k in ['started_us', 'ended_us'])
    need(began <= ended, 'snapshot time reversed')
    commands = value['commands']
    need(set(commands) == {'interfaces', 'qdisc_json', 'qdisc_text'}, 'snapshot command population')
    last = began
    for name in ['interfaces', 'qdisc_json', 'qdisc_text']:
        row = commands[name]
        start, end = (integer(row[k]) for k in ['started_us', 'ended_us'])
        need(last <= start <= end <= ended, 'command ordering')
        last = end
        command = row['command']
        need(command['ok'] is True and command['timed_out'] is False and command['status'] == 0,
             'snapshot command failed')
    interfaces = json.loads(commands['interfaces']['command']['stdout'])
    selected_rows = [r for r in interfaces if r['ifname'] != 'lo' and
                     (side == 'second' or r['ifname'] in ['eth0', 'eth1'])]
    selected = {r['ifname']: r['stats64'] for r in selected_rows}
    if side == 'second':
        need(len(selected) >= 2, 'router ports required')
    else:
        need(set(selected) == {'eth0', 'eth1'}, 'both outer interfaces required')
    links = {r['ifname']: dict(ifindex=integer(r['ifindex']), link_index=r.get('link_index'))
             for r in selected_rows}
    for stats in selected.values():
        for direction in ['rx', 'tx']:
            for key in ['bytes', 'packets', 'errors', 'dropped']:
                integer(stats[direction][key])
    qdiscs = json.loads(commands['qdisc_json']['command']['stdout'])
    nets = [r for r in qdiscs if r['kind'] == 'netem']
    text = commands['qdisc_text']['command']['stdout']
    if slow and side in ['alpha', 'second']:
        need(len(nets) == 1, 'one delay queue in each endpoint/router namespace required')
        if side == 'alpha':
            need(nets[0]['dev'] == 'eth1', 'alpha egress delay on eth1')
        for row in nets:
            need(row['root'] is True and row['options']['delay'] ==
                 dict(delay=.15, jitter=0, correlation=0), 'initial 150ms per direction readback')
            need(not any(row['options'].get(k) for k in ['loss', 'gap', 'ecn']), 'unexpected fault')
            lines = [line for line in text.splitlines()
                     if line.startswith('qdisc netem ') and ' dev '+row['dev']+' root ' in line]
            need(len(lines) == 1 and re.search(r'\bdelay 150(?:\.0)?ms\b', lines[0]),
                 'text delay units readback')
    else:
        need(not nets and not re.search(r'^qdisc netem ', text, re.M), 'restored path still delayed')
    need(not any(r['kind'] == 'tbf' for r in qdiscs), 'unexpected rate cap')
    return dict(started_us=began, ended_us=ended, interfaces=selected, links=links,
                netem_devices=[r['dev'] for r in nets],
                counter_command_started_us=commands['interfaces']['started_us'],
                counter_command_ended_us=commands['interfaces']['ended_us'])


def interval(before, after):
    need(before['ended_us'] <= after['started_us'], 'cost snapshots overlap')
    delta = {}
    need(before['links'] == after['links'], 'interface identities changed')
    need(before['interfaces'].keys() == after['interfaces'].keys(), 'interface population changed')
    for iface in before['interfaces']:
        delta[iface] = {}
        for direction in ['rx', 'tx']:
            delta[iface][direction] = {}
            for key in ['bytes', 'packets', 'errors', 'dropped']:
                number = after['interfaces'][iface][direction][key]-before['interfaces'][iface][direction][key]
                need(number >= 0, 'interface counter decreased')
                delta[iface][direction][key] = number
    return dict(delta=delta,
                elapsed_us_bounds=[after['counter_command_started_us']-before['counter_command_ended_us'],
                                   after['counter_command_ended_us']-before['counter_command_started_us']],
                includes_snapshot_gap=True, probe_only=False)


def case(row, attempts, *, revision=REVISION, quiet_ms=10000, diagnostic='off'):
    name = row['case']
    side, phase = CASES[name]
    peer = 'beta' if side == 'alpha' else 'alpha'
    for key, expected in dict(schema=1, revision=revision, phase_s=phase, direct_before_ms=300,
                              direct_restored_ms=0, relay_edge_ms=40, quiet_ms=quiet_ms,
                              caller_ms=30000, diagnostic=diagnostic, full_N1_accepted=False).items():
        need(row[key] == expected, 'first-action configuration: '+key)
    need(row['from'] == side and row['to'] == peer, 'first-action direction')
    times = {key: integer(row[key]) for key in ['quiet_started_us', 'restore_started_us',
             'restore_ended_us', 'request_started_us', 'request_ended_us', 'caller_elapsed_us', 'paths_observed_us']}
    need(times['restore_started_us']-times['quiet_started_us'] >= quiet_ms*1000, 'quiet interval shortened')
    need(times['restore_ended_us'] >= times['restore_started_us'], 'restore ordering')
    need(times['request_started_us']-times['restore_ended_us'] >= phase*1000000, 'nominal phase shortened')
    need(times['request_ended_us']-times['request_started_us'] >= times['caller_elapsed_us'], 'caller elapsed')
    reads = {label: {device: snapshot(row[label][device], device, label == 'before_idle')
                    for device in SIDES}
             for label in ['before_idle', 'before_action', 'after_action']}
    for label in reads:
        need(set(row[label]) == set(SIDES), 'snapshot vantage population')
        for device in ['alpha', 'beta']:
            endpoint = reads[label][device]['links']['eth1']
            peers = [name for name, link in reads[label]['second']['links'].items()
                     if link['ifindex'] == endpoint['link_index'] and link['link_index'] == endpoint['ifindex']]
            need(len(peers) == 1, 'cross-namespace veth identity')
            if device == 'alpha':
                need(reads[label]['second']['netem_devices'] == (peers if label == 'before_idle' else []),
                     'ingress delay must be on alpha peer veth')
    for device in SIDES:
        if device != 'second':
            prepared = row['prepared_paths'][device]
            need(prepared['hop'] == 'Relay' and integer(prepared['observed_us']) <= reads['before_idle'][device]['started_us'],
                 'prepared relay path')
        need(reads['before_idle'][device]['ended_us'] <= times['quiet_started_us'], 'idle start before snapshot')
        need(times['restore_ended_us'] <= reads['before_action'][device]['started_us'], 'pre-action snapshot before restore')
        need(reads['before_action'][device]['ended_us'] <= times['request_started_us'], 'pre-action snapshot after request')
        need(times['request_ended_us'] <= reads['after_action'][device]['started_us'], 'post snapshot before request end')
        need(reads['after_action'][device]['ended_us'] <= times['paths_observed_us'], 'path read before cost interval end')
    need(len(attempts) == 2 and [a['phase'] for a in attempts] == ['started', 'finished'], 'one complete logical request')
    a, b = attempts
    for key in ['attempt', 'pid', 'case', 'from', 'to', 'operation', 'scope', 'within_ms']:
        need(a[key] == b[key], 'attempt identity changed')
    for item in attempts:
        need(item['schema'] == 1 and item['measurement_revision'] == 'attempt_outcomes_v4', 'caller revision')
        need(integer(item['pid']) > 0 and integer(item['attempt']) > 0, 'caller identity')
    need(integer(b['at_ms']) >= integer(a['at_ms']), 'caller wall clock ordering')
    need(a['case'] == name and a['to'] == peer and a['within_ms'] == 30000 and
         a['from'] == row['from_id'] and a['operation'] == 'list' and
         a['scope'] == 'logical_request', 'logical request contract')
    need(a['outcome'] is None and a['error'] is None and a['elapsed_us'] == 0, 'request start fields')
    need(b['elapsed_us'] == times['caller_elapsed_us'] and b['error'] == row['error'], 'request completion fields')
    need(b['outcome'] in ['ok', 'error', 'timeout'], 'request outcome')
    success = b['outcome'] == 'ok'
    need(success == (row['error'] is None) == row['functional_ok'], 'functional result contradiction')
    return dict(case=name, direction=side+'->'+peer, phase_s=phase, outcome=b['outcome'], error=row['error'],
                caller_elapsed_us=times['caller_elapsed_us'],
                restore_to_request_us=times['request_started_us']-times['restore_ended_us'],
                paths_after=row['paths_after'], prepared_paths=row['prepared_paths'],
                counters={device: {label: interval(reads[first][device], reads[last][device])
                                  for label, first, last in [('idle', 'before_idle', 'before_action'),
                                                             ('action', 'before_action', 'after_action')]}
                          for device in SIDES}, functional_ok=success)


def verify(report):
    report = Path(report)
    result = dict(schema=1, suite=REVISION, source=json.loads((report/'source.json').read_text()),
                  complete=False, passed=False, cases=[], actual_first_action_samples=0,
                  p99_claimed=False, full_N1_accepted=False, energy_measured=False,
                  continuous_stream_silence_verified=False, full_quiet_contract_accepted=False,
                  limits=['One fresh case per direction/phase; no latency SLO or tail estimate',
                          'Interface totals include all wire traffic and snapshot gaps; not probe-only',
                          'Endpoint counters overlap the same traffic; do not sum endpoints as unique bytes',
                          'No CPU, wakeups, phone energy or final recovery acceptance'])
    try:
        need(json.loads((report/'first-action-mode.json').read_text()) == MODE, 'measurement mode')
        spec = importlib.util.spec_from_file_location('first_action_guard', Path(__file__).with_name('netlab-idle-action-guard.py'))
        guard = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(guard)
        result['guard'] = guard.verify(report)
        need(result['guard']['complete'] and result['guard']['passed'], 'ownership admission')
        logs = sorted((report/'first-actions').glob('*/scenarios.log'))
        need(len(logs) == 1, 'first-action log population')
        path = logs[0]; raw = path.read_text()
        need('[netlab-diagnostic]' not in raw and not list((path.parent/'diagnostics').glob('*')), 'diagnostics enabled')
        rows = [json.loads(line.split('[netlab-first-action] ', 1)[1]) for line in raw.splitlines()
                if '[netlab-first-action] ' in line]
        need(len(rows) == 6 and [r['case'] for r in rows] == list(CASES), 'first-action case population/order')
        attempts = [json.loads(line.split('[netlab-attempt] ', 1)[1]) for line in raw.splitlines()
                    if '[netlab-attempt] ' in line]
        need(len(attempts) == 12 and {a['case'] for a in attempts} == set(CASES), 'caller population')
        for row in rows:
            result['cases'].append(case(row, [a for a in attempts if a['case'] == row['case']]))
        notes = re.findall(r'^\[netlab-result\] (p8-[^ |]+) \| (green|red) \| ([^\n]+)', raw, re.M)
        need(len(notes) == 6 and [n[0] for n in notes] == list(CASES), 'scenario result population')
        for note, value in zip(notes, result['cases']):
            need((note[1] == 'green') == value['functional_ok'], 'scenario result verdict')
        need(len({(a['pid'], a['attempt']) for a in attempts}) == 6, 'duplicate caller identity')
        footers = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw, re.M)
        need(len(footers) == 1, 'Rust footer population')
        status, good, bad, ignored = footers[0]
        passed = sum(c['functional_ok'] for c in result['cases'])
        need((int(good), int(bad), int(ignored)) == (passed, 6-passed, 0), 'Rust result population')
        need(status == ('ok' if passed == 6 else 'FAILED'), 'Rust result status')
        result.update(complete=True, passed=passed == 6, actual_first_action_samples=6,
                      log=str(path.relative_to(report)), sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                      rust=dict(status=status, passed=int(good), failed=int(bad), ignored=0))
    except (AssertionError, KeyError, ValueError, TypeError, OSError, IndexError) as error:
        result.update(complete=False, passed=False, error=type(error).__name__+': '+str(error))
    return result


if __name__ == '__main__':
    report = Path(sys.argv[1])
    value = verify(report)
    (report/'first-action-contracts.json').write_text(json.dumps(value, indent=2)+'\n')
    sys.exit(0 if value['passed'] else 1)
