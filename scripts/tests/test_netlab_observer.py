"""Synthetic evidence tests only: no Rust, network namespaces or product runs."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('observer_evidence', Path(__file__).resolve().parents[1] / 'netlab-observer.py')
observer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(observer)


def capture(name):
    device = 'v7' if name.startswith('gateway') else 'eth0'
    rows = [dict(kind='netem', dev=device, root=True, handle='1:', bytes=0 if name.endswith('before') else 20000000,
                 drops=0, options=dict(delay=dict(delay=.02, jitter=0, correlation=0))),
            dict(kind='tbf', dev=device, parent='1:1', options=dict(rate=1000000, burst=4096, lat=40000))]
    return dict(qdisc_json=dict(ok=True, stdout=json.dumps(rows)),
                qdisc_text=dict(ok=True, stdout=f'qdisc netem 1: dev {device} root delay 20.0ms\nqdisc tbf 10: dev {device} parent 1:1 rate 8Mbit lat 40.0ms\n'),
                interfaces=dict(ok=True, stdout='[]'))


def case_rows(mode):
    rows = [dict(event='manifest', mode=mode, samples_per_operation_per_phase=120, period_us=100000,
                 caller_us=2000000, probe_period_us=20000, logger_capacity=512, bulk_bytes=observer.BYTES,
                 calibrated=False, slo=False)]
    for phase_id, phase in enumerate(observer.PHASES):
        base = phase_id*20000000
        end = base + (19100000 if phase == 'bulk' else 12100000)
        rows.append(dict(event='phase_started', mode=mode, phase=phase, at_us=base))
        for operation in ['directory', 'terminal']:
            for index in range(120):
                start = base+index*100000
                rows.append(dict(event='sample', mode=mode, phase=phase, operation=operation, index=index,
                                 scheduled_us=start, started_us=start, ended_us=start+40000, elapsed_us=40000,
                                 caller_us=2000000, outcome='ok', error=None))
        ticks = [[base+i*20000, base+i*20000+1000] for i in range(1, 951 if phase == 'bulk' else 601)]
        rows.append(dict(event='probe', mode=mode, phase=phase, period_us=20000, capacity=20000,
                         capped=False, ticks_us=ticks, started_us=base, ended_us=end, missed_tick_behavior='skip'))
        rows.append(dict(event='phase_finished', mode=mode, phase=phase, at_us=end+1000,
                         logger_overflow=False, probe_capped=False, record_flush_us=100))
    rows.append(dict(event='bulk', mode=mode, phase='bulk', started_us=20000000, ended_us=38000000,
                     outcome='ok', moved=observer.BYTES, error=None))
    for name in ['x_before', 'gateway_before', 'x_after', 'gateway_after']:
        rows.append(dict(event='capture', mode=mode, name=name, capture=capture(name)))
    rows.append(dict(event='complete', mode=mode, samples=720, directory_overlap=120, terminal_overlap=120,
                     functional_ok=True, all_answers=True, exact_bytes=True, closed=True, direct_after=True))
    return rows


def write_log(path, rows, prefix=False, passed=1):
    path.parent.mkdir(parents=True, exist_ok=True)
    for row in rows:
        row.update(schema=1, revision=observer.REVISION)
    text = ''.join('[netlab-workload] '+json.dumps(row)+'\n' for row in rows)
    if prefix:
        text = 'test workload::fixture ... '+text
    path.write_text(text+f'test result: ok. {passed} passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 50s\n')


class ObserverEvidence(unittest.TestCase):
    def test_complete_population_probes_order_and_profiles_are_required(self):
        modes = ['success', 'harness_prefix', 'missing_sample', 'duplicate_sample', 'timeout', 'cancelled',
                 'wrong_order', 'missing_round', 'missing_probe', 'bad_tick', 'capped', 'overflow',
                 'wrong_rate', 'wrong_text', 'missing_capture', 'missing_guard', 'guard_lied',
                 'missing_startup_guard', 'unreaped_startup', 'wrong_guard_summary_count',
                 'wrong_case_summary_count', 'bad_bytes']
        for mutation in modes:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                (root/'source.json').write_text(json.dumps(dict(tree='a'*40)))
                guard = []
                for mode in ['streaming', 'buffered']:
                    guard.extend([dict(event='sample', mode=mode, outcome='cancelled'),
                                  dict(event='sample', mode=mode, outcome='ok')])
                guard.append(dict(event='observer_guard_calibration', modes=2, cancelled=2, completed=2, passed=True))
                guard.append(dict(event='observer_resident_startup_guard',unpolled_cancelled=1,polled_cancelled=1,remaining=0,passed=True))
                if mutation == 'guard_lied': guard[0]['outcome'] = 'ok'
                elif mutation == 'missing_startup_guard': guard.pop()
                elif mutation == 'unreaped_startup': guard[-1]['remaining'] = 1
                if mutation != 'missing_guard':
                    write_log(root/'observer-guards/0/scenarios.log', guard, True,
                              passed=1 if mutation=='wrong_guard_summary_count' else 2)
                for number, mode in enumerate(observer.MODES[:5] if mutation == 'missing_round' else observer.MODES):
                    rows = case_rows('buffered' if number == 0 and mutation == 'wrong_order' else mode)
                    if number == 0:
                        sample = next(r for r in rows if r['event'] == 'sample')
                        probe = next(r for r in rows if r['event'] == 'probe')
                        if mutation == 'missing_sample': rows.remove(sample)
                        elif mutation == 'duplicate_sample': next(r for r in rows if r['event']=='sample' and r['index']==1)['index'] = 0
                        elif mutation in ['timeout', 'cancelled']: sample.update(outcome=mutation, error='retained failure')
                        elif mutation == 'missing_probe': rows.remove(probe)
                        elif mutation == 'bad_tick': probe['ticks_us'][0][1] = probe['ticks_us'][0][0]-1
                        elif mutation == 'capped': probe['capped'] = True
                        elif mutation == 'overflow': next(r for r in rows if r['event']=='phase_finished')['logger_overflow'] = True
                        elif mutation in ['wrong_rate', 'wrong_text']:
                            c = next(r for r in rows if r['event']=='capture')['capture']
                            if mutation == 'wrong_rate':
                                q = json.loads(c['qdisc_json']['stdout']);q[1]['options']['rate'] = 8000000;c['qdisc_json']['stdout'] = json.dumps(q)
                            else: c['qdisc_text']['stdout'] = c['qdisc_text']['stdout'].replace('8Mbit', '4Mbit')
                        elif mutation == 'missing_capture': rows.remove(next(r for r in rows if r['event']=='capture'))
                        elif mutation == 'bad_bytes': next(r for r in rows if r['event']=='complete')['exact_bytes'] = False
                    write_log(root/f'netlab/{number}/scenarios.log', rows, mutation == 'harness_prefix',
                              passed=2 if number==0 and mutation=='wrong_case_summary_count' else 1)
                result = observer.verify(root)
                self.assertEqual(result['passed'], mutation in ['success', 'harness_prefix'], json.dumps(result)[:300])
                self.assertFalse(result['full_N3_accepted'])
                if mutation in ['timeout', 'cancelled']:
                    group = result['rounds'][0]['groups']['idle_before/directory']
                    self.assertEqual(group['outcomes'][mutation], 1)
                    self.assertEqual(group['all_samples'], 120)
                    self.assertEqual(group['success_service_us']['n'], 119)
