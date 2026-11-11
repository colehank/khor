import { useCallback, useSyncExternalStore } from "react";
import type { DeviceRow, SessionRow } from "@/api";
import { gui } from "@/gen/catalog";

export type DelegatedChild = {
  kind?: "agent" | "command"; run: string; key: string; agent: string; machine: string; home?: string;
  state: string; task: string | null; context: string | null; result: string | null;
  cwd?: string | null; can_cancel: boolean; can_retry: boolean; truncated?: boolean;
};
export type DelegatedSessionRef = { parent: string; owner: string; run: string; key: string; kind: "agent" | "command" };
const PREFIX = "delegated-session:";
const metadata = new Map<string, Pick<DelegatedChild, "agent" | "machine" | "home" | "state" | "cwd" | "result">>();
const listeners = new Set<() => void>();
let revision = 0;
const subscribe = (listener: () => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; };

/** A view identity, never a new backend session or a writable attachment. */
export function delegatedSessionId(parent: SessionRow, child: Pick<DelegatedChild, "run" | "key" | "kind">) {
  return PREFIX + encodeURIComponent(JSON.stringify([parent.id, parent.execution_home, child.run, child.key, ...(child.kind === "command" ? ["command"] : [])]));
}
export function delegatedSessionRef(id: string | null | undefined): DelegatedSessionRef | null {
  if (!id?.startsWith(PREFIX) || id.length > 4096) return null;
  try {
    const value: unknown = JSON.parse(decodeURIComponent(id.slice(PREFIX.length)));
    if (!Array.isArray(value) || ![4,5].includes(value.length) || value.length===5 && value[4]!=="command" || !value.slice(0,4).every(part => typeof part === "string" && part.length > 0 && part.length <= 512 && !/[\x00-\x1f\x7f]/.test(part))) return null;
    const [parent, owner, run, key] = value as string[];
    return {parent, owner, run, key,kind:value[4]==="command"?"command":"agent"};
  } catch { return null; }
}
export function rememberDelegatedSession(id: string, child: DelegatedChild) {
  if (!delegatedSessionRef(id)) return;
  const next = {agent: child.agent, machine: child.machine, home: child.home, state: child.state, cwd: child.cwd, result: child.result?.slice(0, 200) ?? null};
  if (JSON.stringify(metadata.get(id)) === JSON.stringify(next)) return;
  metadata.set(id, next);
  if (metadata.size > 256) metadata.delete(metadata.keys().next().value!);
  revision++;
  for (const listener of listeners) listener();
}

/** Only tab presentation is derived here. Ordinary session lists stay server-owned. */
export function useSessionRowLookup(rows: readonly SessionRow[], devices: readonly DeviceRow[]) {
  const version = useSyncExternalStore(subscribe, () => revision);
  return useCallback((id: string | null | undefined): SessionRow | null => {
    const ordinary = rows.find(row => row.id === id);
    if (ordinary) return ordinary;
    if (!id || !delegatedSessionRef(id)) return null;
    const child = metadata.get(id), device = devices.find(device => device.id === child?.home);
    const word = child ? ({submitted: "busy", working: "busy", waiting: "blocked", completed: "done", failed: "failed", canceled: "idle"} as Record<string, string>)[child.state] ?? null : null;
    return {
      id, kind: "delegated", title: child?.agent || gui.child_sessions, category: delegatedSessionRef(id)?.kind==="command"?"shell":child?.agent ?? null,
      activity: [], observation_owner: null, content_locator: null, can_control: false,
      word, at_ms: 0, unread: 0, source: null, pinned: false, imported: false, stale: false, quiet: false,
      group: "", face: device?.face ?? null, hosted: false, attachable: false, last: child?.result ?? null,
      doing: null, via: null, via_face: null, home: device?.name ?? child?.machine ?? child?.home ?? "",
      place: child?.cwd ?? null, mux: false, ended: null, execution_home: child?.home ?? null,
    };
  }, [rows, devices, version]);
}
