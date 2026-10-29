import type { AttachmentKind } from "./gen/bindings/AttachmentKind";
import { attachmentView } from "./lib/attachment-view";
import type { SessionContent } from "./gen/bindings/SessionContent";
import type { HistoryRecordChunk } from "./gen/bindings/HistoryRecordChunk";
// The one transport seam. Three ways in, one set of commands
// (`khor_gui_core::api`): the app talks tauri IPC, browser verification
// talks to the dev bridge, and a phone talks to the web face this page
// was served from. Never a hand-written mock — the backend is the same
// Rust in all three.
import { gui, msg } from "./gen/catalog";
// **The menu's shape is the menu's, not the wire's.** What crosses
// here is exactly what the list component already defines; a second
// spelling of a row would be one restyle away from drawing two
// different menus for one idea, which is the failure `OmniChip`'s
// head describes about copying a token.
import type { MenuRows } from "./components/OmniMenu";
import { nativeMenuQueue } from "./lib/native-menu-queue";
import { listenNativeEvent } from "./lib/native-events";
import type { SyncOutcome } from "./gen/bindings/SyncOutcome";
import type { Connections } from "./gen/bindings/Connections";
import type { ReachReport } from "./gen/bindings/ReachReport";
import type { ProxyLine } from "./gen/bindings/ProxyLine";
import type { JournalLine } from "./gen/bindings/JournalLine";
import type { DropItem } from "./gen/bindings/DropItem";
import type { DropEnd } from "./gen/bindings/DropEnd";
import type { Miss } from "./gen/bindings/Miss";
import type { AddAsk } from "./gen/bindings/AddAsk";
import type { AddRun } from "./gen/bindings/AddRun";
import type { LoginAsk } from "./gen/bindings/LoginAsk";
import type { TabRow } from "./gen/bindings/TabRow";
import type { TabsOpen } from "./gen/bindings/TabsOpen";
import type { WorkspaceRow } from "./gen/bindings/WorkspaceRow";
import type { FileText } from "./gen/bindings/FileText";
import type { FileStat } from "./gen/bindings/FileStat";
import type { DirWatchScope } from "./gen/bindings/DirWatchScope";
import type { DirWatchCoverageMode } from "./gen/bindings/DirWatchCoverageMode";
import type { DirWatchCoverage } from "./gen/bindings/DirWatchCoverage";
import type { DirWatchBatch } from "./gen/bindings/DirWatchBatch";
import type { DirWatchOpened } from "./gen/bindings/DirWatchOpened";
import type { GitStatus } from "./gen/bindings/GitStatus";
import type { Editor } from "./gen/bindings/Editor";
import type { GitRow } from "./gen/bindings/GitRow";
import type { GitDiff } from "./gen/bindings/GitDiff";
import type { GitBlame } from "./gen/bindings/GitBlame";
import type { GitLog } from "./gen/bindings/GitLog";
import type { GitHistoryPage } from "./gen/bindings/GitHistoryPage";
import type { GitCommitSummary } from "./gen/bindings/GitCommitSummary";
import type { GitShow } from "./gen/bindings/GitShow";
import type { GitHunk } from "./gen/bindings/GitHunk";
import type { GitCommitShow } from "./gen/bindings/GitCommitShow";
import type { Wrote } from "./gen/bindings/Wrote";
import type { Merged } from "./gen/bindings/Merged";
import type { CodeAt } from "./gen/bindings/CodeAt";
import type { Preview } from "./gen/bindings/Preview";
import type { SessionRow } from "./gen/bindings/SessionRow";
import type { DeviceRow } from "./gen/bindings/DeviceRow";
import type { DayRun } from "./gen/bindings/DayRun";
import type { DayMark } from "./gen/bindings/DayMark";
import type { FaceChoices } from "./gen/bindings/FaceChoices";
import type { RelayChoices } from "./gen/bindings/RelayChoices";
import type { RelayWant } from "./gen/bindings/RelayWant";
import type { RoadShare } from "./gen/bindings/RoadShare";
import type { RecognitionEntry } from "./gen/bindings/RecognitionEntry";
import type { HooksState } from "./gen/bindings/HooksState";
import type { Ticket } from "./gen/bindings/Ticket";
import type { LeaveOutcome } from "./gen/bindings/LeaveOutcome";
import type { Waiting } from "./gen/bindings/Waiting";
import type { PendingRow } from "./gen/bindings/PendingRow";
import type { About } from "./gen/bindings/About";
import type { DoctorRow } from "./gen/bindings/DoctorRow";
import type { LockFacts } from "./gen/bindings/LockFacts";
import type { ConfigRow } from "./gen/bindings/ConfigRow";
import type { ConfigChoice } from "./gen/bindings/ConfigChoice";
import type { Usage } from "./gen/bindings/Usage";
import type { Strain } from "./gen/bindings/Strain";
import type { ChatBatch } from "./gen/bindings/ChatBatch";
import type { HistoryPage } from "./gen/bindings/HistoryPage";
import type { ChatFrame } from "./gen/bindings/ChatFrame";
import type { DirListing } from "./gen/bindings/DirListing";
import type { Found } from "./gen/bindings/Found";
import { termSizeHint } from "./lib/term-size-hint";
import { CONFIG_TOLD, LOCK_TOLD, tellOthers } from "./lib/other-windows";
import type { HistoryRow } from "./gen/bindings/HistoryRow";
import type { SearchHit } from "./gen/bindings/SearchHit";
import type { RecentRow } from "./gen/bindings/RecentRow";
import type { WebBorrow } from "./gen/bindings/WebBorrow";
import type { TermStart } from "./gen/bindings/TermStart";
import type { EndedPage } from "./gen/bindings/EndedPage";
import type { TermChunk } from "./gen/bindings/TermChunk";
import type { PaneAnchor } from "./gen/bindings/PaneAnchor";
import type { PaneHistory } from "./gen/bindings/PaneHistory";
import type { LaunchPreset } from "./gen/bindings/LaunchPreset";
import type { AgentRow } from "./gen/bindings/AgentRow";
import type { KnownAgent } from "./gen/bindings/KnownAgent";
import type { AgentSaid } from "./gen/bindings/AgentSaid";
import type { CandidateRow } from "./gen/bindings/CandidateRow";
import type { FindHit } from "./gen/bindings/FindHit";
import type { SeenSession } from "./gen/bindings/SeenSession";
import type { QuotaAnswer } from "./gen/bindings/QuotaAnswer";
import type { QuotaCredits } from "./gen/bindings/QuotaCredits";
import type { QuotaWindow } from "./gen/bindings/QuotaWindow";
import type { QuotaTrouble } from "./gen/bindings/QuotaTrouble";
import type { MachineLog } from "./gen/bindings/MachineLog";
import type { ConsoleCommand } from "./gen/bindings/ConsoleCommand";
import type { Completion as ConsoleCompletion } from "./gen/bindings/Completion";
import type { Choice as ConsoleChoice } from "./gen/bindings/Choice";
import type { SeenMessage } from "./gen/bindings/SeenMessage";
import type { AccountRing } from "./gen/bindings/AccountRing";
import type { ChatMsgRow } from "./gen/bindings/ChatMsgRow";
import type { ChatFileRow } from "./gen/bindings/ChatFileRow";
import type { Load } from "./gen/bindings/Load";
import type { TopRow } from "./gen/bindings/TopRow";
import type { CpuDay } from "./gen/bindings/CpuDay";
import type { WatchedDir } from "./gen/bindings/WatchedDir";

