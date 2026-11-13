"""Diagnostic STREAM-silence witnesses; these are not ordinary latency samples."""
import ast
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(file))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


first = load('stream_first_action', 'netlab-first-action.py')
guard = load('stream_ownership_guard', 'netlab-idle-action-guard.py')
REVISION = 'first_foreground_stream_witness_v2'
MODE = dict(layer='node_stream_silence_diagnostic', measurement_revision=REVISION,
            diagnostic='full', test_threads=1, ignored=True, slo=False, quiet_ms=12000,
            guard_filter='paths::idle_action::p8g_', expected_guard_tests=2,
            action_filter='paths::first_action::p8_', expected_action_tests=6,
            cases=list(first.CASES), vantage_points=first.SIDES,
            required_stream_silence_us=10000000, observer_margin_us=500000,
            max_clock_delta_disagreement_us=100000, max_snapshot_gap_us=1000000,
            preparation_stability_us=1000000, preparation_deadline_ms=10000,
            preparation_snapshot_freshness_us=1000000,
            ordinary_latency_samples=0, full_N1_accepted=False)


def records(raw, peer):
    rows = []
    for line in raw.splitlines():
        if 'khor_stream_snapshot_v1' not in line:
            continue
        found = re.search(r'\bpeer=([^\s]+)', line)
        first.need(found is not None, 'STREAM peer field missing')
        if found[1] != peer:
            continue
        row = {}
        for key in ['monotonic_us', 'unix_us', 'actor_serial', 'registry_epoch']:
            match = re.search(r'\b'+key+r'=(\d+)\b', line)
            first.need(match is not None, 'STREAM counter field missing: '+key)
            row[key] = int(match[1])
        match = re.search(r'\bcounters=(\[.*?\])(?:\s|$)', line)
        first.need(match is not None, 'STREAM connection population missing')
        counters = ast.literal_eval(match[1])
        first.need(isinstance(counters, list), 'STREAM counter list')
        first.need(all(isinstance(c, tuple) and len(c) == 3 and
                       all(type(n) is int and n >= 0 for n in c) for c in counters), 'STREAM counter tuple')
        first.need(len({c[0] for c in counters}) == len(counters), 'duplicate connection')
        row['counters'] = {c[0]: list(c[1:]) for c in counters}
        row['signature'] = line.split('actor_serial=', 1)[1]
        rows.append(row)
    first.need(len(rows) >= 2, 'STREAM snapshot population missing')
    for a, b in zip(rows, rows[1:]):
        first.need(b['monotonic_us'] >= a['monotonic_us'] and b['unix_us'] >= a['unix_us'], 'observer clock reversed')
        first.need(abs((b['unix_us']-a['unix_us'])-(b['monotonic_us']-a['monotonic_us'])) <= 100000,
                   'observer clock disagreement')
    return rows


def preparation(raw, peer, row, side):
    samples = records(raw, peer)
    drain = row['drain']; evidence = drain['sides'][side]
    first.need(set(drain['sides']) == {'alpha', 'beta'}, 'preparation population')
    duration = drain['ended_us']-drain['started_us']
    first.need(0 <= duration <= 10000000 and
               abs((drain['ended_at_unix_us']-drain['started_at_unix_us'])-duration) <= 100000,
               'preparation deadline/clock')
    first.need(row['prepared_paths'][side]['observed_us'] <= drain['started_us'] <= drain['ended_us'] <=
               row['before_idle'][side]['started_us'], 'preparation must precede idle baseline')
    stable = [s for s in samples if evidence['stable_since_monotonic_us'] <= s['monotonic_us'] <=
              evidence['observed_monotonic_us']]
    first.need(len(stable) >= 2 and stable[0]['monotonic_us'] == evidence['stable_since_monotonic_us'] and
               stable[-1]['monotonic_us'] == evidence['observed_monotonic_us'], 'preparation source snapshots')
    first.need(stable[-1]['monotonic_us']-stable[0]['monotonic_us'] >= 1000000, 'preparation stability')
    first.need(drain['started_at_unix_us'] <= stable[0]['unix_us'] <= stable[-1]['unix_us'] <=
               drain['ended_at_unix_us'], 'preparation snapshot time window')
    first.need(stable[-1]['unix_us'] == evidence['observed_at_unix_us'] and
               drain['ended_at_unix_us']-stable[-1]['unix_us'] <= 1000000, 'stale preparation snapshot')
    first.need(stable[-1]['signature'] == evidence['signature'] and stable[0]['counters'] and
               all(s['signature'] == evidence['signature'] for s in stable), 'preparation STREAM/registry changed')
    return dict(stable_us=stable[-1]['monotonic_us']-stable[0]['monotonic_us'],
                snapshot_count=len(stable), actor_serial=stable[-1]['actor_serial'],
                registry_epoch=stable[-1]['registry_epoch'], passed=True)


