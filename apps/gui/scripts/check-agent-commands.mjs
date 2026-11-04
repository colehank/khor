import assert from 'node:assert/strict';
import {agentCommands, agentCommandUpdate, matchingAgentCommands} from '../src/lib/agent-commands.ts';

const first = agentCommandUpdate({sessionUpdate: 'available_commands_update', availableCommands: [
  {name: 'review', description: 'Review code', input: {hint: 'files to review'}},
  {name: 'compact', description: 'Compact context'},
]});
assert.deepEqual(first[0], {name: 'review', description: 'Review code', input: {hint: 'files to review'}});
assert.deepEqual(matchingAgentCommands(first, '/RE').map(command => command.name), ['review']);
assert.deepEqual(matchingAgentCommands(first, '/review file.ts'), [], 'arguments close the command picker');
assert.deepEqual(matchingAgentCommands(first, 'ordinary text'), []);
assert.deepEqual(agentCommandUpdate({sessionUpdate: 'available_commands_update', availableCommands: []}), [], 'an empty publication clears previous commands');
assert.equal(agentCommandUpdate({sessionUpdate: 'agent_message_chunk', content: {text: '/review'}}), null, 'message text is not a command announcement');
assert.deepEqual(agentCommandUpdate({sessionUpdate: 'available_commands_update', availableCommands: [{name: 'session', description: 'Statistics'}]}).map(command => command.name), ['session'], 'a new publication replaces the complete list');
assert.deepEqual(agentCommands([{name: 'review'}, {name: 'review'}, {name: '/bad'}, {name: 'two words'}, null]), [{name: 'review', description: ''}]);
assert.deepEqual(agentCommands(null), []);
console.log('Agent commands: full replacement/clear, input hints, exact prefix matching, malformed data and message isolation passed');
