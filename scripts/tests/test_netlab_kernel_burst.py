"""Synthetic evidence populations for the fixed burst diagnostic."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from test_netlab_kernel_high import rows

spec = importlib.util.spec_from_file_location('burst', Path(__file__).resolve().parents[1]/'netlab-kernel-burst.py')
burst = importlib.util.module_from_spec(spec); spec.loader.exec_module(burst)


def fixture(expected, slow=False):
    data = rows()
    data[0].update(requested_burst_bytes=expected, socket_diagnostic=False)
    for side in ['x', 'gateway']:
        cap = next(r['capture'] for r in data if r['event'] == 'capture' and r['name'] == side+'_before')
        initial = json.loads(cap['qdisc_json']['stdout'])
        initial[1]['handle'] = '10:'
        dev = initial[1]['dev']
        data.append(dict(event='shaper_change', side=side, device=dev, requested_burst_bytes=expected,
            latency_us=40000, derived_fifo_limit_changes=True,
            before=dict(ok=True, timed_out=False, stdout=json.dumps(initial)),
            args=['qdisc','change','dev',dev,'parent','1:1','handle','10:','tbf','rate','50mbit','burst',str(expected),'latency','40ms'],
            command=dict(ok=True, timed_out=False, stdout='', stderr='')))
    for row in data:
        if row['event'] == 'capture':
            q = json.loads(row['capture']['qdisc_json']['stdout']); q[1]['options']['burst'] = expected-3
            row['capture']['qdisc_json']['stdout'] = json.dumps(q)
        elif slow and row['event'] == 'bulk':
            row.update(elapsed_us=40000000, ended_us=row['started_us']+40000000)
        elif slow and row['event'] == 'complete':
            row.update(goodput_mbit=[burst.high.BYTES*8/40000000]*2, bounds_ok=False, functional_and_bounds_ok=False)
        row.update(schema=1, revision=burst.REVISION)
    return data


class BurstEvidence(unittest.TestCase):
    def test_order_actual_shapers_payloads_and_original_failures(self):
        mutations = ['success','physical_red','missing_case','extra_case','wrong_order','wrong_burst_readback',
                     'missing_change','duplicate_side','wrong_initial_burst','wrong_initial_delay','wrong_parent',
                     'wrong_command','command_failed','command_timeout','wrong_device','wrong_manifest',
                     'wrong_deadline','incomplete_payload','false_physical_summary']
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                report = Path(temporary); (report/'source.json').write_text(json.dumps(dict(tree='d'*40)))
                order = list(burst.ORDER)
                if mutation == 'missing_case': order.pop()
                elif mutation == 'extra_case': order.append(4096)
                elif mutation == 'wrong_order': order[0], order[1] = order[1], order[0]
                for index, expected in enumerate(order):
                    data = fixture(expected, mutation in ['physical_red', 'false_physical_summary'])
                    if index == 0:
                        change = next(r for r in data if r['event'] == 'shaper_change')
                        cap = next(r for r in data if r['event'] == 'capture')['capture']
                        if mutation == 'wrong_burst_readback':
                            q=json.loads(cap['qdisc_json']['stdout']);q[1]['options']['burst']=65533;cap['qdisc_json']['stdout']=json.dumps(q)
                        elif mutation == 'missing_change': data.remove(change)
                        elif mutation == 'duplicate_side': change['side']='gateway'
                        elif mutation in ['wrong_initial_burst','wrong_initial_delay','wrong_parent']:
                            q=json.loads(change['before']['stdout'])
                            if mutation == 'wrong_initial_burst':q[1]['options']['burst']=65533
                            elif mutation == 'wrong_initial_delay':q[0]['options']['delay']['delay']=150
                            else:q[1]['parent']='2:1'
                            change['before']['stdout']=json.dumps(q)
                        elif mutation == 'wrong_command': change['args'][-1]='400ms'
                        elif mutation == 'command_failed': change['command']['ok']=False
                        elif mutation == 'command_timeout': change['command']['timed_out']=True
                        elif mutation == 'wrong_device': change['device']='unrelated'
                        elif mutation == 'wrong_manifest': data[0]['requested_burst_bytes']=65536
                        elif mutation == 'wrong_deadline': data[0]['bulk_caller_us']=120000000
                        elif mutation == 'incomplete_payload': next(r for r in data if r['event']=='bulk')['exact_bytes']=False
                    footer = 'FAILED. 0 passed; 1 failed;' if mutation == 'physical_red' else 'ok. 1 passed; 0 failed;'
                    path=report/f'netlab/{index}/scenarios.log';path.parent.mkdir(parents=True)
                    path.write_text(''.join('[netlab-workload-calibration] '+json.dumps(r)+'\n' for r in data)+'test result: '+footer+' 0 ignored; 0 measured; 70 filtered out;\n')
                result=burst.verify(report)
                self.assertEqual(result['observations_verified'],mutation in ['success','physical_red'],json.dumps(result)[:600])
                self.assertFalse(result['physical_calibration_accepted'])
                self.assertFalse(result['full_N3_accepted'])
