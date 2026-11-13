"""Audit actual warm request streams after a peer relay's physical network move.

This fixed-topology four-second contract is independent of availability and of
source/transport/executor verification. It does not assert per-packet routing.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys

CONTRACT = 'peer_relay_replug_warm_selection_within_four_seconds_v1'
AFTER_MS = 4000


def inspect(folder):
    proof = json.loads((folder/'peer-mobility-calibration.json').read_text())
    assert proof['identity_verified'] and proof['availability_verified']
    assert proof['result']['revision']=='peer_relay_replug_v1'
    parsed = json.loads((folder/'report/attempts.json').read_text())
    assert parsed['valid'] and parsed['outcomes']=={'ok':124}
    pivot = next(e['at_ms'] for e in proof['phases'] if e['phase']=='middle_replug_finished')
    events = [row['event'] for row in parsed['records']]
    starts = {(e['pid'],e['attempt']):e for e in events if e['phase']=='started'}
    result = proof['result']
    requests, problems = [], []
    for end in (e for e in events if e['phase']=='finished'):
        direction = next((label for label in ['ab','ba'] if end['case']==result['case']+'-after-'+label),None)
        if direction is None:
            continue
        begin = starts[(end['pid'],end['attempt'])]
        if begin['at_ms']<pivot+AFTER_MS:
            continue
        node = result['nodes'][0 if direction=='ab' else 1]
        candidates = []
        for record in proof['lifecycle']:
            event = record['event']
            if (event['phase']!='request_stream_opened' or event['peer']==result['middle']
                    or not record['file'].endswith('-'+node+'.log') or not event['details']['reading']):
                continue
            if begin['at_ms']<=event['at']<=end['at_ms']:
                selected = [fact for fact in event['details']['measurements'] if fact['selected']]
                passed = len(selected)==1 and selected[0]['carried'] and not selected[0]['relay']
                candidates.append(dict(wire=event['details']['wire'],stream=event['details']['stream'],
                                       at_ms=event['at'],selected=selected,passed=passed))
        passed = bool(candidates) and all(row['passed'] for row in candidates)
        requests.append(dict(direction=direction,attempt=end['attempt'],start_ms=begin['at_ms']-pivot,
                             end_ms=end['at_ms']-pivot,streams=candidates,passed=passed))
        if not passed:
            problems.append('warm '+direction+' request '+str(end['attempt'])+' did not select the peer relay after four seconds')
    assert any(row['direction']=='ab' for row in requests)
    assert sum(row['direction']=='ba' for row in requests)==30
    return dict(schema=1,contract=CONTRACT,run_id=proof['run_id'],source=proof['source'],
        pivot_phase='middle_replug_finished',pivot_at_ms=pivot,after_ms=AFTER_MS,
        passed=not problems,requests=requests,problems=problems,
        limits=['Only original peer_relay_replug_v1 20 ms device-leg topology, not arbitrary WAN',
                'Request-opening path facts, not per-packet or reply tracing',
                'Availability/physical/source/control/executor evidence must also pass',
                'Original 10-second holds, all 124 outcomes and failure records retained; no p99 claim'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('evidence',type=Path)
    args = parser.parse_args()
    result = inspect(args.evidence)
    print(json.dumps(result,indent=2))
    return 0 if result['passed'] else 1


if __name__=='__main__':
    sys.exit(main())
