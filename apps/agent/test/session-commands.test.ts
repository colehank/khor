import {test} from 'node:test';
import assert from 'node:assert/strict';
import type {AgentSession} from '@earendil-works/pi-coding-agent';
import {availableCommands, sessionCommand} from '../src/session-commands.ts';

test('embedded commands invoke Pi session APIs and never translate into network CLI verbs', async () => {
  const calls: unknown[] = [];
  const session = {
    compact: async (instructions?: string) => {calls.push(['compact', instructions]);},
    getSessionStats: () => ({totalMessages: 4, toolCalls: 2, tokens: {total: 123}}),
  } as unknown as Pick<AgentSession, 'compact' | 'getSessionStats'>;
  assert(availableCommands.some(command => command.name === 'compact' && command.input?.hint));
  assert.match((await sessionCommand(session, '/compact keep decisions'))!, /compacted/);
  assert.deepEqual(calls, [['compact', 'keep decisions']]);
  assert.match((await sessionCommand(session, '/session'))!, /Messages: 4/);
  assert.match((await sessionCommand(session, '/help'))!, /\/compact/);
  assert.deepEqual(calls, [['compact', 'keep decisions']], 'statistics and help do not compact or call a model');
  assert.equal(await sessionCommand(session, '/status'), null, 'unknown commands remain ordinary Pi input');
  assert.equal(await sessionCommand(session, 'a normal task'), null);
  await assert.rejects(sessionCommand(session, '/session ignored argument'), /does not accept arguments/);
});
