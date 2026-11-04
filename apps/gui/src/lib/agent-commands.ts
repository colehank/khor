export type AgentCommand = {name: string; description: string; input?: {hint: string}};

export function agentCommands(value: unknown): AgentCommand[] {
  if (!Array.isArray(value)) return [];
  const names = new Set<string>();
  return value.flatMap(command => {
    if (!command || typeof command.name !== 'string' || !/^[^\s/]+$/.test(command.name) || names.has(command.name)) return [];
    names.add(command.name);
    return [{name: command.name, description: typeof command.description === 'string' ? command.description : '',
      ...(typeof command.input?.hint === 'string' ? {input: {hint: command.input.hint}} : {})}];
  });
}
export function agentCommandUpdate(value: unknown): AgentCommand[] | null {
  const update = value as {sessionUpdate?: unknown; availableCommands?: unknown} | null;
  return update?.sessionUpdate === 'available_commands_update' ? agentCommands(update.availableCommands) : null;
}
export function matchingAgentCommands(commands: AgentCommand[], text: string): AgentCommand[] {
  return /^\/\S*$/.test(text) ? commands.filter(command => command.name.toLowerCase().startsWith(text.slice(1).toLowerCase())) : [];
}
