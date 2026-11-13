"""Fixture ownership admission; no first-action or network SLO result."""
import hashlib
import json
from pathlib import Path
import re
import sys

MODE = dict(layer='fixture', measurement_revision='idle_action_fixture_ownership_v1',
            diagnostic='off', test_threads=1, ignored=True, expected_tests=2,
            filters=['paths::idle_action::p8g_'], full_N1_accepted=False)
EXPECTED = {
    'p8g-with-cancel': 'caller_drop_released_device_future',
    'p8g-detached-control': 'detached_future_detected_and_reaped',
}


def verify(report):
    report = Path(report)
    result = dict(schema=1, suite='idle_action_fixture_ownership_v1',
                  source=json.loads((report/'source.json').read_text()),
                  complete=False, passed=False, results=[], actual_first_action_samples=0,
                  full_N1_accepted=False, full_N2_accepted=False,
                  limits=['Fixture caller-to-device ownership only; not remote RPC or process ownership',
                          'No idle first-action latency, path selection, probe cost or phone outcome'])
    try:
        assert json.loads((report/'measurement-mode.json').read_text()) == MODE
        logs = sorted((report/'netlab').glob('*/scenarios.log'))
        assert len(logs) == 1
        path = logs[0]
        raw = path.read_text()
        assert '[netlab-diagnostic]' not in raw and not list((path.parent/'diagnostics').glob('*'))
        summaries = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw, re.M)
        assert len(summaries) == 1
        status, good, bad, ignored = summaries[0]
        assert int(good)+int(bad) == 2 and int(ignored) == 0
        result['rust'] = dict(status=status, passed=int(good), failed=int(bad), ignored=int(ignored))
        notes = re.findall(r'\[netlab-result\] (p8g-[^ |]+) \| (green|red) \| ([^\n]+)', raw)
        assert len(notes) == 2 and sorted(n[0] for n in notes) == sorted(EXPECTED)
        for name, verdict, detail in notes:
            assert detail == EXPECTED[name]+' '+('true' if verdict == 'green' else 'false')
            result['results'].append(dict(name=name, passed=verdict == 'green', detail=detail))
        result.update(log=str(path.relative_to(report)), sha256=hashlib.sha256(path.read_bytes()).hexdigest(), complete=True)
        green = sum(row['passed'] for row in result['results'])
        assert green == int(good) and 2-green == int(bad)
        result['passed'] = green == 2 and status == 'ok'
    except (AssertionError, KeyError, ValueError, TypeError, OSError) as error:
        result.update(complete=False, passed=False, error=type(error).__name__+': '+str(error))
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1])
    value = verify(report)
    (report/'idle-action-guard-contracts.json').write_text(json.dumps(value, indent=2)+'\n')
    sys.exit(0 if value['passed'] else 1)
