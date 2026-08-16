// The one transport seam. In the app, calls go over tauri IPC; in the
// browser (verification), the same commands go to the dev bridge — the
// real backend behind an HTTP skin, never a hand-written mock.
import type { SessionRow } from "./gen/bindings/SessionRow";
import type { DeviceRow } from "./gen/bindings/DeviceRow";
import type { FaceChoices } from "./gen/bindings/FaceChoices";

export type { SessionRow, DeviceRow, FaceChoices };

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

/** `by` is a `khor_node::list::Arrange` key — the backend arranges and
    groups; this layer passes the choice through and paints the answer. */
export const fetchSessions = (by: string) => call<SessionRow[]>("sessions", { by });
export const fetchDevices = () => call<DeviceRow[]>("devices");
export const markSeen = (id: string) => call<null>("seen", { id });
export const closeSession = (id: string) => call<null>("close_session", { id });
export const tell = (machine: string, text: string) => call<null>("tell", { machine, text });
// `on` is explicit rather than a toggle: the caller already knows the
// row's current state, and a toggle raced against a pin arriving from
// another device would flip the wrong way.
export const pinSession = (id: string, on: boolean) => call<null>("pin_session", { id, on });
export const pinDevice = (machine: string, on: boolean) => call<null>("pin_device", { machine, on });
/** What this machine wears and every option painted as it would look —
    derived by the node, one call for the whole screen. */
export const fetchFaceChoices = () => call<FaceChoices>("face_choices");
/** Changes the axes it is given; an axis left out stays where it is. The
    same call `khor face` makes, with the same shape of arguments. */
export const restyle = (change: {
  colors?: string[];
  variant?: string;
  shape?: string;
}) => call<null>("restyle", change);
export const invite = () => call<string>("invite");
export const pair = (ticket: string) => call<string>("pair", { ticket });
