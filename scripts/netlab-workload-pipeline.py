"""Matched batch/rolling campaign using the unchanged low/high recorder gates."""
import importlib.util
import json
from pathlib import Path
import re
import sys

spec = importlib.util.spec_from_file_location('prefetch_recorder', Path(__file__).with_name('netlab-workload-prefetch.py'))
recorder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(recorder)


def verify(report, source_root=None):
    report = Path(report)
    source_root = source_root or Path(__file__).resolve().parents[1]
    mode = json.loads((report / 'measurement-mode.json').read_text())
    assert mode['comparison_revision'] == 'workload_pipeline_v1'
    assert mode['diagnostic'] == 'off' and mode['profiles'] == ['low', 'high']
    assert mode['cases_per_profile'] == mode['test_threads'] == 1
    result = recorder.verify(report, source_root)
    link = (source_root / 'crates/node/src/link.rs').read_text()
    pull = link[link.index('    async fn pull_into('):link.index('    /// Where a file dropped onto a terminal')]
    rolling = 'let mut window = pull_window::Window::new(offset, PULL_AHEAD as usize);' in pull
    batch = 'offsets.iter().zip(join_all(asks).await)' in pull
    assert rolling != batch
    width = re.findall(r'const PULL_AHEAD: u64 = (\d+);', link)
    assert len(width) == 1 and int(width[0]) in {2, 8}
    result.update(revision='workload_pipeline_v1', recording_component='workload_prefetch_v1',
                  scheduler='rolling' if rolling else 'batch', width=int(width[0]),
                  limits=['One block of the predeclared width8 then width2, ABBAAB per width, low/high independent campaign',
                          'Only matched sources within the same width may be compared; old prefetch results are not a baseline',
                          'Recorder validation and original file/operation deadlines, full populations and failures remain unchanged',
                          'Three fresh cases per arm/profile are exploratory; no p99, significance, energy or full N3 acceptance'])
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1])
    result = verify(report)
    (report / 'workload-pipeline-contracts.json').write_text(json.dumps(result, indent=2) + '\n')
    sys.exit(0 if result['passed'] else 1)
