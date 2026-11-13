"""Synthetic socket observer contracts; no socket, Rust or product execution."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from test_netlab_kernel_high import rows

spec=importlib.util.spec_from_file_location('sockets',Path(__file__).resolve().parents[1]/'netlab-kernel-sockets.py')
sockets=importlib.util.module_from_spec(spec);spec.loader.exec_module(sockets)


def records():
    data=rows();data[0]['socket_diagnostic']=True
    for operation,port in [('upload',41001),('download',41002)]:
        for side in ['client','server']:
            local,peer=f'198.18.0.11:{port}','198.18.0.12:5000'
            if side=='server':local,peer=peer,local
            for second in [3,8,15,25]:
                data.append(dict(event='tcp_snapshot',side=side,operation=operation,local=local,peer=peer,
                    clock_origin='local_payload_start',planned_after_us=second*1000000,
                    started_after_us=second*1000000+1000,ended_after_us=second*1000000+2000,
                    command=dict(ok=True,timed_out=False,stdout=f'{local} {peer}\n skmem:(r0,rb2097152,t0,tb4194304)\n cubic rtt:310/1 cwnd:700 mss:1448 rcv_space:1000000\n',stderr='')))
    for r in data:r.update(schema=1,revision=sockets.REVISION)
    return data


class SocketEvidence(unittest.TestCase):
    def test_observer_integrity_and_physical_failure_are_separate(self):
        mutations=['success','physical_red','physical_red_wrong_summary','optional_late_absent','missing_round',
                   'missing_side','missing_first','missing_second','duplicate','wrong_peer_pair',
                   'command_failed','command_timeout','missing_cwnd','missing_memory','before_plan','wrong_revision',
                   'ordinary_manifest','incomplete_payload']
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                report=Path(temporary);(report/'source.json').write_text(json.dumps(dict(tree='b'*40)))
                for i in range(2 if mutation=='missing_round' else 3):
                    rs=records();suffix='ok. 1 passed; 0 failed; 0 ignored;'
                    if mutation.startswith('physical_red'):
                        for r in rs:
                            if r['event']=='bulk':r.update(elapsed_us=40000000,ended_us=r['started_us']+40000000)
                            elif r['event']=='complete':r.update(goodput_mbit=[sockets.high.BYTES*8/40000000]*2,bounds_ok=False,functional_and_bounds_ok=False)
                        if mutation=='physical_red':suffix='FAILED. 0 passed; 1 failed; 0 ignored;'
                    if i==0:
                        sample=next(r for r in rs if r['event']=='tcp_snapshot')
                        if mutation=='optional_late_absent':rs=[r for r in rs if r['event']!='tcp_snapshot' or r['planned_after_us']<15000000]
                        elif mutation=='missing_side':rs=[r for r in rs if r['event']!='tcp_snapshot' or r['side']!='server']
                        elif mutation=='missing_first':rs.remove(sample)
                        elif mutation=='missing_second':rs.remove(next(r for r in rs if r['event']=='tcp_snapshot' and r['planned_after_us']==8000000))
                        elif mutation=='duplicate':rs.append(dict(sample))
                        elif mutation=='wrong_peer_pair':
                            for r in rs:
                                if r['event']=='tcp_snapshot' and r['side']=='client' and r['operation']=='upload':
                                    r['command']['stdout']=r['command']['stdout'].replace(r['peer'],'198.18.0.99:5000');r['peer']='198.18.0.99:5000'
                        elif mutation=='command_failed':sample['command']['ok']=False
                        elif mutation=='command_timeout':sample['command']['timed_out']=True
                        elif mutation=='missing_cwnd':sample['command']['stdout']=sample['command']['stdout'].replace('cwnd:','unknown:')
                        elif mutation=='missing_memory':sample['command']['stdout']=sample['command']['stdout'].replace('skmem:','unknown:')
                        elif mutation=='before_plan':sample['started_after_us']=0
                        elif mutation=='wrong_revision':sample['revision']='wrong'
                        elif mutation=='ordinary_manifest':rs[0]['socket_diagnostic']=False
                        elif mutation=='incomplete_payload':next(r for r in rs if r['event']=='bulk')['exact_bytes']=False
                    p=report/f'netlab/{i}/scenarios.log';p.parent.mkdir(parents=True)
                    p.write_text(''.join('[netlab-workload-calibration] '+json.dumps(r)+'\n' for r in rs)+'test result: '+suffix+' 0 measured; 60 filtered out;\n')
                result=sockets.verify(report)
                self.assertEqual(result['observations_verified'],mutation in ['success','physical_red','optional_late_absent'],json.dumps(result)[:400])
                self.assertFalse(result['physical_calibration_accepted'])
                if mutation=='physical_red':self.assertFalse(result['physical_bounds_met'])
