// The one transport seam. Three ways in, one set of commands
// (`khor_gui_core::api`): the app talks tauri IPC, browser verification
// talks to the dev bridge, and a phone talks to the web face this page
// was served from. Never a hand-written mock — the backend is the same
// Rust in all three.
import { msg } from "./gen/catalog";
import type { SessionRow } from "./gen/bindings/SessionRow";
import type { DeviceRow } from "./gen/bindings/DeviceRow";
import type { DayRun } from "./gen/bindings/DayRun";
import type { DayMark } from "./gen/bindings/DayMark";
import type { FaceChoices } from "./gen/bindings/FaceChoices";
import type { HooksState } from "./gen/bindings/HooksState";
import type { Ticket } from "./gen/bindings/Ticket";
import type { Usage } from "./gen/bindings/Usage";
import type { Strain } from "./gen/bindings/Strain";
import type { ChatBatch } from "./gen/bindings/ChatBatch";
import type { ChatFrame } from "./gen/bindings/ChatFrame";
import type { DirListing } from "./gen/bindings/DirListing";
import type { DirPinRow } from "./gen/bindings/DirPinRow";
import type { RecentRow } from "./gen/bindings/RecentRow";
import type { WebPinRow } from "./gen/bindings/WebPinRow";
import type { WebBorrow } from "./gen/bindings/WebBorrow";
import type { TermBatch } from "./gen/bindings/TermBatch";
import type { TermScreen } from "./gen/bindings/TermScreen";
import type { TermRun } from "./gen/bindings/TermRun";
import type { TermColor } from "./gen/bindings/TermColor";
import type { AgentRow } from "./gen/bindings/AgentRow";
import type { KnownAgent } from "./gen/bindings/KnownAgent";
import type { AgentSaid } from "./gen/bindings/AgentSaid";
import type { SeenSession } from "./gen/bindings/SeenSession";
import type { QuotaAnswer } from "./gen/bindings/QuotaAnswer";
import type { QuotaWindow } from "./gen/bindings/QuotaWindow";
import type { QuotaTrouble } from "./gen/bindings/QuotaTrouble";
import type { MachineLog } from "./gen/bindings/MachineLog";
import type { ChatMsgRow } from "./gen/bindings/ChatMsgRow";
import type { ChatFileRow } from "./gen/bindings/ChatFileRow";

export type {
  SessionRow,
  DeviceRow,
  DayRun,
  DayMark,
  FaceChoices,
  HooksState,
  Ticket,
  Usage,
  Strain,
  ChatBatch,
  ChatFrame,
  DirListing,
  DirPinRow,
  RecentRow,
  WebPinRow,
  WebBorrow,
  TermBatch,
  TermScreen,
  TermRun,
  TermColor,
  AgentRow,
  KnownAgent,
  AgentSaid,
  SeenSession,
  QuotaAnswer,
  QuotaWindow,
  QuotaTrouble,
  MachineLog,
  ChatMsgRow,
  ChatFileRow,
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

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (bridge) return post<T>(`http://127.0.0.1:${bridge}/${cmd}`, {}, args);
  if (inApp) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  // The web face. Same origin — the page came from here — so the
  // address is relative and no CORS is involved in either direction.
  //
  // No stored key is not a dead end any more (记住一次批): the page
  // itself now sits behind the key, so this script running at all means
  // the server accepted a credential — usually the remembered cookie,
  // which same-origin fetches carry on their own. The Bearer header
  // rides along only when this page also holds the key; the server
  // treats the two channels as one credential.
  return post<T>(
    `/api/${cmd}`,
    webKey === null ? {} : { authorization: `Bearer ${webKey}` },
    args,
  );
}

/** `by` is a `khor_node::list::Arrange` key — the backend arranges and
    groups; this layer passes the choice through and paints the answer. */
export const fetchSessions = (by: string) => call<SessionRow[]>("sessions", { by });
/** Each row's last 24 hours, keyed by session id (`khor_node::day`).

    **Its own call, deliberately not a field on `SessionRow`.** The row
    list is a two-second heartbeat and a day of cells does not belong on
    one. `cells` is how many blocks the strip has room for — a pixel
    count only the screen knows — and every merge down to it happened in
    the node: `khor_core::DayCell::rank` decides which of several cells
    a coarser one shows, and this layer must never re-derive that
    (docs/UX.md 状态呈现). */
export const fetchDayStrips = (cells: number) =>
  call<Record<string, DayRun[]>>("day_strips", { cells });
