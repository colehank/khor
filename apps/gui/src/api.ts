// The one transport seam. In the app, calls go over tauri IPC; in the
// browser (verification), the same commands go to the dev bridge — the
// real backend behind an HTTP skin, never a hand-written mock.
import type { SessionRow } from "./gen/bindings/SessionRow";
import type { DeviceRow } from "./gen/bindings/DeviceRow";
import type { FaceChoices } from "./gen/bindings/FaceChoices";
import type { HooksState } from "./gen/bindings/HooksState";
import type { Ticket } from "./gen/bindings/Ticket";
import type { Usage } from "./gen/bindings/Usage";
import type { Strain } from "./gen/bindings/Strain";
import type { ChatBatch } from "./gen/bindings/ChatBatch";
import type { ChatFrame } from "./gen/bindings/ChatFrame";
import type { DirListing } from "./gen/bindings/DirListing";
import type { DirPinRow } from "./gen/bindings/DirPinRow";
import type { WebPinRow } from "./gen/bindings/WebPinRow";
import type { WebBorrow } from "./gen/bindings/WebBorrow";
import type { TermBatch } from "./gen/bindings/TermBatch";
import type { TermScreen } from "./gen/bindings/TermScreen";
import type { TermRun } from "./gen/bindings/TermRun";
import type { TermColor } from "./gen/bindings/TermColor";

export type {
  SessionRow,
  DeviceRow,
  FaceChoices,
  HooksState,
  Ticket,
  Usage,
  Strain,
  ChatBatch,
  ChatFrame,
  DirListing,
  DirPinRow,
  WebPinRow,
  WebBorrow,
  TermBatch,
  TermScreen,
  TermRun,
  TermColor,
};

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
/** What every machine has spent, by day and by vendor — the whole
    answer, because a screen scrolls. Asked on its own rather than riding
    the device rows: those are polled every couple of seconds and this is
    a list of every day there has ever been. */
export const fetchUsage = () => call<Usage>("usage");
export const markSeen = (id: string) => call<null>("seen", { id });
export const closeSession = (id: string) => call<null>("close_session", { id });
export const tell = (machine: string, text: string) => call<null>("tell", { machine, text });
/** The wizard (会话身份批B): a fresh claude session in `dir`, as a
    conversation ("chat") or a terminal ("term"). Answers the new row's id. */
export const openSession = (dir: string, title: string, form: "chat" | "term") =>
  call<string>("open_session", { dir, title, form });
/** 接管 (批C): end the session's terminal side; the conversation continues here. */
export const takeover = (id: string) => call<null>("takeover", { id });
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
/** Whether claude on **this** machine is set up to tell khor what it is
    doing. No machine argument on any of these three: the settings file
    is rooted at this node's own vendor home, so there is nothing to
    point at somebody else. Each write answers with the state after it,
    so a button never has to guess what it just did. */
export const fetchHooks = () => call<HooksState>("hooks_state");
export const installHooks = () => call<HooksState>("install_hooks");
export const uninstallHooks = () => call<HooksState>("uninstall_hooks");
/** The ticket and how long it lasts. The window comes from the library
    that enforces it — a `15` written into the dialog would be a second
    copy of a number only `link::INVITE_WINDOW_MS` decides. */
export const invite = () => call<Ticket>("invite");
export const pair = (ticket: string) => call<string>("pair", { ticket });
/** The chat six: an attachment registry in gui-core holds the socket
    (a webview cannot), a reader thread collects frames, and the poll
    answers instantly from a cursor — nothing is lost between polls.
    `chatLeave` detaches only; ending a conversation is `closeSession`. */
/** Answers whether this call attached anew — replay history only then:
    a second open of a living chat (dev double-mount) must not ask the
    host to replay again. */
export const chatOpen = (id: string) => call<boolean>("chat_open", { id });
/** Ends the running turn — not the session (`GuiOp::Stop`). */
export const chatStop = (id: string) => call<null>("chat_stop", { id });
export const chatPoll = (id: string, since: number) =>
  call<ChatBatch>("chat_poll", { id, since });
export const chatSay = (id: string, text: string) => call<null>("chat_say", { id, text });
export const chatAnswer = (id: string, ask: number, option: string | null) =>
  call<null>("chat_answer", { id, ask, option });
export const chatReplay = (id: string) => call<null>("chat_replay", { id });
export const chatLeave = (id: string) => call<null>("chat_leave", { id });
/** A discovered session's recorded past: the vendor's transcript in
    replay-shaped frames, whole, ending in history_end — so the same
    fold paints it and a live replay alike. */
export const fetchHistory = (id: string) => call<ChatFrame[]>("history", { id });
/** One machine's directory — order, cap and the answered-about path
    all come from the node; `""` asks for that machine's home. */
export const fetchLs = (machine: string, path: string) =>
  call<DirListing>("ls", { machine, path });
/** Takes a file into this machine's downloads; answers where it
    landed — the place was chosen silently, so it must be said. */
export const pullFile = (machine: string, path: string) =>
  call<string>("pull", { machine, path });
/** Every pinned directory, machine names already looked up. */
export const fetchDirPins = () => call<DirPinRow[]>("dir_pins");
export const pinDir = (machine: string, path: string, on: boolean) =>
  call<null>("pin_dir", { machine, path, on });
/** Every pinned page, exit-machine names already looked up. */
export const fetchWebPins = () => call<WebPinRow[]>("web_pins");
export const pinWeb = (machine: string, url: string, on: boolean) =>
  call<null>("pin_web", { machine, url, on });
/** Opens a page through a machine's network. In the app the tauri skin
    builds the proxied window and this resolves once it is up; over the
    dev bridge there is no window, so the same call answers where the
    proxy listens (the borrow) and the window half is the app's alone. */
export const openWeb = (machine: string, url: string) =>
  call<WebBorrow | null>("open_web", { machine, url });
/** Attaches a terminal to a hosted session at cols×rows; the host
    resizes its PTY to match, so the screen repaints whole. */
export const termOpen = (id: string, cols: number, rows: number) =>
  call<null>("term_open", { id, cols, rows });
/** The current screen if it changed since `since`, else none — a
    terminal is a state, so a poll wants the latest whole screen. */
export const termPoll = (id: string, since: number) =>
  call<TermBatch>("term_poll", { id, since });
/** Keystrokes as the bytes a terminal sends; the face maps keys to
    bytes, the backend stays a pipe. */
export const termKey = (id: string, keys: string) => call<null>("term_key", { id, keys });
export const termResize = (id: string, cols: number, rows: number) =>
  call<null>("term_resize", { id, cols, rows });
export const termLeave = (id: string) => call<null>("term_leave", { id });
