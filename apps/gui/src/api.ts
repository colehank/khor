// The one transport seam. Three ways in, one set of commands
// (`khor_gui_core::api`): the app talks tauri IPC, browser verification
// talks to the dev bridge, and a phone talks to the web face this page
// was served from. Never a hand-written mock — the backend is the same
// Rust in all three.
import { msg } from "./gen/catalog";
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
import type { AgentRow } from "./gen/bindings/AgentRow";
import type { QuotaAnswer } from "./gen/bindings/QuotaAnswer";
import type { QuotaWindow } from "./gen/bindings/QuotaWindow";
import type { QuotaTrouble } from "./gen/bindings/QuotaTrouble";

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
  AgentRow,
  QuotaAnswer,
  QuotaWindow,
  QuotaTrouble,
};

const params = new URLSearchParams(window.location.search);
const bridge = params.get("bridge");

/**
 * Inside the desktop app's webview.
 *
 * Exported because a few things exist **only in the app shell**, and the
 * honest thing is to not set them up at all elsewhere rather than to set
 * them up and watch them never fire. The file drop is the first: tauri
 * intercepts OS drops before the webview sees them, so the paths arrive
 * on a tauri event that a browser has no equivalent of — an HTML5 `drop`
 * there would hand back a `File` with no path on it.
 *
 * **This used to be called `onBridge`**, back when "not the app" and
 * "the dev bridge" were the same thing. They stopped being the same
 * thing the moment a phone could open this page, and a name that only
 * described one of two cases is the kind that sends the next person
 * grepping to the wrong file (`chat-*` → `to-bottom`, same lesson).
 * Asked positively, too: this is the one of the three that is true when
 * tauri's own IPC global is present, rather than a guess from the
 * absence of something else.
 */
export const inApp = "__TAURI_INTERNALS__" in window;

/**
 * The web face's key, taken off the link once and kept.
 *
 * `khor web` prints an address with `?k=…` on it; the address bar is a
 * thing that gets screenshotted, bookmarked and read over a shoulder,
 * so the key comes off it on the first paint and the URL is rewritten
 * without it. What is left is a clean address that still works, because
 * the key is in storage.
 *
 * **`localStorage`, not `sessionStorage`**: the promise is that a phone
 * bookmarks this and it opens. A key that died with the tab would mean
 * digging out the original link every time, and the person would end up
 * bookmarking the version *with* the key in it — which is the thing
 * this is trying to avoid.
 */
const KEY_HELD = "khor.web.key";
const webKey: string | null = (() => {
  const fromLink = params.get("k");
  if (fromLink === null) return window.localStorage.getItem(KEY_HELD);
  window.localStorage.setItem(KEY_HELD, fromLink);
  const clean = new URL(window.location.href);
  clean.searchParams.delete("k");
  window.history.replaceState(null, "", clean.toString());
  return fromLink;
})();

// khor answers a refusal as a JSON string, and its refusals are whole
// sentences meant for a person — handing the raw body onward puts the
// encoding's quotation marks around one.
const unwrap = (body: string): string => {
  try {
    const v: unknown = JSON.parse(body);
    return typeof v === "string" ? v : body;
  } catch {
    return body;
  }
};

async function post<T>(url: string, headers: Record<string, string>, args?: Record<string, unknown>): Promise<T> {
  const r = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json", ...headers },
    body: JSON.stringify(args ?? {}),
  });
  if (!r.ok) throw new Error(unwrap(await r.text()));
  return (await r.json()) as T;
}

/**
 * This page is the web face — served to a browser that is neither the
 * app nor a verification run.
 *
 * The third of the three, and the only one named by elimination. That is
 * not laziness: the other two announce themselves (tauri's IPC global,
 * a `bridge` in the query), and a page khor served carries no mark of
 * its own to check.
 */
export const onWebFace = !inApp && bridge === null;

/**
 * …and it was never given a key.
 *
 * A distinct state rather than one more failed request: **every poll in
 * this app swallows its errors**, because a poll that threw on a blip
 * would take a screen down for a network hiccup. So without this, the
 * only symptom of arriving without a key is a screen where nothing ever
 * appears — which is what a broken khor looks like too. Measured that
 * way first (`scripts/web-face.mjs`), which is why it is knowable here,
 * synchronously, before anything renders (`main.tsx`).
 */
