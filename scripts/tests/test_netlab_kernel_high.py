"""Synthetic high-kernel metadata contracts only; never execute the product."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('high', Path(__file__).resolve().parents[1]/'netlab-kernel-high.py')
high = importlib.util.module_from_spec(spec); spec.loader.exec_module(high)


def rows():
    data = [dict(event='manifest',warmup=4,echoes=64,echo_bytes=64,bulk_bytes=high.BYTES,
                 bulk_caller_us=240000000,rtt_ms=300,mbit_each_direction=50,queue_ms=40,
                 min_rtt_us_bounds=[270000,420000],goodput_mbit_bounds=[40.0,60.0],layer='kernel_TCP',
                 residents=False,slo=False,qdisc_units_review_required=True),dict(event='connect',ok=True)]
    for i in range(68):
        data.append(dict(event='echo',index=i,warmup=i<4,bytes=64,started_us=i*1000000,
                         ended_us=i*1000000+300000,elapsed_us=300000,outcome='ok',error=None))
    duration=int(high.BYTES*8/50)
    for direction in ['upload','download']:
        data.append(dict(event='bulk',direction=direction,bytes=high.BYTES,started_us=70000000,
                         ended_us=70000000+duration,elapsed_us=duration,outcome='ok',error=None,exact_bytes=True))
    for name in ['x_before','gateway_before','x_after','gateway_after']:
        n=0 if name.endswith('before') else 2*high.BYTES
        dev='v7' if name.startswith('gateway') else 'eth0'
        q=[dict(kind='netem',dev=dev,root=True,handle='1:',bytes=n,drops=3 if n else 0,
                options=dict(delay=dict(delay=.15,jitter=0,correlation=0),limit=1000)),
           dict(kind='tbf',dev=dev,parent='1:1',bytes=n,drops=2 if n else 0,
                options=dict(rate=6250000,lat=40000,burst=4096))]
        interface=[dict(ifname=dev,stats64=dict(rx=dict(bytes=n),tx=dict(bytes=n)))]
        capture=dict(qdisc_json=dict(ok=True,stdout=json.dumps(q)),
                     qdisc_text=dict(ok=True,stdout='qdisc netem 1: root delay 150ms\nqdisc tbf 10: parent 1:1 rate 50Mbit lat 40ms\n'),
                     interfaces=dict(ok=True,stdout=json.dumps(interface)))
        data.append(dict(event='capture',name=name,capture=capture))
    data.append(dict(event='complete',samples=70,min_rtt_us=300000,goodput_mbit=[high.BYTES*8/duration]*2,
                     bounds_ok=True,calibrated=False,qdisc_units_review_required=True,
                     functional_and_bounds_ok=True,server_ok=True,qdiscs_present=True))
    for r in data:r.update(schema=1,revision=high.REVISION)
    return data


class HighKernelEvidence(unittest.TestCase):
    def test_population_physics_readback_and_both_directions(self):
        mutations=['success','harness_prefix','missing_round','missing_echo','duplicate_direction','timeout',
                   'wrong_json_rate','wrong_text_rate','wrong_delay','wrong_parent','wrong_bytes',
                   'server_false','missing_capture','zero_burst','short_counters','old_revision',
                   'wrong_deadline','wrong_warmup','wrong_bounds','missing_exact','two_tests']
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                report=Path(temporary);(report/'source.json').write_text(json.dumps(dict(tree='a'*40)))
                for i in range(2 if mutation=='missing_round' else 3):
                    rs=rows()
                    if i==0:
                        echo=next(r for r in rs if r['event']=='echo')
                        bulk=next(r for r in rs if r['event']=='bulk')
                        cap=next(r for r in rs if r['event']=='capture')['capture']
                        if mutation=='missing_echo':rs.remove(echo)
                        elif mutation=='duplicate_direction':bulk['direction']='download'
                        elif mutation=='timeout':echo.update(outcome='timeout',error='retained timeout')
                        elif mutation in ['wrong_json_rate','wrong_delay','wrong_parent','zero_burst','short_counters']:
                            q=json.loads(cap['qdisc_json']['stdout'])
                            if mutation=='wrong_json_rate':q[1]['options']['rate']=50000000
                            elif mutation=='wrong_delay':q[0]['options']['delay']['delay']=150
                            elif mutation=='wrong_parent':q[1]['parent']='2:1'
                            elif mutation=='zero_burst':q[1]['options']['burst']=0
                            else:q[0]['bytes']=1000000000
                            cap['qdisc_json']['stdout']=json.dumps(q)
                        elif mutation=='wrong_text_rate':cap['qdisc_text']['stdout']=cap['qdisc_text']['stdout'].replace('50Mbit','8Mbit')
                        elif mutation=='wrong_bytes':bulk['bytes']=8388608
                        elif mutation=='server_false':rs[-1]['server_ok']=False
                        elif mutation=='missing_capture':rs.remove(next(r for r in rs if r['event']=='capture'))
                        elif mutation=='old_revision':rs[0]['revision']='workload_kernel_tcp_calibration_v1'
                        elif mutation=='wrong_deadline':rs[0]['bulk_caller_us']=120000000
                        elif mutation=='wrong_warmup':echo['warmup']=False
                        elif mutation=='wrong_bounds':rs[0]['goodput_mbit_bounds']=[1,100]
                        elif mutation=='missing_exact':bulk['exact_bytes']=False
                    log=report/f'netlab/{i}/scenarios.log';log.parent.mkdir(parents=True)
                    text=''.join('[netlab-workload-calibration] '+json.dumps(r)+'\n' for r in rs)
                    if mutation=='harness_prefix':text='test workload::calibration::h0_ ... '+text
                    count=2 if mutation=='two_tests' and i==0 else 1
                    log.write_text(text+f'test result: ok. {count} passed; 0 failed; 0 ignored; 0 measured; 60 filtered out;\n')
                result=high.verify(report)
                self.assertEqual(result['passed'],mutation in ['success','harness_prefix'],json.dumps(result)[:400])
                self.assertFalse(result['full_N3_accepted'])
                if result['passed']:self.assertEqual(result['results'][0]['calibration']['profile_counters']['x']['delta']['netem_drops'],3)
