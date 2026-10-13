import { useEffect, useRef, useState } from "react";
import { agentRecognition, type DeviceRow, type RecognitionEntry } from "@/api";
import { ActionRow } from "@/components/settings/ActionRow";
import { PopupRow } from "@/components/settings/PopupRow";
import { RegionState } from "@/components/RegionState";
import { gui } from "@/gen/catalog";
import { MoreSection } from "./MorePage";

const word = (entry: RecognitionEntry): string => {
  switch (entry.state) {
    case "configured": return gui.recognition_configured;
    case "changed": return gui.recognition_changed;
    case "absent": return gui.recognition_absent;
    case "error": return gui.recognition_error;
    default: return entry.agent === "claude" ? gui.recognition_claude_auto : gui.thread_unbound;
  }
};
function useRecognition(machine: string) {
  const [entries, setEntries] = useState<RecognitionEntry[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const serial = useRef(0), alive = useRef(true), mutating = useRef(false), reading = useRef<number | null>(null);
  const request = async (action: "read" | "enable" | "disable", agent?: string) => {
    if (mutating.current || (action === "read" && reading.current !== null)) return;
    const ticket = ++serial.current;
    if (action === "read") reading.current = ticket;
    if (action !== "read") { mutating.current = true; setBusy(true); }
    try {
      const answer = await agentRecognition(machine, action, agent);
      if (!alive.current || ticket !== serial.current) return;
      setEntries(answer); setError(null);
    } catch (why) {
      if (alive.current && ticket === serial.current) setError(String(why));
    } finally {
      if (reading.current === ticket) reading.current = null;
      if (action !== "read") { mutating.current = false; if (alive.current) setBusy(false); }
    }
  };
  useEffect(() => {
    alive.current = true;
    const reread = () => { if (!document.hidden) void request("read"); };
    reread();
    const timer = window.setInterval(reread, 30_000);
    window.addEventListener("focus", reread);
    document.addEventListener("visibilitychange", reread);
    return () => { alive.current = false; serial.current++; reading.current = null; clearInterval(timer); window.removeEventListener("focus", reread); document.removeEventListener("visibilitychange", reread); };
    // Owners key this component by machine; responses cannot enter another row.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [machine]);
  return { entries, error, busy, request };
}
function RecognitionRows({ machine }: { machine: string }) {
  const { entries, error, busy, request } = useRecognition(machine);
  if (error) return <RegionState state="error" message={error} onRetry={() => void request("read")} />;
  if (!entries) return <RegionState state="loading-never" />;
  return <>{entries.map((entry) => {
    const label = entry.agent === "claude" ? "Claude Code" : "Codex";
    const canChange = entry.state !== "absent" && entry.state !== "error" && (entry.state !== "configured" || entry.managed);
    const note = entry.error ?? (entry.state === "configured" && !entry.managed ? gui.recognition_user_config : entry.agent === "claude" ? gui.recognition_claude_note : gui.recognition_codex_note);
    return canChange ? <ActionRow key={entry.agent} name={`recognition-${entry.agent}`}
      label={<span className="flex flex-wrap items-baseline gap-x-in"><span>{label}</span><span className="text-callout text-secondary-foreground">{word(entry)}</span></span>}
      hint={<>{note}{entry.managed && <span className="block">{gui.recognition_removed_note}</span>}</>}
      actionLabel={entry.managed ? gui.recognition_disable : gui.recognition_enable}
      onPress={() => void request(entry.managed ? "disable" : "enable", entry.agent)} busy={busy}
      rowMark={{ "data-recognition-agent": entry.agent, "data-recognition-state": entry.state }} /> :
      <div key={entry.agent} data-setting-row="fact" data-setting-row-name={`recognition-${entry.agent}`} data-recognition-agent={entry.agent} data-recognition-state={entry.state} className="flex flex-col gap-0 px-row py-in">
        <span className="flex flex-wrap items-baseline gap-x-in text-body"><span>{label}</span><span className="text-callout text-secondary-foreground">{word(entry)}</span></span>
        {entry.state !== "absent" && <span className="text-callout text-muted-foreground">{note}</span>}
      </div>;
  })}</>;
}
export function AgentRecognition({ devices, machine, onMachine }: { devices: DeviceRow[]; machine: string | null; onMachine: (id: string) => void }) {
  const computers = devices.filter((d) => d.hosts && !d.sponsor);
  const target = machine === null ? computers.find((d) => d.me) ?? computers[0] : computers.find((d) => d.id === machine);
  useEffect(() => { if (machine === null && target) onMachine(target.id); }, [machine, target?.id, onMachine]);
  return <MoreSection name="agent-recognition" label={gui.agent_recognition}>
    {computers.length > 0 && <PopupRow name="recognition-machine" label={gui.recognition_machine} value={target?.id ?? ""} choices={[...(!target ? [{ value: "", label: gui.recognition_empty }] : []), ...computers.map((d) => ({ value: d.id, label: d.shown }))]} onPick={(id) => { if (id) onMachine(id); }} />}
    {target ? <RecognitionRows key={target.id} machine={target.id} /> : <RegionState state="empty" message={gui.recognition_empty} />}
  </MoreSection>;
}
/** Machine detail links to the sole configuration home. */
export function RecognitionSummary({ machine, onOpen }: { machine: string; onOpen: () => void }) {
  const { entries, error } = useRecognition(machine);
  return <ActionRow name="recognition-link" label={gui.agent_recognition}
    hint={error ? gui.recognition_error : entries?.map((e) => `${e.agent === "claude" ? "Claude Code" : "Codex"} · ${word(e)}`).join(" / ")}
    actionLabel={gui.recognition_open} onPress={onOpen} />;
}
