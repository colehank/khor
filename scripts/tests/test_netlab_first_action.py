"""Retain caller failures and reject corrupted first-action/cost populations."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS/file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


first = load('first_action', 'netlab-first-action.py')
guard = load('first_action_guard_test', 'netlab-idle-action-guard.py')


def capture(start, side, slow, count):
    stats = {direction: dict(bytes=count*100, packets=count, errors=0, dropped=0)
             for direction in ['rx', 'tx']}
    identity = {'alpha':[('eth0',10,20),('eth1',11,21)],
                'beta':[('eth0',12,22),('eth1',13,23)],
                'second':[('v-a',21,11),('v-b',23,13)]}[side]
    interfaces = [dict(ifname=name, ifindex=index, link_index=peer, stats64=copy.deepcopy(stats))
                  for name,index,peer in identity]
    nets = [dict(kind='netem', dev=name, root=True,
                 options=dict(delay=dict(delay=.15, jitter=0, correlation=0)))
            for name in (['eth1'] if side == 'alpha' else ['v-a'])] if side != 'beta' and slow else []
    text = '\n'.join('qdisc netem 1: dev '+r['dev']+' root refcnt 2 limit 1000 delay 150ms' for r in nets)
    commands = {}
    for i, (name, stdout) in enumerate([('interfaces', json.dumps(interfaces)),
                                        ('qdisc_json', json.dumps(nets)), ('qdisc_text', text)]):
        commands[name] = dict(started_us=start+1+2*i, ended_us=start+2+2*i,
                              command=dict(ok=True, timed_out=False, status=0, stdout=stdout, stderr=''))
    return dict(started_us=start, ended_us=start+10, commands=commands)


def sample(name, index):
    side, phase = first.CASES[name]
    peer = 'beta' if side == 'alpha' else 'alpha'
    before = {d: capture(2, d, True, 10) for d in first.SIDES}
    pre_start = 10000050+phase*1000000
    pre = {d: capture(pre_start, d, False, 20) for d in first.SIDES}
    request_start = pre_start+20
    request_end = request_start+80010
    after = {d: capture(request_end+10, d, False, 30) for d in first.SIDES}
    row = dict(schema=1, revision=first.REVISION, case=name, **{'from':side}, to=peer,
               from_id=side+'-id', to_id=peer+'-id', phase_s=phase,
               direct_before_ms=300, direct_restored_ms=0, relay_edge_ms=40,
               quiet_ms=10000, caller_ms=30000, diagnostic='off', quiet_started_us=20,
               restore_started_us=10000020, restore_ended_us=10000040,
               request_started_us=request_start, request_ended_us=request_end,
               caller_elapsed_us=80000, error=None, functional_ok=True,
               before_idle=before, before_action=pre, after_action=after,
               prepared_paths={d:dict(hop='Relay', raw='Relay', observed_us=1) for d in ['alpha', 'beta']},
               paths_after={d:dict(hop='Direct', raw='Direct') for d in ['alpha', 'beta']},
               paths_observed_us=request_end+40, full_N1_accepted=False)
    common = dict(schema=1, measurement_revision='attempt_outcomes_v4', attempt=index+1,
                  pid=100, case=name, **{'from':side+'-id'}, to=peer, operation='list',
                  scope='logical_request', within_ms=30000)
    attempts = [dict(common, phase='started', at_ms=1000, elapsed_us=0, outcome=None, error=None),
                dict(common, phase='finished', at_ms=1080, elapsed_us=80000, outcome='ok', error=None)]
    return row, attempts


class FirstActionEvidence(unittest.TestCase):
    def test_population_geometry_cost_and_failed_caller(self):
        mutations = ['green', 'remote_error', 'timeout', 'missing_case', 'duplicate_case',
                     'wrong_phase', 'short_quiet', 'wrong_delay', 'restore_still_delayed',
                     'counter_decrease', 'missing_interface', 'missing_attempt',
                     'duplicate_attempt', 'early_path_read', 'wrong_footer', 'wrong_mode',
                     'missing_router', 'wrong_peer', 'wrong_ingress_delay']
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as tmp:
                report = Path(tmp)
                (report/'source.json').write_text(json.dumps(dict(tree='d'*40)))
                (report/'measurement-mode.json').write_text(json.dumps(guard.MODE))
                mode = copy.deepcopy(first.MODE)
                if mutation == 'wrong_mode':
                    mode['test_threads'] = 2
                (report/'first-action-mode.json').write_text(json.dumps(mode))
                path = report/'netlab/0/scenarios.log';path.parent.mkdir(parents=True)
                path.write_text('\n'.join([
                    '[netlab-result] p8g-with-cancel | green | caller_drop_released_device_future true',
                    '[netlab-result] p8g-detached-control | green | detached_future_detected_and_reaped true',
                    'test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1s']))
                values = [sample(name, i) for i, name in enumerate(first.CASES)]
                row, attempts = values[0]
                good = 6
                if mutation in ['remote_error', 'timeout']:
                    row.update(error='caller failure', functional_ok=False)
                    attempts[1].update(error='caller failure', outcome='error' if mutation == 'remote_error' else 'timeout')
                    good = 5
                elif mutation == 'missing_case':
                    values.pop()
                elif mutation == 'duplicate_case':
                    values[-1] = copy.deepcopy(values[0])
                elif mutation == 'wrong_phase':
                    row['phase_s'] = 14
                elif mutation == 'short_quiet':
                    row['quiet_started_us'] += 1
                elif mutation == 'wrong_delay':
                    command = row['before_idle']['alpha']['commands']['qdisc_json']['command']
                    command['stdout'] = command['stdout'].replace('0.15', '0.015')
                elif mutation == 'restore_still_delayed':
                    row['before_action']['alpha']['commands']['qdisc_json']['command'] = copy.deepcopy(
                        row['before_idle']['alpha']['commands']['qdisc_json']['command'])
                elif mutation == 'counter_decrease':
                    command = row['after_action']['alpha']['commands']['interfaces']['command']
                    data = json.loads(command['stdout']);data[0]['stats64']['rx']['packets'] = 0
                    command['stdout'] = json.dumps(data)
                elif mutation == 'missing_interface':
                    command = row['before_action']['beta']['commands']['interfaces']['command']
                    command['stdout'] = json.dumps(json.loads(command['stdout'])[:1])
                elif mutation == 'missing_attempt':
                    attempts.pop()
                elif mutation == 'duplicate_attempt':
                    values[1][1][0]['attempt'] = 1
                    values[1][1][1]['attempt'] = 1
                elif mutation == 'early_path_read':
                    row['paths_observed_us'] = row['request_started_us']
                elif mutation == 'missing_router':
                    del row['before_action']['second']
                elif mutation == 'wrong_peer':
                    command = row['before_idle']['second']['commands']['interfaces']['command']
                    data = json.loads(command['stdout']);data[0]['link_index'] = 111
                    command['stdout'] = json.dumps(data)
                elif mutation == 'wrong_ingress_delay':
                    command = row['before_idle']['second']['commands']['qdisc_json']['command']
                    command['stdout'] = command['stdout'].replace('0.15', '0.015')
                lines = []
                for value, calls in values:
                    lines.extend('[netlab-attempt] '+json.dumps(a) for a in calls)
                    lines.append('[netlab-first-action] '+json.dumps(value))
                    lines.append('[netlab-result] '+value['case']+' | '+
                                 ('green' if value['functional_ok'] else 'red')+' | measurement only')
                status = 'ok' if good == 6 else 'FAILED'
                footer = f'test result: {status}. {good} passed; {6-good} failed; 0 ignored; 0 measured; 0 filtered out; finished in 1s'
                if mutation == 'wrong_footer':
                    footer = footer.replace('6 passed', '5 passed')
                lines.append(footer)
                path = report/'first-actions/0/scenarios.log';path.parent.mkdir(parents=True)
                path.write_text('\n'.join(lines)+'\n')
                result = first.verify(report)
                complete = mutation in ['green', 'remote_error', 'timeout']
                self.assertEqual(result['complete'], complete, result)
                self.assertEqual(result['passed'], mutation == 'green', result)
                if complete:
                    self.assertEqual(result['actual_first_action_samples'], 6)
                    self.assertEqual(result['cases'][0]['counters']['alpha']['action']['delta']['eth0']['rx']['packets'], 10)
                self.assertFalse(result['full_N1_accepted'])
                self.assertFalse(result['energy_measured'])
                self.assertFalse(result['continuous_stream_silence_verified'])
                self.assertFalse(result['full_quiet_contract_accepted'])
