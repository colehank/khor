import { useEffect, useRef, useState } from "react";
import { cancelMachineUpdate, checkMachineUpdate, fetchMachineInstallation, fetchMachineUpdateQueue, inApp, onMachineUpdates, startMachineUpdates, type DeviceRow, type MachineInstallation, type MachineUpdateJob, type MachineUpdateQueue } from "@/api";
import type { UpdateChannel } from "@/gen/bindings/UpdateChannel";
import { SectionHead } from "@/components/SectionHead";
import { MachineFace } from "@/components/SessionFace";
import { Button } from "@/components/ui/button";
import { PopupButton } from "@/components/ui/popup-button";
import { Dialog, DialogBody, DialogHeader, DialogTitle, TaskOverlay, TaskActions } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";

function jobLabel(job: MachineUpdateJob): string {
  switch (job.phase) {
    case "queued": return gui.update_queued;
    case "waiting": return gui.update_waiting_machine;
    case "checking": return gui.update_checking;
    case "installing": return gui.update_installing;
    case "confirming": return gui.update_confirming;
    case "complete": return job.target ? gui.update_complete : gui.update_current;
    case "failed": return job.source === "app" ? gui.update_machine_app
      : job.source === "development" ? gui.update_development
      : job.source === "external" ? gui.update_external : gui.update_failed;
  }
}

function useMachineUpdates() {
  const [queue, setQueue] = useState<MachineUpdateQueue>({ revision: 0, jobs: [] });
  const [failure, setFailure] = useState(false);
  const alive = useRef(false);
  const receive = (next: MachineUpdateQueue) => {
    if (alive.current) setQueue(previous => next.revision >= previous.revision ? next : previous);
  };
  useEffect(() => {
    alive.current = true;
    const stop = onMachineUpdates(receive);
    void fetchMachineUpdateQueue().then(receive).catch(() => { if (alive.current) setFailure(true); });
    return () => { alive.current = false; stop(); };
  }, []);
  const start = async (ids: string[], channel: UpdateChannel | null) => {
    setFailure(false);
    try { receive(await startMachineUpdates(ids, channel)); return true; }
    catch { if (alive.current) setFailure(true); return false; }
  };
  const cancel = async (job: MachineUpdateJob) => {
    setFailure(false);
    try { receive(await cancelMachineUpdate(job.machine, job.request)); }
    catch { if (alive.current) setFailure(true); }
  };
  return { jobs: queue.jobs, failure, start, cancel };
}

export function MachineUpdates({ row, rows }: { row: DeviceRow; rows: DeviceRow[] }) {
  if (!inApp || !row.hosts || row.sponsor) return null;
  return <MachineUpdateControls key={row.id} row={row} rows={rows} />;
}

function MachineUpdateControls({ row, rows }: { row: DeviceRow; rows: DeviceRow[] }) {
  const updates = useMachineUpdates();
  const [installation, setInstallation] = useState<MachineInstallation | null>(null);
  const [channel, setChannel] = useState<UpdateChannel | null>(null);
  const [available, setAvailable] = useState<string | null>(null);
  const [checked, setChecked] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const [batch, setBatch] = useState(false);
  const alive = useRef(false);
  const job = updates.jobs.find(job => job.machine === row.id);
  useEffect(() => {
    let retired = false;
    alive.current = true;
    if (job?.phase === "complete") { setAvailable(null); setChecked(false); }
    void fetchMachineInstallation(row.id).then(info => {
      if (!retired) setInstallation(info);
    }).catch(() => { if (!retired) setFailed(true); });
    return () => { retired = true; alive.current = false; };
  }, [row.id, job?.phase]);
  const check = async () => {
    if (busy) return;
    setBusy(true); setFailed(false);
    try {
      const info = await fetchMachineInstallation(row.id);
      if (!alive.current) return;
      setInstallation(info);
      if (info.source === "cli") {
        const result = await checkMachineUpdate(row.id, channel);
        if (alive.current) { setAvailable(result.available); setChecked(true); }
      }
    } catch { if (alive.current) setFailed(true); }
    finally { if (alive.current) setBusy(false); }
  };
  const working = busy || !!job && ["queued", "checking", "installing", "confirming"].includes(job.phase);
  const waiting = job?.phase === "waiting";
  const managed = installation?.source === "cli";
  const hint = installation?.source === "app" ? gui.update_machine_app
    : installation?.source === "development" ? gui.update_development
    : installation?.source === "external" ? gui.update_external : null;
  const activeJob = job?.phase === "complete" ? undefined : job;
  const label = busy ? gui.update_checking : activeJob ? jobLabel(activeJob)
    : failed || updates.failure ? gui.update_failed : available ? gui.update_available
    : checked ? gui.update_current : job?.phase === "complete" ? gui.update_complete : gui.update_label;
  return (
    <section data-machine-updates className="border-t border-border">
      <SectionHead title={gui.update_label}>
        <Button onClick={() => setBatch(true)}>{gui.update_machines}</Button>
      </SectionHead>
      <div className="flex items-center gap-row py-in">
        <div className="min-w-0 flex-1">
          <div className="text-body">{hint ?? label}</div>
          <div className="truncate text-callout text-muted-foreground tabular-nums">{activeJob?.target ?? available ?? installation?.selected ?? installation?.running}</div>
        </div>
        {!hint && (waiting && job ? <Button onClick={() => void updates.cancel(job)}>{gui.update_cancel_waiting}</Button>
          : <Button disabled={working} onClick={() => void (available || job?.phase === "failed" ? updates.start([row.id], channel) : check())}>
            {available || job?.phase === "failed" ? gui.update_start : gui.update_check}
          </Button>)}
        {!installation && failed && !job && <Button onClick={() => void updates.start([row.id], channel)}>{gui.update_queue_machine}</Button>}
      </div>
      {managed && <div className="flex items-center gap-row py-in">
        <span className="flex-1 text-body">{gui.update_channel}</span>
        <PopupButton value={channel ?? installation.channel ?? "stable"} label={gui.update_channel} disabled={working || waiting}
          choices={[{ value: "stable", label: gui.update_stable }, { value: "beta", label: gui.update_beta }]}
          onPick={value => { if (!working && (value === "stable" || value === "beta")) { setChannel(value); setChecked(false); setAvailable(null); } }} />
      </div>}
      <MachineUpdateBatch open={batch} onOpenChange={setBatch} rows={rows} updates={updates} />
    </section>
  );
}