export const fetchDevices = () => call<DeviceRow[]>("devices");
/* **There is no `fetchUsage` any more**, and its absence is the point.
   It asked the node for every day there has ever been — an eighteen
   second first pass on the machine answering (`khor_node::usage`) — for
   one line on one screen, and that line went on 2026-08-29 (「session 页
   这个今天输出 xx k 也不要」). The op is still there and still answered:
   `khor usage` is the landing for 花了多少, and the type below is still
   exported because the bindings barrel exports every binding. Adding
   this call back means having somewhere to draw it first. */
/** What this machine's Claude subscription has left — read through the
    login Claude Code already stored here. Never rejects: every way it
    can come back empty is a reason the person might act on, so the
    reason rides in the answer. */
export const fetchQuota = () => call<QuotaAnswer>("quota");
export const markSeen = (id: string) => call<null>("seen", { id });
export const closeSession = (id: string) => call<null>("close_session", { id });
/** Names a session for the whole network; an empty title takes the name
    back so the row falls back to its derived title. The empty string is
    sent, never omitted — it is the eraser, not a missing argument. */
export const renameSession = (id: string, title: string) =>
  call<null>("rename_session", { id, title });
export const tell = (machine: string, text: string) => call<null>("tell", { machine, text });
/** A machine conversation's readout — what `khor log` prints, shaped
    for the pane. Takes the row's id (`chat/<channel>`); the core layer
    peels the prefix. */
export const machineLog = (id: string) => call<MachineLog>("machine_log", { id });
/** The waiting file chip's 收下 — `khor accept`, same node call
    (docs/KHOR.md equivalence). Answers where the files landed. */
export const acceptTransfer = (msg: string) => call<string[]>("accept_transfer", { msg });
/** The composer's 发文件, desktop form: the native picker answers with
    a real local path and the path goes straight to `khor send`'s call.
    `null` = cancelled, which is not an error. App only (`inApp`) — the
    browser forms upload bytes instead (`uploadFile`). */
export const pickAndSendFile = (machine: string) =>
  call<string | null>("pick_and_send", { machine });
/** The composer's 发文件, browser form: the bytes stream to the sending
    machine's staging area and the same send runs there
    (`khor_gui_core::send_upload`). Raw-bytes POST, not a JSON command —
    a file does not belong inside a JSON envelope — so this is the one
    call in this file that does not go through `call`. The size cap and
    its refusal sentence are the backend's (`UPLOAD_CAP_BYTES`). */
export const uploadFile = async (machine: string, file: File): Promise<string> => {
  const q = `machine=${encodeURIComponent(machine)}&name=${encodeURIComponent(file.name)}`;
  const url = bridge ? `http://127.0.0.1:${bridge}/upload?${q}` : `/api/upload?${q}`;
  const r = await fetch(url, {
    method: "POST",
    // Same rule as `call`: the header rides when the key is held; a
    // bare page leans on the remembered cookie (记住一次批).
    headers: bridge || webKey === null ? {} : { authorization: `Bearer ${webKey}` },
    body: file,
  });
  if (!r.ok) throw new Error(unwrap(await r.text()));
  return (await r.json()) as string;
};
/** Every ACP agent this person registered (批⑥, `khor agents add`).
    **khor's own two are not in here** — they are not registrations, and
    the wizard puts them beside this list rather than inside it. */
export const agents = () => call<AgentRow[]>("agents");
/** The ACP registry snapshot khor ships with (`khor agents known`) —
    all 40, not just the ones khor can start. Listing the other 18 is
    the point: 「khor 不替你装这个」 and 「khor 接不了这个」 are
    different sentences and only the first is true. */
export const agentsKnown = () => call<KnownAgent[]>("agents_known");
/** `[when it was taken, how many, how many khor can start]`. A table
    that ages has to say when it is from. */
export const agentsKnownTaken = () => call<[string, number, number]>("agents_known_taken");
/** Attaches one snapshot entry, **by id** — the command is looked up on
    the far side and never travels from here. It registers under the id
    as the name, which is the one word two machines both already have.
    Starts nothing. */
export const registerKnownAgent = (id: string) =>
  call<null>("register_known_agent", { id });
/** What khor **remembers** this agent said about its own conversations
    — a document read, no process (0.01s). This is what a face calls
    when a person picks an agent.

    The answer is three-shaped on purpose (`AgentSaid`): never asked,
    asked and it cannot list, and it answered — where answering with
    nothing is its own case. `rows` exists on the last variant only, so
    a reader cannot reach for a list on an answer that is not one. */
