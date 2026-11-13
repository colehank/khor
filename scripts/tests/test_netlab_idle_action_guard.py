"""Reject incomplete or contradictory fixture-ownership evidence."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('idle_guard', Path(__file__).resolve().parents[1]/'netlab-idle-action-guard.py')
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)


class IdleActionGuardEvidence(unittest.TestCase):
    def test_complete_red_control_and_corrupted_populations(self):
        cases = ['green', 'old_red', 'missing_note', 'duplicate_note', 'wrong_count',
                 'contradictory_footer', 'wrong_filter', 'diagnostics', 'missing_footer']
        for case in cases:
            with self.subTest(case=case), tempfile.TemporaryDirectory() as tmp:
                report = Path(tmp)
                (report/'source.json').write_text(json.dumps(dict(tree='c'*40)))
                mode = dict(guard.MODE)
                if case == 'wrong_filter':
                    mode['filters'] = ['paths::p7_']
                (report/'measurement-mode.json').write_text(json.dumps(mode))
                notes = [
                    '[netlab-result] p8g-with-cancel | green | caller_drop_released_device_future true',
                    '[netlab-result] p8g-detached-control | green | detached_future_detected_and_reaped true',
                ]
                footer = 'test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 75 filtered out; finished in 0.10s'
                if case == 'old_red':
                    notes[0] = notes[0].replace('green', 'red').replace('true', 'false')
                    footer = footer.replace('ok. 2 passed; 0 failed', 'FAILED. 1 passed; 1 failed')
                elif case == 'missing_note':
                    notes.pop()
                elif case == 'duplicate_note':
                    notes[1] = notes[0]
                elif case == 'wrong_count':
                    footer = footer.replace('2 passed', '1 passed')
                elif case == 'contradictory_footer':
                    footer = footer.replace('ok. 2 passed; 0 failed', 'FAILED. 1 passed; 1 failed')
                elif case == 'diagnostics':
                    notes.append('[netlab-diagnostic] unexpected')
                elif case == 'missing_footer':
                    footer = ''
                log = report/'netlab/0/scenarios.log'
                log.parent.mkdir(parents=True)
                log.write_text('\n'.join(notes+[footer])+'\n')
                value = guard.verify(report)
                self.assertEqual(value['complete'], case in ['green', 'old_red'], value)
                self.assertEqual(value['passed'], case == 'green', value)
                self.assertEqual(value['actual_first_action_samples'], 0)
                self.assertFalse(value['full_N1_accepted'])
                if case == 'old_red':
                    self.assertEqual(value['rust']['failed'], 1)
                    self.assertFalse(next(r for r in value['results'] if r['name']=='p8g-with-cancel')['passed'])