export type {
  DropEnd,
  Miss,
  AccountRing,
  TabRow,
  TabsOpen,
  WorkspaceRow,
  FileText,
  FileStat,
  DirWatchScope,
  DirWatchCoverageMode,
  DirWatchCoverage,
  DirWatchBatch,
  DirWatchOpened,
  GitStatus,
  GitRow,
  GitDiff,
  GitBlame,
  GitLog,
  GitHistoryPage,
  GitCommitSummary,
  GitShow,
  GitHunk,
  GitCommitShow,
  Wrote,
  Merged,
  CodeAt,
  Preview,
  SessionRow,
  DeviceRow,
  DayRun,
  DayMark,
  FaceChoices,
  RelayChoices,
  RelayWant,
  RoadShare,
  Connections,
  HooksState,
  RecognitionEntry,
  Ticket,
  LeaveOutcome,
  Waiting,
  PendingRow,
  About,
  DoctorRow,
  LockFacts,
  ConfigRow,
  ConfigChoice,
  Usage,
  Strain,
  ChatBatch,
  ChatFrame,
  DirListing,
  AddAsk,
  AddRun,
  LoginAsk,
  Found,
  SearchHit,
  RecentRow,
  WebBorrow,
  AgentRow,
  KnownAgent,
  AgentSaid,
  CandidateRow,
  FindHit,
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
//
// **A body that is not one of khor's sentences is not painted.** An
// empty body or a page of HTML (a proxy's error page, a dev server's
// 404) is somebody else's answer; what reaches the glass is the
// catalog's sentence, with the raw body kept as the error's `cause`.
const unwrap = (body: string): string => {
  try {
    const v: unknown = JSON.parse(body);
    if (typeof v === "string") return v;
  } catch {
    /* not JSON — judged below */
  }
  const plain = body.trim();
  return plain === "" || plain.startsWith("<") ? gui.backend_garbled : plain;
};

/**
 * `fetch`, with a failure to reach khor said in the catalog's words.
 *
 * 2026-09-28: 「Failed to fetch」 was painted as-is at the top of the
 * browse board and in red inside the new-session dialog. That string is
 * the browser's, and it is the same for every way the request did not
 * arrive (no bridge, a refused port, CORS) — so it carries nothing a
 * person can act on beyond 「khor did not answer」, which is exactly
 * what `backend-unreachable` already says. **One seam, so every
 * `e.message` downstream gets the sentence without each view learning
 * to translate.** The browser's own error rides along as `cause` for
 * whoever debugs it.
 */
async function reach(url: string, init: RequestInit): Promise<Response> {
  try {
    return await fetch(url, init);
  } catch (e) {
    throw new Error(gui.backend_unreachable, { cause: e });
  }
}

/** The answer's JSON, or the catalog's sentence when it is not JSON —
    a `SyntaxError: Unexpected token '<'` is the parser talking, not
    khor. */
async function answer<T>(r: Response): Promise<T> {
  const body = await r.text();
  try {
    return JSON.parse(body) as T;
  } catch (e) {
    throw new Error(gui.backend_garbled, { cause: e });
  }
}

async function post<T>(url: string, headers: Record<string, string>, args?: Record<string, unknown>): Promise<T> {
  const r = await reach(url, {
    method: "POST",
    headers: { "content-type": "application/json", ...headers },
    body: JSON.stringify(args ?? {}),
  });
  if (!r.ok) throw new Error(unwrap(await r.text()));
  return answer<T>(r);
}

/**
 * What a failed call says to a person: the error's own message, which
 * after `reach`/`unwrap` is always either khor's sentence or the
 * catalog's. `String(e)` on an `Error` prints 「Error: …」, the class
 * name in front of the sentence — so every view reads it through here.
 */
export function said(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
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

/** **The settings window** (UX-10; ux-9 M1): the same page told to be
    the 「…」 pages alone, in a window of their own (`show_settings`),
    with the pane to open first. */
export const settingsFace = params.get("face") === "settings";
export const settingsPane = params.get("pane");
export const settingsMachine = params.get("machine");

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

/** Every workspace, pinned first then by path
    (`khor_gui_core::workspaces`).

    **One read and no machine named.** A workspace is a decision and
    decisions replicate, so the whole list is on this disk — there is no
    「那台机器答不了」state here, because no machine is asked. What does
    need a far machine is what a row *shows*: `fetchGitStatus` and
    `fetchTouched`, asked per row, each leaving its corner blank when
    that machine cannot answer. */
/* ── the browsing tabs (docs/SHELL.md 第 2 条) ──────────────────────
 *
 * **They live in `.khor/tabs.json` on this machine, not in this
 * window's storage**, and that is what `khor tabs` reads. The list was
 * `localStorage` until 2026-09-20 and everything worked except the part
 * of the ruling that says the terminal has to be able to list, open and
 * close them — and CLI 与 GUI 等价 is one of this repo's three standing
 * rules, so a list only the webview can read is not an option.
 *
 * **Every write answers with the list as it now stands.** A caller that
 * had to ask again to learn what its own write did would be two reads
 * out of step for the length of a round trip, and the column would
 * flicker through the old order on every close.
 *
 * `kind` is `page` today; the store is keyed by it so a second browsing
 * kind lands beside rather than inside (`khor_node::tabs`). */
export const fetchTabs = (kind: string) => call<TabsOpen>("tabs_list", { kind });
/** Opens one and puts it in front; answers its id. Both places empty is
    the fresh tab the 「+」 mints — a real state, not a half-made row.
    **The id is minted by the node**, because the terminal opens tabs
    too and two minters is two id shapes. `request` identifies a retry of
    one opening operation; distinct starting tabs never deduplicate by URL. */
export const openTab = (kind: string, device: string, url: string, session?: string, after?: string, view?: "workspace" | "file" | "page", request?: string) =>
  call<string>("tabs_open", { kind, device, url, ...(session ? { session } : {}), ...(after ? { after } : {}), ...(view ? { view } : {}), ...(request ? { request } : {}) });
/** Points an open tab somewhere and names it. Refused by name for a tab
    that is not there, rather than quietly adding one. */
export const goTab = (kind: string, id: string, device: string, url: string, title: string, view?: "workspace" | "file" | "page") =>
  call<TabsOpen>("tabs_go", { kind, id, device, url, title, ...(view ? { view } : {}) });
export const focusTab = (kind: string, id: string) =>
  call<TabsOpen>("tabs_focus", { kind, id });
export const closeTab = (kind: string, id: string) =>
  call<TabsOpen>("tabs_close", { kind, id });
/** 置顶 / 取消置顶 one (docs/SHELL.md 第二轮) — `khor tabs pin|unpin`. The
    node keeps pinned tabs first and refuses to close one. */
export const pinTab = (kind: string, id: string, on: boolean) =>
  call<TabsOpen>("tabs_pin", { kind, id, on });
/** The whole column at once — the one gesture that cannot be spelled as
    a close and an open: a drag that reorders. Ids are still the node's
    to mint, and a row carrying one it never minted is refused. */
/** Reorder a session's known ids without overwriting live rows or focus. */
export const reorderTabs = (kind: string, session: string, ids: string[]) =>
  call<TabsOpen>("tabs_reorder", {kind,session,ids});
export const saveTabs = (kind: string, open: TabsOpen) =>
  call<TabsOpen>("tabs_save", { kind, open });

export const fetchWorkspaces = () => call<WorkspaceRow[]>("workspaces");
/** Declares a directory on a machine a workspace; `machine` empty is
    this one. **The one workspace call that goes out on the wire**, and
    only to ask whether that path is a directory — a path nobody can
    stand in is refused while somebody is here to be told. */
export const wsAdd = (machine: string, path: string) =>
  call<null>("ws_add", { machine, path });
/** Takes it off the list. **The directory is not touched** (user,
    2026-09-14).

    **Keyed by `device` and not by a machine name**, this one and
    `wsPin`: a row whose machine has left the device table carries no
    name, and it is precisely the row somebody needs to take off. An
    empty name would mean「this machine」to the node and the write would
    land on a key about another disk (`khor_node::Node::ws_write`). */
export const wsRm = (device: string, path: string) =>
  call<null>("ws_rm", { device, path });
/** Floats it to the top, or lets it back down — still a workspace
    either way. `on` is explicit for the reason every pin API's is. */
export const wsPin = (device: string, path: string, on: boolean) =>
  call<null>("ws_pin", { device, path, on });
/** When that directory last changed: the newest file mtime under it,
    ms, `0` where there was nothing to date
    (`khor_node::workspace::touched`).

    Asked per row, of the machine holding the files. It refuses out
    loud; the row paints no time rather than a wrong one, and stays
    exactly where it is. */
export const fetchTouched = (machine: string, path: string) =>
  call<number>("touched", { machine, path });
/** One file off a machine, as much of it as one slice holds
    (`khor_gui_core::files::read_text`). For reading; `pullFile` is the
    other verb and it spends a disk. */
export const fetchFileText = (machine: string, path: string) =>
  call<FileText>("read_text", { machine, path });
/** What an open file is now, without its bytes: size, mtime, and the
    digest of a file that fits one slice (`khor_gui_core::files::
    stat_text`). Asked instead of the file on coming back to a window. */
export const fetchFileStat = (machine: string, path: string) =>
  call<FileStat>("stat_text", { machine, path });
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
/** Whether that directory is a repository at all — one `stat` on the
    machine holding it (`khor_node::git::is_checkout`).

    **Asked beside `fetchGitStatus`, never after it** (docs/SHELL.md
    第五轮 第 6 条): the workspace row's branch-or-folder corner is this
    answer, and the status walk is the one that can take seconds on a
    big tree or a network disk, or fail. A khor that predates it refuses,
    and the corner falls back to whether a status came back at all. */
export const fetchCheckout = (machine: string, dir: string) =>
  call<boolean>("checkout", { machine, dir });
/** Where one file differs from HEAD (`khor_gui_core::git::diff`), in
    the coordinates of the file as it is now. Refuses out loud for
    `fetchGitStatus`'s reason. */
export const fetchGitDiff = (machine: string, path: string) =>
  call<GitDiff>("git_diff", { machine, path });
/** One file's history (round UX-4, `khor_gui_core::git`): the commits
    that changed it, newest first; who wrote each of HEAD's lines; and the
    file as one revision had it, with where the current file differs. */
export const fetchGitLog = (machine: string, path: string) => call<GitLog>("git_log", { machine, path });
export const fetchGitHistory = (machine: string, dir: string, cursor?: string | null, limit = 0) =>
  call<GitHistoryPage>("git_history", { machine, dir, cursor, limit });
export const fetchGitBlame = (machine: string, path: string) => call<GitBlame>("git_blame", { machine, path });
export const fetchGitShow = (machine: string, path: string, rev: string) =>
  call<GitShow>("git_show", { machine, path, rev });
/** One commit of the repository `dir` is in, and what it changed (the
    commit page, COMMITS-BRIEF 2026-10-12). */
export const fetchGitCommit = (machine: string, dir: string, rev: string) =>
  call<GitCommitShow>("git_commit", { machine, dir, rev });
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

/** A window of one machine's own, with that machine's editor in it
    (工作区第六轮 第 46 条). */
export type CodeSeat = {
  /** Put the editor in it — the first call — or bring it to the front.
      `url` is what `code_open` answered. */
  raise: (url: string, title: string) => Promise<void>;
  /** Whether the person has closed it since. */
  gone: () => boolean;
  /** Whether [`gone`] is a question worth asking on a beat
      (工作区第七轮 第 57 条). True on the web face, where the tab
      answers `closed` and nothing else; false in the app, which
      announces `code:closed` the moment the window dies and would
      have a timer asking a road that always answers no. */
  asks: boolean;
  /** Give it up: the node refused, and an empty window is worse than
      none. */
  drop: () => void;
};

/**
 * Claim a seat for that machine's editor — **a window in the app, a
 * named tab in a browser** (工作区第六轮 第 46 条:「VSCODE做成弹出式的,
 * 弹出一个窗口显示vscode」— the user).
 *
 * **It must be called inside the press, before anything is awaited.**
 * A browser's popup blocker allows `window.open` only from the task a
 * person's own gesture started, and the editor's URL is not known until
 * the node has answered — so the tab is opened blank here and pointed
 * at the editor by [`CodeSeat.raise`] when the answer lands. The app
 * has no such rule and builds its window in `raise`; the shape is the
 * same on both so that the caller has one sequence rather than two.
 *
 * **Named, so pressing the switch twice does not make two.** The tab
 * name and the window label are both `khor-code:<machine>`: the browser
 * hands back the tab it already has, and the app's own command focuses
 * the window it already built.
 *
 * **`gone` is the web face's question and the app's announcement.** A
 * tab showing another origin tells this page nothing, and `closed` is
 * the one thing it will still answer — so the browser road is asked on
 * a beat while the tab is held (`asks`, 工作区第七轮 第 57 条). The app
 * emits `code:closed` instead, the moment the window is destroyed, and
 * answers `false` here: one fact, one road each.
 */
export function codeSeat(machine: string): CodeSeat {
  const name = `khor-code:${machine}`;
  if (inApp) {
    return {
      raise: (url, title) => call<null>("open_code_window", { machine, url, title }).then(() => {}),
      // **The app does not answer this; it announces.** A window it
      // built emits `code:closed` the moment it is destroyed, which the
      // face is already listening for — asking as well would be a
      // second road to one fact, and this one would have to guess.
      gone: () => false,
      asks: false,
      drop: () => {},
    };
  }
  let tab: Window | null = window.open("", name);
  /** Whether that tab has been sent to the editor yet. **Remembered
      here rather than read off the tab**: once it is showing the
      editor it is another origin, and asking it where it is throws. */
  let pointed = false;
  return {
    raise: async (url) => {
      if (tab === null || tab.closed) {
        tab = window.open(url, name);
        pointed = true;
      } else if (!pointed) {
        tab.location.replace(url);
        pointed = true;
      }
      // Already up and pointed: brought forward rather than navigated
      // again, because a reload would throw away whatever the editor
      // has open.
      tab?.focus();
    },
    gone: () => tab === null || tab.closed,
    asks: true,
    drop: () => {
      tab?.close();
      tab = null;
    },
  };
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
/** Every machine, **with a road this face can stand behind** (2026-10-04,
    the owner on a phone: 「进入页面显示无法获取，应该是加载中才对」).

    The node's 「not connected」 (`hop` `quiet`) is *no live wire right
    now* — true, too, of a machine nobody has tried yet. Entering a page
    in the first moments after the node came up, every machine it had
    not dialled yet read 未连接, and every row on one dimmed with 无法获取,
    a result before any answer. So one rule, here, for every face that
    reads machines (the shell's poll, the new-session picker, the
    directory picker):

    - **unknown**: `quiet`, never heard since the resident started
      (`heard_ms` null), and this face has been reading machines for less
      than a dial's budget (`DIAL_TIMEOUT` in `khor_node::link`, 10 s).
      The road is `null`, the shape every surface already draws as 「not
      named yet」: no 未连接, no 无法获取, no dimming; what is asked of the
      machine itself wears its skeleton or `Waiting` meanwhile;
    - **idle**: `quiet` after that, or `quiet` with a time it was last
      heard — no live wire, nothing known to be wrong: 未连, undimmed;
    - **unreachable**: `quiet` with `down_ms` (round NET-13: the
      resident's last attempt failed and nothing was heard since) — the
      one reading that dims a row and says 无法获取, at once (P6), the
      budget or not;
    - **reachable**: a road.

    The clock is this page's first answer, not the node's start, which no
    face is told: a face opened long after its node waits the budget once
    before naming a machine idle. */
export const fetchDevices = () =>
  call<DeviceRow[]>("devices").then((rows) => {
    const now = Date.now();
    devicesFirstRead ??= now;
    if (now - devicesFirstRead >= DIAL_BUDGET_MS) return rows;
    return rows.map((d) => (d.hop === "quiet" && d.heard_ms == null && d.down_ms == null ? { ...d, hop: null } : d));
  });
/** When this page first read the machines (`fetchDevices`). */
let devicesFirstRead: number | null = null;
/** How long a dial may take before the node gives up on a machine
    (`khor_node::link::DIAL_TIMEOUT`). */
const DIAL_BUDGET_MS = 10_000;
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
export const fetchSubscriptionCandidates = (machine: string) =>
  call<import("./gen/bindings/SubscriptionCandidate").SubscriptionCandidate[]>("subscription_candidates", { machine });
export const importSubscription = (machine: string, provider: string) =>
  call<string>("import_subscription", { machine, provider });
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
/** Renames an account, network-wide — the 额度 row's double-click
    (2026-09-30). The name alone: `updateAccount` writes every field the
    编辑 window holds. A blank name is refused in the node's words. */
export const renameAccount = (id: string, name: string) => call<null>("rename_account", { id, name });
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
export const markSeen = (id: string, at_ms: number) => call<null>("seen", { id, at_ms });
export const markSeenSnapshot = (id: string, messages: SeenMessage[]) =>
  call<null>("seen_snapshot", { id, messages });
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
/** **A message composed into the broadcast** (STORE-BRIEF phase 5,
    `Node::post`): the typed form (`@机器:路径` leading, then
    `机器:路径` files, then the words) and this machine's files, by path.
    Answers the message id; refused with the node's sentence — a target
    that is not reachable is named, and nothing was written. */
export const compose = (typed: string, files: string[] = []) => call<string>("post", { typed, files });
/** The broadcast's readout — what `khor log` prints, shaped for the
    pane. */
export const machineLog = (id: string) => call<MachineLog>("machine_log", { id });
export type { ConsoleCommand, ConsoleChoice, ConsoleCompletion };
export const consoleCatalog = () => call<ConsoleCommand[]>("console_catalog");
export type { KhorAgentView } from "./gen/bindings/KhorAgentView";
export const khorAgent = (action: string, machine = "", payload: Record<string, unknown> = {}) =>
  call<import("./gen/bindings/KhorAgentView").KhorAgentView>("khor_agent", { action, machine, payload });
export const consoleComplete = (line: string, caret: number) => call<ConsoleCompletion>("console_complete", { line, caret });
export const consoleRun = (line: string, token: string, input?: string) =>
  call<string>("console_run", { line, token, ...(input === undefined ? {} : { input }) });
/** A broadcast file taken, or taken again after it failed or was
    cancelled — `khor retry`, same node call (docs/KHOR.md equivalence).
    Answers where the files landed. */
export const retryTransfer = (msg: string) => call<string[]>("retry_transfer", { msg });
/** A file's move cancelled on this machine (the chip's ×): nothing
    partial stays. */
export const stopTransfer = (msg: string) => call<null>("stop_transfer", { msg });
/** The composer's ＋, app form: the native picker answers with real
    local paths, which ride the message as they are. Empty = cancelled,
    which is not an error. App only (`inApp`) — the browser forms stage
    bytes instead (`stageFile`). */
export const pickPaths = () => call<string[]>("pick_paths");
/** The composer's ＋, browser form: the bytes stream to this machine's
    staging area and the staged path comes back, to ride the message
    (`khor_gui_core::stage_upload`). Raw-bytes POST, not a JSON command —
    a file does not belong inside a JSON envelope — so this is the one
    call in this file that does not go through `call`. The size cap and
    its refusal sentence are the backend's (`UPLOAD_CAP_BYTES`). */
export const stageFile = async (file: File): Promise<string> => {
  const q = `name=${encodeURIComponent(file.name)}`;
  const url = bridge ? `http://127.0.0.1:${bridge}/upload?${q}` : `/api/upload?${q}`;
  const r = await reach(url, {
    method: "POST",
    // Same rule as `call`: the header rides when the key is held; a
    // bare page leans on the remembered cookie (记住一次批).
    headers: bridge || webKey === null ? {} : { authorization: `Bearer ${webKey}` },
    body: file,
  });
  if (!r.ok) throw new Error(unwrap(await r.text()));
  return answer<string>(r);
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

/** 导入 (docs/SHELL.md 第八轮): what could be brought onto the session
    list — agents started elsewhere, tmux sessions, and the ACP
    conversations a registered agent said it has (as recorded; asking is
    `agentAsk`, behind a press). Newest first. */
/** 全局搜索 (docs/SHELL.md 第十轮): one word over sessions, machines,
    workspaces, browsing tabs and the app's pages. **The matching and the
    order are the node's** (`Node::find`), so ⌘K and `khor find` answer
    with the same list — a second rule on this side is how the two come
    to disagree about the first hit. */
export const find = (word: string) => call<FindHit[]>("find", { word });
/** **Windows' own three, pressed** (docs/SHELL.md 第十轮). Only that
    platform draws them: macOS has the system's inside the row and Linux
    keeps its decorations. A no-op outside the app. */
export const windowMinimize = () => call<null>("window_minimize");
export const windowMaximizeToggle = () => call<null>("window_maximize_toggle");
export const windowClose = () => call<null>("window_close");
/**
 * Whether this is the app on Windows — the one face that draws the
 * window's three buttons itself.
 *
 * `navigator.platform` for `trafficLights`' reason, one line up: it is
 * deprecated and still the only thing that answers without a round trip,
 * and a wrong answer costs three buttons that are not needed rather than
 * a broken face.
 */
export const windowsApp = inApp && /win/i.test(navigator.platform || navigator.userAgent);
/** How much of the title row's left end the system's three buttons stand
    in, **as the window measured them** (docs/SHELL.md 第十一轮): the SDK
    decides their size, so this is asked rather than written down. Zero
    off the app and off macOS. */
export const trafficRoom = () => call<number>("traffic_room");
export const fetchImportCandidates = () => call<CandidateRow[]>("import_candidates");
/** Brings these onto the list, in order; answers their session ids.
    Stops at the first refusal and says it. */
export const importSessions = (ids: string[]) => call<string[]>("import_sessions", { ids });
/** The wizard (会话身份批B): a fresh agent session in `dir`, as a
    conversation ("chat") or a terminal ("term"), for the chosen agent.
    `agent` is an **open position** since 批⑥: "claude", "codex", or the
    name of anything registered — the node resolves it and refuses by
    name, so this layer neither narrows nor validates it. `machine` is
    where the session is born (在哪台): empty or this machine's name is
    here, any other paired name carries the four answers over there
    (`Node::open_agent_on`). Answers the new row's id.
    `token` makes it at most once (round NET-7, `khor_node`'s `once`):
    asked again with the same token — after an answer that never came —
    the far machine answers the session the first ask made. */
export const openSession = (
  dir: string,
  title: string,
  form: "chat" | "term",
  agent: string,
  machine = "",
  token?: string,
) => call<string>("open_session", { dir, title, form, agent, machine, token });
/** The wizard's shell kind — `open --on` with that machine's own shell,
    through the same core call both skins answer with (docs/KHOR.md
    equivalence). `machine` empty is this one; `title` empty is named
    after the shell; `cwd` empty is the home, and `~` is expanded on
    the machine it names. Answers the new row's id. */
export const openTermOn = (machine: string, title = "", cwd = "", token?: string) =>
  // At this device's terminal size, so the first prompt is written for
  // the width it will be shown at (`lib/term-size-hint`). `token` as
  // `openSession`'s.
  call<string>("open_on", { machine, title, cwd, ...termSizeHint(), token });
export type { LaunchPreset } from './gen/bindings/LaunchPreset';
export const launchPresets = () => call<LaunchPreset[]>('launch_presets');
export const saveLaunchPreset = (preset: LaunchPreset) => call<LaunchPreset[]>('launch_preset_save', { preset });
export const deleteLaunchPreset = (id: string) => call<LaunchPreset[]>('launch_preset_delete', { id });
export const openShell = (machine: string, cwd: string, command: string, title: string, token: string) =>
  call<string>('open_shell', { machine, cwd, command, title, token, ...termSizeHint() });

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
/** Where files sent to a machine land (存储's location row), for
    everyone; `""` takes it back to the default (`~/khor-drop`). The node
    refuses a relative path in its own sentence and answers the setting
    as kept. */
export const setReceiveDir = (machine: string, dir: string) =>
  call<string>("set_receive_dir", { machine, dir });
/** What a machine wears and every option painted as it would look —
    derived by the node, one call for the whole editor.

    `machine` is a device id and **empty is this one**. Every swatch is
    painted under *that* machine's seed, so what the editor shows is the
    face that machine is about to have rather than this one's. */
export const fetchFaceChoices = (machine = "") =>
  call<FaceChoices>("face_choices", { machine });
/** Changes the axes it is given; an axis left out stays where it is. The
    same call `khor avatar` makes, with the same shape of arguments —
    including which machine, because a face is 全网同一份 and anyone may
    set anyone's (`khor_sync::devices`). */
export const restyle = (
  machine: string,
  change: {
    colors?: string[];
    variant?: string;
    shape?: string;
  },
) => call<null>("restyle", { machine, ...change });
/** This machine's relays, both halves: `mine` is what the user typed
    here and the only part a control can remove, `in_use` is everything
    the machine actually binds with (the Khor tier included). `keyed`
    says which of `mine` carry a key — never the key itself. */
export const fetchRelays = () => call<RelayChoices>("relays");
/** Replaces the user's own relays and answers with both halves again.
    An entry that is not a relay url is refused whole — nothing is
    written and the message names it, and so is `onlyMine` with an empty
    list (that combination leaves the machine with no relay at all).
    A want whose `token` is null keeps the key already stored for that
    url, so only the relay being added carries one.
    ⚠️ `in_use` describes the **next** node to open: a running
    `khor serve` keeps the list it started with. */
export const setRelays = (relays: RelayWant[], onlyMine: boolean) =>
  call<RelayChoices>("set_relays", { relays, only_mine: onlyMine });
/** 直连占比: of the resident's recent dials that settled, how many went
    direct. `unread` says why there is no reading when there is none. */
export const fetchRoadShare = () => call<RoadShare>("road_share");
/** The 网络 page's 连接 group: home relay, relay round trips, and per
    machine its road, round trip, age and last failure (round NET-2). */
export const fetchConnections = () => call<Connections>("connections");
/** 全网谁够得着 (round ROAD-1): every online machine connects once to
    `host:port` (`null` = ssh's 22) from its own network and reports what
    it saw; the entrance is the fastest. Takes up to ~20 s when a machine
    does not answer. App only: the web face does not offer it. */
export const reachProbe = (host: string, port: number | null) =>
  call<ReachReport>("reach_probe", { host, port });
/** 经由: the ProxyCommand that reaches `host:port` through `machine`
    (`khor pipe --via`), as an ssh config line and a one-off command, for
    a copy button. khor never writes the user's ssh config. */
export const proxyLine = (machine: string, host: string, port: number | null) =>
  call<ProxyLine>("proxy_line", { machine, host, port });
/** 连接记录 (NET-STABLE 第一步): what this machine's resident saw on its
    way to `machine` (an id), this machine's own events interleaved, over
    the last `minutes`. Read off files: answers with no resident. */
export const journal = (machine: string, minutes: number) =>
  call<JournalLine[]>("journal", { machine, minutes });
/** 报告: every machine's last `minutes` frozen into one file under this
    home; answers its path. */
export const journalReport = (minutes: number) => call<string>("journal_report", { minutes });
/** 添加机器 over SSH (round join-d): starts the add as the run `token`
    (made by the face, so a second press starts nothing). Read it with
    `addRead`. The secret in `ask` goes to the OS secret store of the
    machine that runs ssh and nowhere else. App only. */
export const addStart = (token: string, ask: AddAsk) => call<null>("add_start", { token, ask });
/** The run `token`'s steps so far, and how it ended once it has. */
export const addRead = (token: string) => call<AddRun>("add_read", { token });
/** 安装 khor on an SSH 主机 this machine sponsors, as the run `token`. */
export const promoteStart = (token: string, machine: string) => call<null>("promote_start", { token, machine });
/** The Host names of this machine's ssh config, read only. */
export const fetchSshConfigHosts = () => call<string[]>("ssh_config_hosts");
/** 上传公钥: a terminal session on this machine running `ssh-copy-id`
    for the form's host. Answers the session's id. */
export const openCopyId = (ask: LoginAsk) => call<string>("open_copy_id", { ask });
/** 选择密钥文件…: the system's picker in `~/.ssh`; the path, or `null`
    when cancelled. */
export const pickKeyFile = () => call<string | null>("pick_key_file");
/** This face is on screen, or stopped being: what paces the resident's
    pump (`khor_gui_core::face_here`, round NET-6). */
export const faceHere = (visible: boolean) => call<null>("face_here", { visible });
/** A machine's page was opened: the resident has a round with it now,
    past its backoff (`khor_gui_core::visit`, NET-STABLE 第二步). */
export const visitMachine = (machine: string) => call<null>("visit", { machine });
/** **立即重试** on a session reconnecting to `machine` (NOTICE-BRIEF):
    its rejoin tries now, and the machine is dialled past its backoff
    (`khor_gui_core::rejoin_now`). */
export const rejoinNow = (machine: string) => call<null>("rejoin_now", { machine });
/** 首选中继, or `null` for the nearest. */
export const setPreferredRelay = (url: string | null) => call<null>("set_preferred_relay", { url });
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
/** `khor sync` — a round with every machine, now: what pulling a list
    down asks for. One answer per machine, reached or not. */
export const syncNow = () => call<SyncOutcome[]>("sync_now");

/* ── 加入网络's 扫码 (docs/IOS.md 第三步) ──────────────────────────────
 *
 * **One join, two ways to enter the ticket.** The invite's QR code is
 * the ticket itself, byte for byte (`khor_gui_core::qr`), so what the
 * camera reads goes into the same field a paste fills and down the same
 * `pair`. Nothing here joins anything.
 *
 * The camera is the phone app's (`tauri-plugin-barcode-scanner`, under
 * the grant in `src-tauri/capabilities/scan.json`); whether this face
 * has one is the shell's answer, not a guess from the browser. */

/** Whether this face can read a ticket's QR code with a camera — the
    phone builds say yes, a desktop and every HTTP skin no. A face that
    hears no draws no entry (absent, not refused). Asked once. */
let scannerAsked: Promise<boolean> | null = null;
export const fetchScannerHere = (): Promise<boolean> =>
  (scannerAsked ??= call<boolean>("scanner_here").catch(() => false));

/** How a scan ended. `refused` carries which sentence to say: the
    camera permission is off (`denied`), or the camera could not be had
    at all (`failed` — a simulator has none). */
export type Scan = { ticket: string } | { refused: "denied" | "failed" } | { cancelled: true };

/** Set while a scan this face started is being stopped, so its rejection
    reads as the person's 取消 and not as a camera that failed. */
let stopping = false;

/**
 * Reads a ticket off a QR code with the camera: asks for the permission
 * when it was never asked, then scans **windowed** — the camera behind a
 * page gone transparent, so the page draws its own way out (`stopScan`);
 * the plugin's full-screen mode has none. `scanning` is called when the
 * camera is about to come up, so the page can clear itself away first.
 */
export async function scanTicket(scanning: () => void): Promise<Scan> {
  const scanner = await import("@tauri-apps/plugin-barcode-scanner");
  let camera: string;
  try {
    camera = await scanner.checkPermissions();
    if (camera === "prompt" || camera === "prompt-with-rationale") {
      camera = await scanner.requestPermissions();
    }
  } catch {
    return { refused: "failed" };
  }
  if (camera !== "granted") return { refused: "denied" };
  stopping = false;
  scanning();
  try {
    const read = await scanner.scan({ windowed: true, formats: [scanner.Format.QRCode] });
    return { ticket: read.content.trim() };
  } catch {
    return stopping ? { cancelled: true } : { refused: "failed" };
  } finally {
    stopping = false;
  }
}

/** 取消 on the camera: the scan ends and answers `cancelled`. */
export async function stopScan(): Promise<void> {
  stopping = true;
  const scanner = await import("@tauri-apps/plugin-barcode-scanner");
  await scanner.cancel().catch(() => {});
}

/** The system's settings for khor — where a refused camera permission
    is given back. */
export async function openCameraSettings(): Promise<void> {
  const scanner = await import("@tauri-apps/plugin-barcode-scanner");
  await scanner.openAppSettings().catch(() => {});
}
/** 待批 (docs/CONNECT.md 根不在场): who waits for the root holder, and
    whether this machine is it — the holder gets a button per row, every
    other machine a sentence saying where the button is. */
export const fetchWaiting = () => call<Waiting>("pending_list");
/** `khor approve` — the holder signs one in and pushes the roster out;
    answers the list as it now stands. */
export const approveMachine = (machine: string) => call<Waiting>("approve", { machine });
/** The 24 words, for 「…」 → 安全's 「显示…」. **App-only**: the web face
    has no such command (`khor_gui_core` `tests/command_tables.rs`), so
    only the app asks. (`root_unshown` / `root_words_seen` were the
    first-start prompt's two questions; since 2026-10-01 nothing opens on
    its own, so the face asks neither.) */
export const rootWords = () => call<string>("root_words");
/** `khor leave` — this machine out of the network (docs/MORE.md 离开).
    Takes this machine's own name typed back, because that is the
    confirmation and **the node is what checks it** (`Node::leave`, the
    rule `lock::reset_identity` set: one rule for the verb and the page,
    or the one nobody runs is the one that drifts).

    Answers who heard it and who did not. The two are kept apart all the
    way to the screen: a machine in `untold` goes on painting this one
    offline until somebody over there runs `khor forget`, and that is a
    different thing left to do from 「everybody knows」. */
export const leaveNetwork = (machine: string) => call<LeaveOutcome>("leave", { machine });

/** `khor pause` / `khor resume` — this machine stops talking to the
    network without leaving it (docs/MORE.md 暂停). Answers the fresh
    `About`, because pausing is a fact the card is already showing and
    re-reading it is how the two stay one answer. The node refuses the
    state it is already in, and that refusal is a sentence worth
    showing. */
export const setPaused = (on: boolean) => call<About>("set_paused", { on });
/** `khor pause <机器>` / `khor resume <机器>` — this machine stops (or
    starts again) talking to that one; it runs on as it was (docs/SHELL.md
    第五轮 第 3、10 条). This machine's own name is `setPaused`. Answers
    the node's sentence; the row's grey arrives with the next device
    list, which reads the same file. */
export const setMachinePaused = (machine: string, on: boolean) =>
  call<string>("set_machine_paused", { machine, on });
/** Which khor this is and which store it is talking to — the identity
    card's foot (docs/MORE.md 头像). A fact about **this** installation:
    what another machine is running is `khor doctor`'s question, asked
    over the wire, never a field on the device table. */
export const fetchAbout = () => call<About>("about");
/**
 * 外观 (批 D2): hands the app's **window** the theme the face resolved.
 *
 * **A no-op outside the app, and that is the whole seam.** Three things
 * on an app screen are not the document — the ground painted from Rust
 * before any HTML exists, the macOS title bar, and every editor window
 * khor opened — and none of them follows `data-theme`. A page in a
 * browser has none of the three: the window it sits in was placed by
 * somebody else. So there is no HTTP arm for this (`command_tables`'s
 * `ONE_SIDED` carries the reason from the other end), and calling it
 * anywhere but the app silently does nothing rather than throwing — the
 * same shape `widenWindow` has.
 *
 * Takes the **resolved** theme, never the choice: `auto` is a question
 * only `matchMedia` can answer, and a second resolver on the Rust side
 * would be two answers that disagree whenever one is late.
 */
export const setWindowTheme = (theme: "light" | "dark" | "auto") =>
  inApp ? call<null>("set_window_theme", { theme }) : Promise.resolve(null);
/** Only the desktop main window can show the desktop behind the page. */
export const saveTerminalPalette = (text: string) => call<boolean>("save_terminal_palette", { text });
export const windowAppearanceSupport = () => call<{ supported: boolean; blur: boolean }>("window_appearance_support");
export const setWindowAppearance = (material: "solid" | "clear" | "frost") => call<null>("set_window_appearance", { material });
/** 诊断 (docs/MORE.md): twelve checks, each with a three-second budget
    of its own, so the whole call is bounded at that — a relay that
    swallows packets costs one row, never the page. */
export const fetchDoctor = () => call<DoctorRow[]>("doctor");
/** One check's remedy, by the check's own key — and the sentence saying
    what it did. Refused by name for a key with no remedy. */
export const doctorFix = (key: string) => call<string>("doctor_fix", { key });
/** 设置 (docs/MORE.md): every setting and the value it has now. **No
    node is opened on the other side** — the preferences file is read
    straight off the store — so the shell can ask this in the same breath
    as the lock, before anything is drawn.

    Every key comes back, never only the ones somebody has set: a row
    missing from this answer would be a switch the page simply does not
    draw, which is indistinguishable from a switch khor does not have. */
export const fetchConfig = () => call<ConfigRow[]>("config");
/** Sets one, and answers **the whole list again** so no page ever paints
    from a copy it patched itself. Refused by name for an unknown key,
    and by a second sentence for a value outside that key's ladder.
    Every other window of the app is told to read the store again
    (`lib/other-windows`): the Mac's 设置 is a window of its own. */
export const setConfig = (name: string, value: string) =>
  call<ConfigRow[]>("set_config", { name, value }).then(told(CONFIG_TOLD));
/** 安全 (docs/MORE.md): how this machine's faces are set to lock. **No
    node is opened on the other side** — the lock file is read straight
    off the store — so the shell can ask this before it draws anything,
    which is what lets a locked face come up locked rather than flashing
    a conversation first. */
export const fetchLock = () => call<LockFacts>("lock_state");
export const setLock = (on: boolean) => call<LockFacts>("set_lock", { on }).then(told(LOCK_TOLD));
/** An empty PIN clears it, and clearing turns the lock off in the same
    move — splitting those is how a machine claims to be locked with no
    key to it. */
export const setLockPin = (pin: string) => call<LockFacts>("set_lock_pin", { pin }).then(told(LOCK_TOLD));
export const setLockTimeout = (minutes: number) =>
  call<LockFacts>("set_lock_timeout", { minutes }).then(told(LOCK_TOLD));
/** Passes an answer through, once the other windows have been told. */
function told<T>(key: string): (answer: T) => T {
  return (answer) => {
    tellOthers(key);
    return answer;
  };
}
/** Whether this is the PIN. **`false` is a wrong PIN, not a failure**:
    only khor being unable to read its own record rejects. */
export const unlock = (pin: string) => call<boolean>("unlock", { pin });
/** The web face's key as it is on disk, `null` on a machine that has
    never raised a face. Shown masked; revealed on a press. */
export const fetchWebKey = () => call<string | null>("web_key");
/** A new key — the same call `khor web --new` makes. Every link printed
    before it stops working, which is the whole point. */
export const rotateWebKey = () => call<string>("rotate_web_key");
/** 重置身份. The machine's own name is the confirmation and **the node
    is what checks it**, so this dialog and `khor id --reset` cannot come
    to disagree about one irreversible act. */
export const resetIdentity = (name: string) => call<null>("reset_identity", { name });
/**
 * 「系统验证」 on the lock screen — Touch ID, the watch, or the login
 * password, whichever this Mac accepts.
 *
 * **`null` outside the app, and that is the whole seam**: a browser has
 * no LocalAuthentication to ask, so the lock screen draws no button
 * rather than drawing one that refuses (`command_tables`'s `ONE_SIDED`
 * carries the same reasoning from the other end). It is an extra key,
 * never the key — `khor lock` will not come on until a PIN is set,
 * because the PIN is the one every face has.
 *
 * Answers whether it opened. A cancelled sheet, a Mac with no sensor and
 * a policy that will not evaluate are all `false`: telling them apart
 * would be telling somebody about the security hardware of a machine
 * they have not unlocked.
 */
export const systemUnlock: ((reason?: string) => Promise<boolean>) | null = inApp
  ? (reason = msg.lock_system_reason) => call<boolean>("system_unlock", { reason })
  : null;
/** The chat six: an attachment registry in gui-core holds the socket
    (a webview cannot), a reader thread collects frames, and the poll
    answers instantly from a cursor — nothing is lost between polls.
    `chatLeave` detaches only; ending a conversation is `closeSession`. */
/** Answers whether this call attached anew — replay history only then:
    a second open of a living chat (dev double-mount) must not ask the
    host to replay again. */
export const attachmentBegin = (id: string, kind: AttachmentKind) => call<string>("attachment_begin", { id, kind });
export const chatOpen = (id: string, lease: string) => call<boolean>("chat_open", { id, lease });
/** Hands a page to whatever this machine opens pages with. Refuses
    anything that is not http/https — the address came out of an agent's
    own words (`khor_gui_core::web::open_link`). */
export const openLink = (url: string) => call<null>("open_link", { url });
/** A page's site icon as a `data:` URI, asked through the page's own
    exit — `proxy` is its borrow's local address, `""` for a page this
    machine opens itself (`khor_node::favicon`). */
export const pageIcon = (url: string, proxy: string) => call<string>("page_icon", { url, proxy });
/** Every site icon `pageIcon` has fetched before, kept on this disk by
    the site's origin (`khor_node::favicon::kept`, docs/SHELL.md 第五轮
    第 8 条). **A read of a file, never a fetch** — a restart draws these
    without asking any site anything. */
export const fetchPageIcons = () => call<Record<string, string>>("page_icons");
/** 接管 the other way: a conversation khor holds moves into a terminal
    (`Node::takeover_term`). Same row, same conversation. */
export const takeoverTerm = (id: string) => call<null>("takeover_term", { id });
/** Ends the running turn — not the session (`GuiOp::Stop`). */
export const chatStop = (id: string, lease: string) => call<null>("chat_stop", { id, lease });
/** Puts the session in one of the modes its agent listed
    (`ChatFrame::Settings`'s `modes.availableModes`), by the agent's own
    id. The answer comes back on the poll — a fresh `settings` frame when
    it took, a `refused` one when the agent said no — never here. */
export const chatSetMode = (id: string, lease: string, mode: string) => call<null>("chat_set_mode", { id, lease, mode });
/** Sets one of the agent's options: a string is a listed value's id, a
    boolean is a switch. Same answer channel as {@link chatSetMode}. */
export const chatSetOption = (id: string, lease: string, option: string, value: string | boolean) =>
  call<null>("chat_set_option", { id, lease, option, value });
export const chatRespond=(id:string, lease: string,ask:number,response:unknown)=>call<null>('chat_respond',{id, lease,ask,response});
export const chatControl=(id:string, lease: string,token:string,operation:string,payload:Record<string,unknown>={})=>call<null>('chat_control',{id, lease,token,operation,payload});
export const chatPoll = (id: string, lease: string, since: number) =>
  call<ChatBatch>("chat_poll", { id, lease, since });
/** `chatPoll`, held open until a frame lands past `since` (or `ms`
    passes) — `termWait`'s twin, and the app's only for its reason. */
export const chatWait = (id: string, lease: string, since: number, ms: number) =>
  call<ChatBatch>("chat_wait", { id, lease, since, ms });
/** One user turn. `images` are `[mime, base64]` pairs and are only ever
    non-empty when the agent said it takes them (`ChatFrame::Agent`'s
    `images`) — the host puts them on a different op (`GuiOp::SayWith`),
    so an empty list costs the old frame exactly nothing.
    With a client token, delivery follows send_committed / unsent /
    send_cancelled receipts. HTTP may resolve this request at admission;
    resolving the promise alone must not release the retained draft. */
export const chatSay = (
  id: string, lease: string,
  text: string,
  images: [string, string][] = [],
  files: [string, string][] = [],
  token?: string,
) =>
  call<null>("chat_say", {
    id, lease,
    text,
    ...(images.length ? { images } : {}),
    ...(files.length ? { files } : {}),
    ...(token === undefined ? {} : { token }),
  });
/** A file for an agent conversation (round CHAT), put on the machine the
    conversation runs on, in that machine's chat files folder
    (`khor_gui_core::chat::chat_put`). Answers its path **there** — what
    the agent is told. The bytes ride raw: the bridge's and the web
    face's body route, the app's raw invoke body. `into`: a directory
    on that machine the person chose; `null` is its receive location. */
export const chatPut = async (id: string, lease: string, file: File, into: string | null = null): Promise<string> => {
  if (inApp && !bridge) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("chat_put", new Uint8Array(await file.arrayBuffer()), {
      headers: {
        "x-khor-chat": encodeURIComponent(id),
        "x-khor-lease": encodeURIComponent(lease),
        "x-khor-name": encodeURIComponent(file.name),
        ...(into ? { "x-khor-into": encodeURIComponent(into) } : {}),
      },
    });
  }
  const q = `id=${encodeURIComponent(id)}&lease=${encodeURIComponent(lease)}&name=${encodeURIComponent(file.name)}${into ? `&into=${encodeURIComponent(into)}` : ""}`;
  const url = bridge ? `http://127.0.0.1:${bridge}/chat_put?${q}` : `/api/chat_put?${q}`;
  const r = await reach(url, {
    method: "POST",
    headers: bridge || webKey === null ? {} : { authorization: `Bearer ${webKey}` },
    body: file,
  });
  if (!r.ok) throw new Error(unwrap(await r.text()));
  return answer<string>(r);
};
/** Files this machine already has (a drop's paths, the app only), put
    where the conversation's machine keeps them (or in `into` there).
    Answers their paths there, in order. */
export const chatPutPaths = (id: string, lease: string, paths: string[], into: string | null = null) =>
  call<string[]>("chat_put_paths", { id, lease, paths, into });
export const chatAnswer = (id: string, lease: string, ask: number, option: string | null) =>
  call<null>("chat_answer", { id, lease, ask, option });
export const chatReplay = (id: string, lease: string) => call<null>("chat_replay", { id, lease });
export const chatLeave = (id: string, lease: string) => call<null>("chat_leave", { id, lease });
export const fetchSessionContent = (id: string, locator: string) =>
  call<SessionContent>("session_content", { id, locator });
export const beginHistoryRequest = (id: string, binding: string) => call<string>("history_begin", { id, binding });
export const cancelHistoryRequest = (id: string, binding: string, requestKey: string) =>
  call<null>("history_cancel", { id, binding, requestKey });
export const fetchHistoryRecord = (id: string, binding: string, requestKey: string, cursor: string, from: number) =>
  call<HistoryRecordChunk>("history_record", { id, binding, requestKey, cursor, from });
export const fetchHistoryPage = (id: string, binding: string, requestKey: string, cursor: string | null = null, newer = false) =>
  call<HistoryPage>("history_page", { id, binding, cursor, newer, requestKey });

/** One machine's directory — order, cap and the answered-about path
    all come from the node; `""` asks for that machine's home. */
export const fetchLs = (machine: string, path: string) =>
  call<DirListing>("ls", { machine, path });
/** Begin resolves the machine once; subsequent calls use its pinned DeviceId. */
export const dirsBegin = (machine: string, scopes: DirWatchScope[]) =>
  call<DirWatchOpened>("dirs_begin", { machine, scopes });
export const dirsWait = (machine_id: string, id: string, cursor: string | null, waitMs: number) =>
  call<DirWatchBatch>("dirs_wait", { machine_id, id, cursor, wait_ms: waitMs });
export const dirsLeave = (machine_id: string, id: string) =>
  call<null>("dirs_leave", { machine_id, id });
/** Takes a file into this machine's downloads; answers where it
    landed — the place was chosen silently, so it must be said. */
export const pullFile = (machine: string, path: string) =>
  call<string>("pull", { machine, path });
/** The same file in a temporary folder on this machine, for handing to
    its other apps (`khor_gui_core::files::pull_temp`); a path already
    here comes back as it is. */
export const pullTemp = (machine: string, path: string) => call<string>("pull_temp", { machine, path });
export type { FileHandoff } from "./gen/bindings/FileHandoff";
import type { FileHandoff } from "./gen/bindings/FileHandoff";
export const fileHandoffSupported = () => inApp ? call<boolean>("file_handoff_supported") : Promise.resolve(false);
export const fileHandoffStart = (id:string,machine:string,path:string) => call<FileHandoff>("file_handoff_start",{id,machine,path});
export const fileHandoffState = (id:string) => call<FileHandoff>("file_handoff_state",{id});
export const fileHandoffCancel = (id:string) => call<null>("file_handoff_cancel",{id});
export const fileHandoffOpen = (id:string) => call<boolean>("file_handoff_open",{id});
export const filePreviewSupported = () => inApp ? call<boolean>("file_preview_supported") : Promise.resolve(false);
export const filePreviewOpen = (id:string) => call<boolean>("file_preview_open",{id});


/** Takes a path off whichever machine holds it (`khor_node::Link::
    remove_path`). `machine` empty is this one.

    **This machine's goes to the recycle bin; every other machine's is
    deleted.** Which of the two is not a choice made here — the node
    decides it by whose disk the path is on, and `fetchTrashHere` says
    whether this machine has a bin at all, so the sentence a person
    confirms is never a guess about the platform. */
export const removePath = (machine: string, path: string) =>
  call<null>("remove", { machine, path });
/** One new directory on whichever machine holds the parent
    (`khor_node::Link::mkdir`). **One level, and a name already taken is
    refused** — the caller is a person typing into a row of a tree whose
    parent is on the screen, so a chain of directories nobody asked for
    is the mistake, not the convenience. */
export const makeDir = (machine: string, path: string) =>
  call<null>("mkdir", { machine, path });
/** One new empty file (`khor_node::Link::touch`). A name already taken
    is refused **and the file wearing it is not emptied**, which is the
    half unix `touch` would have got wrong. */
export const makeFile = (machine: string, path: string) =>
  call<null>("touch", { machine, path });
/** Moves a name (`khor_node::Link::rename_path`). Crossing directories
    is the same verb. **The target is never overwritten**: a taken name
    is refused, there is no force. */
export const renamePath = (machine: string, from: string, to: string) =>
  call<null>("rename_path", { machine, from, to });
/** Opens a path or a page **on the machine that holds it**
    (`khor_node::Link::launch`).

    **Not `openLink`**, and the pair is the whole point. `openLink` hands
    a page to the machine this window is running on, which is right for
    something a person wants to read. This hands it to the machine the
    session ran on — the only machine the file named in that session
    exists on, and this face can be a browser on a phone. 都在宿主机器上
    打开 (the user, 2026-09-17).

    Only an absolute path over there or an `http(s)` address; the node
    refuses everything else by name. */
export const launchOn = (machine: string, target: string) =>
  call<null>("launch", { machine, target });
/** Everything under `dir` on that machine that holds `query` — in a
    name, or in a line (`khor_node::files::search`, 工作区第六轮 第 47
    条).

    **The walk happens over there.** Two lists come back rather than one
    ranked one, because 「that is the file I meant」 and 「that is the
    place I meant」 are two answers; `more` says a cap cut it, the rule
    every capped answer in khor follows.

    Case-insensitive, a plain substring, never a pattern. `.gitignore`
    is honoured on the machine's own road. */
export const searchIn = (machine: string, dir: string, query: string) =>
  call<Found>("search", { machine, dir, query });
/** A machine's commands, searched (roadmap 2.6): words in the line, the
    directory it ran in or under (empty: anywhere), only the failed. Asked
    of the machine that ran them; newest first. */
export const fetchCommandHistory = (machine: string, text: string, dir: string, failed: boolean) =>
  call<HistoryRow[]>("term_history", { machine, text, dir, failed });
/** The prompt before (`up`) or after the one in view in a tmux row's
    front pane (item 21): tmux holds that screen, so tmux moves. */
export const termPrompt = (id: string, lease: string, up: boolean) => call<null>("term_prompt", { id, lease, up });
/** The wheel over a shell's own tmux client (item 58): `rows` up when
    positive; false when no tmux client is in front (0 rows only asks). */
export const termScroll = (id: string, lease: string, rows: number) => call<boolean>("term_scroll", { id, lease, rows });
/** ⌘K kept by this attachment's feed and, on a host that speaks version
    2, by its ring; `redraw` sends the shell its ⌃L after. True when the
    host kept it (audit/term-attach.md). */
export const termForget = (id: string, lease: string, redraw: boolean) => call<boolean>("term_forget", { id, lease, redraw });
/** A tmux row's front pane history from `since` on, or all of it
    (item 31): what the phone scrolls instead of tmux's copy mode. */
export const termPaneHistory = (id: string, lease: string, since: PaneAnchor | null) =>
  call<PaneHistory>("term_pane_history", { id, lease, since });
/** Whether the machine khor's backend runs on has a recycle bin. Asked
    rather than read off the browser: the same page is served to a phone
    looking at a desktop's face (`khor_gui_core::files::trash_here`). */
export const fetchTrashHere = () => call<boolean>("trash_here");
/** Native desktop capability belongs to the viewing app, never to a phone
    browsing a desktop backend. The dev bridge is the local desktop test seam. */
export const fetchRevealHere = async (): Promise<boolean> => {
  const mobile = /Android|iPhone|iPad|iPod/i.test(navigator.userAgent + navigator.platform)
    || (/Mac/i.test(navigator.platform) && navigator.maxTouchPoints > 0);
  return (inApp || bridge !== null) && !mobile
    && /Mac|Win|Linux/i.test(navigator.platform)
    && await call<boolean>("reveal_here");
};
/** Locate a local path in this desktop's system file manager. */
export const reveal = (path: string) => call<null>("reveal", { path });
/** Every pinned directory, machine names already looked up. */
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
    one is opened — the list is places, not trips. `session`: the session
    it was opened from, which puts it in that session's 附栏 slice
    (UX-37 §1); kinds `dir`, `page` and `file`. */
export const recentVisit = (kind: string, machine: string, place: string, session?: string) =>
  call<null>("recent_visit", { kind, machine, place, ...(session ? { session } : {}) });
/** "Stop suggesting this", for every device. Going back there puts it
    back on its own. */
export const forgetRecent = (kind: string, machine: string, place: string) =>
  call<null>("forget_recent", { kind, machine, place });

/** A recent row's immutable identity; it can outlive its machine's name or membership. */
export const forgetRecentByDevice = (kind: string, device: string, place: string) =>
  call<null>("forget_recent", { kind, device, place });
/** Every pinned page, exit-machine names already looked up. */
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

/** Current native history, also read when a retained tab is shown again. */
export const pageSteps = (tab: string) =>
  onWebFace ? Promise.resolve(null) : call<{back:boolean;forward:boolean}|null>("page_steps", { tab });

/** Back, forward, reload, retry, stop and certificate continuation. */
export const pageStep = (tab: string, way: "back" | "forward" | "reload" | "retry" | "stop" | "pass") =>
  onWebFace ? Promise.resolve(null) : call<null>("page_step", { tab, way });

/** **The browser's deck, drawn natively over an iPhone's page** (UX-13,
    `lib/native-deck`): the deck as the face reads it, or `null` to put it
    away. Nothing anywhere else: a Mac's page sits beside its DOM deck. */
/** **What is behind the folded capsule** (UX-29): asks the phone's page
    for its ground `y` up from the page view's foot; it answers as
    `page:ground` (`r,g,b` or a `theme-color`, or nothing). */
export const pageGround = (tab: string, y: number) => (onWebFace ? Promise.resolve(null) : call<null>("page_ground", { tab, y }));

export const pageSheet = (tab:string, options:{height:number;full:number}|null) =>
  onWebFace ? Promise.resolve(null) : call<null>("page_sheet",{tab,options});

export const pageDeck = (tab: string, drawing: import("@/lib/native-deck").DeckDrawing | null) =>
  onWebFace ? Promise.resolve(null) : call<null>("page_deck", { tab, drawing });

/**
 * What a page said about itself, addressed to one tab.
 *
 * **Only in the app.** The events come from a webview only the app has;
 * the other two roads have no page and nothing to hear, so this hands
 * back a cleanup that undoes nothing rather than pretending to listen.
 */
export function onPageWord(
  event: "page:sheet" | "page:went" | "page:title" | "page:load" | "page:popup" | "page:steps" | "page:scroll" | "page:deck" | "page:fail" | "page:ground" | "page:download",
  tab: string,
  said: (text: string) => void,
  onReady?: () => void,
): () => void {
  if (!inApp) return () => {};
  return listenNativeEvent<{ tab: string; text: string }>(event, payload => {
    if (payload.tab === tab) said(payload.text);
  }, { onReady });
}

// ── Trusted native menu presenters (desktop window / iOS sibling WK) ──

/** Where an overlay has to appear, in this page's own CSS pixels, with
    the viewport it was measured in. Desktop converts this to screen
    coordinates; iOS converts it to sibling UIKit points. */
export type Spot = { x: number; y: number; w: number; view: number; across: number };

/**
 * Presents the shared menu above a native webpage. The command receives
 * `{request: {id, at, rows, html?}}`; native verifies the sending WebView.
 *
 * **Answers whether the float took it**, and `false` is not a failure: a
 * platform with no native presenter draws it in the page. Ordinary web
 * browsers and Android currently have no independent embedded page layer.
 * A presenter refusal also uses that DOM fallback (`useFloatedMenu`).
 */
export type NativeMenuState = { id: string; rows: MenuRows };
export type MenuRequest = NativeMenuState & { at: Spot & { above: boolean }; html?: string };
const nativeMenus = nativeMenuQueue<MenuRequest>({
  show: request => call<boolean>("show_menu", { request }),
  hide: id => call<null>("hide_menu", { id }),
});
export const showMenu = (request: MenuRequest): Promise<boolean> =>
  inApp ? nativeMenus.show(request) : Promise.resolve(false);

/** Retires only this lease. Desktop reuses its hidden window; iOS removes
    its private sibling presenter. Stale owners cannot close a newer menu. */
export const hideMenu = (id: string) => (inApp ? nativeMenus.hide(id) : Promise.resolve(null));

/** The columns that ask the window for their room: the session's 附栏
    (`App`) and the workspace's sessions (`WorkspacePane`). Each one's
    room is its own account on the app side. */
export type RoomColumn = "session-side" | "ws-sessions";

/**
 * Grows the app's window for `column`, `w` CSS pixels wide — rightward,
 * and leftward for what the screen's right edge cannot give.
 *
 * 「右上角的icon,都是点击后,在最右侧,拓宽窗口, 而不是挤占现有窗口」—
 * the user, 2026-09-16. A column that really is a column asks the window
 * for its room instead of taking it off the file beside it.
 *
 * **The window keeps the account, not the face** (UX-46, the owner
 * 2026-10-07: 「本来的窗口还变了」): it notes its frame at the press, and
 * `giveBackWindow` puts that frame back exactly; a window the person only
 * moved meanwhile gets its pressed size back where it stands, and one
 * they resized keeps their size (the lead's ruling, 2026-10-07).
 * ⚠️ 墓碑: `widenWindow(delta)`, `+w` on the press and `-w` on the close,
 * which came back wrong whenever the two did not meet (`src-tauri/src/
 * room.rs` has the three ways).
 *
 * **Answers the width the window grew by, once the page has it** (0 when
 * it did not grow: fullscreen, maximised, no room at either edge, or
 * outside the app). The caller holds its layout still until then, so
 * nothing on the glass moves while the window does (UX-46 §3).
 *
 * **A no-op everywhere but the app, and that is the honest answer
 * rather than a missing feature.** A browser window is not the page's
 * to resize — the web face and the verification bridge both run inside
 * one somebody else placed — so this asks nothing at all there rather
 * than asking and being refused. `check-face` asserts the silence.
 *
 * **CSS pixels, which are tauri's logical pixels.** The app side scales
 * them by the window's own factor; a number measured on the glass must
 * not be sent as a physical one.
 */
/** `least` (sb-a, 2026-10-09): the narrowest the column takes before the
    window moves left for it — where the right edge has at least that,
    the answer is what the right edge had and the window stays put. */
export const widenWindow = (column: RoomColumn, w: number, least?: number) =>
  roomAsk("widen", { column, w: Math.round(w), ...(least !== undefined ? { least: Math.round(least) } : {}) });
/** What `widenWindow` took for `column`, given back; answers the width
    given back, once the page has it. */
export const giveBackWindow = (column: RoomColumn) => roomAsk("unwiden", { column });

async function roomAsk(cmd: "widen" | "unwiden", args: Record<string, unknown>): Promise<number> {
  if (!inApp) return 0;
  const from = window.innerWidth;
  // A refused ask took no room: the column takes its room off the page.
  const px = (await call<number | null>(cmd, args).catch(() => 0)) ?? 0;
  if (px < 1) return 0;
  // The window answers when it has its new frame; the page hears its new
  // width a moment later. Waited for (with a ceiling, a resize that never
  // arrives must not hold a column back), then one frame for the layout.
  await new Promise<void>((done) => {
    const end = () => {
      window.removeEventListener("resize", heard);
      clearTimeout(cap);
      requestAnimationFrame(() => done());
    };
    const heard = () => window.innerWidth !== from && end();
    const cap = setTimeout(end, 400);
    window.addEventListener("resize", heard);
    heard();
  });
  return px;
}

/** How tall the list came out, said by the window that drew it — the only
    side that can measure a laid-out list. Showing it is the app's answer
    to this call. */
export const sizeMenu = (id: string, h: number) =>
  inApp ? call<null>("size_menu", { id, h }) : Promise.resolve(null);

/** What the menu has to show right now. **Pulled on mount** because the
    window and its first rows are decided in the same breath, and a
    webview still loading cannot be listening yet. */
export const menuRows = () =>
  inApp ? call<NativeMenuState | null>("menu_rows", {}) : Promise.resolve(null);

/** A row was pressed. Goes back to the face that opened the menu. */
export const menuPick = (id: string, key: string) =>
  inApp ? call<null>("menu_pick", { id, key }) : Promise.resolve(null);

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
  return listenNativeEvent(event, said);
}
/**
 * The app's window is being closed, and something wants a say
 * (工作区第五轮 第 39 条).
 *
 * **`ask` answers whether to stop it.** True means the window stays and
 * whoever answered is expected to put something on screen; false means
 * it closes now.
 *
 * **Tauri prevents the close for us the moment this listener exists** —
 * the Rust side sees a JS listener on `tauri://close-requested` and
 * calls `api.prevent_close()`, and the window then only goes when
 * something calls `destroy` ([`quitNow`]). So the shape here is: always
 * prevent, and close by hand when nobody objected. That is also why
 * this must never be installed conditionally — a listener that comes
 * and goes is a window that sometimes cannot be closed at all.
 *
 * A no-op outside the app, where there is no window of its own to
 * close: the browser's own road is `beforeunload`, which is a different
 * contract (a browser will not let a page put a dialog of its own up)
 * and is wired where it is spent.
 *
 * Loud on refusal, for `hear`'s reason.
 */
export function onQuitRequested(ask: () => boolean): () => void {
  if (!inApp) return () => {};
  let drop: (() => void) | undefined;
  let question: typeof ask | null = ask;
  let gone = false;
  void import("@tauri-apps/api/window")
    .then(({ getCurrentWindow, CloseRequestedEvent }) => {
      if (gone) return;
      const win = getCurrentWindow();
      drop = listenNativeEvent<null>("tauri://close-requested", async (_, event, isActive) => {
        // Match Window.onCloseRequested: await the handler, then destroy
        // only when it did not prevent the close and this owner still lives.
        const requested = new CloseRequestedEvent(event);
        await (() => { if (question?.()) requested.preventDefault(); })();
        if (isActive() && !requested.isPreventDefault()) await win.destroy();
      }, { target: { kind: "Window", label: win.label } });
    })
    .catch((error: unknown) => console.error("khor: cannot hear the window closing", error));
  return () => {
    gone = true;
    question = null;
    drop?.();
  };
}

/** Closes the app's window for good, past every guard — what 「不存,
    退出」 presses. `destroy` rather than `close`, because `close` asks
    again and the answer would be the same guard that is on screen. */
export function quitNow(): void {
  if (!inApp) return;
  void import("@tauri-apps/api/window")
    .then(({ getCurrentWindow }) => getCurrentWindow().destroy())
    .catch((e: unknown) => {
      console.error("khor: cannot close the window", e);
    });
}

/**
 * Whether this face has the window's own three buttons floating over
 * its top-left corner (工作区第八轮 第 58 条).
 *
 * **macOS and the app, and neither half is negotiable.**
 * `titleBarStyle: Overlay` is a macOS-only key in `tauri.conf.json` —
 * Linux and Windows read it and keep their system title bar, so there
 * is nothing over their corner to leave room for. The web face has no
 * window of its own at all.
 *
 * `navigator.platform` is deprecated and still the only thing that
 * answers this without asking the backend: the user agent of a
 * WKWebView on macOS says Macintosh too, so the fallback agrees with
 * the reading rather than widening it. A wrong answer here costs 28
 * blank pixels at the head of the rail, not a broken face — which is
 * why a guess is allowed to stand in for a round trip.
 */
export const trafficLights =
  inApp && /mac/i.test(navigator.platform || navigator.userAgent);

/**
 * Whether the window is full screen, now and whenever that changes.
 *
 * **The window's, not the page's.** macOS full screen hands the window
 * to its own space and hides the three buttons with the title bar; the
 * Fullscreen API — the one `:fullscreen` and `document.fullscreenElement`
 * answer for — is a different thing that nothing in this app ever asks
 * for. So the answer has to come from tauri.
 *
 * **On the resize, not on a timer.** Entering and leaving full screen
 * resizes the window, and this repo's rule is that a timer has to have
 * a reason (第七轮 第 57 条 is the one that does). `onResized` fires
 * more often than the answer changes, which is the harmless direction:
 * the caller sets a boolean it already holds.
 *
 * Silent outside the app: there is no window to ask, and `false` is the
 * truthful answer for a page in somebody's browser tab.
 */
export function onFullscreen(tell: (full: boolean) => void): () => void {
  if (!inApp) return () => {};
  let drop: (() => void) | undefined;
  let receive: typeof tell | null = tell;
  let gone = false;
  void import("@tauri-apps/api/window")
    .then(({ getCurrentWindow }) => {
      if (gone) return;
      const win = getCurrentWindow();
      const say = () => {
        if (gone) return;
        void win.isFullscreen()
          .then(full => { if (!gone) receive?.(full); })
          .catch((error: unknown) => console.error("khor: cannot ask the window about full screen", error));
      };
      say();
      drop = listenNativeEvent("tauri://resize", say, { target: { kind: "Window", label: win.label } });
    })
    .catch((error: unknown) => console.error("khor: cannot watch the window's size", error));
  return () => {
    gone = true;
    receive = null;
    drop?.();
  };
}

/** Attaches a terminal to a hosted session at cols×rows; the host
    resizes its PTY to match, and keeps `history` rows of it (the face's
    历史行数, item 56; capped by the host). */
export const termOpen = (id: string, lease: string, cols: number, rows: number, history: number) =>
  call<null>("term_open", { id, lease, cols, rows, history });
/** Everything the feed holds so far (it began with the host's ring, so
    history rides along) and the cursor to stream from. The pane writes
    it into xterm and then follows with `termStream`.

    (The cell-snapshot road — `term_poll` and the `TermScreen` family,
    with `term_wheel`'s mouse judgment and `term_paste`'s bracketing —
    left with the 2026-09-03 pivot: xterm.js consumes bytes and speaks
    its own mouse and paste. 「不必考虑旧版兼容」 is why there is no
    bridge.) */
export const termAttach = (id: string, lease: string) => call<TermStart>("term_attach", { id, lease });
/** The initial replay is parsed; live terminal queries can now be answered. */
export const termReady = (id: string, lease: string) => call<null>("term_ready", { id, lease });
/** **A dead terminal row's page** (P5-SHELL): how it ended, its last
    screen when its host left one (`ring.bin`), else the commands it ran,
    newest first. `null` while the session is not dead on this machine. */
export const termEnded = (id: string) => call<EndedPage | null>("term_ended", { id });
export const termStream = (id: string, lease: string, since: number) =>
  call<TermChunk>("term_stream", { id, lease, since });
/** `termStream`, held open until the feed grows past `since` (or `ms`
    passes): output arrives the moment it is there instead of on the next
    beat. The app's only — the bridge and the web face answer one request
    at a time, so there it is the stream's answer as it stands. */
export const termWait = (id: string, lease: string, since: number, ms: number) =>
  call<TermChunk>("term_wait", { id, lease, since, ms });
/** Keystrokes as the bytes a terminal sends — xterm encodes every key,
    modifier, mouse report and paste; the backend stays a pipe. */
export const termKey = (id: string, lease: string, keys: string) => call<null>("term_key", { id, lease, keys });
/** A line from the face into the app's own log (its stderr, which the
    device's console keeps): what a page saw that only a device shows. */
export const faceNote = (line: string) => call<null>("face_note", { line });
/** **khor's items in the system's edit menu** (item 54,
    `src-tauri/src/edit_menu.rs`): `[id, title]` pairs the next menu over
    a selection adds; none clears them. Only the phone app has the menu. */
export const editMenu = (items: [string, string][]) => uikit("edit_menu", { items });
/** **The system's edit menu at a selection khor drew** (item 73): with
    the keyboard up the terminal selects in its own cells, and the menu is
    shown at `rect` (CSS px from the document's origin) with `items`;
    `copy` is what its 拷贝 writes to the pasteboard, natively. */
export const editMenuAt = (rect: { x: number; y: number; w: number; h: number }, items: [string, string][], copy: string) =>
  uikit("edit_menu_at", { ...rect, items, copy });
/** Puts `editMenuAt`'s menu away. */
export const editMenuHide = () => uikit("edit_menu_hide", {});
/** A call that is UIKit's, in this app, whatever node answers the rest:
    the dev bridge beside the app (a simulator walk) does not have it. */
async function uikit(cmd: string, args: Record<string, unknown>): Promise<null> {
  if (!inApp) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<null>(cmd, args);
}
/** A bell felt: a light haptic tap on a phone (roadmap 4.2). */
export const bellHaptic = () => call<null>("bell_haptic");
/** The cursor joystick, felt (item 23, `joy_haptic`): `prepare` at
    touch-down, a `tick` per arrow sent, a `turn` when the way changes,
    `end` when the finger lifts. A phone's only; nothing elsewhere. */
export const joyHaptic = (kind: "prepare" | "tick" | "turn" | "end") => call<null>("joy_haptic", { kind });
/** **A haptic with a meaning** (UX-10; ux-9 B5): the system's own
    patterns, played on a phone — `selection` when a picked value moves,
    `success` when a task completes, `warning` when the act was refused,
    `threshold` when a drag crosses the line where letting go acts (a
    light impact, UX-49). The app's only; nothing for a browser to reach
    (`lib/haptics`). */
export const haptic = (kind: "selection" | "success" | "warning" | "threshold") => call<null>("haptic", { kind });
/** Opens the settings window, or brings it forward on `pane` (UX-10). */
export const openSettings = (pane?: string, machine?: string) => call<null>("open_settings", { pane: pane ?? null, machine: machine ?? null });
/** The settings window's title, its pane's name (HIG Settings). */
export const nameSettings = (title: string) => call<null>("name_settings", { title });
/** A finished command, said by the system (roadmap 2.4, `lib/finished`). */
export const notifyFinished = (title: string, body: string) => call<null>("notify_finished", { title, body });
/** Files dropped on a terminal. The paths go over as they are and the
    quoting happens in the registry — shell syntax is not something a
    face should be assembling, and one wrong escape turns a filename
    into a command (`khor_gui_core::term::term_drop`). `into`: the
    shell's current directory, where 「放到当前目录」 lands them. */
export const termDrop = (id: string, lease: string, paths: string[], from?: string | null, into?: string | null) =>
  call<DropEnd>("term_drop", { id, lease, paths, from: from ?? null, into: into ?? null });
/** **The drop under way on session `id`, cancelled** (the owner
    2026-10-07 12:50, item 71): the pull stops, its part file goes,
    nothing is pasted; with no drop under way, nothing happens.
    `staged`: files `termDropStage` put up for a drop that will now
    never be asked for, which go at once. */
export const termDropCancel = (id: string, lease: string, staged: string[] = []) => call<null>("term_drop_cancel", { id, lease, staged });
/** A file picked in a face (the phone's photos and files, item 60 part
    3), put where this machine's node serves it by path, for `termDrop`
    with `from` null. Answers its path here. The bytes ride raw, as
    `chatPut`'s do. */
export const termDropStage = (file: File): Promise<string> => stageBytes("term_drop_stage", { name: file.name }, file);
export const termDropStageInto = (folder: string, path: string, file: File): Promise<string> => stageBytes("term_drop_stage_into", { folder, path }, file);
async function stageBytes(command: string, fields: Record<string, string>, file: File): Promise<string> {
  if (inApp && !bridge) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>(command, new Uint8Array(await file.arrayBuffer()), {
      headers: Object.fromEntries(Object.entries(fields).map(([k, v]) => [`x-khor-${k}`, encodeURIComponent(v)])),
    });
  }
  const q = new URLSearchParams(fields);
  const url = bridge ? `http://127.0.0.1:${bridge}/${command}?${q}` : `/api/${command}?${q}`;
  const r = await reach(url, {
    method: "POST",
    headers: bridge || webKey === null ? {} : { authorization: `Bearer ${webKey}` },
    body: file,
  });
  if (!r.ok) throw new Error(unwrap(await r.text()));
  return answer<string>(r);
}
export const termDropItems = (id: string, lease: string, items: DropItem[], into: string | null) => call<DropEnd>("term_drop_items", { id, lease, items, into });
export const termDropFolder = (name: string, dirs: string[]) => call<string>("term_drop_folder", { name, dirs });
/** Releases only app-owned selection tokens, without cancelling a different transfer. */
export const termDropForget = (paths: string[]) => call<null>("term_drop_forget", { paths });
export const nativeFolders = () => inApp && !bridge && !/Android/i.test(navigator.userAgent);
export const termPickFolders = () => call<string[]>("term_pick_folders");
/** How far this terminal's drop has got, 0 to 1, or `null` when none is
    under way (item 60): what its pill's line draws. */
export const termDropProgress = (id: string, lease: string) => call<number | null>("term_drop_progress", { id, lease });
export const termResize = (id: string, lease: string, cols: number, rows: number) =>
  call<null>("term_resize", { id, lease, cols, rows });
export const termLeave = (id: string, lease: string) => call<null>("term_leave", { id, lease });

/** What the Mac app's menu bar should say applies right now
    (`src/lib/shortcuts.ts` → `src-tauri/src/menu.rs`): whether ⌘W has
    something to close, whether there are tabs to step through, the
    landing's 「+」 by its own name, whether ⌘L can lock, and whether the
    face is locked. The app alone — no other face has a menu bar. */
export type MenuState = {
  close: boolean;
  step: boolean;
  new: string | null;
  lock: boolean;
  locked: boolean;
  /** The strip's landings in its order (`nav-order.ts`). */
  order: string[];
  /** `null` where there is no list to fold; else whether it is folded. */
  list: boolean | null;
};
export const menuState = (state: MenuState) =>
  inApp ? call<null>("menu_state", { state }) : Promise.resolve(null);

export const agentRecognition = (machine: string, action: "read" | "enable" | "disable" = "read", agent?: string) =>
  call<RecognitionEntry[]>("agent_recognition", { machine, action, agent });


const attachmentTransport = {
  begin: attachmentBegin,
  leave: async (id: string, kind: AttachmentKind, lease: string): Promise<void> => {
    await (kind === "chat" ? chatLeave(id, lease) : termLeave(id, lease));
  },
};

export function chatAttachment(id: string) {
  const owner = attachmentView<boolean>(id, "chat", attachmentTransport);
  return {
    id,
    open: () => owner.open(lease => chatOpen(id, lease)),
    poll: (since: number) => owner.run(lease => chatPoll(id, lease, since)),
    wait: (since: number, ms: number) => owner.run(lease => chatWait(id, lease, since, ms)),
    say: (text: string, images: [string, string][], files: [string, string][], token?: string) =>
      owner.run(lease => chatSay(id, lease, text, images, files, token)),
    answer: (ask: number, option: string | null) => owner.run(lease => chatAnswer(id, lease, ask, option)),
    respond: (ask: number, response: unknown) => owner.run(lease => chatRespond(id, lease, ask, response)),
    control: (token: string, operation: string, payload: Record<string, unknown>) =>
      owner.run(lease => chatControl(id, lease, token, operation, payload)),
    setMode: (mode: string) => owner.run(lease => chatSetMode(id, lease, mode)),
    setOption: (option: string, value: string | boolean) => owner.run(lease => chatSetOption(id, lease, option, value)),
    replay: () => owner.run(lease => chatReplay(id, lease)),
    stop: () => owner.run(lease => chatStop(id, lease)),
    put: (file: File, into: string | null) => owner.run(lease => chatPut(id, lease, file, into)),
    putPaths: (paths: string[], into: string | null) => owner.run(lease => chatPutPaths(id, lease, paths, into)),
    dispose: owner.dispose,
  };
}

export function terminalAttachment(id: string) {
  const owner = attachmentView<null>(id, "terminal", attachmentTransport);
  return {
    id,
    open: (cols: number, rows: number, history: number) => owner.open(lease => termOpen(id, lease, cols, rows, history)),
    attach: () => owner.run(lease => termAttach(id, lease)),
    ready: () => owner.run(lease => termReady(id, lease)),
    stream: (since: number) => owner.run(lease => termStream(id, lease, since)),
    wait: (since: number, ms: number) => owner.run(lease => termWait(id, lease, since, ms)),
    key: (keys: string) => owner.run(lease => termKey(id, lease, keys)),
    resize: (cols: number, rows: number) => owner.run(lease => termResize(id, lease, cols, rows)),
    forget: (redraw: boolean) => owner.run(lease => termForget(id, lease, redraw)),
    prompt: (up: boolean) => owner.run(lease => termPrompt(id, lease, up)),
    scroll: (rows: number) => owner.run(lease => termScroll(id, lease, rows)),
    paneHistory: (since: PaneAnchor | null) => owner.run(lease => termPaneHistory(id, lease, since)),
    drop: (paths: string[], from?: string | null, into?: string | null) => owner.run(lease => termDrop(id, lease, paths, from, into)),
    dropItems: (items: DropItem[], into: string | null) => owner.run(lease => termDropItems(id, lease, items, into)),
    dropCancel: (staged: string[] = []) => owner.run(lease => termDropCancel(id, lease, staged)),
    dropProgress: () => owner.run(lease => termDropProgress(id, lease)),
    dispose: owner.dispose,
  };
}
