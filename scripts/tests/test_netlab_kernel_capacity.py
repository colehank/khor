"""Synthetic full-population UDP evidence; does not send packets or run Rust."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from test_netlab_kernel_queues import fixture as topology

spec = importlib.util.spec_from_file_location('capacity', Path(__file__).resolve().parents[1]/'netlab-kernel-capacity.py')
capacity = importlib.util.module_from_spec(spec); spec.loader.exec_module(capacity)
ALL = [(1 << min(64,capacity.PACKETS-64*i))-1 for i in range(capacity.WORDS)]


def distribution(n, length):
    return [(i+1)*n//20-i*n//20 for i in range(20)]+[0]*(length-20)


def fixture(slow=False):
    data = [r for r in topology(True) if r['event'] in ['capture','rate_install']]
    for row in data:
        if row['event']=='capture':
            c=row['capture'];ifaces=json.loads(c['interfaces']['stdout'])
            for iface in ifaces:iface['mtu']=1500
            c['interfaces']['stdout']=json.dumps(ifaces)
    data.append(dict(event='udp_manifest',layout='forwarding_split',payload_bytes=1200,offered_bit_per_sec=70000000,
        planned_packets=capacity.PACKETS,send_seconds=20,receive_seconds=25,ready_limit_seconds=2,direction_limit_seconds=28,
        steady_seconds=[5,15],rtt_ms=300,mbit_each_direction=50,queue_ms=40,burst_bytes=65536,
        capacity_mbit_bounds=[40,60],sender_steady_mbit_bounds=[65,75],receiver_first_limit_us=2000000,pacer_tick_us=5000,
        pacer_max_lateness_us=20000,pacer_late_10ms_percent_limit=1,pacer_skip_percent_limit=1,
        layer='kernel_UDP',residents=False,slo=False,physical_calibration_accepted=False))
    received=[0]*capacity.WORDS
    for seq in range(capacity.PACKETS):
        if seq%7 < (3 if slow else 5):received[seq//64] |= 1 << (seq%64)
    unique=sum(bin(w).count('1') for w in received)
    for name,source,dest in [('upload','198.18.1.2:41001','198.18.2.2:42001'),('download','198.18.2.2:41002','198.18.1.2:42002')]:
        s=dict(local=source,peer=dest,elapsed_us=20000000,sent=list(ALL),failed=[0]*capacity.WORDS,
            by_second=distribution(capacity.PACKETS,21),ticks=4000,max_lateness_us=500,late_over_10ms=0,first_error=None)
        r=dict(local=dest,peer=source,elapsed_us=25000000,first_after_us=150000,received=list(received),
            by_second=distribution(unique,25),datagrams=unique,duplicates=0,malformed=0,foreign=0,out_of_range=0)
        rate=sum(r['by_second'][5:15])*1200*8/10000000
        data.append(dict(event='udp_direction',direction=name,tag=capacity.TAGS[name],sender=s,receiver=r,
            observations_ok=True,sent_packets=capacity.PACKETS,unique_packets=unique,missing_packets=capacity.PACKETS-unique,
            unique_payload_bytes=unique*1200,receiver_steady_mbit=rate,capacity_bounds_ok=not slow))
    data.append(dict(event='udp_complete',observations_ok=True,capacity_bounds_ok=not slow,
        physical_calibration_accepted=False,full_N3_accepted=False,slo=False))
    for row in data:row.update(schema=1,revision=capacity.REVISION)
    return data


class CapacityEvidence(unittest.TestCase):
    def test_plan_pacer_bitmaps_tuples_and_original_capacity_failures(self):
        mutations=['success','physical_red','missing_case','extra_case','wrong_direction','wrong_tag','excessive_skip',
            'sent_and_failed_overlap','non_subset','out_of_plan_bit','wrong_bitmap_type','pacer_underfed','late_pacer',
            'wrong_tuple','content_error','missing_denominator','wrong_rate','wrong_bucket_total','missing_capture',
            'wrong_mtu','wrong_plan','wrong_deadline','wrong_revision','accepted_model','false_red_footer']
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                report=Path(temporary);(report/'source.json').write_text(json.dumps(dict(tree='f'*40)))
                data=fixture(mutation in ['physical_red','false_red_footer'])
                for index in range(2 if mutation=='missing_case' else 4 if mutation=='extra_case' else 3):
                    rs=copy.deepcopy(data)
                    if index==0:
                        d=next(r for r in rs if r['event']=='udp_direction');s=d['sender'];r=d['receiver']
                        if mutation=='wrong_direction':d['direction']='download'
                        elif mutation=='wrong_tag':d['tag']=0
                        elif mutation=='excessive_skip':s['sent'][:40]=[0]*40
                        elif mutation=='sent_and_failed_overlap':s['failed'][0]=1
                        elif mutation=='non_subset':s['sent'][0] &= ~1
                        elif mutation=='out_of_plan_bit':r['received'][-1] |= 1 << 63
                        elif mutation=='wrong_bitmap_type':s['sent'][0]=float(s['sent'][0])
                        elif mutation=='pacer_underfed':s['by_second'][0]+=s['by_second'][5];s['by_second'][5]=0
                        elif mutation=='late_pacer':s['max_lateness_us']=20001
                        elif mutation=='wrong_tuple':r['peer']='198.18.1.99:41001'
                        elif mutation=='content_error':r['malformed']=1
                        elif mutation=='missing_denominator':d['missing_packets']=0
                        elif mutation=='wrong_rate':d['receiver_steady_mbit']=55
                        elif mutation=='wrong_bucket_total':r['by_second'][0]+=1
                        elif mutation=='missing_capture':rs.remove(next(x for x in rs if x['event']=='capture'))
                        elif mutation=='wrong_mtu':
                            cap=next(x['capture'] for x in rs if x['event']=='capture');ifaces=json.loads(cap['interfaces']['stdout']);ifaces[0]['mtu']=1200;cap['interfaces']['stdout']=json.dumps(ifaces)
                        elif mutation=='wrong_plan':next(x for x in rs if x['event']=='udp_manifest')['offered_bit_per_sec']=50000000
                        elif mutation=='wrong_deadline':next(x for x in rs if x['event']=='udp_manifest')['direction_limit_seconds']=60
                        elif mutation=='wrong_revision':rs[0]['revision']='old'
                        elif mutation=='accepted_model':rs[-1]['physical_calibration_accepted']=True
                    footer='FAILED. 0 passed; 1 failed;' if mutation=='physical_red' else 'ok. 1 passed; 0 failed;'
                    p=report/f'netlab/{index}/scenarios.log';p.parent.mkdir(parents=True)
                    p.write_text(''.join('[netlab-workload-calibration] '+json.dumps(x)+'\n' for x in rs)+'test result: '+footer+' 0 ignored; 0 measured; 70 filtered out;\n')
                result=capacity.verify(report)
                self.assertEqual(result['observations_verified'],mutation in ['success','physical_red'],json.dumps(result)[:600])
                self.assertEqual(result['capacity_bounds_met'],mutation=='success')
                self.assertFalse(result['physical_calibration_accepted']);self.assertFalse(result['full_N3_accepted'])