function MachineUpdateBatch({ open, onOpenChange, rows, updates }: {
  open: boolean; onOpenChange: (open: boolean) => void; rows: DeviceRow[];
  updates: ReturnType<typeof useMachineUpdates>;
}) {
  const [selected, setSelected] = useState<string[]>([]);
  const [starting, setStarting] = useState(false);
  const eligible = rows.filter(row => row.hosts && !row.sponsor);
  const allowed = new Set(eligible.map(row => row.id));
  const ids = selected.filter(id => allowed.has(id));
  const start = async () => {
    if (starting || !ids.length) return;
    setStarting(true);
    try { if (await updates.start(ids, null)) setSelected([]); }
    finally { setStarting(false); }
  };
  return <Dialog open={open} onOpenChange={onOpenChange}>
    <TaskOverlay purpose="selection" initialHeight="fit" scrollable>
      <DialogHeader><DialogTitle>{gui.update_machines}</DialogTitle></DialogHeader>
      <DialogBody>
        <p className="text-callout text-muted-foreground">{gui.update_selected_hint}</p>
        <div className="divide-y divide-border">
          {eligible.map(row => {
            const job = updates.jobs.find(job => job.machine === row.id);
            const running = !!job && ["queued", "checking", "installing", "confirming", "waiting"].includes(job.phase);
            return <div key={row.id} className="flex items-center gap-row py-in">
              <label className="flex min-w-0 flex-1 items-center gap-row">
                <input type="checkbox" className="size-4 min-h-touch min-w-touch" checked={ids.includes(row.id)} disabled={running}
                  onChange={event => {
                    const checked = event.currentTarget.checked;
                    setSelected(previous => checked ? previous.includes(row.id) ? previous : [...previous, row.id] : previous.filter(id => id !== row.id));
                  }} />
                <MachineFace face={row.face} system={row.vitals?.vitals.system} />
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-headline">{row.shown}</span>
                  {job && <span className="block truncate text-callout text-muted-foreground">{jobLabel(job)}{job.target ? ` · ${job.target}` : ""}</span>}
                </span>
              </label>
              {job?.phase === "waiting" && <Button onClick={() => void updates.cancel(job)}>{gui.update_cancel_waiting}</Button>}
              {job && ["complete", "failed"].includes(job.phase) && <Button onClick={() => void updates.cancel(job)}>{gui.update_clear_record}</Button>}
            </div>;
          })}
        </div>
        {updates.failure && <p role="alert" className="text-callout text-muted-foreground">{gui.update_failed}</p>}
      </DialogBody>
      <TaskActions busy={starting} close={<Button onClick={() => onOpenChange(false)}>{gui.close}</Button>} primary={<Button data-footer-trailing disabled={starting || !ids.length} onClick={() => void start()}>{gui.update_start}</Button>}/>
    </TaskOverlay>
  </Dialog>;
}
