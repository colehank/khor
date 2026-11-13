"""Synthetic extended-flow evidence only; no product execution."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from test_netlab_kernel_sockets import records

spec=importlib.util.spec_from_file_location('long_flow',Path(__file__).resolve().parents[1]/'netlab-kernel-long.py')
long_flow=importlib.util.module_from_spec(spec);spec.loader.exec_module(long_flow)


def fixture(slow=False):
    data=records();data[0].update(bulk_bytes=long_flow.BYTES,socket_plan_seconds=list(long_flow.PLAN))
    duration=int(long_flow.BYTES*8/(25 if slow else 50))
    for r in data:
        if r['event']=='bulk':r.update(bytes=long_flow.BYTES,elapsed_us=duration,ended_us=r['started_us']+duration)
        elif r['event']=='complete':r.update(goodput_mbit=[long_flow.BYTES*8/duration]*2,bounds_ok=not slow,functional_and_bounds_ok=not slow)
        elif r['event']=='capture' and r['name'].endswith('after'):
            c=r['capture'];q=json.loads(c['qdisc_json']['stdout'])
            for row in q:row['bytes']=2*long_flow.BYTES
            c['qdisc_json']['stdout']=json.dumps(q)
            interfaces=json.loads(c['interfaces']['stdout']);interfaces[0]['stats64']=dict(rx=dict(bytes=2*long_flow.BYTES),tx=dict(bytes=2*long_flow.BYTES));c['interfaces']['stdout']=json.dumps(interfaces)
    for side in ['client','server']:
        for operation in ['upload','download']:
            example=next(r for r in data if r['event']=='tcp_snapshot' and r['side']==side and r['operation']==operation)
            for second in ([50,80] if slow else [50]):
                r=copy.deepcopy(example);r.update(planned_after_us=second*1000000,started_after_us=second*1000000+1000,ended_after_us=second*1000000+2000);data.append(r)
    for r in data:r['revision']=long_flow.REVISION
    return data


class LongKernelEvidence(unittest.TestCase):
    def test_exact_length_deadline_and_single_exploratory_case(self):
        for mutation in ['success','physical_red','wrong_length','wrong_deadline','wrong_plan','missing_early','incomplete_payload','wrong_revision','extra_case']:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                report=Path(temporary);(report/'source.json').write_text(json.dumps(dict(tree='c'*40)))
                for i in range(2 if mutation=='extra_case' else 1):
                    rs=fixture(mutation=='physical_red')
                    if mutation=='wrong_length':rs[0]['bulk_bytes']=96*1024*1024
                    elif mutation=='wrong_deadline':rs[0]['bulk_caller_us']=120000000
                    elif mutation=='wrong_plan':rs[0]['socket_plan_seconds']=[3,8,15,25]
                    elif mutation=='missing_early':rs.remove(next(r for r in rs if r['event']=='tcp_snapshot'))
                    elif mutation=='incomplete_payload':next(r for r in rs if r['event']=='bulk')['exact_bytes']=False
                    elif mutation=='wrong_revision':rs[0]['revision']='workload_kernel_tcp_sockets_v1'
                    p=report/f'netlab/{i}/scenarios.log';p.parent.mkdir(parents=True)
                    footer='FAILED. 0 passed; 1 failed;' if mutation=='physical_red' else 'ok. 1 passed; 0 failed;'
                    p.write_text(''.join('[netlab-workload-calibration] '+json.dumps(r)+'\n' for r in rs)+'test result: '+footer+' 0 ignored; 0 measured; 60 filtered out;\n')
                result=long_flow.verify(report)
                self.assertEqual(result['observations_verified'],mutation in ['success','physical_red'],json.dumps(result)[:400])
                self.assertFalse(result['physical_calibration_accepted'])
