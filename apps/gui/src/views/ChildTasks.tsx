import { useEffect, useRef, useState } from "react";
import { khorAgent, type SessionRow } from "@/api";
import { StatusWord } from "@/components/StatusWord";
import { Button } from "@/components/ui/button";
import { Dialog, DialogHeader, DialogTitle, TaskActions, TaskOverlay } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";

type ChildTask = {
  run: string;
  key: string;
  agent: string;
  machine: string;
  state: string;
  task: string | null;
  context: string | null;
  result: string | null;
  can_cancel: boolean;
  can_retry: boolean;
  truncated?: boolean;
};

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

export function ChildTasks({ parent, onClose }: { parent: SessionRow; onClose: () => void }) {
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
        if (!stopped) setGroups({children: rows.children, commands: rows.commands ?? []});
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

  return <Dialog open onOpenChange={open => { if (!open) onClose(); }}>
    <TaskOverlay purpose="preview" initialHeight="fit">
      <DialogHeader><DialogTitle>{task?.agent ?? gui.child_tasks}</DialogTitle></DialogHeader>
      {error && <p role="alert" className="text-callout text-destructive">{error}</p>}
      {task ? <div className="flex min-w-0 flex-col gap-row" data-child-task-detail>
        <div className="flex items-center justify-between gap-row">
          <span>{task.machine}</span><TaskStatus state={task.state}/>
        </div>
        {task.context && <span className="break-all text-caption text-muted-foreground">{task.context}</span>}
        {task.result && <p className="whitespace-pre-wrap break-words text-read">{task.result}</p>}
        {task.truncated && <span className="text-caption text-muted-foreground">{gui.child_task_output_truncated}</span>}
        <div className="flex flex-wrap gap-in">
          {task.can_cancel && <Button disabled={busy} onClick={() => void act("cancel", task)}>{gui.child_task_cancel}</Button>}
          {task.can_retry && <Button disabled={busy} onClick={() => void act("retry", task)}>{gui.child_task_retry}</Button>}
        </div>
      </div> : groups && (groups.children.length > 0 || groups.commands.length > 0) ? <div className="flex flex-col gap-base" data-child-tasks>
        {([[gui.child_task_agents, groups.children], [gui.key_group_commands, groups.commands]] as const).map(([label, rows]) =>
          rows.length > 0 && <section key={label} aria-label={label}>
          <h3 className="text-headline">{label}</h3>
          <div className="divide-y divide-border">
          {rows.map(child => <button key={identity(child)} type="button" data-child-task-row
          className="flex min-h-touch w-full items-center justify-between gap-row py-in text-left"
          onClick={() => { setDetail(identity(child)); setError(""); }}>
          <span className="flex min-w-0 flex-col">
            <span className="truncate text-headline">{child.agent}</span>
            <span className="truncate text-caption text-muted-foreground">{child.machine}</span>
          </span>
          <TaskStatus state={child.state}/>
          </button>)}
          </div>
          </section>)}
      </div> : <p aria-busy={groups === null && !error} className="text-callout text-muted-foreground">
        {groups !== null ? gui.child_tasks_empty : null}
      </p>}
      <TaskActions
        back={task ? <Button onClick={() => {setDetail(null); setError("");}}>{gui.back}</Button> : undefined}
        close={<Button onClick={onClose}>{gui.close}</Button>}/>
    </TaskOverlay>
  </Dialog>;
}
