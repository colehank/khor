import assert from 'node:assert/strict';
import {readNewSessionChoices, rememberNewSessionChoice, recentLaunchChoices, launchId} from '../src/lib/new-session-choices.ts';

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

for (const raw of ['{', 'null', '[]', '{"machine":42,"machines":{}}']) {
  assert.deepEqual(readNewSessionChoices({getItem: () => raw, setItem() {}}), {machine: '', machines: {}});
}
const unsafe = readNewSessionChoices({getItem: () => JSON.stringify({machine: 'machine-a', machines: {'machine-a': {dir: '/okay', launch: {kind: 'tell'}, recent: ['khor', 'khor', null, 5]}}}), setItem() {}});
assert.equal(unsafe.machines['machine-a'].launch, undefined);
assert.deepEqual(unsafe.machines['machine-a'].recent, ['khor']);
console.log('New session choices: per-machine persistence, stable launch IDs, MRU, typing, storage failure and corrupt data passed');
