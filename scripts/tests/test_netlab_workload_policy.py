"""Synthetic fixed-policy evidence only; never compile or execute the product."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from test_netlab_observer import case_rows, write_log

spec = importlib.util.spec_from_file_location('policy',Path(__file__).resolve().parents[1]/'netlab-workload-policy.py')
policy=importlib.util.module_from_spec(spec);spec.loader.exec_module(policy)


class PolicyEvidence(unittest.TestCase):
    def test_complete_populations_logging_mode_and_source_model(self):
        for mutation in ['normal','lower_priority','missing_round','missing_sample','wrong_priority','wrong_inflight','diagnostics','missing_guard']:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temporary:
                root=Path(temporary);report=root/'report';report.mkdir()
                (report/'source.json').write_text(json.dumps(dict(tree='a'*40)))
                source=root/'source';link=source/'crates/node/src/link.rs';link.parent.mkdir(parents=True)
                priority=-1 if mutation=='lower_priority' else 0
                if mutation=='wrong_priority':priority=1
                ahead=2 if mutation=='wrong_inflight' else 8
                link.write_text(f'const PULL_AHEAD: u64 = {ahead};\nconst CONTINUED_FILE_RESPONSE_PRIORITY: i32 = {priority};\n')
                guard=[]
                for mode in ['streaming','buffered']:
                    guard.extend([dict(event='sample',mode=mode,outcome='cancelled'),dict(event='sample',mode=mode,outcome='ok')])
                guard.append(dict(event='observer_guard_calibration',modes=2,cancelled=2,completed=2,passed=True))
                guard.append(dict(event='observer_resident_startup_guard',unpolled_cancelled=1,polled_cancelled=1,remaining=0,passed=True))
                if mutation!='missing_guard':write_log(report/'observer-guards/0/scenarios.log',guard,passed=2)
                for i in range(2 if mutation=='missing_round' else 3):
                    rows=case_rows('buffered')
                    if i==0 and mutation=='missing_sample':rows.remove(next(r for r in rows if r['event']=='sample'))
                    path=report/f'netlab/{i}/scenarios.log';write_log(path,rows)
                    if i==0 and mutation=='diagnostics':path.write_text(path.read_text()+'[netlab-diagnostic] unexpected lifecycle log\n')
                result=policy.verify(report,source)
                self.assertEqual(result['passed'],mutation in ['normal','lower_priority'],json.dumps(result)[:500])
                self.assertFalse(result['full_N3_accepted'])
                if result['passed']:
                    self.assertEqual(result['configuration']['pull_ahead'],8)
                    self.assertEqual(result['results'][0]['transfer_populations']['directory']['all_bulk_phase']['all_samples'],120)
