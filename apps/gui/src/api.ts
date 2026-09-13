// The one transport seam. Three ways in, one set of commands
// (`khor_gui_core::api`): the app talks tauri IPC, browser verification
// talks to the dev bridge, and a phone talks to the web face this page
// was served from. Never a hand-written mock — the backend is the same
// Rust in all three.
import { msg } from "./gen/catalog";
// **The menu's shape is the menu's, not the wire's.** What crosses
// here is exactly what the list component already defines; a second
// spelling of a row would be one restyle away from drawing two
// different menus for one idea, which is the failure `OmniChip`'s
// head describes about copying a token.
import type { MenuRows } from "./components/OmniMenu";
import type { PlaceRow } from "./gen/bindings/PlaceRow";
import type { WorkspaceRow } from "./gen/bindings/WorkspaceRow";
import type { FileText } from "./gen/bindings/FileText";
import type { GitStatus } from "./gen/bindings/GitStatus";
import type { Editor } from "./gen/bindings/Editor";
import type { GitRow } from "./gen/bindings/GitRow";
import type { GitDiff } from "./gen/bindings/GitDiff";
import type { GitHunk } from "./gen/bindings/GitHunk";
import type { Wrote } from "./gen/bindings/Wrote";
import type { Merged } from "./gen/bindings/Merged";
import type { CodeAt } from "./gen/bindings/CodeAt";
import type { Preview } from "./gen/bindings/Preview";
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
import type { TermStart } from "./gen/bindings/TermStart";
import type { TermChunk } from "./gen/bindings/TermChunk";
import type { AgentRow } from "./gen/bindings/AgentRow";
import type { KnownAgent } from "./gen/bindings/KnownAgent";
import type { AgentSaid } from "./gen/bindings/AgentSaid";
import type { SeenSession } from "./gen/bindings/SeenSession";
import type { QuotaAnswer } from "./gen/bindings/QuotaAnswer";
import type { QuotaCredits } from "./gen/bindings/QuotaCredits";
import type { QuotaWindow } from "./gen/bindings/QuotaWindow";
import type { QuotaTrouble } from "./gen/bindings/QuotaTrouble";
import type { MachineLog } from "./gen/bindings/MachineLog";
import type { AccountRing } from "./gen/bindings/AccountRing";
import type { ChatMsgRow } from "./gen/bindings/ChatMsgRow";
import type { ChatFileRow } from "./gen/bindings/ChatFileRow";
import type { Load } from "./gen/bindings/Load";
import type { TopRow } from "./gen/bindings/TopRow";
import type { CpuDay } from "./gen/bindings/CpuDay";
import type { WatchedDir } from "./gen/bindings/WatchedDir";

