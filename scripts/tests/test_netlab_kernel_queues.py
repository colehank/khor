"""Synthetic topology population/counter contracts; no product execution."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from test_netlab_kernel_burst import fixture as combined

spec = importlib.util.spec_from_file_location('queues', Path(__file__).resolve().parents[1]/'netlab-kernel-queues.py')
queues = importlib.util.module_from_spec(spec); spec.loader.exec_module(queues)


def fixture(separate, slow=False):
    data = combined(65536, slow)
    data[0]['separate_queues'] = separate
    if not separate:
        return data
    data = [r for r in data if r['event'] not in ['capture', 'shaper_change']]
    for side in ['x', 'y', 'gateway', 'right_gateway']:
        router = 'gateway' in side
        for when in ['before', 'after']:
            n = 0 if when == 'before' else 2*queues.high.BYTES
            stats = dict(rx=dict(bytes=n), tx=dict(bytes=n))
            if router:
                delay = dict(kind='netem',dev='v7',handle='1:',root=True,bytes=n,drops=0,
                    options=dict(delay=dict(delay=.15,jitter=0,correlation=0),limit=1000))
                rate = dict(kind='tbf',dev='ix',handle='10:',root=True,bytes=n,drops=0,
                    options=dict(rate=6250000,burst=65531,lat=40000))
                q = [delay,rate]
                text = 'qdisc netem 1: dev v7 root refcnt 2 limit 1000 delay 150.0ms\nqdisc tbf 10: dev ix root refcnt 2 rate 50Mbit burst 64Kb/1 lat 40.0ms\n'
                interfaces = [dict(ifname=dev,stats64=copy.deepcopy(stats)) for dev in ['ix','v7']]
                if when == 'before':
                    initial = [copy.deepcopy(delay),dict(kind='noqueue',dev='ix',handle='0:',root=True)]
                    data.append(dict(event='rate_install',side=side,device='ix',uplink_ip='198.18.0.1',
                        before=dict(ok=True,timed_out=False,stdout=json.dumps(initial)),
                        args=['qdisc','replace','dev','ix','root','handle','10:','tbf','rate','50mbit','burst','65536','latency','40ms'],
                        command=dict(ok=True,timed_out=False,stdout='',stderr='')))
            else:
                q=[dict(kind='noqueue',dev='eth0',handle='0:',root=True)]
                text='qdisc noqueue 0: dev eth0 root refcnt 2\n'
                interfaces=[dict(ifname='eth0',stats64=stats)]
            data.append(dict(event='capture',name=side+'_'+when,capture=dict(
                qdisc_json=dict(ok=True,stdout=json.dumps(q)),qdisc_text=dict(ok=True,stdout=text),
                interfaces=dict(ok=True,stdout=json.dumps(interfaces)))))
    for row in data:row.update(schema=1,revision=queues.REVISION)
    return data


class QueueTopologyEvidence(unittest.TestCase):
    def test_forwarding_roles_counters_population_and_original_reds(self):
        mutations=['success','physical_red','missing_case','wrong_order','missing_right_router','endpoint_shaped',
                   'same_interface','rate_child','wrong_rate','wrong_delay','wrong_text','wrong_burst',
                   'short_counter','install_wrong_interface','install_timeout','missing_install',
                   'wrong_initial_root','wrong_deadline','incomplete_payload','wrong_revision','false_red_summary']
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                report=Path(temporary);(report/'source.json').write_text(json.dumps(dict(tree='e'*40)))
                order=list(queues.ORDER)
                if mutation=='missing_case':order.pop()
                elif mutation=='wrong_order':order[0],order[1]=order[1],order[0]
                for i,layout in enumerate(order):
                    slow=mutation in ['physical_red','false_red_summary'];data=fixture(layout=='separated',slow)
                    if i==1:
                        cap=next(r['capture'] for r in data if r['event']=='capture' and r['name']=='gateway_after')
                        install=next((r for r in data if r['event']=='rate_install'),None)
                        if mutation=='missing_right_router':data.remove(next(r for r in data if r['event']=='capture' and r['name']=='right_gateway_after'))
                        elif mutation=='endpoint_shaped':
                            endpoint=next(r['capture'] for r in data if r['event']=='capture' and r['name']=='x_before');endpoint['qdisc_json']['stdout']=cap['qdisc_json']['stdout']
                        elif mutation in ['same_interface','rate_child','wrong_rate','wrong_delay','wrong_burst']:
                            q=json.loads(cap['qdisc_json']['stdout'])
                            if mutation=='same_interface':q[1]['dev']='v7'
                            elif mutation=='rate_child':q[1]['root']=False;q[1]['parent']='1:1'
                            elif mutation=='wrong_rate':q[1]['options']['rate']=50000000
                            elif mutation=='wrong_delay':q[0]['options']['delay']['delay']=150
                            else:q[1]['options']['burst']=4093
                            cap['qdisc_json']['stdout']=json.dumps(q)
                        elif mutation=='wrong_text':cap['qdisc_text']['stdout']=cap['qdisc_text']['stdout'].replace('50Mbit','8Mbit')
                        elif mutation=='short_counter':
                            rows=json.loads(cap['interfaces']['stdout']);rows[0]['stats64']['tx']['bytes']=10;cap['interfaces']['stdout']=json.dumps(rows)
                        elif mutation=='install_wrong_interface':install['device']='v7'
                        elif mutation=='install_timeout':install['command']['timed_out']=True
                        elif mutation=='missing_install':data.remove(install)
                        elif mutation=='wrong_initial_root':
                            q=json.loads(install['before']['stdout']);q[-1]['kind']='tbf';install['before']['stdout']=json.dumps(q)
                        elif mutation=='wrong_deadline':data[0]['bulk_caller_us']=120000000
                        elif mutation=='incomplete_payload':next(r for r in data if r['event']=='bulk')['exact_bytes']=False
                        elif mutation=='wrong_revision':data[0]['revision']='old'
                    footer='FAILED. 0 passed; 1 failed;' if mutation=='physical_red' else 'ok. 1 passed; 0 failed;'
                    p=report/f'netlab/{i}/scenarios.log';p.parent.mkdir(parents=True)
                    p.write_text(''.join('[netlab-workload-calibration] '+json.dumps(r)+'\n' for r in data)+'test result: '+footer+' 0 ignored; 0 measured; 70 filtered out;\n')
                result=queues.verify(report)
                self.assertEqual(result['observations_verified'],mutation in ['success','physical_red'],json.dumps(result)[:600])
                self.assertFalse(result['physical_calibration_accepted']);self.assertFalse(result['full_N3_accepted'])
