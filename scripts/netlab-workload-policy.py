"""Fixed v4 workload with lifecycle logging off; data-only policy comparison."""
from collections import Counter
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys

spec = importlib.util.spec_from_file_location('observer', Path(__file__).with_name('netlab-observer.py'))
observer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(observer)


def configuration(source_root, allowed_ahead=(8,)):
    path = Path(source_root)/'crates/node/src/link.rs'
    text = path.read_text()
    priority = re.findall(r'^const CONTINUED_FILE_RESPONSE_PRIORITY: i32 = (-?\d+);$', text, re.M)
    ahead = re.findall(r'^const PULL_AHEAD: u64 = (\d+);$', text, re.M)
    assert len(priority) == len(ahead) == 1
    priority, ahead = int(priority[0]), int(ahead[0])
    assert priority in [0, -1] and ahead in allowed_ahead
    return dict(continued_file_response_priority=priority, pull_ahead=ahead,
                source_file_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                evidence='static configuration in the exact compiled source; readback is a separate diagnostic cohort')


def populations(path, bulk, revision=observer.REVISION, allow_failed=False):
    rows = [r for r in observer.records(path, revision=revision, allow_failed=allow_failed) if r['event']=='sample' and r['phase']=='bulk']
    groups = {}
    for operation in ['directory', 'terminal']:
        samples = [r for r in rows if r['operation']==operation]
        subsets = dict(all_bulk_phase=samples,
                       started_during_transfer=[r for r in samples if bulk['started_us']<=r['started_us']<bulk['ended_us']],
                       fully_during_transfer=[r for r in samples if bulk['started_us']<=r['started_us']<=r['ended_us']<=bulk['ended_us']])
        groups[operation] = {}
        for name, rs in subsets.items():
            success = [r for r in rs if r['outcome']=='ok']
            groups[operation][name] = dict(all_samples=len(rs),outcomes=dict(Counter(r['outcome'] for r in rs)),
                successful_service_us=observer.distribution([r['elapsed_us'] for r in success]),
                successful_scheduled_to_end_us=observer.distribution([r['ended_us']-r['scheduled_us'] for r in success]))
    return groups


def verify(report, source_root=None):
    report = Path(report)
    source_root = source_root or Path(__file__).resolve().parents[1]
    result = dict(schema=1, revision='workload_response_priority_v1', source=json.loads((report/'source.json').read_text()),
                  results=[], passed=False, slo=False, full_N3_accepted=False, p99_claimed=False,
                  limits=['Fixed 40ms/8Mbit model only; no high-BDP/fairness/device-scale/energy acceptance',
                          'Compare the predeclared block order and complete outcome populations; not an absolute user SLO',
                          'First slice and ordinary controls remain at default priority; initial pull-ahead stays eight',
                          'Source configuration is not a runtime getter result; lifecycle readback has separate evidence'])
    try:
        result['configuration'] = configuration(source_root)
        result['guard'] = observer.guard(report)
        logs = sorted((report/'netlab').glob('*/scenarios.log'))
        assert len(logs)==3
        for path in logs:
            row = dict(log=str(path.relative_to(report)),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),valid=False)
            try:
                assert '[netlab-diagnostic]' not in path.read_text()
                assert not list((path.parent/'diagnostics').glob('*'))
                row['workload'] = observer.case(path, 'buffered')
                row['transfer_populations'] = populations(path,row['workload']['bulk'])
                row['valid'] = row['workload']['functional_ok']
            except (AssertionError,KeyError,ValueError,TypeError,IndexError,OSError) as error:
                row['error'] = type(error).__name__+': '+str(error)
            result['results'].append(row)
        result['passed'] = all(r['valid'] for r in result['results'])
    except (AssertionError,KeyError,ValueError,TypeError,IndexError,OSError) as error:
        result['error'] = type(error).__name__+': '+str(error)
    return result


if __name__=='__main__':
    assert len(sys.argv)==2
    report=Path(sys.argv[1]);result=verify(report)
    (report/'workload-policy-contracts.json').write_text(json.dumps(result,indent=2)+'\n')
    sys.exit(0 if result['passed'] else 1)
