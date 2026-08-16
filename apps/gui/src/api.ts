// The one transport seam. In the app, calls go over tauri IPC; in the
// browser (verification), the same commands go to the dev bridge — the
// real backend behind an HTTP skin, never a hand-written mock.
import type { SessionRow } from "./gen/bindings/SessionRow";
import type { DeviceRow } from "./gen/bindings/DeviceRow";

export type { SessionRow, DeviceRow };

const bridge = new URLSearchParams(window.location.search).get("bridge");

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (bridge) {
    const r = await fetch(`http://127.0.0.1:${bridge}/${cmd}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(args ?? {}),
    });
    if (!r.ok) throw new Error(await r.text());
    return (await r.json()) as T;
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export const fetchSessions = () => call<SessionRow[]>("sessions");
export const fetchDevices = () => call<DeviceRow[]>("devices");
export const markSeen = (id: string) => call<null>("seen", { id });
export const closeSession = (id: string) => call<null>("close_session", { id });
export const tell = (machine: string, text: string) => call<null>("tell", { machine, text });
export const invite = () => call<string>("invite");
export const pair = (ticket: string) => call<string>("pair", { ticket });
