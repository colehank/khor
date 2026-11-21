import assert from 'node:assert/strict';
import {readNewSessionChoices, rememberNewSessionChoice, recentLaunchChoices, launchId, preferredLaunch, windowsShells, KHOR_OWN_AGENT} from '../src/lib/new-session-choices.ts';

const values = new Map();
const storage = {getItem: key => values.get(key) ?? null, setItem: (key, value) => values.set(key, value)};
let choices = readNewSessionChoices(storage);
choices = rememberNewSessionChoice(choices, 'machine-a', {dir: '/projects/a', launch: {kind: 'khor'}}, storage);
choices = rememberNewSessionChoice(choices, 'machine-b', {dir: 'C:\\projects\\b', launch: {kind: 'agent', agent: 'claude'}}, storage);
assert.deepEqual(readNewSessionChoices(storage), choices, 'new windows restore persisted selections');
assert.equal(choices.machine, 'machine-b');
assert.equal(choices.machines['machine-a'].dir, '/projects/a');
assert.equal(choices.machines['machine-b'].dir, 'C:\\projects\\b');
assert.deepEqual(choices.machines['machine-a'].launch, {kind: 'khor'});
assert.deepEqual(choices.machines['machine-b'].launch, {kind: 'agent', agent: 'claude'});
assert(!JSON.stringify(choices).includes('title'), 'session titles are not creation preferences');

choices = rememberNewSessionChoice(choices, 'machine-a', {launch: {kind: 'agent', agent: 'claude'}}, storage);
choices = rememberNewSessionChoice(choices, 'machine-a', {launch: {kind: 'shell', command: 'claude'}}, storage);
choices = rememberNewSessionChoice(choices, 'machine-a', {launch: {kind: 'khor'}}, storage);
const recent = choices.machines['machine-a'].recent;
assert.deepEqual(recent, ['khor', 'term:claude', 'agent:claude']);
const options = ['shell', 'agent:claude', 'khor', 'term:claude', 'agent:codex'].map(id => ({id}));
assert.deepEqual(recentLaunchChoices(options, recent).map(row => row.id), ['khor', 'term:claude', 'agent:claude', 'shell', 'agent:codex']);
assert.equal(options[0].id, 'shell', 'ranking does not mutate the source menu');
choices = rememberNewSessionChoice(choices, 'machine-a', {launch: {kind: 'shell', command: 'unfinished command'}, used: false}, storage);
assert.deepEqual(choices.machines['machine-a'].recent, recent, 'typing does not reorder an open menu');
assert.equal(launchId({kind: 'shell', command: 'before', preset: 'stable-id'}), launchId({kind: 'shell', command: 'after', preset: 'stable-id'}));
const transient = rememberNewSessionChoice(choices, 'machine-a', {dir: '/not-saved'}, {getItem: () => null, setItem: () => {throw new Error('storage unavailable');}});
assert.equal(transient.machines['machine-a'].dir, '/not-saved');

assert.deepEqual(windowsShells('windows').map(row => row.shell), ['powershell', 'cmd']);
for (const system of ['macos', 'linux', undefined]) assert.equal(windowsShells(system).length, 0);
const cmd = {kind: 'shell', command: '', shell: 'cmd'};
const powershell = {kind: 'shell', command: '', shell: 'powershell'};
choices = rememberNewSessionChoice(choices, 'windows-a', {launch: cmd}, storage);
choices = rememberNewSessionChoice(choices, 'windows-b', {launch: powershell}, storage);
const restored = readNewSessionChoices(storage);
assert.deepEqual(restored.machines['windows-a'].launch, cmd);
assert.deepEqual(restored.machines['windows-b'].launch, powershell);
assert.deepEqual(restored.machines['windows-a'].recent, ['shell:cmd']);
assert.deepEqual(restored.machines['windows-b'].recent, ['shell:powershell']);

for (const raw of ['{', 'null', '[]', '{"machine":42,"machines":{}}']) {
  assert.deepEqual(readNewSessionChoices({getItem: () => raw, setItem() {}}), {machine: '', machines: {}});
}
const unsafe = readNewSessionChoices({getItem: () => JSON.stringify({machine: 'machine-a', machines: {'machine-a': {dir: '/okay', launch: {kind: 'tell'}, recent: ['khor', 'khor', null, 5]}}}), setItem() {}});
assert.equal(unsafe.machines['machine-a'].launch, undefined);
assert.deepEqual(unsafe.machines['machine-a'].recent, ['khor']);
console.log('New session choices: per-machine persistence, stable launch IDs, MRU, typing, storage failure and corrupt data passed');

// ── The default a wizard opens on, from what this machine has ───────────
// Pi is Khor's own agent and the plan's default, but only when its ACP
// road is open: with no package runner its road is the terminal floor,
// and a *conversation* must not default to an agent that cannot answer.
assert.deepEqual(
  preferredLaunch(false, [{id: 'pi', road: 'registry-acp'}, {id: 'codex', road: 'builtin'}]),
  {kind: 'agent', agent: 'pi'},
  'khor own agent wins when it can back a conversation',
);
assert.deepEqual(
  preferredLaunch(false, [{id: 'pi', road: 'terminal'}, {id: 'codex', road: 'builtin'}]),
  {kind: 'agent', agent: 'codex'},
  'a terminal-only Pi is not a conversation default; the builtin road is',
);
assert.deepEqual(
  preferredLaunch(false, [{id: 'codex', road: 'builtin'}, {id: 'claude', road: 'terminal'}]),
  {kind: 'agent', agent: 'codex'},
  'builtin roads need no npm, so they come before the shipped default',
);
assert.deepEqual(
  preferredLaunch(false, [{id: 'aider', road: 'terminal'}]),
  {kind: 'shell', command: 'aider'},
  'with nothing able to answer, a terminal is the only form that starts — naming an absent agent is a dead 创建 button',
);
assert.deepEqual(
  preferredLaunch(true, [{id: 'pi', road: 'registry-acp'}, {id: 'codex', road: 'builtin'}]),
  {kind: 'shell', command: 'pi'},
  'the terminal form still gets the first installed program',
);
assert.equal(KHOR_OWN_AGENT, 'pi');
console.log('Preferred launch: Pi when it can answer, builtin next, a terminal when nothing can, terminal form unchanged — passed');