export const missingWebKey = onWebFace && webKey === null;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (bridge) return post<T>(`http://127.0.0.1:${bridge}/${cmd}`, {}, args);
  if (inApp) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  // The web face. Same origin — the page came from here — so the
  // address is relative and no CORS is involved in either direction.
  if (webKey === null) throw new Error(msg.web_no_key);
  return post<T>(`/api/${cmd}`, { authorization: `Bearer ${webKey}` }, args);
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
/** What this machine's Claude subscription has left — read through the
    login Claude Code already stored here. Never rejects: every way it
    can come back empty is a reason the person might act on, so the
    reason rides in the answer. */
export const fetchQuota = () => call<QuotaAnswer>("quota");
export const markSeen = (id: string) => call<null>("seen", { id });
export const closeSession = (id: string) => call<null>("close_session", { id });
export const tell = (machine: string, text: string) => call<null>("tell", { machine, text });
/** Every ACP agent this person registered (批⑥, `khor agents add`).
    **khor's own two are not in here** — they are not registrations, and
    the wizard puts them beside this list rather than inside it. */
export const agents = () => call<AgentRow[]>("agents");
/** The wizard (会话身份批B): a fresh agent session in `dir`, as a
    conversation ("chat") or a terminal ("term"), for the chosen agent.
    `agent` is an **open position** since 批⑥: "claude", "codex", or the
    name of anything registered — the node resolves it and refuses by
    name, so this layer neither narrows nor validates it. Answers the
    new row's id. */
export const openSession = (
  dir: string,
  title: string,
  form: "chat" | "term",
  agent: string,
) => call<string>("open_session", { dir, title, form, agent });
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
/** Hands a page to whatever this machine opens pages with. Refuses
    anything that is not http/https — the address came out of an agent's
    own words (`khor_gui_core::web::open_link`). */
export const openLink = (url: string) => call<null>("open_link", { url });
/** 接管 the other way: a conversation khor holds moves into a terminal
    (`Node::takeover_term`). Same row, same conversation. */
export const takeoverTerm = (id: string) => call<null>("takeover_term", { id });
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
/**
 * Opens a page through a machine's network — **except on the web face,
 * which says why it cannot.**
 *
 * In the app the tauri skin builds the proxied window and this resolves
 * once it is up; over the dev bridge there is no window, so the same
 * call answers where the proxy listens and the window half is the app's
 * alone.
 *
 * A phone has neither. What a borrow hands back is a proxy port on the
 * khor machine, and using it means pointing a whole browser at that
 * proxy — the app sets its window's `proxy_url`, and a page cannot
 * point itself at anything. So there is nothing this face could do with
 * the answer.
 *
 * Refused before the call rather than after, because the call is not
 * free: it opens a real borrow session on the far machine, and one
 * nobody can use is one nobody thinks to close. "Started something and
 * nothing happened" is the worst of the three possible answers — the
 * other two are doing it and saying why not.
 */
export const openWeb = (machine: string, url: string) => {
  if (onWebFace) return Promise.reject(new Error(msg.web_borrow_needs_app));
  return call<WebBorrow | null>("open_web", { machine, url });
};
/** Attaches a terminal to a hosted session at cols×rows; the host
    resizes its PTY to match, so the screen repaints whole. */
export const termOpen = (id: string, cols: number, rows: number) =>
  call<null>("term_open", { id, cols, rows });
/** The current screen if it changed since `since`, else none — a
    terminal is a state, so a poll wants the latest whole screen.

    `back` is how many lines above the live screen to look, and the
    answer carries where it actually landed: asking to go further than
    there is history is clamped, and the clamped number is the one to
    believe. */
export const termPoll = (id: string, since: number, back: number) =>
  call<TermBatch>("term_poll", { id, since, back });
/** Keystrokes as the bytes a terminal sends; the face maps keys to
    bytes, the backend stays a pipe. */
export const termKey = (id: string, keys: string) => call<null>("term_key", { id, keys });
/** Pasted text, as a paste rather than as keystrokes. The wrapping is
    the registry's to decide — it holds the screen that knows whether the
    program asked for bracketed paste (`khor_gui_core::term::term_paste`),
    and a face that wrapped it here would be guessing at somebody else's
    mode. */
export const termPaste = (id: string, text: string) =>
  call<null>("term_paste", { id, text });
/** Files dropped on a terminal. The paths go over as they are and the
    quoting happens in the registry — shell syntax is not something a
    face should be assembling, and one wrong escape turns a filename
    into a command (`khor_gui_core::term::term_drop`). */
export const termDrop = (id: string, paths: string[]) =>
  call<null>("term_drop", { id, paths });
export const termResize = (id: string, cols: number, rows: number) =>
  call<null>("term_resize", { id, cols, rows });
export const termLeave = (id: string) => call<null>("term_leave", { id });
