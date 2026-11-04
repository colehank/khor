import {useId} from 'react';
import {MenuViewport} from '@/components/ui/menu-viewport';
import type {AgentCommand} from '@/lib/agent-commands';
import {gui} from '@/gen/catalog';
import {cn} from '@/lib/utils';

export function AgentCommandMenu({commands, pick, onPick}: {commands: AgentCommand[]; pick: number; onPick: (name: string) => void}) {
  const id = useId();
  if (!commands.length) return null;
  const selected = Math.min(pick, commands.length - 1);
  return <div data-slash-menu data-menu-frame role="listbox" aria-label={gui.agent_commands} className="glass absolute bottom-full left-0 z-20 mb-in w-full overflow-hidden rounded-menu border-0 bg-glass-menu shadow-[var(--glass-in),var(--glass-shadow)]">
    <MenuViewport active={selected}>{commands.map((command, index) => <button key={command.name} id={`${id}-${index}`} type="button" role="option" aria-selected={index === selected} data-slash-item={command.name} data-on={index === selected}
      className={cn('flex min-h-touch w-full flex-col items-start justify-center rounded-sm px-in py-in text-left text-callout pointer-fine:min-h-ctl-md', index === selected && 'bg-fill-strong text-foreground')}
      onPointerDown={event => event.preventDefault()} onMouseDown={event => event.preventDefault()} onClick={() => onPick(command.name)}>
      <span className="w-full truncate">/{command.name}</span>
      {command.description && <span className={cn('w-full truncate text-caption', index !== selected && 'text-secondary-foreground')}>{command.description}</span>}
      {command.input?.hint && <span className={cn('w-full truncate text-caption', index !== selected && 'text-secondary-foreground')}>{command.input.hint}</span>}
    </button>)}</MenuViewport>
  </div>;
}
