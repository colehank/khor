"""Synthetic resident evidence for fixed high parameters and download paths."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from test_netlab_observer import case_rows, write_log
from test_netlab_kernel_queues import fixture as kernel_rows

spec = importlib.util.spec_from_file_location('high_resident',Path(__file__).resolve().parents[1]/'netlab-workload-high.py')
high = importlib.util.module_from_spec(spec);spec.loader.exec_module(high)


def fixture():
    rows = case_rows('buffered')
    rows = [r for r in rows if r['event'] not in ['capture', 'probe']]
    for r in rows:
        event = r['event']
        if event == 'manifest':
            r.update(period_us=400000,bulk_bytes=high.BYTES,impairment_installed_before_residents=True,network_profile='forwarding_split_300ms_50mbit',
                nominal_rtt_ms=300,mbit_per_direction=50,queue_ms=40,random_loss_pct=0,
                phases=high.observer.PHASES,operations=['directory','terminal'],seed='6d697865646c6f61',
                connection_layout='directory_and_fetch_Main__terminal_Tunnel',diagnostic_recording=True)
        elif event == 'sample':
            base=high.observer.PHASES.index(r['phase'])*100000000
            elapsed=300000 if r['operation']=='directory' else 600000
            started=base+r['index']*max(400000,elapsed)
            r.update(scheduled_us=base+r['index']*400000,started_us=started,ended_us=started+elapsed,
                elapsed_us=elapsed,schedule_lag_us=started-(base+r['index']*400000))
        elif event in ['phase_started','phase_finished']:
            base=high.observer.PHASES.index(r['phase'])*100000000;r['at_us']=base if event=='phase_started' else base+72001000
        elif event == 'bulk':
            r.update(started_us=100000000,ended_us=140000000,moved=high.BYTES,expected=high.BYTES,caller_ms=240000)
    for phase_id, phase in enumerate(high.observer.PHASES):
        base=phase_id*100000000
        rows.append(dict(event='probe',mode='buffered',phase=phase,period_us=20000,capacity=20000,capped=False,
            ticks_us=[[base+i*20000,base+i*20000+1000] for i in range(1,3600)],started_us=base,
            ended_us=base+72000000,missed_tick_behavior='skip'))
    for row in kernel_rows(True):
        if row['event'] not in ['capture','rate_install']:continue
        row=copy.deepcopy(row)
        if row['event']=='capture':
            row['mode']='buffered'
            if row['name'].endswith('_after'):
                side=row['name'].rsplit('_',1)[0];cap=row['capture'];interfaces=json.loads(cap['interfaces']['stdout'])
                for interface in interfaces:
                    for direction in ['rx','tx']:
                        n=high.BYTES
                        if side=='x' and direction=='tx' or side=='y' and direction=='rx':n=1024
                        elif side=='gateway' and (interface['ifname']=='ix' and direction=='tx' or interface['ifname']=='v7' and direction=='rx'):n=1024
                        elif side=='right_gateway' and (interface['ifname']=='ix' and direction=='rx' or interface['ifname']=='v7' and direction=='tx'):n=1024
                        interface['stats64'][direction]['bytes']=n
                cap['interfaces']['stdout']=json.dumps(interfaces)
                q=json.loads(cap['qdisc_json']['stdout'])
                if side=='gateway':next(x for x in q if x['kind']=='tbf')['bytes']=1024
                elif side=='right_gateway':next(x for x in q if x['kind']=='netem')['bytes']=1024
                cap['qdisc_json']['stdout']=json.dumps(q)
        rows.append(row)
    bulk=next(r for r in rows if r['event']=='bulk');done=next(r for r in rows if r['event']=='complete')
    for op in ['directory','terminal']:
        done[op+'_overlap']=sum(r['event']=='sample' and r['phase']=='bulk' and r['operation']==op and bulk['started_us']<=r['started_us']<=r['ended_us']<=bulk['ended_us'] for r in rows)
    for r in rows:r.update(schema=1,revision=high.REVISION)
    done.update(bulk=copy.deepcopy(bulk),profile_readback_commands_ok=True)
    return rows


def write_high(path, rows, failed=False):
    path.parent.mkdir(parents=True)
    text=''.join(('[netlab-workload-calibration] ' if r['event']=='rate_install' else '[netlab-workload] ')+json.dumps(r)+'\n' for r in rows)
    path.write_text(text+'test result: '+('FAILED. 0 passed; 1 failed;' if failed else 'ok. 1 passed; 0 failed;')+' 0 ignored; 0 measured; 70 filtered out;\n')


class HighResidentEvidence(unittest.TestCase):
    def test_full_population_low_regression_and_single_download_path(self):
        mutations=['success','missing_guard','missing_low','missing_high','wrong_size','wrong_period','wrong_deadline',
            'wrong_revision','late_impairment','wrong_profile','wrong_rate','missing_right_router','short_payload_path','wrong_install',
            'same_operation_overlap','payload_not_exact','missing_sample','diagnostics','source_inflight_changed','source_priority_invalid','rpc_timeout']
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary);report=root/'report';report.mkdir();(report/'source.json').write_text(json.dumps(dict(tree='a'*40)))
                source=root/'source';link=source/'crates/node/src/link.rs';link.parent.mkdir(parents=True)
                link.write_text('const PULL_AHEAD: u64 = '+('2' if mutation=='source_inflight_changed' else '8')+';\nconst CONTINUED_FILE_RESPONSE_PRIORITY: i32 = '+('1' if mutation=='source_priority_invalid' else '-1')+';\n')
                guards=[]
                for mode in ['streaming','buffered']:guards.extend([dict(event='sample',mode=mode,outcome='cancelled'),dict(event='sample',mode=mode,outcome='ok')])
                guards.extend([dict(event='observer_guard_calibration',modes=2,cancelled=2,completed=2,passed=True),dict(event='observer_resident_startup_guard',unpolled_cancelled=1,polled_cancelled=1,remaining=0,passed=True)])
                if mutation!='missing_guard':write_log(report/'observer-guards/0/scenarios.log',guards,passed=2)
                for i in range(2 if mutation=='missing_low' else 3):write_log(report/f'low-regression/{i}/scenarios.log',case_rows('buffered'))
                for i in range(2 if mutation=='missing_high' else 3):
                    rs=fixture();failed=False
                    if i==0:
                        m=next(r for r in rs if r['event']=='manifest');b=next(r for r in rs if r['event']=='bulk');done=next(r for r in rs if r['event']=='complete')
                        if mutation=='wrong_size':m['bulk_bytes']=96*1024*1024
                        elif mutation=='wrong_period':m['period_us']=100000
                        elif mutation=='wrong_deadline':b['caller_ms']=480000;done['bulk']=copy.deepcopy(b)
                        elif mutation=='wrong_revision':m['revision']='old'
                        elif mutation=='late_impairment':m['impairment_installed_before_residents']=False
                        elif mutation=='wrong_profile':m['network_profile']='combined'
                        elif mutation=='wrong_rate':
                            c=next(r['capture'] for r in rs if r['event']=='capture' and r['name']=='gateway_after');q=json.loads(c['qdisc_json']['stdout']);next(x for x in q if x['kind']=='tbf')['options']['rate']=50000000;c['qdisc_json']['stdout']=json.dumps(q)
                        elif mutation=='missing_right_router':rs.remove(next(r for r in rs if r['event']=='capture' and r['name']=='right_gateway_after'))
                        elif mutation=='short_payload_path':
                            c=next(r['capture'] for r in rs if r['event']=='capture' and r['name']=='x_after');q=json.loads(c['interfaces']['stdout']);q[0]['stats64']['rx']['bytes']=1000;c['interfaces']['stdout']=json.dumps(q)
                        elif mutation=='wrong_install':next(r for r in rs if r['event']=='rate_install')['args'][-1]='400ms'
                        elif mutation=='same_operation_overlap':
                            r=next(r for r in rs if r['event']=='sample' and r['operation']=='terminal' and r['index']==0);r['ended_us']+=100000;r['elapsed_us']+=100000
                        elif mutation=='payload_not_exact':done['exact_bytes']=False;done['functional_ok']=False;failed=True
                        elif mutation=='missing_sample':rs.remove(next(r for r in rs if r['event']=='sample'))
                        elif mutation=='rpc_timeout':
                            r=next(r for r in rs if r['event']=='sample');r.update(outcome='timeout',error='retained timeout');done.update(functional_ok=False,all_answers=False);failed=True
                    p=report/f'netlab/{i}/scenarios.log';write_high(p,rs,failed)
                    if i==0 and mutation=='diagnostics':p.write_text(p.read_text()+'[netlab-diagnostic] unexpected\n')
                result=high.verify(report,source)
                self.assertEqual(result['passed'],mutation=='success',json.dumps(result)[:600]);self.assertFalse(result['full_N3_accepted'])
                if mutation=='rpc_timeout':self.assertEqual(result['results'][0]['workload']['groups']['idle_before/directory']['outcomes']['timeout'],1)