export const agentSeen = (name: string) => call<AgentSaid>("agent_seen", { name });
/** Puts the question to the agent itself and records the answer.

    **This starts a real agent process — measured 1.5s with the package
    cached and 5.64s without, against 0.017s for {@link agentSeen}.** So
    it hangs off something a person
    pressed and nothing else: no poll, no effect that re-runs, no
    "refresh while we're here". */
export const agentAsk = (name: string) => call<AgentSaid>("agent_ask", { name });
/** Opens a conversation the agent already has, by the id it gave.

    **Not {@link openSession} with an extra argument.** That one answers
    the wizard's four questions for a conversation that does not exist;
    this one exists, and brings its own directory and its own name —
    which is why neither is passed. Answers the new row's id. */
export const resumeSession = (agent: string, session: string) =>
  call<string>("resume_session", { agent, session });
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
/** The machine card's 在这台开终端 — `open --on` with the far machine's
    own shell, through the same core call both skins answer with
    (docs/KHOR.md equivalence). Answers the new row's id. */
export const openTermOn = (machine: string) => call<string>("open_on", { machine });
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
/** One user turn. `images` are `[mime, base64]` pairs and are only ever
    non-empty when the agent said it takes them (`ChatFrame::Agent`'s
    `images`) — the host puts them on a different op (`GuiOp::SayWith`),
    so an empty list costs the old frame exactly nothing. */
export const chatSay = (
  id: string,
  text: string,
  images: [string, string][] = [],
  files: [string, string][] = [],
) =>
  call<null>("chat_say", {
    id,
    text,
    ...(images.length ? { images } : {}),
    ...(files.length ? { files } : {}),
  });
export const chatAnswer = (id: string, ask: number, option: string | null) =>
  call<null>("chat_answer", { id, ask, option });
export const chatReplay = (id: string) => call<null>("chat_replay", { id });
export const chatLeave = (id: string) => call<null>("chat_leave", { id });
/** A discovered session's recorded past: the vendor's transcript in
    replay-shaped frames, whole, ending in history_end — so the same
    fold paints it and a live replay alike. */
export const fetchHistory = (id: string) => call<ChatFrame[]>("history", { id });

/** A discovered session's transcript, **followed**: whatever it has said
    since `from`, and where to continue.

    Appended lines come back as `note` frames and append; a rewound file
    (a compaction) comes back whole as `history` + `history_end` and
    repaints. Both shapes are ones the fold already knows — see
    `chat::history_follow` for why that split is the design and not a
    convenience. */
export const followHistory = (id: string, from: number) =>
  call<{ frames: ChatFrame[]; at: number; following: boolean }>("history_follow", { id, from });
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
/**
 * Where this person has been, of one kind ("dir" | "page"), most recent
 * first and machine names already looked up.
 *
 * **Not the tab strip.** What a screen currently has open belongs to
 * that screen and stays in its own storage; this is the memory behind
 * it, and it is the same on every device the person picks up
 * (`khor_sync::recents`).
 */
export const fetchRecents = (kind: string, limit: number) =>
  call<RecentRow[]>("recents", { kind, limit });
/** Records a landing. Called when a tab arrives somewhere, not when
    one is opened — the list is places, not trips. */
export const recentVisit = (kind: string, machine: string, place: string) =>
  call<null>("recent_visit", { kind, machine, place });
/** "Stop suggesting this", for every device. Going back there puts it
    back on its own. */
export const forgetRecent = (kind: string, machine: string, place: string) =>
  call<null>("forget_recent", { kind, machine, place });
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
 * point itself at anything. **That is a browser invariant, not a gap in
 * khor**: a page that could set its own proxy could hijack the traffic
 * of every other page.
 *
 * **What the sentence says, and what it leaves out.** The paragraph
 * above is the why; the line a person reads is one clause naming the
 * place where it does work, and nothing else. It used to carry the
 * mechanism (a local proxy port), the reason (a page cannot aim
 * itself), and a consolation ("the list and the pins are useful here")
 * — all three are what docs/UX.md forbids a sentence to do. Someone
 * standing here wants one thing: where it does work.
 *
 * Two non-app roads exist and neither is free; the ledger carries them
 * (setting the proxy by hand, and a rewriting gateway) along with the
 * security question that has to be answered before the first could be
 * built at all.
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