def witness(raw, peer, row):
    samples = records(raw, peer)
    began, ended = row['quiet_started_at_unix_us'], row['restore_started_at_unix_us']
    first.need(ended-began >= 12000000, 'diagnostic quiet interval shortened')
    mono = row['restore_started_us']-row['quiet_started_us']
    first.need(abs((ended-began)-mono) <= 100000, 'fixture clock disagreement')
    quiet = [r for r in samples if began <= r['unix_us'] <= ended]
    inner = [r for r in quiet if began+500000 <= r['unix_us'] <= ended-500000]
    reasons = []
    if len(inner) < 2 or inner[-1]['monotonic_us']-inner[0]['monotonic_us'] < 10000000:
        reasons.append('no full ten-second inner witness')
    if len(quiet) < 2 or any(b['monotonic_us']-a['monotonic_us'] > 1000000 for a, b in zip(quiet, quiet[1:])):
        reasons.append('missing diagnostic snapshots')
    if not quiet or not quiet[0]['counters']:
        reasons.append('no live connection population')
    if quiet and any((r['actor_serial'], r['registry_epoch'], r['counters']) !=
                     (quiet[0]['actor_serial'], quiet[0]['registry_epoch'], quiet[0]['counters']) for r in quiet):
        reasons.append('STREAM activity or connection registry changed during quiet')
    post = [r for r in samples if row['request_ended_at_unix_us'] <= r['unix_us'] <= row['request_ended_at_unix_us']+2000000]
    positive = bool(quiet) and any(
        r['actor_serial'] == quiet[-1]['actor_serial'] and any(
            key in r['counters'] and any(n > old for n, old in zip(r['counters'][key], values))
            for key, values in quiet[-1]['counters'].items()) for r in post)
    if not positive:
        reasons.append('real request did not establish a positive STREAM control')
    return dict(peer=peer, snapshots=len(samples), quiet_snapshots=len(quiet),
                quiet_witness_us=inner[-1]['monotonic_us']-inner[0]['monotonic_us'] if len(inner) >= 2 else None,
                actor_serial=quiet[0]['actor_serial'] if quiet else None,
                registry_epoch=quiet[0]['registry_epoch'] if quiet else None,
                connection_ids=sorted(quiet[0]['counters']) if quiet else [],
                stream_counters=quiet[0]['counters'] if quiet else {},
                positive_request_control=positive, passed=not reasons, failures=reasons)


def verify(report):
    report = Path(report)
    out = dict(schema=1, suite=REVISION, source=json.loads((report/'source.json').read_text()),
               complete=False, passed=False, cases=[], ordinary_latency_samples=0,
               full_N1_accepted=False, energy_measured=False,
               limits=['Full diagnostic logging has overhead; do not mix with ordinary latency runs',
                       'Witness is a continuous ten-second interval wholly before Direct restoration',
                       'All tracked live connections and registry changes are checked; no packet-type inference from bytes',
                       'No severe-tail, migration, phone energy or full N1 acceptance'])
    try:
        first.need(json.loads((report/'first-action-stream-mode.json').read_text()) == MODE, 'STREAM diagnostic mode')
        out['guard'] = guard.verify(report)
        first.need(out['guard']['complete'] and out['guard']['passed'], 'ownership guard')
        logs = list((report/'first-action-streams').glob('*/scenarios.log'))
        first.need(len(logs) == 1, 'STREAM action log population')
        path = logs[0]; raw = path.read_text()
        rows = [json.loads(s.split('[netlab-first-action] ', 1)[1]) for s in raw.splitlines() if '[netlab-first-action] ' in s]
        calls = [json.loads(s.split('[netlab-attempt] ', 1)[1]) for s in raw.splitlines() if '[netlab-attempt] ' in s]
        first.need(len(rows) == 6 and [r['case'] for r in rows] == list(first.CASES) and len(calls) == 12,
                   'STREAM action population/order')
        first.need(len({(c['pid'], c['attempt']) for c in calls}) == 6, 'caller identities')
        diagnostics = path.parent/'diagnostics'
        first.need({p.name for p in diagnostics.glob('*')} == {r['case']+'-'+s+'.log' for r in rows for s in ['alpha', 'beta']},
                   'both resident logs for every fresh case')
        for row in rows:
            value = first.case(row, [a for a in calls if a['case'] == row['case']],
                               revision=REVISION, quiet_ms=12000, diagnostic='full')
            ids = {row['from']:row['from_id'], row['to']:row['to_id']}
            witnesses = {}
            for side, peer in [('alpha', 'beta'), ('beta', 'alpha')]:
                file = diagnostics/(row['case']+'-'+side+'.log'); text = file.read_text()
                first.need('[khor-net-diagnostics] mode=full samples=debug' in text, 'diagnostic initialization')
                witnesses[side] = witness(text, ids[peer], row)
                witnesses[side]['preparation'] = preparation(text, ids[peer], row, side)
                witnesses[side].update(log=str(file.relative_to(report)), sha256=hashlib.sha256(file.read_bytes()).hexdigest())
            out['cases'].append(dict(case=row['case'], measurement=value, witnesses=witnesses,
                                     passed=value['functional_ok'] and all(w['passed'] for w in witnesses.values())))
        count = sum(c['measurement']['functional_ok'] for c in out['cases'])
        footers = re.findall(r'^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', raw, re.M)
        first.need(footers == [('ok' if count == 6 else 'FAILED', str(count), str(6-count), '0')], 'Rust population')
        out.update(complete=True, passed=all(c['passed'] for c in out['cases']),
                   log=str(path.relative_to(report)), sha256=hashlib.sha256(path.read_bytes()).hexdigest())
    except (AssertionError, KeyError, ValueError, TypeError, OSError, IndexError, SyntaxError) as error:
        out.update(complete=False, passed=False, error=type(error).__name__+': '+str(error))
    return out


if __name__ == '__main__':
    report = Path(sys.argv[1]); value = verify(report)
    (report/'first-action-stream-contracts.json').write_text(json.dumps(value, indent=2)+'\n')
    sys.exit(0 if value['passed'] else 1)
