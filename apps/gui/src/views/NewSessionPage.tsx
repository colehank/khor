import {AcpAgentSetup,EMPTY_ENVIRONMENT,type SessionEnvironment} from "@/views/AcpAgentSetup";
import {agentPresentation} from '@/lib/acp-session';
import {managePi} from "@/lib/pi-accounts";
import {KhorSessionSetup, type KhorModelSelection} from '@/views/KhorSessionSetup';
// New session: the target, its current directory, and what to run. The same
// page sits behind the desktop strip and the narrow screen's bottom deck.
import { useEffect, useRef, useState } from 'react';
import { agentAsk, agents as fetchAgents, agentsKnown, agentsKnownTaken, fetchDevices, launchPresets, saveLaunchPreset, deleteLaunchPreset, openShell, openSession, khorAgent, registerKnownAgent, type AgentRow, type DeviceRow, type KnownAgent, type LaunchPreset } from '@/api';
import { MachineField } from '@/components/MachineField';
import { PathField, settledPath } from '@/components/PathField';
import { sessionKind } from '@/components/SessionFace';
import { Button } from '@/components/ui/button';
import { ChoiceField, type ChoiceOption } from '@/components/ui/choice-field';
import { Input } from '@/components/ui/input';
import { FormGrid, FormLabel } from '@/components/ui/form-grid';
import { Dialog, DialogContent, TaskOverlay, TaskActions, DialogBody, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { IconClose, IconGear, IconPencil, IconPlus } from '@/components/icons';
import { gui } from '@/gen/catalog';
import { dropHandOff, peekHandOff } from '@/lib/composer-handoff';
import {launchId, readNewSessionChoices, recentLaunchChoices, rememberNewSessionChoice, type SessionLaunch as Launch} from '@/lib/new-session-choices';
import { useHereHosts } from '@/lib/here';
import { useSetting } from '@/settings';
import { imeKey } from '@/lib/ime';
type LaunchOption = ChoiceOption & {
  launch: Launch;
};
const defaultLaunch = (terminal: boolean): Launch => terminal ? {kind: 'shell', command: 'claude'} : {kind: 'agent', agent: 'claude'};
const creationTarget = (devices: DeviceRow[], machine: string) => devices.find(d => d.name === machine || d.id === machine) ?? (machine === '' ? devices.find(d => d.me && d.hosts) ?? devices.find(d => d.hosts && d.sponsor === null) ?? devices.find(d => d.me) : undefined);
const kindIcon = (kind: string, agent: string | null) => sessionKind(kind, agent).glyph('size-kind-mark');
function returnedSession(value: unknown): string {
  const session = (value as {session?: unknown} | null)?.session;
  if (typeof session !== 'string' || !session) throw new Error(gui.new_session_create_missing);
  return session;
}
async function createSession({launch, target, cwd, title, token, environment, selection}: {
  launch: Launch; target: DeviceRow; cwd: string; title: string; token: string;
  environment: SessionEnvironment; selection: Pick<KhorModelSelection, 'account' | 'model'> | null;
}): Promise<string> {
  if (launch.kind === 'khor') return returnedSession((await khorAgent('create', target.id, {token, title, cwd, selection})).data);
  if (launch.kind === 'shell') return openShell(target.name, cwd, launch.command, title, token);
  if (environment.mcpServers.length || environment.additionalDirectories.length)
    return returnedSession(await managePi(target.name, {operation: 'acp_open', agent: launch.agent, cwd, title, environment, token}));
  return openSession(cwd, title, 'chat', launch.agent, target.name, token);
}
export function NewSessionPage({
  onCancel,
  onOpened,
  machine: picked = '',
  dir: at = ''
}: {
  onCancel: () => void;
  onOpened: (id: string) => void;
  machine?: string;
  dir?: string;
}) {
  const here = useHereHosts(),
    wantsTerm = useSetting('session-form') === 'term';
  const [preferences, setPreferences] = useState(readNewSessionChoices);
  const [devices, setDevices] = useState<DeviceRow[]>([]),
    [machine, setMachine] = useState(picked || preferences.machine),
    [paths, setPaths] = useState<Record<string, string>>({});
  const [known, setKnown] = useState<AgentRow[]>([]),
    [presets, setPresets] = useState<LaunchPreset[]>([]);
  const [launchesLoaded, setLaunchesLoaded] = useState(false);
  const [launch, setLaunch] = useState<Launch>(() => defaultLaunch(wantsTerm));
  const [name, setName] = useState('');
  const [contextValid,setContextValid]=useState(true);
  const [environment,setEnvironment]=useState<SessionEnvironment>(EMPTY_ENVIRONMENT);
  const [khorSelection, setKhorSelection] = useState<KhorModelSelection | null>(null);
  const [pending, setPending] = useState(false),
    [opening, setOpening] = useState(false),
    [error, setError] = useState<string | null>(null);
  const [manage, setManage] = useState(false),
    [edit, setEdit] = useState<LaunchPreset | null>(null),
    [saving, setSaving] = useState(false),
    [editError, setEditError] = useState<string | null>(null);
  const [removing, setRemoving] = useState<LaunchPreset | null>(null);
  const [snapshot, setSnapshot] = useState<KnownAgent[] | null>(null),
    [taken, setTaken] = useState<[string, number, number] | null>(null),
    [browsing, setBrowsing] = useState(false),
    [query, setQuery] = useState(''),
    [attaching, setAttaching] = useState<string | null>(null);
  const [handed, setHanded] = useState<string | null>(peekHandOff);
  const contextMachine = useRef<string | null>(null),
    startedAgent = useRef(false),
    generation = useRef(0),
    creation = useRef<{key: string; token: string} | null>(null);
  const why = (e: unknown) => setError(String(e instanceof Error ? e.message : e));
  useEffect(() => {
    let live = true;
    generation.current++;
    creation.current = null;
    const restored = readNewSessionChoices();
    setPreferences(restored);
    setDevices([]);
    setLaunchesLoaded(false);
    setMachine(picked || restored.machine);
    setPaths({});
    setName('');
    setEnvironment(EMPTY_ENVIRONMENT);
    setContextValid(true);
    contextMachine.current = null;
    setOpening(false);
    setError(null);
    setHanded(peekHandOff());
    startedAgent.current = false;
    fetchDevices().then(rows => {
      if (!live) return;
      const target = creationTarget(rows, picked || restored.machine);
      contextMachine.current = target?.id ?? null;
      setDevices(rows);
      setLaunch(target && restored.machines[target.id]?.launch || defaultLaunch(wantsTerm));
    }, e => {
      if (live) why(e);
    });
    void Promise.allSettled([fetchAgents(), launchPresets()]).then(([agents, presets]) => {
      if (!live) return;
      if (agents.status === 'fulfilled') setKnown(agents.value); else why(agents.reason);
      if (presets.status === 'fulfilled') setPresets(presets.value); else why(presets.reason);
      setLaunchesLoaded(true);
    });
    return () => {
      live = false;
      generation.current++;
      creation.current = null;
    };
  }, [picked, at]);
  const target = creationTarget(devices, machine);
  const channel = target?.name || '';
  const machineId = target?.id ?? '';
  const dir = paths[machineId] ?? (machineId === contextMachine.current && at ? at : preferences.machines[machineId]?.dir ?? '');
  const agents = [...new Set(['claude', 'codex', ...known.map(a => a.name)])];
  const allChoices: LaunchOption[] = [{id:'khor',label:'khor',icon:kindIcon('broadcast','khor'),launch:{kind:'khor'}}, {
    id: 'shell',
    label: 'Shell',
    icon: kindIcon('shell', null),
    launch: {
      kind: 'shell',
      command: ''
    }
  }, ...['claude', 'codex'].map(name => ({
    id: 'term:' + name,
    label: (name === 'claude' ? 'Claude' : 'Codex') + ' · ' + gui.view_terminal,
    terms: name,
    aside: name,
    icon: kindIcon('tui', name),
    launch: {
      kind: 'shell' as const,
      command: name
    }
  })), ...agents.map(name => ({
    id: 'agent:' + name,
    label: agentPresentation(name).name + ' · ' + gui.view_chat,
    aside: agentPresentation(name).source,
    terms: name,
    icon: kindIcon('agent', name),
    launch: {
      kind: 'agent' as const,
      agent: name
    }
  })), ...presets.map(p => ({
    id: 'custom:' + p.id,
    label: p.name,
    terms: p.command,
    aside: p.command,
    icon: kindIcon('shell', null),
    launch: {
      kind: 'shell' as const,
      command: p.command,
      preset: p.id
    }
  }))];
  const choices = recentLaunchChoices(allChoices, preferences.machines[machineId]?.recent ?? []);
  const selected = choices.find(c => c.id === launchId(launch));
  const launchReady = !!selected && !(launch.kind === 'shell' && launch.preset && (selected.launch.kind !== 'shell' || selected.launch.command !== launch.command));
  const command = launch.kind === 'shell' ? launch.command : '';
  const contextReady = launch.kind === 'khor' ? khorSelection?.home === target?.id && !!khorSelection : launch.kind !== 'agent' || contextValid;
  const remember = (id: string, change: Parameters<typeof rememberNewSessionChoice>[2] = {}) => {
    const next = rememberNewSessionChoice(preferences, id, change);
    setPreferences(next);
    return next;
  };
  const pickMachine = (name: string) => {
    const target = creationTarget(devices, name);
    if (!target) return;
    const next = remember(target.id);
    setMachine(target.id);
    setLaunch(next.machines[target.id]?.launch ?? defaultLaunch(wantsTerm));
    setEnvironment(EMPTY_ENVIRONMENT); setContextValid(true);
  };
  const pickLaunch = (launch: Launch, used = true) => {
    setLaunch(launch);
    if (machineId) remember(machineId, {launch, used});
    setEnvironment(EMPTY_ENVIRONMENT); setContextValid(true);
  };
  const create = () => {
    if (opening || pending || !target?.hosts || !launchReady || !contextReady) return;
    const cwd = settledPath(dir);
    if (!cwd) {
      setError(gui.need_path);
      return;
    }
    const title = name.trim() || (launch.kind === 'shell' && launch.preset ? presets.find(p => p.id === launch.preset)?.name || '' : '');
    const selection = launch.kind === 'khor' && khorSelection ? {account: khorSelection.account, model: khorSelection.model} : null;
    const key = JSON.stringify([channel, cwd, launch, title,environment,selection]);
    if (creation.current?.key !== key) creation.current = {key, token: crypto.randomUUID()};
    const {token} = creation.current;
    const version = generation.current;
    const ownsRequest = () => generation.current === version && creation.current?.token === token;
    setOpening(true);
    setError(null);
    createSession({launch, target, cwd, title, token, environment, selection}).then(id => {
      if (!ownsRequest()) return;
      creation.current = null;
      remember(target.id, {dir: cwd, launch});
      startedAgent.current = launch.kind === 'agent';
      onOpened(id);
    }).catch(error => {
      if (ownsRequest()) why(error);
    }).finally(() => {
      if (generation.current === version && (!creation.current || creation.current.token === token)) setOpening(false);
    });
  };
  const cancel = () => {
    generation.current++;
    creation.current = null;
    if (!startedAgent.current) dropHandOff();
    onCancel();
  };
  const add = () => {
    setEditError(null);
    setEdit({
      id: crypto.randomUUID(),
      name: '',
      command
    });
  };
  const save = () => {
    if (!edit || saving) return;
    setSaving(true);
    setEditError(null);
    saveLaunchPreset(edit).then(rows => {
      setPresets(rows);
      pickLaunch({
        kind: 'shell',
        command: edit.command,
        preset: edit.id
      });
      setEdit(null);
    }).catch(e => setEditError(String(e instanceof Error ? e.message : e))).finally(() => setSaving(false));
  };
  const remove = () => {
    if (!removing || saving) return;
    setSaving(true);
    deleteLaunchPreset(removing.id).then(rows => {
      setPresets(rows);
      setRemoving(null);
    }).catch(e => setEditError(String(e instanceof Error ? e.message : e))).finally(() => setSaving(false));
  };
  const browse = () => {
    setError(null);
    setBrowsing(true);
    if (snapshot) return;
    Promise.all([agentsKnown(), agentsKnownTaken()]).then(([rows, when]) => {
      setSnapshot(rows);
      setTaken(when);
    }).catch(why);
  };
  const attach = (id: string) => {
    if (attaching) return;
    setAttaching(id);
    setError(null);
    registerKnownAgent(id).then(fetchAgents).then(rows => {
      setKnown(rows);
      pickLaunch({
        kind: 'agent',
        agent: id
      });
      setBrowsing(false);
      setManage(false);
      if (here) agentAsk(id).catch(() => {});
    }).catch(why).finally(() => setAttaching(null));
  };
  return <>
  <div data-new-session-page className="h-full min-h-0 overflow-y-auto"><div className="mx-auto flex w-full max-w-measure flex-col gap-group px-page pt-pane pb-nav-room">
   <FormGrid data-new-session-form>
    <FormLabel>{gui.new_session_machine}</FormLabel><div data-new-session-machines data-machine={channel}><MachineField devices={devices.filter(d=>d.sponsor===null)} value={channel || machine} disabled={opening} onPick={pickMachine} onPendingChange={setPending} /></div>
    <FormLabel>{gui.new_session_dir}</FormLabel><PathField key={channel} data-new-session-dir aria-label={gui.new_session_dir} machine={channel} value={dir} disabled={opening || !target} onValueChange={value => {setPaths(was => ({
            ...was,
            [machineId]: value
          })); remember(machineId, {dir: value});}} onSubmit={create} />
    <FormLabel>{gui.launch_method}</FormLabel><div data-new-session-launch data-kind={launch.kind} data-selected={selected?.id} className="flex min-w-0 flex-col gap-in"><ChoiceField label={gui.launch_method} value={launch.kind === 'shell' ? launch.command : selected?.label || (launch.kind === 'agent' ? launch.agent : 'khor')} selected={selected?.id} options={choices} disabled={opening || !target} multiline placeholder={gui.launch_shell_hint} onInput={text => pickLaunch({
              kind: 'shell',
              command: text
            }, false)} onPick={id => {
              const choice = choices.find(c => c.id === id);
              if (choice) pickLaunch(choice.launch);
            }} onSubmit={create} actions={[...(launch.kind === 'shell' && command.trim() ? [{
              id: 'save-command',
              label: gui.launch_add,
              icon: <IconPlus />,
              run: add
            }] : []), {
              id: 'manage',
              label: gui.launch_manage,
              icon: <IconGear />,
              run: () => setManage(true)
            }]} /></div>
    {launch.kind === 'shell' && <span data-new-session-launch-note className="col-start-2 text-caption text-tertiary">{gui.launch_command_note}</span>}
    {launch.kind === 'khor' && target?.hosts && <KhorSessionSetup key={target.id} machine={target.id} disabled={opening} onChange={setKhorSelection} />}
    {handed && launch.kind === 'agent' && <><FormLabel>{gui.new_session_handed}</FormLabel><span data-new-session-handed className="min-w-0 truncate font-mono text-body">{handed}</span></>}
    <FormLabel htmlFor="new-session-name">{gui.new_session_name}</FormLabel><Input id="new-session-name" data-new-session-name value={name} disabled={opening} placeholder={gui.new_session_name_hint} onChange={e => setName(e.target.value)} onKeyDown={e => { if (e.key === 'Enter' && !imeKey(e)) { e.preventDefault(); create(); } }} />
   </FormGrid>
   {launch.kind==='agent'&&<AcpAgentSetup key={`${channel}:${launch.agent}`} agent={launch.agent} machine={channel} environment={environment} onEnvironment={setEnvironment} onValid={setContextValid}/>}
   {devices.length > 0 && (!target || !target.hosts || launchesLoaded && !launchReady) && <p data-new-session-unavailable role="status" className="text-callout text-secondary-foreground">{!target ? gui.new_session_machine_missing : !target.hosts ? gui.new_session_host_required : gui.new_session_launch_missing}</p>}
   {error && <p data-new-session-error role="alert" className="text-callout text-destructive">{error}</p>}
   <div className="flex justify-end gap-in"><Button data-new-tab-cancel onMouseDown={e => e.preventDefault()} onClick={cancel}>{gui.delete_keep}</Button><Button variant="primary" data-new-session-create disabled={opening || pending || !target?.hosts || !launchReady || !contextReady || !dir.trim()} aria-busy={opening || undefined} onMouseDown={e => e.preventDefault()} onClick={create}>{gui.create}</Button></div>
  </div></div>
  <Dialog open={!!edit} onOpenChange={on => {
      if (!on && !saving) setEdit(null);
    }}><TaskOverlay purpose="edit" scrollable><DialogTitle>{presets.some(p => p.id === edit?.id) ? gui.launch_edit_title : gui.launch_add_title}</DialogTitle><DialogBody><FormGrid>
   <FormLabel htmlFor="launch-preset-name">{gui.new_session_name}</FormLabel><Input id="launch-preset-name" autoFocus value={edit?.name || ''} onChange={e => setEdit(was => was && {
            ...was,
            name: e.target.value
          })} />
   <FormLabel htmlFor="launch-preset-command">{gui.launch_command}</FormLabel><textarea id="launch-preset-command" aria-label={gui.launch_command} className="min-h-[calc(4*var(--ctl-md))] w-full resize-y rounded-sm bg-fill p-in font-mono text-body text-foreground focus-visible:outline-2 focus-visible:outline-ring" value={edit?.command || ''} onChange={e => setEdit(was => was && {
            ...was,
            command: e.target.value
          })} />
  </FormGrid>{editError && <p role="alert" className="text-callout text-destructive">{editError}</p>}</DialogBody><TaskActions busy={saving} cancel={<Button disabled={saving} onClick={() => setEdit(null)}>{gui.delete_keep}</Button>} primary={<Button variant="primary" disabled={saving} onClick={save}>{gui.watch_save}</Button>}/></TaskOverlay></Dialog>
  <Dialog open={manage && !browsing} onOpenChange={setManage}>
    <TaskOverlay purpose="selection" scrollable data-launch-manage>
      <DialogTitle>{gui.launch_method}</DialogTitle>
      <DialogBody><MenuRows choices={allChoices} presets={presets} onEdit={p => {
        setManage(false);
        setEditError(null);
        setEdit({...p});
      }} onRemove={p => {
        setEditError(null);
        setRemoving(p);
      }}/></DialogBody>
      <TaskActions close={<Button data-footer-trailing onClick={() => setManage(false)}>{gui.close}</Button>} tools={<><Button onClick={()=>{setManage(false);add()}}><IconPlus className="size-kind-mark"/>{gui.launch_add_title}</Button><Button variant="ghost" onClick={browse}>{gui.launch_attach_agent}</Button></>}/>
    </TaskOverlay>
  </Dialog>
  <Dialog open={manage && browsing} onOpenChange={on => {if(!on)setBrowsing(false)}}>
    <TaskOverlay purpose="selection" scrollable data-launch-catalog>
      <DialogTitle>{gui.launch_attach_agent}</DialogTitle>
      <Input aria-label={gui.new_session_search} placeholder={gui.new_session_search} value={query} onChange={e => setQuery(e.target.value)}/>
      <span className="text-caption text-muted-foreground">{taken ? gui.new_session_registry_taken(taken[0], taken[1]) : gui.new_session_registry}</span>
      <DialogBody data-launch-catalog-rows>
        {(snapshot || []).filter(k => `${k.id} ${k.name} ${k.about}`.toLowerCase().includes(query.toLowerCase())).map(k => <div key={k.id} className="flex min-w-0 flex-none items-center gap-in py-in">
          <div className="min-w-0 flex-1"><div className="truncate text-body">{k.name}</div><div className="truncate text-caption text-muted-foreground">{k.argv || k.about}</div></div>
          {known.some(a => a.name === k.id) ? <span className="text-caption text-muted-foreground">{gui.new_session_attached}</span> : k.argv === null ? <span className="text-caption text-muted-foreground">{gui.new_session_byhand}</span> : <Button size="sm" disabled={attaching !== null} onClick={() => attach(k.id)}>{gui.new_session_attach}</Button>}
        </div>)}
        {snapshot && !snapshot.some(k=>`${k.id} ${k.name} ${k.about}`.toLowerCase().includes(query.toLowerCase())) && <p className="text-callout text-muted-foreground">{gui.no_matches}</p>}
      </DialogBody>
      {error && <p role="alert" className="text-callout text-destructive">{error}</p>}
      <TaskActions busy={attaching !== null} back={<Button onClick={()=>setBrowsing(false)}>{gui.back}</Button>}/>
    </TaskOverlay>
  </Dialog>
  <Dialog open={!!removing} onOpenChange={on => !on && setRemoving(null)}><DialogContent><DialogTitle>{gui.launch_remove}</DialogTitle><p className="text-body">{removing?.name}</p>{editError && <p role="alert" className="text-callout text-destructive">{editError}</p>}<DialogFooter><Button disabled={saving} onClick={() => setRemoving(null)}>{gui.delete_keep}</Button><Button disabled={saving} onClick={remove}>{gui.delete}</Button></DialogFooter></DialogContent></Dialog>
 </>;
}
function MenuRows({
  choices,
  presets,
  onEdit,
  onRemove
}: {
  choices: LaunchOption[];
  presets: LaunchPreset[];
  onEdit: (p: LaunchPreset) => void;
  onRemove: (p: LaunchPreset) => void;
}) {
  return <div>{choices.map(c => {
      const preset = presets.find(p => 'custom:' + p.id === c.id);
      return <div key={c.id} className="flex min-w-0 items-center gap-row border-b py-in last:border-0"><span className="choice-row-icon">{c.icon}</span><div className="min-w-0 flex-1"><div className="truncate text-body">{c.label}</div><div className="truncate text-caption text-muted-foreground">{c.aside || (preset ? gui.launch_command : gui.launch_built_in)}</div></div>{preset && <><Button variant="ghost" aria-label={gui.accounts_edit + ' ' + preset.name} onClick={() => onEdit(preset)}><IconPencil className="size-kind-mark" /></Button><Button variant="ghost" aria-label={gui.delete + ' ' + preset.name} onClick={() => onRemove(preset)}><IconClose className="size-kind-mark" /></Button></>}</div>;
    })}</div>;
}
