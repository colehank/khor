"""Supplement unchanged recovery-decision evidence with strict retained-road facts.

A missing Go alone never passes. The optional continuity branch requires full
logs, the original physical and sampled-request gates, the same wire and only
carried selections, plus no peer selection/dial/retirement after the pivot.
"""
import datetime
import hashlib
import json
from pathlib import Path
import runpy
import sys


def selected_carried(event):
    selected = [m for m in event['details']['measurements'] if m['selected']]
    return len(selected) == 1 and selected[0]['carried'] and not selected[0]['relay']


def retained(folder, role, peer, pivot, calibration, attempts):
    files = list((folder / 'report/netlab').glob('*/diagnostics/peer-middle-v4-' + role + '.log'))
    assert len(files) == 1, 'unique complete role log required'
    hashes = json.loads((folder / 'report-sha256.json').read_text())
    def verified(path):
        data = path.read_bytes()
        assert hashlib.sha256(data).hexdigest() == hashes[str(path.relative_to(folder / 'report'))]
        return data.decode()
    raw = verified(files[0])
    assert 'mode=full samples=debug' in raw and 'lagged' not in raw.lower()
    transitions = []
    for line in raw.splitlines():
        header = line.split(' readings=', 1)[0]
        if peer not in header:
            continue
        initial = 'select without a current road' in header
        if not initial and not ('move decision' in header and 'step=Go' in header):
            continue
        at = datetime.datetime.fromisoformat(line.split()[0].replace('Z', '+00:00'))
        transitions.append(dict(at_ms=at.timestamp() * 1000, initial=initial,
                                carried='to=Custom' in header, raw=header))
    previous = [e for e in transitions if e['at_ms'] < pivot]
    assert previous and previous[-1]['carried'], 'last actual pre-pivot selection must be carried'
    assert not [e for e in transitions if e['at_ms'] >= pivot], 'any post-pivot selection/reset needs transition evidence'
    events = []
    for line in raw.splitlines():
        if '[khor-carried-lifecycle] ' in line:
            event = json.loads(line.split('[khor-carried-lifecycle] ', 1)[1])
            if event['phase'] == 'request_stream_opened' and event['peer'] == peer:
                events.append(event)
    pre = [e for e in events if e['at'] < pivot]
    post = [e for e in events if e['at'] >= pivot]
    assert pre and post and selected_carried(pre[-1]), 'actual before/after wire facts required'
    wire = pre[-1]['details']['wire']
    assert all(e['details']['wire'] == wire and selected_carried(e) for e in post)
    assert 0 <= post[0]['at'] - pivot <= 4000, 'first carried stream must open by four seconds'
    first_case = 'peer-middle-v4-first-' + ('ab' if role == 'x' else 'ba')
    finished = [e for e in attempts if e['case'] == first_case and e['phase'] == 'finished']
    assert len(finished) == 1 and finished[0]['outcome'] == 'ok'
    assert 0 <= finished[0]['at_ms'] - pivot <= 4000, 'successful actual first reply required by deadline'
    scenario_files = list((folder / 'report/netlab').glob('*/scenarios.log'))
    assert len(scenario_files) == 1
    journals = []
    for line in verified(scenario_files[0]).splitlines():
        if '[netlab-mobile-journal] ' in line:
            node, scope, data = line.split('[netlab-mobile-journal] ', 1)[1].split(' ', 2)
            event = json.loads(data)
            if node == role and event.get('peer') == peer:
                journals.append(event)
    pre_paths = [e for e in journals if e['e'] == 'path' and e['at'] < pivot and e.get('wire') == wire]
    assert pre_paths and pre_paths[-1]['road'] == 'carried', 'same-wire path journal must corroborate selection'
    forbidden = {'path', 'dial', 'gone', 'retired', 'closed', 'asleep', 'replaced'}
    assert not [e for e in journals if e['at'] >= pivot and e['e'] in forbidden], 'post-pivot path/dial/retirement prevents continuity proof'
    return dict(passed=True, method='retained_carried_state', wire=wire,
                first_stream_after_ms=post[0]['at'] - pivot,
                first_success_after_ms=finished[0]['at_ms'] - pivot,
                carried_post_streams=len(post), peer_transitions_after_pivot=0,
                log_sha256=hashlib.sha256(files[0].read_bytes()).hexdigest())


def check(folder):
    folder = Path(folder)
    mode = json.loads((folder / 'report/measurement-mode.json').read_text())
    assert mode['diagnostic'] == '1', 'complete full-sample profile required'
    cal = json.loads((folder / 'peer-mobility-calibration.json').read_text())
    requests = json.loads((folder / 'peer-selection-contract.json').read_text())
    source = json.loads((folder / 'report/source.json').read_text())
    assert cal['identity_verified'] and cal['availability_verified'] and requests['passed']
    assert cal['outcomes'] == {'ok':124}
    for key in ['tree','snapshot_commit']:
        assert cal['source'][key] == requests['source'][key] == source[key]
    legacy = runpy.run_path(str(Path(__file__).with_name('netlab-peer-decision.py')))['check'](folder)
    attempts = [r['event'] for r in json.loads((folder / 'report/attempts.json').read_text())['records']]
    ids = {}
    for role, direction in [('x','ab'),('y','ba')]:
        starts = [e for e in attempts if e['case'] == 'peer-middle-v4-first-' + direction and e['phase'] == 'started']
        assert len(starts) == 1
        ids[role] = starts[0]['from']
    roles = []
    for role, peer in [('x',ids['y']),('y',ids['x'])]:
        decision = next(e for e in legacy['decisions'] if e['role'] == role)
        if decision['passed']:
            evidence = dict(passed=True, method='original_recovery_transition', first_go_after_replug_us=decision['first_go_after_replug_us'], log_sha256=decision['log_sha256'])
        else:
            try:
                evidence = retained(folder, role, peer, requests['pivot_at_ms'], cal, attempts)
            except AssertionError as error:
                evidence = dict(passed=False, method='insufficient_retained_state', reason=str(error), original_decision=decision)
        roles.append(dict(role=role, **evidence))
    return dict(schema=1, contract='peer_relay_four_second_transition_or_retained_state_v1',
                run_id=folder.name, source=source, after_ms=4000,
                legacy_decision_gate_passed=legacy['passed'], original_request_gate_passed=requests['passed'],
                roles=roles, passed=all(e['passed'] for e in roles),
                limits=['Original physical fixture, request checker and recovery-decision checker unchanged',
                        'Retained state combines full transition logs, same-wire journals and actual first successful request; no per-packet/WAN/phone/energy claim',
                        'Post-hoc calibration is exploratory; freeze then require independent prospective repeats'])


if __name__ == '__main__':
    try:
        result = check(sys.argv[1])
    except (AssertionError, ValueError, KeyError, FileNotFoundError) as error:
        result = dict(schema=1, contract='peer_relay_four_second_transition_or_retained_state_v1',
                      run_id=Path(sys.argv[1]).name, passed=False, reason=str(error))
    print(json.dumps(result, indent=2))
    raise SystemExit(0 if result['passed'] else 1)
