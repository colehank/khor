"""Frozen NAT rebind contract: warm remote streams select the actual new tuple.

Audit raw physical/logical/stream records. Source/transport/executor identities
remain independently verified by the trusted driver and retained receipts.
"""
import argparse
from collections import defaultdict
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

CONTRACT = 'warm_remote_requests_select_the_rebound_tuple_v1'
FIXTURE_SHA256 = 'ac97726906429a5dd63dd91b5c9ed99624c273d9a253f7facb916ffcf47e8a9b'


def inspect(report, source_repo):
    fixture = source_repo/'crates/cli/tests/netlab/paths/rebind.rs'
    assert hashlib.sha256(fixture.read_bytes()).hexdigest()==FIXTURE_SHA256, 'physical fixture changed; freeze a new contract'
    mode = json.loads((report/'measurement-mode.json').read_text())
    assert mode['diagnostic']=='lifecycle'
    spec = importlib.util.spec_from_file_location('attempts', Path(__file__).with_name('netlab-attempts.py'))
    attempts = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(attempts)
    parsed = attempts.inspect(sorted((report/'netlab').rglob('scenarios.log')))
    assert parsed['valid'] and parsed['outcomes']=={'ok':124}
    markers = defaultdict(list)
    for log in sorted((report/'netlab').rglob('scenarios.log')):
        for line in log.read_text().splitlines():
            for kind in ['calibration','addresses','result']:
                prefix = '[netlab-rebind-'+kind+'] '
                if prefix in line:
                    markers[kind].append(json.loads(line.split(prefix,1)[1]))
    assert len(markers['calibration'])==len(markers['result'])==1
    calibration, result = markers['calibration'][0], markers['result'][0]
    assert calibration['revision']=='nat_port_rebind_calibration_v1'
    assert result['revision']=='node_nat_port_rebind_v1'
    for value, case in [(calibration,'rebind-calibration'),(result,result['case'])]:
        assert value['addresses_unchanged']
        old_ip, old_port = value['old'].rsplit(':',1)
        new_ip, new_port = value['new'].rsplit(':',1)
        assert old_ip==new_ip and (old_port,new_port)==('50001','50002')
        states = {v['phase']:v for v in markers['addresses'] if v['case']==case}
        assert set(states)=={'before','after'}
        for family in ['v4','v6']:
            assert states['before'][family]==states['after'][family]
    for phase, port in [('before','50001'),('deleted','50001'),('after','50002')]:
        value = calibration[phase]
        assert value['code']==0 and ('dport='+port) in value['stdout']
        assert value['delete']==(phase=='deleted')
    assert result['observed_mapping']==result['new']
    lifecycle, mismatches, seen = [], [], set()
    logs = sorted((report/'netlab').rglob('diagnostics/*.log'))
    assert len(logs)==2
    for path in logs:
        text = path.read_text()
        assert 'mode=lifecycle samples=off' in text
        for number, line in enumerate(text.splitlines(),1):
            if '[khor-path-tuple-mismatch]' in line:
                mismatches.append(dict(file=str(path.relative_to(report)),line=number,text=line))
            marker = '[khor-carried-lifecycle] '
            if marker not in line:
                continue
            event = json.loads(line.split(marker,1)[1])
            key = json.dumps(event,sort_keys=True)
            if key not in seen:
                seen.add(key)
                lifecycle.append(dict(file=str(path.relative_to(report)),event=event))
    events = [row['event'] for row in parsed['records']]
    starts = {(e['pid'],e['attempt']):e for e in events if e['phase']=='started'}
    finished = [e for e in events if e['phase']=='finished' and e['case']==result['case']+'-after-ba']
    assert len(finished)==30
    expected = 'Ip('+result['new']+')'
    requests, problems = [], []
    for end in finished:
        begin = starts[(end['pid'],end['attempt'])]
        candidates = []
        for record in lifecycle:
            event = record['event']
            if event['phase']!='request_stream_opened' or not record['file'].endswith('-'+result['nodes'][1]+'.log'):
                continue
            if begin['at_ms']<=event['at']<=end['at_ms']:
                selected = [f for f in event['details']['measurements'] if f['selected']]
                okay = (len(selected)==1 and selected[0]['addr']==expected
                        and not selected[0]['relay'] and not selected[0]['carried'])
                candidates.append(dict(wire=event['details']['wire'],stream=event['details']['stream'],selected=selected,passed=okay))
        okay = bool(candidates) and all(c['passed'] for c in candidates)
        requests.append(dict(attempt=end['attempt'],start=begin['at_ms'],end=end['at_ms'],streams=candidates,passed=okay))
        if not okay:
            problems.append('warm remote request '+str(end['attempt'])+' did not use the new selected direct tuple')
    if mismatches:
        problems.append('cached vs actual tuple mismatches remain: '+str(len(mismatches)))
    if not result['new_quic_path_selected']:
        problems.append('no new external QUIC tuple selected in peer journal')
    return dict(schema=1,contract=CONTRACT,source=json.loads((report/'source.json').read_text()),
        whole_physical_fixture_sha256=FIXTURE_SHA256,physical_calibration_verified=True,
        outcomes=parsed['outcomes'],expected=expected,passed=not problems,requests=requests,
        mismatches=mismatches,problems=problems,
        limits=['Actual request opening selection, not per-packet or reply tracing',
                'No added latency threshold, p99, full mobility or phone claim',
                'Verify retained source/control/transport/executor receipts separately'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('report',type=Path)
    parser.add_argument('--source-repo',type=Path,default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    result = inspect(args.report,args.source_repo)
    print(json.dumps(result,indent=2))
    return 0 if result['passed'] else 1


if __name__=='__main__':
    sys.exit(main())
