"""One fixed high-profile priority block; admission is audited before launch."""
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

spec = importlib.util.spec_from_file_location('high_resident', Path(__file__).with_name('netlab-workload-high.py'))
high = importlib.util.module_from_spec(spec)
spec.loader.exec_module(high)


def verify(report, source_root=None):
    report = Path(report)
    source_root = source_root or Path(__file__).resolve().parents[1]
    result = dict(schema=1, revision='workload_high_response_priority_v1',
        source=json.loads((report/'source.json').read_text()), results=[], passed=False,
        slo=False, full_N3_accepted=False, p99_claimed=False,
        limits=['One of six predeclared high-profile blocks; no automatic winner or N3 acceptance',
                'Independent capacity, real resident preflight and exact source-pair audit required before launch',
                'All failure outcomes, schedule lag and transfer populations remain in the evidence',
                'Static source configuration; runtime priority readback is a separate diagnostic cohort'])
    try:
        result['configuration'] = high.policy.configuration(source_root)
        result['guard'] = high.observer.guard(report)
        logs = sorted((report/'netlab').glob('*/scenarios.log'))
        for path in logs:
            row = dict(log=str(path.relative_to(report)), sha256=hashlib.sha256(path.read_bytes()).hexdigest(), valid=False)
            try:
                assert '[netlab-diagnostic]' not in path.read_text()
                assert not list((path.parent/'diagnostics').glob('*'))
                row['workload'] = high.case(path)
                row['valid'] = row['workload']['functional_ok']
            except (AssertionError, KeyError, ValueError, TypeError, IndexError, OSError) as error:
                row['error'] = type(error).__name__+': '+str(error)
            result['results'].append(row)
        result['passed'] = len(logs) == 3 and all(r['valid'] for r in result['results'])
    except (AssertionError, KeyError, ValueError, TypeError, IndexError, OSError) as error:
        result['error'] = type(error).__name__+': '+str(error)
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1])
    result = verify(report)
    (report/'workload-high-policy-contracts.json').write_text(json.dumps(result, indent=2)+'\n')
    sys.exit(0 if result['passed'] else 1)
