"""Policy block boundaries, source identity, and retained failed populations."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from test_netlab_observer import write_log
from test_netlab_workload_high import fixture, write_high

spec = importlib.util.spec_from_file_location('high_policy', Path(__file__).resolve().parents[1]/'netlab-workload-high-policy.py')
policy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(policy)


class HighPolicyEvidence(unittest.TestCase):
    def test_fixed_blocks_and_retained_timeout(self):
        for mutation in ['success', 'missing_guard', 'missing_round', 'extra_round', 'timeout', 'diagnostics', 'wrong_source']:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary); report=root/'report'; report.mkdir()
                (report/'source.json').write_text(json.dumps(dict(tree='b'*40)))
                source=root/'source'; path=source/'crates/node/src/link.rs'; path.parent.mkdir(parents=True)
                path.write_text('const PULL_AHEAD: u64 = 8;\nconst CONTINUED_FILE_RESPONSE_PRIORITY: i32 = '+('1' if mutation=='wrong_source' else '0')+';\n')
                guards=[]
                for mode in ['streaming', 'buffered']:
                    guards.extend([dict(event='sample',mode=mode,outcome='cancelled'),dict(event='sample',mode=mode,outcome='ok')])
                guards.extend([dict(event='observer_guard_calibration',modes=2,cancelled=2,completed=2,passed=True),
                    dict(event='observer_resident_startup_guard',unpolled_cancelled=1,polled_cancelled=1,remaining=0,passed=True)])
                if mutation!='missing_guard':write_log(report/'observer-guards/0/scenarios.log',guards,passed=2)
                count=2 if mutation=='missing_round' else 4 if mutation=='extra_round' else 3
                for index in range(count):
                    rows=fixture(); failed=mutation=='timeout' and index==1
                    if failed:
                        sample=next(r for r in rows if r['event']=='sample');sample.update(outcome='timeout',error='preserved timeout')
                        next(r for r in rows if r['event']=='complete').update(functional_ok=False,all_answers=False)
                    log=report/f'netlab/{index}/scenarios.log';write_high(log,rows,failed)
                    if mutation=='diagnostics' and index==0:log.write_text(log.read_text()+'[netlab-diagnostic] unexpected\n')
                result=policy.verify(report,source)
                self.assertEqual(result['passed'],mutation=='success',json.dumps(result)[:600])
                self.assertFalse(result['full_N3_accepted'])
                if mutation=='timeout':
                    self.assertEqual(len(result['results']),3)
                    self.assertEqual(result['results'][1]['workload']['groups']['idle_before/directory']['outcomes']['timeout'],1)
