import type {AgentSession} from '@earendil-works/pi-coding-agent';
import commands from '../session-commands.json' with {type: 'json'};

export const availableCommands = commands;
export async function sessionCommand(session: Pick<AgentSession, 'compact' | 'getSessionStats'>, text: string): Promise<string | null> {
  const match = /^\/([^\s]+)(?:\s+([\s\S]*))?$/.exec(text.trim());
  if (!match || !commands.some(command => command.name === match[1])) return null;
  const [, name, argument = ''] = match;
  if (name === 'compact') {
    await session.compact(argument.trim() || undefined);
    return 'Context compacted';
  }
  if (argument.trim()) throw new Error(`/${name} does not accept arguments`);
  if (name === 'session') {
    const stats = session.getSessionStats();
    return `Messages: ${stats.totalMessages}\nTool calls: ${stats.toolCalls}\nTokens: ${stats.tokens.total}`;
  }
  return commands.map(command => `- /${command.name}${command.input ? ` ${command.input.hint}` : ''}: ${command.description}`).join('\n');
}