export type {
  AccountRing,
  PlaceRow,
  WorkspaceRow,
  FileText,
  GitStatus,
  GitRow,
  GitDiff,
  GitHunk,
  Wrote,
  Merged,
  CodeAt,
  Preview,
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
  AgentRow,
  KnownAgent,
  AgentSaid,
  SeenSession,
  QuotaAnswer,
  QuotaCredits,
  QuotaWindow,
  QuotaTrouble,
  MachineLog,
  ChatMsgRow,
  ChatFileRow,
  Load,
  TopRow,
  CpuDay,
  WatchedDir,
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
 * This page is the app's small overlay window rather than its window.
 *
 * **Both are this same page, told which half of itself to be** (`main.tsx`
 * picks a root off it). A second HTML entry point was the other road and
 * it is the worse one: the menu's look would then be defined in two
 * places, and the two would drift the first time either was restyled.
 */
export const isMenuFace = params.get("face") === "menu";

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

/** 常去的地方 — the directories a machine's own records say work
    happened in, folded to project roots and newest first
    (`khor_node::place`). `machine` empty is this one; a name asks that
    machine, because it is where the wizard is about to start something
    (`Node::places_on`). */
export const fetchPlaces = (machine = "") => call<PlaceRow[]>("places", { machine });
/** The workspace landing's list for **one** machine: the checkouts its
    own records say work happened in (`khor_node::place`, the checkout
    half). `machine` empty is this one.

    **Asked per machine and never folded.** The landing shows them all
    in one list, but which machine could not answer is a fact about that
    machine, and a folded call would flatten「没问过」「够不着」「那儿
    没有」into one empty array — the same three-way collapse
    `check-acp` 第二批 exists to catch. A refusal here is an error, not
    an empty list. */
export const fetchWorkspaces = (machine = "") =>
  call<WorkspaceRow[]>("workspaces", { machine });
/** One file off a machine, as much of it as one slice holds
    (`khor_gui_core::files::read_text`). For reading; `pullFile` is the
    other verb and it spends a disk. */
export const fetchFileText = (machine: string, path: string) =>
  call<FileText>("read_text", { machine, path });
/** One checkout's branch and marks (`khor_gui_core::git::status`).
    `dir` is the workspace root in that machine's own spelling.

    **Asked once per workspace, not once per directory.** The tree walks
    and this answer does not change while it does; re-asking on every
    step would make a person's own walking look like a repository
    churning. Measured on this repo: 54ms for 35 marked paths out of
    1525 tracked files.

    A refusal is an error, never an empty list — no rows means「nothing
    differs from HEAD」, which is the one reading that must not be a
    guess. */
export const fetchGitStatus = (machine: string, dir: string) =>
  call<GitStatus>("git_status", { machine, dir });
/** Where one file differs from HEAD (`khor_gui_core::git::diff`), in
    the coordinates of the file as it is now. Refuses out loud for
    `fetchGitStatus`'s reason. */
export const fetchGitDiff = (machine: string, path: string) =>
  call<GitDiff>("git_diff", { machine, path });
/** Saves a whole file back (`khor_gui_core::files::write_text`).

    `was` is the `digest` that came with the text. **It is the whole of
    the safety**: the far machine refuses unless the file still hashes
    to it, so two people editing one file end with one of them told
    rather than one of them silently gone. An empty `was` claims there
    is no file at that path — which is why the view only offers to save
    when it was handed a digest.

    A refusal is an error and is never softened anywhere on this path: a
    save that answered「fine」while writing nothing is the worst answer
    khor could give. */
export const saveFileText = (machine: string, path: string, was: string, text: string) =>
  call<Wrote>("write_text", { machine, path, was, text });
/** The third way out of a refused save (`khor_node::merge::three`).

    **Three texts in, one out, and nothing written.** `base` is what was
    read into the box, `mine` is what is in it now, `theirs` is what the
    file holds this second — so the answer is put back in the box and
    the save that follows is an ordinary `saveFileText` pinned to
    `theirs`'s digest. A merge that wrote by itself would break the one
    rule the whole file view is built on: khor touches a file when
    somebody presses save, and not before.

    `clashes` counts the places both sides changed the same lines. They
    are in the text between markers, and they are the person's to
    settle — khor does not pick, and does not pretend it did. */
export const mergeFileText = (base: string, mine: string, theirs: string) =>
  call<Merged>("merge_text", { base, mine, theirs });
/** `code --version`'s first line on that machine, **empty when there is
    none** (`khor_gui_core::code::here`). A machine that could not answer
    comes back empty too, and that is deliberate: both lead to the same
    next move, which is to draw no button. */
/** One file shaped for a screen that is not a code view: an image, an
    SVG, or a document rendered to HTML (`khor_gui_core::files::preview`).

    **What a file turns out to be is the node's answer**, not a question
    this side asks first — so this is one call and `kind` comes back
    with the content. A second opinion here is exactly how `khor` and
    the app would start disagreeing about what a `.docx` is.

    It reads up to 4 MB, which is measured to cover every image and pdf
    on this person's disk; bigger files come back `cut` with nothing to
    draw. */
export const fetchPreview = (machine: string, path: string) =>
  call<Preview>("preview", { machine, path });
export const fetchCodeHere = (machine: string) => call<Editor>("code_here", { machine });
/** Starts (or reuses) that machine's VS Code web face and answers a URL
    to point a webview at, `dir` opened as the folder.

    ⚠️ **This can take minutes the first time a machine runs it** — the
    editor fetches its own server from Microsoft, measured at 643 MB.
    The consent for that is asked before this is called and cannot be
    asked from inside it. */
export const openCode = (machine: string, dir: string) =>
  call<CodeAt>("code_open", { machine, dir });
/** Stops it and drops this machine's pipe. Answers the same whether
    either existed. */
export const stopCode = (machine: string) => call<void>("code_stop", { machine });
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
/** What a machine is busy with — its `top` hungriest processes now and
    the CPU day its serve recorded (`Node::load_on`; `machine` is a
    name, empty for this one). **Asked only while its card is open**,
    never on the row beat: a process table nobody is looking at is a
    table nobody is paying for. An older khor over there rejects with
    its own sentence, which the card prints where the table would be
    — an empty table would read as an idle machine.

    **`by`/`desc` rank the table over there, before `top` cuts it**
    (`khor_core::TopBy`). Measured on turing: 10 175 processes. Ranking
    here would mean either all of them crossing the wire every few
    seconds, or a cut ordered by the wrong column — the process holding
    the most memory is rarely among the hungriest by CPU. */
export const fetchLoad = (machine: string, top: number, by: string, desc: boolean) =>
  call<Load>("load", { machine, top, by, desc });
/** The 额度 page's rings: one per registered account, wherever its key
    lives — this machine's read now, another's as last carried here
    with its age (`Node::quota_board`). */
export const fetchQuotaBoard = () => call<AccountRing[]>("quota_board");
/** 登记 an account **on this machine**: a key is written here and never
    leaves; the description replicates. `provider` is the word the form
    offers (newapi / claude / openai); the node refuses an unknown one
    by name. Answers the new account's id.

    `kind` is `bill` (a key you fill in) or `sub` (that machine's own
    vendor login, `khor_sync::accounts::AccountKind`). **A subscription
    takes no key**, and the node refuses one that is sent rather than
    dropping it — so `key` stays required here and the form sends the
    empty string, which is the shape that cannot silently swallow a
    secret. */
export const registerAccount = (a: {
  name: string;
  provider: string;
  kind: string;
  base_url?: string;
  key: string;
  budget?: number;
}) => call<string>("register_account", a);
export const removeAccount = (id: string) => call<null>("remove_account", { id });
/** 置顶 / 取消置顶. The flag rides the replicated doc, so this pins on
    every machine (`Node::pin_account`). */
export const pinAccount = (id: string, on: boolean) => call<null>("pin_account", { id, on });
/** 编辑 (`Node::update_account`): name, address, budget, and optionally a
    new key. **`key` omitted means「leave the stored one alone」** — an
    edit of the name must not make somebody paste a secret again — and a
    key can only be replaced on the machine that holds it, which the node
    refuses by name rather than dropping. The provider is not editable:
    it decides how the account is read at all. */
export const updateAccount = (a: {
  id: string;
  name: string;
  base_url?: string;
  key?: string;
  budget?: number;
  /** The unit this account is really billed in, overriding what the
      endpoint reports. **Blank means「use what the endpoint said」**,
      so an empty box travels as absent rather than as `""`. */
  unit?: string;
}) => call<null>("update_account", a);
/** What this machine's agents have spent, by day, by vendor and by
    model (`khor_node::usage`).
 *
 * ⚠️ **This was deleted on 2026-08-29 and is back on 2026-09-08.** The
 * tombstone that stood here ended 「adding this call back means having
 * somewhere to draw it first」, and that is the condition this change
 * meets rather than waives: the 额度 detail's lower half draws every
 * day and every model of it (`views/UsageHalf`). What it was removed
 * for was **one line** — 今天输出 xx k — that the user took off the
 * session page, and an eighteen-second first pass is not a price worth
 * paying for one line.
 *
 * **So the thing that made it expensive is also the thing that is
 * different now: when it is called.** It was on the app's heartbeat,
 * every two seconds beside sessions and devices. It is not any more —
 * it is asked **once, when a subscription's detail is opened**, and a
 * poll of it would be that old mistake with a chart in front of it.
 *
 * The cost, from `Meters::tally`'s own note: 18.5 s cold, 0.14 s once
 * the meters have read a tree, and one directory walk when nothing has
 * changed. **The cold pass is a state a person sits in**, which is why
 * `UsageHalf` draws nothing at all rather than an empty frame while
 * this is out. */
export const fetchUsage = () => call<Usage>("usage");
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
/** Names a machine for the whole network; an empty name takes the
    rename back so the row wears the name the machine reports about
    itself again. Answers the name the row now shows — a cleared rename
    falls back to something only the backend knows, so there is nothing
    honest for the screen to echo without asking.

    Takes the **device id**, never a name: the name is what is being
    changed, and resolving one through a table where the new name may
    already exist renames the wrong row. */
export const renameMachine = (id: string, name: string) =>
  call<string>("rename_machine", { id, name });
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
    name, so this layer neither narrows nor validates it. `machine` is
    where the session is born (在哪台): empty or this machine's name is
    here, any other paired name carries the four answers over there
    (`Node::open_agent_on`). Answers the new row's id. */
export const openSession = (
  dir: string,
  title: string,
  form: "chat" | "term",
  agent: string,
  machine = "",
) => call<string>("open_session", { dir, title, form, agent, machine });
/** The wizard's shell kind — `open --on` with that machine's own shell,
    through the same core call both skins answer with (docs/KHOR.md
    equivalence). `machine` empty is this one; `title` empty is named
    after the shell; `cwd` empty is the home, and `~` is expanded on
    the machine it names. Answers the new row's id. */
export const openTermOn = (machine: string, title = "", cwd = "") =>
  call<string>("open_on", { machine, title, cwd });
/** 接管 (批C): end the session's terminal side; the conversation continues here. */
export const takeover = (id: string) => call<null>("takeover", { id });
// `on` is explicit rather than a toggle: the caller already knows the
// row's current state, and a toggle raced against a pin arriving from
// another device would flip the wrong way.
export const pinSession = (id: string, on: boolean) => call<null>("pin_session", { id, on });
export const pinDevice = (machine: string, on: boolean) => call<null>("pin_device", { machine, on });
/** `khor forget` — shows a machine the door for the whole network
    (`Node::forget_device` has why leaving travels the way joining does,
    and why this machine refuses to forget itself). Answers the name of
    the machine that left. */
export const forgetDevice = (machine: string) => call<string>("forget_device", { machine });
/** `khor watch` — the directories a machine is asked to keep an eye on,
    replaced whole (2026-09-06 关注目录). The list rides that machine's
    row in the device table; the machine answers with each path's
    volume on its next visit (`DeviceRow.watched`). The node refuses a
    relative path, a `..` or a duplicate in its own sentence, and
    answers the list as kept. */
export const setWatch = (machine: string, paths: string[]) =>
  call<string[]>("set_watch", { machine, paths });
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
/** Puts the session in one of the modes its agent listed
    (`ChatFrame::Settings`'s `modes.availableModes`), by the agent's own
    id. The answer comes back on the poll — a fresh `settings` frame when
    it took, a `refused` one when the agent said no — never here. */
export const chatSetMode = (id: string, mode: string) => call<null>("chat_set_mode", { id, mode });
/** Sets one of the agent's options: a string is a listed value's id, a
    boolean is a switch. Same answer channel as {@link chatSetMode}. */
export const chatSetOption = (id: string, option: string, value: string | boolean) =>
  call<null>("chat_set_option", { id, option, value });
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
 * Gives a page's borrow back: the far machine's network stops being
 * borrowed and the local proxy port goes.
 *
 * **Not `closeSession`, and the difference is not cosmetic.** A page's
 * borrow deliberately takes no row in the session list (docs/NET.md
 * 可达性), so there is no session to close — what it has is a handle,
 * and this is the verb that handle answers to. Calling `closeSession`
 * with it would name a row that was never written and do nothing at
 * all, silently.
 */
export const dropWebBorrow = (session: string) => call<null>("close_web", { session });

/** Where a tab's page sits in the window, in CSS pixels — which are the
    app's logical pixels, so neither side does DPI arithmetic. */
export type PageBox = { x: number; y: number; w: number; h: number; view: number };

/**
 * Opens (or re-points) the page one browsing tab is showing.
 *
 * **The page is a second webview, not part of this one.** khor's whole
 * interface is a web page, and an arbitrary site cannot be put inside it:
 * most refuse to be framed, a cross-origin frame reads back neither title
 * nor address, and a webview's proxy can only be set when it is built. So
 * the app parents a second webview to the window and lays it over the
 * rectangle this side measures. That is why the rectangle is an argument:
 * only this side can see the layout.
 */
export const openPage = (tab: string, machine: string, url: string, at: PageBox) => {
  if (onWebFace) return Promise.reject(new Error(msg.web_borrow_needs_app));
  // **This page's own identity, handed over so the app can build the
  // browsing tab a browser's one.** A webview inside an app says it is
  // bare WebKit, and sites read the tail it is missing — measured against
  // baidu, which answers an unrecognised client with a 227-byte page that
  // sends itself to plain http, and spins there forever. Only this side
  // can read `navigator.userAgent`; what to append is the app's rule
  // (`as_a_browser`), because that is where the webview is built.
  return call<WebBorrow>("open_page", { tab, machine, url, at, agent: navigator.userAgent });
};

/** Keeps the page over its tab body as the layout moves under it. */
export const movePage = (tab: string, at: PageBox) =>
  onWebFace ? Promise.resolve(null) : call<null>("move_page", { tab, at });

/** Takes a page off the screen without ending it: a tab switch, or
    anything that has to float over it (a native child webview always
    paints above this page's DOM). */
export const hidePage = (tab: string) =>
  onWebFace ? Promise.resolve(null) : call<null>("hide_page", { tab });

/** Ends a tab's page. Not its borrow — that is per machine and outlives
    any one tab (`views/held`). */
export const closePage = (tab: string) =>
  onWebFace ? Promise.resolve(null) : call<null>("close_page", { tab });

/** Puts a page back after `hidePage`. Not `openPage`: coming back from a
    menu must not dial a borrow or cost the page a reload. */
export const showPage = (tab: string) =>
  onWebFace ? Promise.resolve(null) : call<null>("show_page", { tab });

/** Back, forward, reload. **The two arrows cannot know whether there is
    anywhere to go** — tauri has no `can_go_back` — so they stay lit and
    a press with no history behind it quietly does nothing. */
export const pageStep = (tab: string, way: "back" | "forward" | "reload") =>
  onWebFace ? Promise.resolve(null) : call<null>("page_step", { tab, way });

/**
 * What a page said about itself, addressed to one tab.
 *
 * **Only in the app.** The events come from a webview only the app has;
 * the other two roads have no page and nothing to hear, so this hands
 * back a cleanup that undoes nothing rather than pretending to listen.
 */
export function onPageWord(
  event: "page:went" | "page:title" | "page:load" | "page:popup",
  tab: string,
  said: (text: string) => void,
): () => void {
  if (!inApp) return () => {};
  let drop: (() => void) | null = null;
  let gone = false;
  void import("@tauri-apps/api/event")
    .then(({ listen }) =>
      listen<{ tab: string; text: string }>(event, (e) => {
        if (e.payload.tab === tab) said(e.payload.text);
      }).then((un) => {
        // Unsubscribed before the subscription landed: undo it now, or
        // the listener outlives the tab it was speaking for and writes
        // into a component that is gone.
        if (gone) un();
        else drop = un;
      }),
    )
    // **Loud, and this is the whole reason it says anything.** Listening
    // is a core-plugin command and therefore behind tauri's capability
    // list; khor had no capabilities file at all, so every `listen` was
    // refused and this promise rejected into nothing. The address bar
    // stopped following the page and links asking for a new window did
    // nothing, and **neither of them said a word** — measured 2026-09-09,
    // and it cost three wrong diagnoses before somebody looked here.
    .catch((e: unknown) => {
      console.error(`khor: cannot listen for ${event}`, e);
    });
  return () => {
    gone = true;
    drop?.();
  };
}

// ── khor's own overlays, drawn in a window of their own (单元 2, 浮层) ──

/** Where an overlay has to appear, in this page's own CSS pixels, with
    the viewport it was measured in. The app subtracts that viewport from
    the window frame to find the screen — see `on_screen`, which is the
    side that has both numbers. */
export type Spot = { x: number; y: number; w: number; view: number; across: number };

/**
 * Puts a menu on screen in a window of its own, over anything the app has
 * laid over its page.
 *
 * **Answers whether the float took it**, and `false` is not a failure: a
 * platform with no second window to hang a menu in draws it in the page,
 * which is where every menu was drawn until a browsing tab started
 * covering them. A refusal lands in the same place (`useFloatedMenu`).
 */
export const showMenu = (at: Spot, rows: MenuRows): Promise<boolean> =>
  inApp ? call<boolean>("show_menu", { at, rows }) : Promise.resolve(false);

/** Takes it off the screen. Hidden, not closed — the window is reused. */
export const hideMenu = () => (inApp ? call<null>("hide_menu", {}) : Promise.resolve(null));

/** How tall the list came out, said by the window that drew it — the only
    side that can measure a laid-out list. Showing it is the app's answer
    to this call. */
export const sizeMenu = (h: number) =>
  inApp ? call<null>("size_menu", { h }) : Promise.resolve(null);

/** What the menu has to show right now. **Pulled on mount** because the
    window and its first rows are decided in the same breath, and a
    webview still loading cannot be listening yet. */
export const menuRows = () =>
  inApp ? call<MenuRows | null>("menu_rows", {}) : Promise.resolve(null);

/** A row was pressed. Goes back to the face that opened the menu. */
export const menuPick = (key: string) =>
  inApp ? call<null>("menu_pick", { key }) : Promise.resolve(null);

/**
 * One of the app's own events, for a listener that wants every one of
 * them rather than the ones addressed to a tab (`onPageWord`).
 *
 * **Loud on refusal, for `onPageWord`'s reason and it is not a
 * hypothetical one**: `listen` is a core-plugin command behind tauri's
 * capability list, khor had none at all until 2026-09-09, and every
 * subscription in this file was being refused into a promise nobody
 * read. Two features looked broken for a day.
 */
export function hear<T>(event: string, said: (payload: T) => void): () => void {
  if (!inApp) return () => {};
  let drop: (() => void) | null = null;
  let gone = false;
  void import("@tauri-apps/api/event")
    .then(({ listen }) =>
      listen<T>(event, (e) => said(e.payload)).then((un) => {
        if (gone) un();
        else drop = un;
      }),
    )
    .catch((e: unknown) => {
      console.error(`khor: cannot listen for ${event}`, e);
    });
  return () => {
    gone = true;
    drop?.();
  };
}
/** Attaches a terminal to a hosted session at cols×rows; the host
    resizes its PTY to match. */
export const termOpen = (id: string, cols: number, rows: number) =>
  call<null>("term_open", { id, cols, rows });
/** Everything the feed holds so far (it began with the host's ring, so
    history rides along) and the cursor to stream from. The pane writes
    it into xterm and then follows with `termStream`.

    (The cell-snapshot road — `term_poll` and the `TermScreen` family,
    with `term_wheel`'s mouse judgment and `term_paste`'s bracketing —
    left with the 2026-09-03 pivot: xterm.js consumes bytes and speaks
    its own mouse and paste. 「不必考虑旧版兼容」 is why there is no
    bridge.) */
export const termAttach = (id: string) => call<TermStart>("term_attach", { id });
export const termStream = (id: string, since: number) =>
  call<TermChunk>("term_stream", { id, since });
/** Keystrokes as the bytes a terminal sends — xterm encodes every key,
    modifier, mouse report and paste; the backend stays a pipe. */
export const termKey = (id: string, keys: string) => call<null>("term_key", { id, keys });
/** Files dropped on a terminal. The paths go over as they are and the
    quoting happens in the registry — shell syntax is not something a
    face should be assembling, and one wrong escape turns a filename
    into a command (`khor_gui_core::term::term_drop`). */
export const termDrop = (id: string, paths: string[]) =>
  call<null>("term_drop", { id, paths });
export const termResize = (id: string, cols: number, rows: number) =>
  call<null>("term_resize", { id, cols, rows });
export const termLeave = (id: string) => call<null>("term_leave", { id });
