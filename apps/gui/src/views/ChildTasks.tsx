import { useEffect, useState } from "react";
import { khorAgent, type DeviceRow, type SessionRow } from "@/api";
import { StatusWord } from "@/components/StatusWord";
import { Button } from "@/components/ui/button";
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
  const [error, setError] = useState("");
  const [revision, refresh] = useState(0);
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
              {rows.map(record => {
                const child: ChildTask={...record,kind:command?"command":"agent",home:record.home??(command?parent.execution_home??undefined:undefined)};
                const device=devices.find(device=>device.id===child.home);
                const face=device?.face ?? (command || child.home && (child.home===parent.home || child.home===parent.execution_home) ? parent.face : null);
                const halo=({submitted:'busy',working:'busy',waiting:'blocked',completed:'done',failed:'failed',canceled:'idle'} as Record<string,string>)[child.state]??null;
                const preview=child.result?.trim().split('\n')[0] || device?.shown || child.machine;
                return <button key={identity(child)} type="button" data-side-child-open data-child-task-row={identity(child)}
                data-selected={selected === delegatedSessionId(parent, child) || undefined}
                className={cn("flex min-h-list-row w-full min-w-0 items-stretch gap-row rounded-md text-left focus-visible:outline-ring", selected === delegatedSessionId(parent, child) ? "bg-fill-strong" : "hover:bg-fill")}
                onClick={() => { onOpen(parent, child); setError(""); }}>
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
  </>;
}
