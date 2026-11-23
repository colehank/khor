export type WindowsShell = 'powershell' | 'cmd';

export type SessionLaunch = {kind: 'shell'; command: string; preset?: string; shell?: WindowsShell} | {kind: 'agent'; agent: string} | {kind: 'khor'};

const WINDOWS_SHELLS = [
  {shell: 'powershell', label: 'PowerShell', terms: 'pwsh powershell.exe'},
  {shell: 'cmd', label: 'CMD', terms: 'cmd.exe'},
] as const;

export function windowsShells(system: string | null | undefined) {
  return system === 'windows' ? WINDOWS_SHELLS : [];
}

const isWindowsShell = (value: unknown): value is WindowsShell => WINDOWS_SHELLS.some(option => option.shell === value);

/** The agent the wizard ships pointing at, and the one a machine with
    nothing installed still names: Claude Code is the most common
    install, and naming an agent that is absent is at least a sentence a
    person can act on ("install this"), where a substituted name they
    cannot start would not be. */
export const DEFAULT_AGENT = 'claude';

/** **Khor's own agent.** The stage plan's rule is that a Khor-owned
    conversation should default to Pi — the road DeepSeek runs through,
    since the ACP registry has no DeepSeek adapter. It is preferred only
    when the roster says it can actually back a conversation: its road is
    an ACP one (`registry-acp` needs the package runner), never the
    terminal floor. */
export const KHOR_OWN_AGENT = 'pi';

/** Roads that can back a Khor-owned conversation: the agent answers over
    ACP. `terminal` is the floor — it can be typed into, not spoken to. */
const SPEAKS_ACP = ['builtin', 'native-acp', 'registry-acp'];

/**
 * The launch a wizard should open on, given what this machine has.
 *
 * The shipped default is `claude`. That is right on a machine where
 * Claude Code is installed and wrong on one where it is not: it opens a
 * conversation whose agent cannot start, which reads as "khor is broken"
 * rather than "that agent is not installed". So the default follows the
 * roster (`agent_fleet`), in this order:
 *
 * 1. **Khor's own agent (Pi)**, when its ACP road is open — the plan's
 *    rule, and the reason it is not unconditional: on a machine without
 *    the package runner its road is the terminal floor, and defaulting a
 *    *conversation* to an agent that cannot answer is the failure the
 *    independent review caught elsewhere;
 * 2. **a builtin road** — khor's own shims need no npm and no
 *    registration;
 * 3. **the first program this machine actually has**, as a terminal.
 *
 * **Step 3 is the one that matters on Windows, and it used to be the
 * shipped default instead.** The roster could not see Codex at all when
 * its installer kept the binary in a versioned folder
 * (`OpenAI/Codex/bin/<hash>/codex.exe`, not on `PATH`), so the roster came
 * back empty and the wizard opened on Claude — an agent that was not
 * installed either. Naming an absent agent is not a "default worth
 * installing"; it is a form whose 创建 button cannot work. When nothing can
 * answer over ACP, the honest default is a terminal, which every machine
 * has.
 *
 * Only for the agent form: a terminal can run any command, so it gets
 * the first installed program.
 *
 * **Only a default.** The caller applies this when there is no
 * remembered choice; a person's own pick is never routed around.
 */
export function preferredLaunch(
  terminal: boolean,
  agents: readonly {id: string; road: string | null}[],
): SessionLaunch {
  if (terminal) return {kind: 'shell', command: agents[0]?.id ?? ''};
  const speaks = agents.filter(a => a.road !== null && SPEAKS_ACP.includes(a.road));
  const own = speaks.find(a => a.id === KHOR_OWN_AGENT)?.id;
  const builtin = speaks.find(a => a.road === 'builtin')?.id;
  const agent = own ?? builtin;
  if (agent) return {kind: 'agent', agent};
  // Nothing on this machine answers over ACP. A terminal is the one form
  // that is guaranteed to start, and naming an agent that is not installed
  // is how the wizard came to open on a dead choice (see above).
  return {kind: 'shell', command: agents[0]?.id ?? ''};
}
type MachineChoices = {dir?: string; launch?: SessionLaunch; recent: string[]};
export type NewSessionChoices = {machine: string; machines: Record<string, MachineChoices>};
type StoragePort = Pick<Storage, 'getItem' | 'setItem'>;
const KEY = 'khor.new-session.v1';
const text = (value: unknown, limit: number): value is string => typeof value === 'string' && value.length <= limit && !value.includes('\0');
const object = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value);
function browserStorage(): StoragePort | null {
  try {return typeof window === 'undefined' ? null : window.localStorage;} catch {return null;}
}
function launch(value: unknown): SessionLaunch | undefined {
  if (!object(value)) return;
  if (value.kind === 'khor') return {kind: 'khor'};
  if (value.kind === 'agent' && text(value.agent, 256) && value.agent) return {kind: 'agent', agent: value.agent};
  if (value.kind === 'shell' && text(value.command, 16384) && (value.preset === undefined || text(value.preset, 256)) &&
      (value.shell === undefined || isWindowsShell(value.shell)))
    return {kind: 'shell', command: value.command, ...(value.preset ? {preset: value.preset} : {}), ...(value.shell ? {shell: value.shell} : {})};
}
export function launchId(value: SessionLaunch): string {
  if (value.kind === 'agent') return `agent:${value.agent}`;
  if (value.kind === 'khor') return 'khor';
  if (value.preset) return `custom:${value.preset}`;
  if (value.shell) return `shell:${value.shell}`;
  return ['claude', 'codex'].includes(value.command) ? `term:${value.command}` : 'shell';
}
export function readNewSessionChoices(storage: StoragePort | null = browserStorage()): NewSessionChoices {
  const empty = {machine: '', machines: {}};
  try {
    const value: unknown = JSON.parse(storage?.getItem(KEY) ?? 'null');
    if (!object(value) || !text(value.machine, 256) || !object(value.machines)) return empty;
    const machines: [string, MachineChoices][] = [];
    for (const [id, choice] of Object.entries(value.machines)) {
      if (!id || !text(id, 256) || !object(choice)) continue;
      const selected = launch(choice.launch);
      machines.push([id, {
        ...(text(choice.dir, 4096) ? {dir: choice.dir} : {}),
        ...(selected ? {launch: selected} : {}),
        recent: Array.isArray(choice.recent) ? [...new Set(choice.recent.filter(id => text(id, 512)))].slice(0, 32) : [],
      }]);
    }
    return {machine: value.machine, machines: Object.fromEntries(machines)};
  } catch {return empty;}
}
export function rememberNewSessionChoice(choices: NewSessionChoices, machine: string, change: {dir?: string; launch?: SessionLaunch; used?: boolean} = {}, storage: StoragePort | null = browserStorage()): NewSessionChoices {
  const previous = choices.machines[machine];
  const recent = previous?.recent ?? [];
  const id = change.launch && launchId(change.launch);
  const next = {machine, machines: {...choices.machines, [machine]: {
    ...previous,
    ...(change.dir === undefined ? {} : {dir: change.dir}),
    ...(change.launch === undefined ? {} : {launch: change.launch}),
    recent: id && change.used !== false ? [id, ...recent.filter(value => value !== id)].slice(0, 32) : recent,
  }}};
  try {storage?.setItem(KEY, JSON.stringify(next));} catch {}
  return next;
}
export function recentLaunchChoices<T extends {id: string}>(options: T[], recent: string[]): T[] {
  const order = new Map(recent.map((id, index) => [id, index]));
  return [...options].sort((a, b) => (order.get(a.id) ?? Infinity) - (order.get(b.id) ?? Infinity));
}
