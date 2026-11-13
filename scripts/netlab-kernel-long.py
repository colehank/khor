"""One longer TCP diagnostic at unchanged physical settings; data only."""
import importlib.util
import json
from pathlib import Path
import sys

spec = importlib.util.spec_from_file_location('kernel_sockets', Path(__file__).with_name('netlab-kernel-sockets.py'))
sockets = importlib.util.module_from_spec(spec); spec.loader.exec_module(sockets)
REVISION = 'workload_kernel_tcp_long_v1'
BYTES = 384 * 1024 * 1024
PLAN = (3, 8, 15, 25, 50, 80)


def verify(report):
    result = sockets.verify(report, revision=REVISION, bulk_bytes=BYTES, planned_seconds=PLAN, repeats=1)
    result['limits'].append('One extended diagnostic only; not calibration repetition or a replacement for the original 96MiB population')
    return result


if __name__ == '__main__':
    assert len(sys.argv) == 2
    report = Path(sys.argv[1]); result = verify(report)
    (report/'workload-kernel-long-contracts.json').write_text(json.dumps(result, indent=2)+'\n')
    sys.exit(0 if result['observations_verified'] else 1)
