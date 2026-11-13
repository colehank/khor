"""Prefetch factor isolation and honest analysis of incomplete files."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from test_netlab_observer import case_rows, write_log
from test_netlab_workload_high import fixture, write_high

spec = importlib.util.spec_from_file_location('prefetch', Path(__file__).resolve().parents[1]/'netlab-workload-prefetch.py')
prefetch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prefetch)


def rows_for(profile, timed_out=False):
    rows=fixture() if profile=='high' else case_rows('buffered')
    revision=prefetch.high.REVISION if profile=='high' else prefetch.observer.REVISION
    for row in rows:row.update(schema=1,revision=revision)
    bulk=next(r for r in rows if r['event']=='bulk');done=next(r for r in rows if r['event']=='complete')
    bulk.update(expected=prefetch.high.BYTES if profile=='high' else prefetch.observer.BYTES,caller_ms=240000 if profile=='high' else 120000)
    if timed_out:
        bulk.update(outcome='timeout',error='original caller deadline',moved=None,ended_us=bulk['started_us']+bulk['caller_ms']*1000)
        done.update(functional_ok=False,exact_bytes=False)
        for r in rows:
            if r.get('phase')=='idle_after':
                shift=200000000 if profile=='high' else 120000000
                for key in ['scheduled_us','started_us','ended_us','at_us']:
                    if key in r:r[key]+=shift
                if r['event']=='probe':r['ticks_us']=[[a+shift,b+shift] for a,b in r['ticks_us']]
            elif r.get('phase')=='bulk' and r['event']=='probe':
                r['ended_us']=bulk['ended_us']
                r['ticks_us']=[[r['started_us']+i*20000,r['started_us']+i*20000+1000] for i in range(1,(r['ended_us']-r['started_us'])//20000)]
            elif r.get('phase')=='bulk' and r['event']=='phase_finished':r['at_us']=bulk['ended_us']+1000
        if profile=='high':
            cap=next(r['capture'] for r in rows if r['event']=='capture' and r['name']=='x_after')
            data=json.loads(cap['interfaces']['stdout']);data[0]['stats64']['rx']['bytes']=1000;cap['interfaces']['stdout']=json.dumps(data)
        else:
            cap=next(r['capture'] for r in rows if r['event']=='capture' and r['name']=='gateway_after')
            data=json.loads(cap['qdisc_json']['stdout']);next(r for r in data if r['kind']=='netem')['bytes']=1000;cap['qdisc_json']['stdout']=json.dumps(data)
    done.update(bulk=copy.deepcopy(bulk),profile_readback_commands_ok=True)
    for operation in ['directory','terminal']:
        done[operation+'_overlap']=sum(r['event']=='sample' and r['phase']=='bulk' and r['operation']==operation and bulk['started_us']<=r['started_us']<=r['ended_us']<=bulk['ended_us'] for r in rows)
    return rows


class PrefetchEvidence(unittest.TestCase):
    def test_factor_bounds_complete_profiles_and_failed_bulk_population(self):
        mutations=['one','two','eight','bad_ahead','bad_priority','missing_guard','missing_low','missing_high',
                   'low_timeout','high_timeout','short_ok_payload','wrong_deadline','same_operation_overlap','early_timeout','diagnostics']
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as tmp:
                root=Path(tmp);report=root/'report';report.mkdir();(report/'source.json').write_text(json.dumps(dict(tree='c'*40)))
                source=root/'source';link=source/'crates/node/src/link.rs';link.parent.mkdir(parents=True)
                ahead={'one':1,'two':2,'bad_ahead':4}.get(mutation,8)
                link.write_text('const PULL_AHEAD: u64 = '+str(ahead)+';\nconst CONTINUED_FILE_RESPONSE_PRIORITY: i32 = '+('1' if mutation=='bad_priority' else '0')+';\n')
                guards=[]
                for mode in ['streaming','buffered']:guards.extend([dict(event='sample',mode=mode,outcome='ok'),dict(event='sample',mode=mode,outcome='cancelled')])
                guards.extend([dict(event='observer_guard_calibration',modes=2,cancelled=2,completed=2,passed=True),dict(event='observer_resident_startup_guard',unpolled_cancelled=1,polled_cancelled=1,remaining=0,passed=True)])
                if mutation!='missing_guard':write_log(report/'observer-guards/0/scenarios.log',guards,passed=2)
                for profile in ['low','high']:
                    if mutation=='missing_'+profile:continue
                    failed=mutation==profile+'_timeout' or mutation=='early_timeout' and profile=='high'
                    rows=rows_for(profile,failed);bulk=next(r for r in rows if r['event']=='bulk');done=next(r for r in rows if r['event']=='complete')
                    if profile=='high':
                        if mutation=='short_ok_payload':
                            cap=next(r['capture'] for r in rows if r['event']=='capture' and r['name']=='x_after');data=json.loads(cap['interfaces']['stdout']);data[0]['stats64']['rx']['bytes']=1000;cap['interfaces']['stdout']=json.dumps(data)
                        elif mutation=='wrong_deadline':bulk['caller_ms']=480000
                        elif mutation=='early_timeout':bulk['ended_us']=bulk['started_us']+1000
                    elif mutation=='same_operation_overlap':
                        r=next(r for r in rows if r['event']=='sample');r['ended_us']+=100000;r['elapsed_us']+=100000
                    done['bulk']=copy.deepcopy(bulk)
                    path=report/(profile+'-prefetch')/'0/scenarios.log';write_high(path,rows,failed)
                    if mutation=='diagnostics' and profile=='high':path.write_text(path.read_text()+'[netlab-diagnostic] unexpected\n')
                value=prefetch.verify(report,source)
                self.assertEqual(value['passed'],mutation in ['one','two','eight'],json.dumps(value)[:600])
                self.assertFalse(value['full_N3_accepted'])
                if mutation in ['low_timeout','high_timeout']:
                    profile=mutation.split('_')[0];row=next(r['workload'] for r in value['results'] if r['profile']==profile)
                    self.assertFalse(row['functional_ok']);self.assertEqual(row['interval_kind'],'caller_attempt')
                    self.assertIsNone(row['verified_payload_mbit']);self.assertTrue(row['completion_censored_at_caller_deadline'])
                    self.assertEqual(sum(g['all_samples'] for g in row['groups'].values()),720)
                    self.assertIn('fully_during_attempt',row['transfer_populations']['directory'])
                if mutation=='one':
                    with self.assertRaises(AssertionError):prefetch.high.policy.configuration(source)
                if mutation=='high_timeout':
                    with self.assertRaises(AssertionError):prefetch.high.case(report/'high-prefetch/0/scenarios.log')
