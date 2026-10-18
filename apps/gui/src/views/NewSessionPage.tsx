// New session: the target, its current directory, and what to run. The same
// page sits behind the desktop strip and the narrow screen's bottom deck.
import { useEffect, useRef, useState } from 'react';
import { agentAsk, agents as fetchAgents, agentsKnown, agentsKnownTaken, fetchDevices, launchPresets, saveLaunchPreset, deleteLaunchPreset, openShell, openSession, registerKnownAgent, type AgentRow, type DeviceRow, type KnownAgent, type LaunchPreset } from '@/api';
import { MachineField } from '@/components/MachineField';
import { PathField, settledPath } from '@/components/PathField';
import { sessionKind } from '@/components/SessionFace';
import { Button } from '@/components/ui/button';
import { ChoiceField, type ChoiceOption } from '@/components/ui/choice-field';
import { Input } from '@/components/ui/input';
import { FormGrid, FormLabel } from '@/components/ui/form-grid';
import { Dialog, DialogContent, DialogTitle, DialogFooter } from '@/components/ui/dialog';
import { IconClose, IconGear, IconPencil, IconPlus } from '@/components/icons';
import { gui } from '@/gen/catalog';
import { draftFor, dropHandOff, peekHandOff, tellDraft } from '@/lib/composer-handoff';
import { useHereHosts } from '@/lib/here';
import { useSetting } from '@/settings';
type Launch = {
  kind: 'shell';
  command: string;
  preset?: string;
} | {
  kind: 'agent';
  agent: string;
} | {
  kind: 'tell';
};
type LaunchOption = ChoiceOption & {
  launch: Launch;
};
// A response lost in transit does not turn a retry into a second session.
const creates = new Map<string, string>();
const tokenFor = (key: string) => {
  let token = creates.get(key);
  if (!token) {
    token = crypto.randomUUID();
    creates.set(key, token);
  }
  return token;
};
const kindIcon = (kind: string, agent: string | null) => sessionKind(kind, agent).glyph('size-kind-mark');
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
  const [devices, setDevices] = useState<DeviceRow[]>([]),
    [machine, setMachine] = useState(picked),
    [paths, setPaths] = useState<Record<string, string>>({});
  const [known, setKnown] = useState<AgentRow[]>([]),
    [presets, setPresets] = useState<LaunchPreset[]>([]);
  const [launch, setLaunch] = useState<Launch>(() => wantsTerm ? {
    kind: 'shell',
    command: 'claude'
  } : {
    kind: 'agent',
    agent: 'claude'
  });
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
    startedAgent = useRef(false);
  const why = (e: unknown) => setError(String(e instanceof Error ? e.message : e));
  useEffect(() => {
    let live = true;
    setDevices([]);
    setMachine(picked);
    setPaths({});
    contextMachine.current = null;
    setOpening(false);
    setError(null);
    setHanded(peekHandOff());
    startedAgent.current = false;
    fetchDevices().then(rows => {
      if (live) setDevices(rows);
    }, e => {
      if (live) why(e);
    });
    fetchAgents().then(rows => {
      if (live) setKnown(rows);
    }, e => {
      if (live) why(e);
    });
    launchPresets().then(rows => {
      if (live) setPresets(rows);
    }, e => {
      if (live) why(e);
    });
    return () => {
      live = false;
    };
  }, [picked, at]);
  const target = devices.find(d => d.name === machine || d.id === machine) ?? (machine === '' ? devices.find(d => d.me && d.hosts) ?? devices.find(d => d.hosts && d.sponsor === null) ?? devices.find(d => d.me) : undefined);
  const channel = target?.name || '';
  if (contextMachine.current === null && target) contextMachine.current = channel;
  const dir = paths[channel] ?? (channel === contextMachine.current ? at : '');
  const agents = [...new Set(['claude', 'codex', ...known.map(a => a.name)])];
  const allChoices: LaunchOption[] = [{
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
    label: (name === 'claude' ? 'Claude' : name === 'codex' ? 'Codex' : name) + ' · ' + gui.view_chat,
    terms: name,
    icon: kindIcon('agent', name),
    launch: {
      kind: 'agent' as const,
      agent: name
    }
  })), {
    id: 'tell',
    label: gui.message_text,
    icon: kindIcon('broadcast', null),
    launch: {
      kind: 'tell'
    }
  }, ...presets.map(p => ({
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
  const choices = target?.hosts === false ? allChoices.filter(c => c.launch.kind === 'tell') : allChoices;
  useEffect(() => {
    if (target?.hosts === false) setLaunch({
      kind: 'tell'
    });
  }, [target?.hosts]);
  const selected = launch.kind === 'agent' ? choices.find(c => c.id === 'agent:' + launch.agent) : launch.kind === 'tell' ? choices.find(c => c.id === 'tell') : choices.find(c => c.id === 'custom:' + launch.preset && c.launch.kind === 'shell' && c.launch.command === launch.command) ?? choices.find(c => c.launch.kind === 'shell' && c.launch.command === launch.command);
  const command = launch.kind === 'shell' ? launch.command : '';
  const create = () => {
    if (opening || pending || !target) return;
    if (target.hosts === false && launch.kind !== 'tell') return;
    if (launch.kind === 'tell') {
      if (!target.me) draftFor(tellDraft(target.name));
      onOpened('wide/all');
      return;
    }
    const cwd = settledPath(dir);
    if (!cwd) {
      setError(gui.need_path);
      return;
    }
    const title = launch.kind === 'shell' && launch.preset ? presets.find(p => p.id === launch.preset)?.name || '' : '';
    const key = JSON.stringify([channel, cwd, launch, title]);
    const token = tokenFor(key);
    setOpening(true);
    setError(null);
    (launch.kind === 'shell' ? openShell(channel, cwd, launch.command, title, token) : openSession(cwd, '', 'chat', launch.agent, channel, token)).then(id => {
      creates.delete(key);
      startedAgent.current = launch.kind === 'agent';
      onOpened(id);
    }).catch(why).finally(() => setOpening(false));
  };
  const cancel = () => {
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
      setLaunch({
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
      setLaunch({
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
    <FormLabel>{gui.new_session_machine}</FormLabel><div data-new-session-machines data-machine={channel}><MachineField devices={devices.filter(d=>d.sponsor===null)} value={channel || machine} disabled={opening} onPick={setMachine} onPendingChange={setPending} /></div>
    <FormLabel>{gui.new_session_dir}</FormLabel><PathField key={channel} data-new-session-dir aria-label={gui.new_session_dir} machine={channel} value={dir} disabled={opening || !target} onValueChange={value => setPaths(was => ({
            ...was,
            [channel]: value
          }))} onSubmit={create} />
    <FormLabel>{gui.launch_method}</FormLabel><div data-new-session-launch data-kind={launch.kind} data-selected={selected?.id} className="flex min-w-0 flex-col gap-in"><ChoiceField label={gui.launch_method} value={launch.kind === 'shell' ? launch.command : selected?.label || ''} selected={selected?.id} options={choices} disabled={opening} multiline placeholder={gui.launch_shell_hint} onInput={target?.hosts === false ? undefined : text => setLaunch({
              kind: 'shell',
              command: text
            })} onPick={id => {
              const choice = choices.find(c => c.id === id);
              if (choice) setLaunch(choice.launch);
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
            }]} />{launch.kind === 'shell' && <span className="text-caption text-tertiary">{gui.launch_command_note}</span>}</div>
    {handed && launch.kind === 'agent' && <><FormLabel>{gui.new_session_handed}</FormLabel><span data-new-session-handed className="min-w-0 truncate font-mono text-body">{handed}</span></>}
   </FormGrid>
   {error && <p data-new-session-error role="alert" className="text-callout text-destructive">{error}</p>}
   <div className="flex justify-end gap-in"><Button data-new-tab-cancel onMouseDown={e => e.preventDefault()} onClick={cancel}>{gui.delete_keep}</Button><Button variant="primary" data-new-session-create disabled={opening || pending || !target || launch.kind !== 'tell' && !dir.trim()} aria-busy={opening || undefined} onMouseDown={e => e.preventDefault()} onClick={create}>{gui.create}</Button></div>
  </div></div>
  <Dialog open={!!edit} onOpenChange={on => {
      if (!on && !saving) setEdit(null);
    }}><DialogContent form><DialogTitle>{presets.some(p => p.id === edit?.id) ? gui.launch_edit_title : gui.launch_add_title}</DialogTitle><FormGrid>
   <FormLabel htmlFor="launch-preset-name">{gui.new_session_name}</FormLabel><Input id="launch-preset-name" autoFocus value={edit?.name || ''} onChange={e => setEdit(was => was && {
            ...was,
            name: e.target.value
          })} />
   <FormLabel htmlFor="launch-preset-command">{gui.launch_command}</FormLabel><textarea id="launch-preset-command" aria-label={gui.launch_command} className="min-h-[calc(4*var(--ctl-md))] w-full resize-y rounded-sm bg-fill p-in font-mono text-body text-foreground focus-visible:outline-2 focus-visible:outline-ring" value={edit?.command || ''} onChange={e => setEdit(was => was && {
            ...was,
            command: e.target.value
          })} />
  </FormGrid>{editError && <p role="alert" className="text-callout text-destructive">{editError}</p>}<DialogFooter><Button disabled={saving} onClick={() => setEdit(null)}>{gui.delete_keep}</Button><Button variant="primary" disabled={saving} onClick={save}>{gui.watch_save}</Button></DialogFooter></DialogContent></Dialog>
  <Dialog open={manage} onOpenChange={setManage}><DialogContent form><DialogTitle>{gui.launch_method}</DialogTitle><MenuRows choices={allChoices} presets={presets} onEdit={p => {
          setManage(false);
          setEditError(null);
          setEdit({
            ...p
          });
        }} onRemove={p => {
          setEditError(null);
          setRemoving(p);
        }} />
   <div className="flex flex-wrap gap-in"><Button onClick={()=>{setManage(false);add()}}><IconPlus className="size-kind-mark"/>{gui.launch_add_title}</Button><Button variant="ghost" onClick={browse}>{gui.launch_attach_agent}</Button></div>
   {browsing && <div className="flex min-w-0 flex-col gap-in"><span className="text-caption text-muted-foreground">{taken ? gui.new_session_registry_taken(taken[0], taken[1]) : gui.new_session_registry}</span><Input aria-label={gui.new_session_search} placeholder={gui.new_session_search} value={query} onChange={e => setQuery(e.target.value)} /><div className="max-h-56 overflow-y-auto">{(snapshot || []).filter(k => `${k.id} ${k.name} ${k.about}`.toLowerCase().includes(query.toLowerCase())).map(k => <div key={k.id} className="flex min-w-0 items-center gap-in py-in"><div className="min-w-0 flex-1"><div className="truncate text-body">{k.name}</div><div className="truncate text-caption text-muted-foreground">{k.argv || k.about}</div></div>{known.some(a => a.name === k.id) ? <span className="text-caption text-muted-foreground">{gui.new_session_attached}</span> : k.argv === null ? <span className="text-caption text-muted-foreground">{gui.new_session_byhand}</span> : <Button size="sm" disabled={attaching !== null} onClick={() => attach(k.id)}>{gui.new_session_attach}</Button>}</div>)}</div></div>}
   <DialogFooter><Button onClick={() => setManage(false)}>{gui.edit_done}</Button></DialogFooter></DialogContent></Dialog>
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
  return <div className="max-h-[calc(6*var(--pane-head))] overflow-y-auto">{choices.map(c => {
      const preset = presets.find(p => 'custom:' + p.id === c.id);
      return <div key={c.id} className="flex min-w-0 items-center gap-row border-b py-in last:border-0"><span className="choice-row-icon">{c.icon}</span><div className="min-w-0 flex-1"><div className="truncate text-body">{c.label}</div><div className="truncate text-caption text-muted-foreground">{c.aside || (preset ? gui.launch_command : gui.launch_built_in)}</div></div>{preset && <><Button variant="ghost" aria-label={gui.accounts_edit + ' ' + preset.name} onClick={() => onEdit(preset)}><IconPencil className="size-kind-mark" /></Button><Button variant="ghost" aria-label={gui.delete + ' ' + preset.name} onClick={() => onRemove(preset)}><IconClose className="size-kind-mark" /></Button></>}</div>;
    })}</div>;
}
