import { useEffect, useRef, useState } from "react";
import { khorAgent, type DeviceRow, type SessionRow } from "@/api";
import { StatusWord } from "@/components/StatusWord";
import { Button } from "@/components/ui/button";
import { Dialog, DialogHeader, DialogTitle, TaskActions, TaskOverlay } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import { SessionFace } from "@/components/SessionFace";
import { IconChevronRight } from "@/components/icons";
import { delegatedSessionId, type DelegatedChild } from "@/lib/delegated-sessions";
import { cn } from "@/lib/utils";

type ChildTask = DelegatedChild;

const identity = (child: ChildTask) => child.run + "/" + child.key;

function TaskStatus({ state }: {state: string}) {
  switch (state) {
    case "submitted":
    case "working": return <StatusWord word="busy"/>;
    case "completed": return <StatusWord word="done"/>;
    case "failed": return <StatusWord word="failed"/>;
    case "waiting": return <span className="text-caption text-muted-foreground">{gui.child_task_waiting}</span>;
    case "canceled": return <span className="text-caption text-muted-foreground">{gui.child_task_canceled}</span>;
    default: return <span className="text-caption text-muted-foreground">{gui.child_task_unavailable}</span>;
  }
}

export function ChildTasks({ parent, id, devices, selected, onOpen }: {
  parent: SessionRow; id: string; devices: readonly DeviceRow[]; selected: string | null;
  onOpen: (parent: SessionRow, child: DelegatedChild) => void;
}) {
  const [groups, setGroups] = useState<{children: ChildTask[]; commands: ChildTask[]} | null>(null);
  const [detail, setDetail] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [revision, refresh] = useState(0);
  const token = useRef<{ child: string; value: string } | null>(null);
  const task = groups && [...groups.children, ...groups.commands].find(child => identity(child) === detail);
  const home = parent.execution_home ?? "", session = parent.id;

  useEffect(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const read = async () => {
      try {
        const {data} = await khorAgent("manage", home, {operation: "children", session});
        const rows = data as { children?: ChildTask[]; commands?: ChildTask[] } | null;
        if (!Array.isArray(rows?.children)) throw new Error(gui.child_task_unavailable);
        if (!stopped) { setGroups({children: rows.children, commands: rows.commands ?? []}); setError(""); }
      } catch (why) {
        if (!stopped) setError(String(why));
      } finally {
        if (!stopped) timer = setTimeout(read, 5000);
      }
    };
    void read();
    return () => { stopped = true; clearTimeout(timer); };
  }, [home, session, revision]);

  const act = async (action: "cancel" | "retry", child: ChildTask) => {
    if (busy) return;
    setBusy(true);
    setError("");
    if (action === "retry" && token.current?.child !== identity(child)) {
      const bytes = crypto.getRandomValues(new Uint8Array(16));
      token.current = {child: identity(child), value: Array.from(bytes, byte => byte.toString(16).padStart(2, "0")).join("")};
    }
    try {
      await khorAgent("manage", home, {
        operation: "children", session, action, run: child.run, key: child.key,
        ...(action === "retry" ? {token: token.current?.value} : {}),
      });
      token.current = null;
      if (action === "retry") setDetail(null);
      refresh(value => value + 1);
    } catch (why) {
      setError(String(why));
    } finally {
      setBusy(false);
    }
  };

  return <>
    <div id={id} data-child-tasks={parent.id} role="group" aria-label={gui.child_tasks}
      className="min-w-0 py-in pl-[calc(var(--rows-inset)+var(--list-face)/2)] pr-row">
      <div className="flex min-w-0 flex-col gap-in border-l pl-row">
        {error && <div role="alert" className="flex min-w-0 items-center gap-in text-callout text-destructive">
          <span className="min-w-0 flex-1 break-words">{error}</span>
          <Button size="sm" onClick={() => refresh(value => value + 1)}>{gui.region_retry}</Button>
        </div>}
        {groups && (groups.children.length > 0 || groups.commands.length > 0) ?
          ([[gui.child_task_agents, groups.children, false], [gui.key_group_commands, groups.commands, true]] as const).map(([label, rows, command]) =>
            rows.length > 0 && <section key={label} aria-label={label}>
              <h3 className="pb-xxs text-group text-muted-foreground">{label}</h3>
              {rows.map(child => {
                const device=devices.find(device=>device.id===child.home);
                const face=device?.face ?? (command || child.home && (child.home===parent.home || child.home===parent.execution_home) ? parent.face : null);
                const halo=({submitted:'busy',working:'busy',waiting:'blocked',completed:'done',failed:'failed',canceled:'idle'} as Record<string,string>)[child.state]??null;
                const preview=child.result?.trim().split('\n')[0] || device?.shown || child.machine;
                return <button key={identity(child)} type="button" data-side-child-open data-child-task-row={identity(child)}
                data-selected={!command && selected === delegatedSessionId(parent, child) || undefined}
                className={cn("flex min-h-list-row w-full min-w-0 items-stretch gap-row rounded-md text-left focus-visible:outline-ring", !command && selected === delegatedSessionId(parent, child) ? "bg-fill-strong" : "hover:bg-fill")}
                onClick={() => { if (!command) onOpen(parent, child); else setDetail(identity(child)); setError(""); }}>
                <span data-child-task-face={child.home ?? parent.execution_home ?? ''} className="flex w-list-face flex-none items-center justify-center" aria-hidden>
                  <SessionFace face={face} word={halo} kind={command?'shell':'gui'} category={command?null:child.agent}/>
                </span>
                <span className="flex min-w-0 flex-1 items-center gap-row border-b py-1">
                  <span className="flex min-w-0 flex-1 flex-col">
                    <span className="truncate text-headline">{child.agent}</span>
                    <span className="truncate text-callout text-muted-foreground">{preview}</span>
                  </span>
                  <span className="min-w-0 flex-none"><TaskStatus state={child.state}/></span>
                  <IconChevronRight aria-hidden className="size-affordance flex-none text-tertiary"/>
                </span>
              </button>;})}
            </section>) : <p aria-busy={groups === null && !error} className="text-callout text-muted-foreground">
              {groups !== null ? gui.child_tasks_empty : null}
            </p>}
      </div>
    </div>
    {task && <Dialog open onOpenChange={open => { if (!open) setDetail(null); }}>
      <TaskOverlay purpose="preview" initialHeight="fit">
        <DialogHeader><DialogTitle>{task.agent}</DialogTitle></DialogHeader>
        <div className="flex min-w-0 flex-col gap-row" data-child-task-detail>
          <div className="flex min-w-0 items-center justify-between gap-row">
            <span className="min-w-0 truncate">{task.machine}</span><TaskStatus state={task.state}/>
          </div>
          {task.context && <span className="break-all text-caption text-muted-foreground">{task.context}</span>}
          {task.result && <p className="whitespace-pre-wrap break-words text-read">{task.result}</p>}
          {task.truncated && <span className="text-caption text-muted-foreground">{gui.child_task_output_truncated}</span>}
          <div className="flex flex-wrap gap-in">
            {task.can_cancel && <Button disabled={busy} onClick={() => void act("cancel", task)}>{gui.child_task_cancel}</Button>}
            {task.can_retry && <Button disabled={busy} onClick={() => void act("retry", task)}>{gui.child_task_retry}</Button>}
          </div>
          {error && <p role="alert" className="text-callout text-destructive">{error}</p>}
        </div>
        <TaskActions close={<Button onClick={() => setDetail(null)}>{gui.close}</Button>}/>
      </TaskOverlay>
    </Dialog>}
  </>;
}
