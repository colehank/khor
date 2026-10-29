#!/usr/bin/env python3
"""Synthetic ACP v1 peer; no model, network service, or user files."""
import json
import os
import sys
import threading
import time
from pathlib import Path

mode = sys.argv[1]
root = Path(sys.argv[2])
root.mkdir(parents=True, exist_ok=True)
lock = threading.Lock()
stop = threading.Event()
pending = None
turns = 0
(root / 'pid.json').write_text(json.dumps({
    'pid': os.getpid(), 'pgid': os.getpgrp(), 'cwd': os.getcwd(),
    'ambient': os.environ.get('KHOR_ACP_AMBIENT'),
}))


def mark(name, value):
    with (root / (name + '.jsonl')).open('a') as f:
        f.write(json.dumps(value) + '\n')


def send(value):
    with lock:
        print(json.dumps(value, separators=(',', ':')), flush=True)


def note(text, session='synthetic-q037-session'):
    send({'jsonrpc': '2.0', 'method': 'session/update', 'params': {
        'sessionId': session,
        'update': {'sessionUpdate': 'agent_message_chunk',
                   'content': {'type': 'text', 'text': text}},
    }})


def answer(request_id, result):
    send({'jsonrpc': '2.0', 'id': request_id, 'result': result})


def flood():
    n = 0
    large = mode == 'large-flood'
    while not stop.wait(.04 if large else .0005):
        try:
            note('synthetic-noise-' + str(n) + 'x' * (256 * 1024 if large else 128))
            n += 1
            if n == (32 if large else 100):
                mark('flood-progress', {'frames': n})
        except (BrokenPipeError, OSError):
            return


def release_control(request_id):
    while not stop.wait(.005):
        if (root / 'release-control').exists():
            answer(request_id, {})
            mark('control-released', {'id': request_id})
            return


insurance = threading.Timer(55, lambda: os._exit(90))
insurance.daemon = True
insurance.start()
try:
    for line in sys.stdin:
        message = json.loads(line)
        method = message.get('method')
        request_id = message.get('id')
        mark('received', {'method': method, 'id': request_id, 'at': time.monotonic()})
        if method == 'initialize':
            if mode == 'handshake-stall':
                continue
            answer(request_id, {
                'protocolVersion': 2 if mode == 'handshake-refuse' else 1,
                'agentCapabilities': {'loadSession': True,
                                      **({'sessionCapabilities': {'resume': {}}} if mode == 'journal-resume' else {}),
                                      **({'promptCapabilities': {'image': True, 'embeddedContext': True}} if mode == 'journal-load' else {})},
                'authMethods': [{'id': 'synthetic', 'name': 'Synthetic', 'type': 'agent'}],
                'agentInfo': {'name': 'synthetic-q037', 'version': '1'},
            })
        elif method == 'session/new':
            if mode == 'journal-invalid':
                note('invalid source', 'another-session')
            if mode == 'journal-opening':
                for n in range(20000):
                    note('opening-' + str(n))
            answer(request_id, {'sessionId': 'synthetic-q037-session'})
        elif method == 'session/resume':
            answer(request_id, {})
        elif method == 'session/load':
            mark('load', {'id': request_id})
            if mode == 'drip-replay':
                note('drip-large:' + 'd' * (30 * 1024 * 1024))
                for n in range(4096):
                    note('drip-tail-' + str(n) + ':' + 't' * 64)
            elif mode in ('many-small-replay', 'cancel-replay'):
                for n in range(16384):
                    note('replay-item-' + str(n) + ':' + 'r' * (1024 if mode == 'cancel-replay' else 64))
            elif mode == 'journal-load':
                note('journal snapshot base')
            else:
                note('synthetic replay final note')
            answer(request_id, {})
        elif method == 'authenticate':
            mark('control', {'id': request_id})
            if mode != 'control-timeout':
                threading.Thread(target=release_control, args=(request_id,), daemon=True).start()
        elif method == 'session/prompt':
            turns += 1
            pending = request_id
            mark('prompt', {'id': request_id, 'turn': turns})
            if mode == 'journal-load':
                captures = list((root.parent / 'store/.khor/acp-journal').glob('*/current/records.jsonl'))
                prompts = [json.loads(record) for path in captures for record in path.read_text().splitlines() if json.loads(record).get('kind') == 'prompt']
                mark('accepted', {'before_dispatch': bool(prompts), 'content': message['params']['prompt']})
                note('journal repeated delta')
                note('journal repeated delta')
                send({'jsonrpc': '2.0', 'method': 'session/update', 'params': {
                    'sessionId': 'synthetic-q037-session', 'update': {
                        'sessionUpdate': 'tool_call', 'toolCallId': 'journal-tool',
                        'title': 'Edit', 'kind': 'edit', 'status': 'completed',
                        'content': [{'type': 'diff', 'path': 'a.txt', 'oldText': 'old', 'newText': 'new'}],
                        'rawInput': {'path': 'a.txt'},
                    }}})
                answer(request_id, {'stopReason': 'max_tokens'})
                pending = None
                continue
            note('synthetic prompt admitted')
            if mode in ('notes-flood', 'cooperative-flood', 'large-flood') and turns == 1:
                threading.Thread(target=flood, daemon=True).start()
            if turns > 1 or mode in ('normal', 'cancel-replay', 'drip-replay'):
                note('synthetic final note before normal completion')
                answer(request_id, {'stopReason': 'end_turn'})
                pending = None
            if mode == 'stdin-stall':
                while True:
                    time.sleep(.1)
        elif method == 'session/cancel':
            mark('cancel', {'prompt': pending, 'at': time.monotonic()})
            if mode in ('cooperative', 'cooperative-flood') and pending is not None:
                note('synthetic final note after cancel')
                answer(pending, {'stopReason': 'cancelled'})
                pending = None
        elif request_id is not None:
            send({'jsonrpc': '2.0', 'id': request_id, 'error': {
                'code': -32601, 'message': 'synthetic method unavailable',
            }})
finally:
    stop.set()
