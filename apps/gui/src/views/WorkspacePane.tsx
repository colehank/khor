import {SessionFrame} from "@/components/SessionFrame";
import {SessionContentColumn,type SessionContentContext} from "@/views/SessionContentColumn";
import {SessionContentSeat} from "@/hooks/use-session-content-dock";
import {SessionLinkScope} from "@/views/SessionLinkScope";
import {setSessionSide,useSessionSide} from "@/hooks/use-session-side";
import {MobileContentSheet} from "@/views/MobileSessionContent";
import {useSessionTabs} from "@/hooks/use-session-tabs";
import {delegatedSessionId,rememberDelegatedSession,useSessionRowLookup} from "@/lib/delegated-sessions";
import {termName} from "@/components/DoingLine";
import { WorkspaceNavigation } from '@/components/WorkspaceNavigation';
import { useFileHandoff } from "@/hooks/use-file-handoff";
import { sessionCwdOnScreen, sessionInWorkspace } from "@/lib/session-cwd";
import type { EditorHistorySlot } from "@/lib/code/history";
import { useCallback, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { flushSync } from "react-dom";

import {
  fetchGitStatus,
  fetchGitHistory,
  fetchCheckout,

  searchIn,
  type Found,
  type SearchHit,
  type GitStatus,
  type SessionRow,
  type DeviceRow,
} from "@/api";
import { WorkspaceHistory } from "@/views/workspace/WorkspaceHistory";
import { CloseDraftDialog } from "@/components/CloseDraftDialog";
import { CodeButton, useCodeWindow } from "@/components/CodeWindow";
import { FileKindIcon } from "@/components/FileKindIcon";
import { IconBack, IconBranch, IconHistory, IconMore, IconEye } from "@/components/icons";
import { GlassCapsule, GlassCircle } from "@/components/Glass";
import { DeeperHeads, NarrowHead, SideSeat } from "@/components/SideHead";
import { ListPane } from "@/components/ListPane";
import { PaneBar, type ArrangeOption } from "@/components/PaneBar";
import { PathCrumbs } from "@/components/PathCrumbs";
import { RegionLead, WindowControls } from "@/components/Region";
import { WINDOW_DRAG } from "@/components/shell";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { stalestTab, useTabsLimit } from "@/browse/tab-limit";
import { type TabFace } from "@/browse/TabStrip";
import { fileKind } from "@/components/SessionFace";
import type { GitCommitSummary } from "@/gen/bindings/GitCommitSummary";
import type { Avatar } from "@/gen/bindings/Avatar";
import { WorkspaceContentFrame } from "@/views/workspace/WorkspaceContentFrame";
import { cli, gui } from "@/gen/catalog";
import { useHidden } from "@/hooks/use-hidden";
import { useDirectoryWatch } from "@/hooks/use-directory-watch";
import { directoryScopeIsNative, directoryWatchScopes } from "@/lib/directory-watch";
import { roomBeside, useListWidth, type PaneWidth } from "@/hooks/use-list-width";
import type { WorkspaceMemory } from "@/views/workspace/workspace-memory";
import { useWorkspaceField } from "@/views/workspace/use-workspace-memory";
import { requestFileLocation } from "@/lib/file-location";
import { directoryOf } from "@/lib/paths";
import { movedFilePath, useFileObjectActions } from "@/lib/file-actions";
import { discard, flushFile, roadOf, useRoads } from "@/lib/save-road";
import { useReadFirst } from "@/hooks/use-read-first";
import { useTabHost } from "@/lib/shortcuts";
import {
  rememberWords,
  sessionAxes,
  sessionOptions,
  storedWords,
} from "@/lib/session-filter";
import { holds, onTreeChanged, sameMachine } from "@/lib/tree-changed";
import { cn } from "@/lib/utils";
import { band } from "@/lib/scroll-edge";
import { basename } from "@/words";
import { DetailPane } from "@/views/DetailPane";
import { SessionsList } from "@/views/SessionsList";
import { FilesPane } from "@/views/FilesPane";
import { FileTree } from "@/views/FileTree";
import { heavier, IGNORED } from "@/views/fileRow";
import { beginEditing, defaultFaceOf, FileView, rendererOf, type Draft } from "@/views/FileView";
import { DiffView, diffLabel } from "@/views/DiffView";
import { RevisionView, revisionRenders, shortId } from "@/views/FileHistory";
import { closeWorkspaceTab, openWorkspaceTab, pinWorkspaceTab, renameWorkspaceFiles, selectWorkspaceTab, selectedWorkspaceTarget, workspaceTabId, type WorkspaceTarget } from "@/views/workspace/workspace-tabs";
import { WorkspaceNavigationStore } from "@/views/workspace/workspace-navigation-storage";
import { CommitPage } from "@/views/CommitPage";
import { LinkHomeProvider, useKnownFiles, type LinkHome } from "@/views/linkHome";
import { FileVerbs, placeAt, topLine } from "@/views/FilePage";
import { useFullScreen } from "@/lib/full-screen";
import { WorkspaceEmpty, rememberOpened, storedOpened } from "@/views/WorkspaceEmpty";

export function WorkspacePane({
  memory,
  machine,
  machineFace,
  here,
  root,
  rows,
  devices = [],
  channel,
  onPin,
  onClose,
  pinFailed,
  closeRefused,
  arrange,
  onNewSession,
  opened,
  onOpenedSeen,
  back = false,
  onBackSeen,
  reveal,
  onRevealSeen,
  narrow,
  side,
  onBack,
  status,
  sessionContent,
}: {
  sessionContent?:SessionContentContext;
  memory: WorkspaceMemory;
  status?: string;
  machine: string;
  /** Whether this workspace is on the machine khor is running on.
      Handed down to the tree, whose 拉过来 is a direction and not an
      action when there is nowhere to pull from (`FilesPane`'s
      `here`). */
  here: boolean;
  /** The workspace's own directory, absolute. Empty means that
      machine's home — what「看它的文件」hands over when the request was
      about a machine and not about a project. */
  root: string;
  /** Every session the shell knows about. Filtered here rather than
      upstream: which of them belong to this project is a fact about
      this pane's subject, and the shell holds the list for its own
      reasons already. */
  rows: SessionRow[];
  devices?: readonly DeviceRow[];
  /** This workspace's machine as the **channel** name — what
      `SessionRow.home` is spelled in — resolved from `device` by the
      shell, which is where the device table lives (工作区批③).
      `null` when the table has no row for that machine.

      **Not `machine`, which is the name on the glass.** That one is
      `DeviceInfo::shown()`: a label if somebody typed one, the channel
      otherwise. Comparing it with `home` would go on matching until
      the day a machine gets renamed and then silently match nothing —
      the rule `App` already states where it decides whether a tree is
      this machine's: compared by channel, never by the name on the
      glass. */
  channel: string | null;
  /** This workspace's machine's face — the base of every file tab's
      seat (UX-32: the base is always the machine, the file's kind is
      the mark). `null` draws the blank. */
  machineFace: Avatar | null;
  /** The row actions, handed down from the shell unchanged (工作区批③).
      **Not a reduced set**: the same verbs on the same rows, wherever
      the rows are being read — a pin that works on one landing and not
      the other is a rule nobody can hold. `App` owns the optimistic
      state behind them, which is why they come from there rather than
      being made here. */
  onPin: (row: SessionRow) => Promise<void>;
  onClose: (row: SessionRow) => Promise<void>;
  pinFailed: Set<string>;
  closeRefused: Map<string, string>;
  /** 排布, and it is **the shell's own** (工作区批⑩). There is one
      `rows` poll in this app and the node does the arranging
      (`khor_node::list::Arrange`), so a column that sorted for itself
      would be re-deriving an order the backend already decided — the
      thing docs/UX.md forbids in so many words. What it costs is said
      out loud: changing it here changes the sessions landing too,
      because it is one setting and not two. */
  arrange: {
    label: string;
    options: readonly ArrangeOption[];
    chosen: string;
    onChoose: (key: string) => void;
  };
  /** 「+」: open the wizard on this machine, in this directory
      (「这就是最早说的『加入我的工作区』」). The shell owns the sheet,
      so it owns this. */
  onNewSession: (machine: string, dir: string) => void;
  /** The session the wizard just opened **from this column**, or `null`
      (2026-09-16 ruling 34:「在工作区新建的 session, 直接弹到了 session
      页面去打开, 应该是在工作区就地打开」— the user).
   *
   * **The shell hands it down rather than the column keeping it**,
   * because the wizard is the shell's sheet: this pane asks for it to
   * open and does not see it close. What arrives is an id, and what
   * this column does with it is open the sessions and mark that row —
   * it is this project's session, made from this project's screen, and
   * a landing change would take somebody away from the files they were
   * standing in. */
  opened: string | null;
  /** …and this column has taken it. **A one-shot, not a value that
      stays true**: without it the id would still be sitting in the
      shell's state the next time any workspace opened, and every one of
      them would come up standing on somebody else's session (measured
      on the narrow face, where it opened the wrong screen). */
  onOpenedSeen: () => void;
  /** **Back from the new tab this column's ＋ opened** (UX-45): the ＋ is
      a new tab now, so this pane was left and is drawn again — with its
      sessions column up, where the person pressed (取消 or 创建 alike).
      A one-shot like `opened`. */
  back?: boolean;
  onBackSeen?: () => void;
  /** A file this shell was asked to show, from outside this pane
      (2026-09-18 ruling 54: a path pressed in a conversation on the
      **sessions** landing, whose workspace is this one). Absolute, with
      the line the sentence named where it named one.

      **Stamped, and a one-shot like `opened` above it**: pressing the
      same path twice is two requests and the second has to move the
      view again, and an unclaimed value would reopen that file the next
      time any workspace mounted. */
  reveal: { path: string; line: number | null; at: number } | null;
  /** …and this column has taken it (`onOpenedSeen`'s reason). */
  onRevealSeen: () => void;
  narrow: boolean;
  /** **The tree is the list column's** (UX-23 §2), wide: `on` while the
      list is drilled into this workspace — the tree is drawn in the
      list's seat with the list's seam (`pane`), under 「‹ 工作区名」, which
      `set(false)` answers — and `null` on the narrow face, where the tree
      is a screen of this pane's. ⚠️ 墓碑: `treePane`, a column of its own
      between the list and the file. */
  side: { on: boolean; pane: PaneWidth; set: (on: boolean) => void } | null;
  /** Out of this workspace, back to the list — the narrow face's first
      screen's way back (the shell's `screen`). */
  onBack?: () => void;
}) {
  // Where the tree is standing. Starts at the workspace and walks down;
  // `root` changing is a different workspace, not a step.
  const regionLead = useContext(RegionLead);
  const navigationStore = useMemo(() => new WorkspaceNavigationStore(workspaceStorage, machine, root, targetOf), [machine, root]);
  const restoredNavigation = useMemo(() => navigationStore.restore(), [navigationStore]);
  const [sidebarView, setSidebarView] = useWorkspaceField(memory, "sidebarView", () => restoredNavigation.historyOpen ? "history" : "files");
  const historyOpen = sidebarView === "history";
  const setHistoryOpen = (open: boolean) => setSidebarView(open ? "history" : "files");
  const [at, setAt] = useWorkspaceField(memory, "at", () => restoredNavigation.at);
  const [content, setContent] = useWorkspaceField(memory, "content", () => restoredNavigation.content);
  const tabs = content.items.filter((target) => target.kind !== "commit").map(keyOf);
  const selected = selectedWorkspaceTarget(content);
  const file = selected === null || selected.kind === "commit" ? null : keyOf(selected);
  const commit = selected?.kind === "commit" ? selected.commit : null;
  const setFile = useCallback((path: string) => {
    setContent((was) => selectWorkspaceTab(was, workspaceTabId(targetOf(path))));
  }, [setContent]);
  const pin = useCallback((id: string) => setContent((was) => pinWorkspaceTab(was, id)), [setContent]);
  // The file view is replaced when tabs or viewport layouts change; its
  // undo stack belongs to the open tab. Release it with the tab, not at
  // autosave, and never keep another workspace's history in this map.
  const commitState = (id: string) => {
    let state = memory.commits.get(id);
    if (!state) memory.commits.set(id, state = { collapsed: new Set(), scroll: {} });
    return state;
  };
  useEffect(() => {
    const open = new Set(content.items.flatMap((item) => item.kind === "commit" ? [item.commit.id] : []));
    for (const id of memory.commits.keys()) if (!open.has(id)) memory.commits.delete(id);
  }, [content, memory]);
  const historyFiles = memory.histories;
  const historyOf = (path: string): EditorHistorySlot => {
    let slot = historyFiles.get(path);
    if (!slot) historyFiles.set(path, slot = {});
    return slot;
  };
  useEffect(() => {
    const open = new Set(tabs);
    for (const path of historyFiles.keys()) if (!open.has(path)) historyFiles.delete(path);
  }, [tabs, historyFiles]);

  /** The files this workspace was last opened on, newest first — what
      the middle column offers when nothing is open (ruling 37). Kept in
      state as well as in storage so the list under an empty column
      changes as tabs are opened and closed, rather than being whatever
      it was when this pane mounted. */
  const [recent, setRecent] = useWorkspaceField(memory, "recent", () => storedOpened(machine, root));
  /** A line a press in a conversation asked for (ruling 35: 行号后缀
      `:12` 认得, 跳到那一行). **Stamped with the moment it was asked**,
      because pressing the same path twice is two requests and the second
      one has to move the view again — the file view compares the stamp,
      not the number. */
  const [goLine, setGoLine] = useState<{ path: string; line: number; at: number } | null>(
    null,
  );
  /** Narrow shows one column at a time; wide ignores this retained choice. */
  const [screen, setScreen] = useWorkspaceField(memory, "screen", () => restoredNavigation.content.active ? "file" : "tree");
  const preview = content.preview;
  /** When each tab was last in front, for 标签页上限 (`browse/tab-limit`).
      A ref: it is read when a tab is opened, and a change to it is not
      something the screen draws. */
  const touched = useRef(new Map<string, number>());
  const limit = useTabsLimit();
  /** 源码 or 预览 for the file in front (2026-09-16).
   *
   * **Held here because the switch is on the tab strip**, whose tail is
   * drawn by this pane — 「取消源码/预览的按钮,放在页面的右上角那,如果
   * 可预览,就是一个预览的 icon,再次点击回到源码」(the user). `FileView`
   * is handed the answer.
   *
   * Every newly opened or selected file starts as source (2026-10-13),
   * a notebook as its preview (`defaultFaceOf`, 2026-10-20). */
  const [face, setFace] = useWorkspaceField(memory, "face", () => (file === null ? "source" : defaultFaceOf(file)));
  /** A revision from the file's git 历史, shown where the file is
      (round UX-4); cleared when another file comes to the front. */
  const [rev, setRev] = useWorkspaceField(memory, "rev", () => null);
  const [revCompare, setRevCompare] = useWorkspaceField(memory, "revCompare", () => false);
  const previousFile = useRef({ machine, file });
  useEffect(() => {
    if (previousFile.current.machine === machine && previousFile.current.file === file) return;
    previousFile.current = { machine, file };
    setRev(null);
    setFace(file === null ? "source" : defaultFaceOf(file));
  }, [machine, file, setRev, setFace]);
  /** The middle column, where the reader's place is read and put back. */
  const mid = useRef<HTMLDivElement>(null);
  const handoff = useFileHandoff(machine, file ?? "", file !== null, mid);
  /** Where to put the reader once the face has changed (a source line). */
  const restore = useRef<number | null>(null);
  /** A ✓ waiting on its save before the face goes back to reading. */
  // ⚠️ 墓碑 (UX-33): `savingBack` — the Mac's 保存 circle saved and went
  // back to the rendered face; the pill's ✓ saves and stays.
  useLayoutEffect(() => {
    const line = restore.current;
    if (line === null) return;
    restore.current = null;
    placeAt(mid.current, line);
  }, [face]);
  /** Which tab is waiting on 存了再关 / 不存,关掉 / 别关, by path. */
  const [closing, setClosing] = useState<string | null>(null);
  /** A tab whose 存了再关 was pressed and whose file is not the one in
      front yet. **The save has to be made by the view that holds the
      buffer** — it is pinned to a digest that view is holding, and a
      second writer with its own idea of that digest is the clash the
      whole save contract exists to avoid — so the tab is brought to the
      front first and asked to save on the beat after. */
  const [wantSave, setWantSave] = useState<string | null>(null);
  /** Bumped to ask the open file's view to save. A counter for the
      reason `gitAt` is one: two asks are two saves. */
  const [saveAt, setSaveAt] = useState(0);
  /** A tab that will close as soon as it has nothing waiting in it.
      **Watched rather than closed on the save's promise**: a save that
      is refused must leave the tab open with the refusal on it, and the
      only thing that knows a save landed is the draft going away.
      `ask`: closed by ×, saving on the way out (save-a) — a save that
      does not land then asks 保存并关闭 / 不保存 / 继续编辑, which is
      the warning on leaving a file with unsaved edits. A close asked
      from that sheet does not ask again: the file shows why. */
  const [waiting, setWaiting] = useState<{ path: string; ask: boolean } | null>(null);
  /** The save roads' version, for the tab's ✓ / 已保存 / ↻ and the
      close that waits on a save. */
  const roads = useRoads();
  /** Every file in this workspace with something typed in it and not
      saved, by absolute path.
      **Held here because the view is remounted on every file.** Until
      2026-09-12 clicking another row in the tree threw the typing away
      with no warning; now walking away and back finds it where it was,
      and the head says how many are waiting. */
  const [drafts, setDrafts] = useWorkspaceField(memory, "drafts", () => new Map());
  /** The open file reads first under a finger, 「编辑」 / 「保存」 on its
      tab (P1, `hooks/use-read-first`). */
  const readFirst = useReadFirst(file, file !== null && drafts.has(file), () => setSaveAt((n) => n + 1));
  /* ⚠️ **Tombstone: `undos` held what each file looked like before the
     last save made from this face** (工作区批4 收尾 → 2026-09-16). It
     was up here rather than in the view for the drafts' reason — the
     view is remounted on every file — and its one reader was the
     「回到存之前」 button on the strip over the file, which the user
     took off:「修改文件不要显示"存好了"和"回到存之前"」. A map filled on
     every save for a control that no longer exists is state nobody can
     see and nobody can spend. (`FileView`'s own tombstone says what is
     given up.) */
  // **Two seams of the workspace's own** (工作区批 收尾, 2026-09-12),
  // the same hook the list pane's seam uses — parameterised rather than
  // copied, so「drag, snap away, come back to the width it had, and the
  // arrow keys walk the same stops」is one implementation and not three
  // that drift.
  //
  const sidePane = useListWidth(
    narrow,
    "ws-sessions",
    "right",
    "[data-ws-sessions]",
    // 2026-09-17 ruling 52:「session无法再往右拖了, 似乎被固定了最大宽度,
    // 不应该这样」— the hook's own ceiling is half the window, which is
    // the right answer for two columns and not for three.
    roomBeside("[data-ws-sessions]"),
  );
  const tree = side?.on ?? false;
  const sessionDetailOpen = !sidePane.collapsed;
  // **Said as a state, never as a flip** (工作区批⑨): the default below
  // is declared from an effect, and an effect that flips is an effect
  // that undoes itself the second time React runs it.
  const setSessionDetailOpen = sidePane.show;
  // Opening a session uses the workspace's existing frame.
  const showSessionDetail = (on: boolean) => {
    setSessionDetailOpen(on);
    if (narrow) setScreen(on ? "sessions" : "file");
  };
  // (The toolbar buttons' look, `toolClass`, went with them: the switches
  // are glass now, below.)
  /** 新建文件 / 新建目录 from the ＋ at the head of the tree (2026-09-28),
      stamped for the tree to take once (`FileTree`'s `make`). */
  const [make, setMake] = useState<{ kind: "file" | "dir"; at: number } | null>(null);
  /** The item chosen on the ＋ menu, held until the menu has let go of
      the focus. **The box is asked for after the menu closes, not when
      the item is pressed**: the menu keeps the focus inside itself while
      it is open, so a box born then never got its caret, and a menu that
      then handed the focus back to ＋ would have shut it on the spot (it
      closes on blur, `EditableName`). So the close's own focus hand-back
      is where the box is asked for, and that hand-back is skipped — the
      caret goes to the box. A plain dismissal (Esc, a press outside)
      chose nothing and gives ＋ its focus as usual. */
  /** Which session this column is standing in, by id, or `null` for
      the list. **Dropped when the workspace changes** along with the
      tree's position and the open file: a session belongs to one
      project, and carrying it across would leave somebody looking at a
      conversation about a directory that is no longer on screen. */
  const sessionTabs = useSessionTabs(limit, id=>rows.some(row=>row.id===id&&row.pinned),
    'khor.workspace.sessions.'+JSON.stringify([machine,root]));
  const picked = sessionTabs.active;
  const sessionRowOf=useSessionRowLookup(rows,devices);
  const selectedSession=sessionRowOf(picked);
  const contentSide=useSessionSide(picked);
  const openSessionContent=(id:string)=>{
    if(!sessionContent)return;
    sessionContent.navigation.resume(id);setSessionSide(id,{open:true,seat:'workspace'});
  };
  const selectSession=(id:string)=>{sessionTabs.focus(id);if(contentSide.open)openSessionContent(id);};
  const closeSessionTab=(id:string)=>{sessionTabs.close(id);if(sessionTabs.tabs.length===1)showSessionDetail(false);};
  const newSession=()=>onNewSession(channel??machine,root);
  const sessionFaces=sessionTabs.tabs.map(id=>{
    const row=sessionRowOf(id);
    return {id,label:row?termName(row):id,path:row?sessionCwdOnScreen(row)||'':'',face:row?.face??null,
      session:row?{word:row.word,kind:row.kind,category:row.category}:undefined,
      again:sessionContent?{hint:gui.session_tabs_focus,expanded:id===picked&&contentSide.open,
        toggle:()=>{if(id===picked&&contentSide.open)setSessionSide(id,{open:false});else openSessionContent(id);}}:undefined};
  });
  const contentScope=picked?{session:picked,mobile:narrow,faces:sessionFaces,onSelectSession:selectSession,
    onCloseSession:closeSessionTab,onNewSession:newSession,onClose:()=>setSessionSide(picked,{open:false})}:null;
  /** …and the one the wizard just made here, marked on the list until
      somebody presses a row. **Not `picked`**: `picked` is 「this column
      is standing in that session」 and opens its detail, while this is
      「that one is yours, it is the new one」 — the row a person looks
      for when the wizard closes. A fresh session has nothing in it to
      read yet, so putting the column inside it would be a back arrow
      over an empty conversation. */
  const [made, setMade] = useState<string | null>(null);
  useEffect(() => {
    if (opened === null) return;
    setMade(opened);
    // The column has to be on screen for the mark to be seen — and on
    // narrow it is a screen (33), which is the same sentence there.
    setSidebarView("sessions");
    sessionTabs.open(opened);
    showSessionDetail(true);
    // Standing in some other session, the new row is behind a back
    // arrow; the list is where it is visible.
    onOpenedSeen();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opened]);
  const [git, setGit] = useState<GitStatus | null>(null);
  // Keep the last header facts through detach so restoring a log does not
  // grow an initially empty header after its reading position was restored.
  const [branch, setBranch] = useWorkspaceField(memory, "branch", () => "");
  const [, setHeadCommit] = useWorkspaceField(memory, "headCommit", () => null);
  const [historyQuery, setHistoryQuery] = useWorkspaceField(memory, "historyQuery", () => "");
  /** Bumped to ask the marks again. A counter for the same reason the
      file view's is one: two saves are two questions. */
  const [gitAt, setGitAt] = useState(0);
  /** What the open file's view has to add to its tab's hover title —
      today, that its diff stopped short (第十四轮 第 2 条). Held here
      because the tab is this pane's; reported by `FileView`'s `onNote`.
      Null on every other tab, so it never travels to a file it is not
      about. */
  const [fileNote, setFileNote] = useState<string | null>(null);
  /** Bumped to ask the open file to read itself again (工作区批⑩). Its
      own counter rather than `gitAt`'s, because the two are asked of
      different machines' answers about different things — and one of
      them is also bumped by every save this pane makes, which is
      precisely the moment the file must *not* be re-read out from
      under the person who just wrote it. */
  const [readAt, setReadAt] = useState(0);
  /** What this column is searched for. **In memory and per pane**
      (工作区批⑩): a search box is something being done right now, and
      one that followed you into the next workspace would be filtering
      the wrong project on arrival (`App`'s `queries` says the same). */
  const [query, setQuery] = useState("");
  /** The ticked words, remembered under **this pane's own key** rather
      than the sessions landing's (工作区批⑩).
   *
   * Not shared, and the reason is what the landing's ticks are made of:
   * they name machines and directories as well as states, and both of
   * those are what this column already **is**. A tick on another machine
   * would empty every workspace column on this one, for a reason nothing
   * on the glass says — this repo's oldest failure wearing a filter.
   *
   * One key for the pane and not one per workspace, because a tick is a
   * posture about a list（「只看忙碌的」）rather than a fact about a
   * project — the same class of thing as the arrangement, and the
   * opposite class from a pin. */
  const [words, setWords] = useState<string[]>(() => storedWords(WS_PANE));
  const toggleWord = useCallback((key: string) => {
    setWords((was) => {
      const next = was.includes(key) ? was.filter((w) => w !== key) : [...was, key];
      rememberWords(WS_PANE, next);
      return next;
    });
  }, []);
  const hidden = useHidden();
  /** That machine's VS Code, on this workspace's directory — the flow
      the machine page's door shares (`components/CodeWindow`, moved out
      of here 2026-09-30). Held here, not in the button: the button moves
      between R0 and the narrow head, and a remount would drop the
      window's handle and stop the server under it. */
  const codeWin = useCodeWindow(machine, root);

  /** What is typed into the box at the head of the tree
      (2026-09-17 ruling 47). **In memory and per pane**, the sessions
      query's rule one column over: a search is something being done
      right now, and one that followed you into the next workspace would
      be filtering a project it was never typed about. */
  const [hunt, setHunt] = useState("");
  /** What the machine answered about it, or `null` while the tree is
      what this column is showing. **Null and「empty answer」are
      different screens**: one is the tree, the other is 「nothing here
      holds that」. */
  const [found, setFound] = useState<Found | null>(null);
  const [huntError, setHuntError] = useState<string | null>(null);
  /** Which directory the search is about: the workspace, or — on the
      machine-browsing face, which has no workspace — wherever the tree
      is standing. */
  const huntDir = root === "" ? at : root;
  /**
   * One question in flight, 300 ms after the typing stops.
   *
   * **Debounced rather than asked per keystroke**, because the answer
   * is a walk of somebody's project: `k`, `ke`, `kee` would be three
   * walks for two answers nobody reads. And **a late answer to an old
   * question is thrown away** — the stamp is what decides, not the
   * order the promises settle in, which is the classic way a search box
   * ends up showing the results for a word that is no longer in it.
   */
  const asked = useRef(0);
  useEffect(() => {
    const q = hunt.trim();
    if (q === "") {
      // Emptied: back to the tree, and nothing is asked.
      asked.current += 1;
      setFound(null);
      setHuntError(null);
      return;
    }
    const mine = (asked.current += 1);
    const wait = window.setTimeout(() => {
      searchIn(machine, huntDir, q)
        .then((f) => {
          if (asked.current !== mine) return;
          setFound(f);
          setHuntError(null);
        })
        .catch((e: unknown) => {
          if (asked.current !== mine) return;
          setFound(null);
          setHuntError(String(e instanceof Error ? e.message : e));
        });
    }, HUNT_WAIT);
    return () => window.clearTimeout(wait);
  }, [hunt, machine, huntDir]);
  /** Esc, and emptying the box: the two ways back to the tree. */
  const dropHunt = () => setHunt("");

  const [liveHead, setLiveHead] = useState<{ known: boolean; id: string | null }>({ known: false, id: null });
  const [repository, setRepository] = useState(false);
  useLayoutEffect(() => {
    if (hidden || root === "") return;
    return memory.metadata.attach(async () => {
      const [status, head, checkout] = await Promise.allSettled([
        fetchGitStatus(machine, root), fetchGitHistory(machine, root, null, 1), fetchCheckout(machine, root),
      ]);
      return {
        status: status.status === "fulfilled" ? status.value : null,
        headKnown: head.status === "fulfilled",
        head: head.status === "fulfilled" ? head.value.head : null,
        commit: head.status === "fulfilled" ? head.value.commits[0] ?? null : null,
        repository: status.status === "fulfilled" || head.status === "fulfilled" ? true : checkout.status === "fulfilled" ? checkout.value : undefined,
      };
    }, facts => {
      setGit(facts.status);
      setBranch(facts.status?.branch ?? "");
      setHeadCommit(facts.commit);
      setLiveHead({ known: facts.headKnown, id: facts.head });
      setRepository(facts.repository ?? facts.status !== null);
      if (facts.repository === false) {
        setSidebarView(previous => previous === "history" ? "files" : previous);
      }
    }, () => {
      setGit(null);
      setBranch("");
      setHeadCommit(null);
      setLiveHead({ known: false, id: null });
      setRepository(false);
    });
  }, [memory, machine, root, hidden, setBranch, setHeadCommit, setSidebarView]);
  const gitAsked = useRef(gitAt);
  useEffect(() => {
    if (gitAsked.current === gitAt) return;
    gitAsked.current = gitAt;
    memory.metadata.invalidate();
  }, [memory, gitAt]);

  const [treeWatchEntries, setTreeWatchEntries] = useState<string[]>([]);
  const fileParent = file === null ? null : file.slice(0, file.lastIndexOf("/")) || "/";
  const watchPlan = directoryWatchScopes(root === "" ? null : root,
    [...(fileParent === null ? [] : [fileParent]), ...treeWatchEntries]);
  const observation = useDirectoryWatch(machine, watchPlan.scopes, root !== "");
  useEffect(() => {
    const batch = observation.batch;
    if (observation.status !== "watching" || batch === null) return;
    if (batch.reset || batch.workspace) memory.metadata.invalidate();
    if (fileParent !== null && (batch.reset || batch.dirs.includes(fileParent))) setReadAt(n => n + 1);
  }, [observation.batch, observation.status, memory, fileParent]);

  const workspaceCovered = directoryScopeIsNative(observation, { kind: "workspace", root });
  const fileCovered = fileParent === null || directoryScopeIsNative(observation, { kind: "entries", path: fileParent }) ||
    fileParent === root && workspaceCovered;
  useEffect(() => {
    if (hidden || root === "" || workspaceCovered && fileCovered) return;
    const refresh = () => {
      if (document.hidden) return;
      if (!workspaceCovered) memory.metadata.invalidate();
      if (!fileCovered) setReadAt(n => n + 1);
    };
    const slow = window.setInterval(refresh, 60_000);
    window.addEventListener("focus", refresh);
    return () => { window.clearInterval(slow); window.removeEventListener("focus", refresh); };
  }, [memory, root, hidden, workspaceCovered, fileCovered]);

  // A returning text view rechecks through its existing accepted-read owner.
  useEffect(() => {
    if (hidden) return;
    setReadAt((n) => n + 1);
  }, [hidden]);

  // **…and after khor's own acts anywhere in this checkout** (UX-43 §1,
  // `lib/tree-changed`): a file made, renamed, deleted or saved in the
  // 附栏's tree — or in this pane — moves the letters here too.
  useEffect(
    () =>
      onTreeChanged((c) => {
        if (root !== "" && sameMachine(c, { machine, here }) && holds(root, c.dir)) setGitAt((n) => n + 1);
      }),
    [machine, here, root],
  );

  const declaredNarrow = useRef<boolean | null>(null);
  useEffect(() => {
    if (narrow) {
      // No room for a second column at all, so the default there is not
      // about the fifth one.
      declaredNarrow.current = true;
      setSessionDetailOpen(false);
      return;
    }
    if (declaredNarrow.current === false) return;
    declaredNarrow.current = false;
    // Back from this column's new tab, or a session made there: the
    // column is where the person was (UX-45).
    setSessionDetailOpen(picked !== null);
    if (back) { setSidebarView("sessions"); onBackSeen?.(); }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [narrow]);

  // **A session belongs here if it is running on this machine, under
  // this directory** (工作区批③). Both halves, because a workspace *is*
  // the pair (docs/WORKSPACE.md): two machines hold `~/w/khor` and they
  // are two workspaces, so a column that went by path alone would show
  // the other one's work — and with the machine deliberately not
  // repeated on these rows (`showVia`), nothing on screen would say so.
  // That is this repo's oldest failure by name: a branch that cannot
  // answer does not go blank, it becomes its neighbour.
  //
  // **The path half.** This directory itself, or anything below it.
  // 「The same path」 was wrong in both directions against a folded
  // `place` — a session started in `~/w/paper/data` reported
  // `~/w/paper`, so a workspace at `~/w/paper/data` never saw it; and a
  // workspace at a checkout root did collect everything under it, by
  // the accident of the fold having stopped exactly there. Read the
  // current raw cwd, falling back to the recorded place. The separator keeps
  // `/home/peer/workshop` out of `/home/peer/work`: a bare `startsWith`
  // is the classic prefix collision, and sibling directories share
  // prefixes all the time.
  //
  // **The machine half is a channel comparison**, resolved from the id
  // by the shell (`channel`). A machine the device table has no row for
  // leaves it `null` and this column empty — which is the honest
  // answer: nothing can be shown to be running *here* when here cannot
  // be named, and an empty column is a fact where a neighbour's rows
  // would be a lie.
  //
  // Rows carry no group heading: the column *is* one directory on one
  // machine, so there is nothing left to group by, and `SessionsList`
  // draws a heading only where a row declares one.
  const mine = useMemo(() => {
    if (root === "" || channel === null) return [];
    return rows
      .filter((r) => sessionInWorkspace(r, channel, root))
      .map((r) => ({ ...r, group: "" }));
  }, [root, rows, channel]);
  const marks = useMemo(() => folded(root, git), [root, git]);
  /** What each of this project's sessions was saying the last time the
      rows moved, by id. */
  const said = useRef(new Map<string, string | null>());
  // **An agent stopping is a reason to ask git again** (工作区批⑩:
  // 「这个工作区里的会话从 busy 变到别的状态时,git 标记重问一次」).
  // The thing that most often changes files in a khor workspace is an
  // agent, and the moment it has finished changing them is the moment
  // it stops being 忙碌 — so that transition is the ask, and no timer
  // is added to catch it.
  //
  // **busy → anything, not busy → 完成.** A run that failed, was
  // interrupted or is now waiting on an answer has already written
  // whatever it wrote; which of the six words it landed on is a fact
  // about the session and says nothing about the disk.
  //
  // Driven by `rows` arriving rather than by a word being watched:
  // the shell polls, this compares two polls, and a session that was
  // never seen busy has nothing to have stopped from.
  useEffect(() => {
    const now = new Map<string, string | null>();
    let stopped = false;
    for (const r of mine) {
      now.set(r.id, r.word);
      if (said.current.get(r.id) === "busy" && r.word !== null && r.word !== "busy") stopped = true;
    }
    said.current = now;
    if (stopped) setGitAt((n) => n + 1);
  }, [mine]);
  /** The choices this column's filter offers: **the state words and
      nothing else** (工作区批⑩). The other three axes the sessions
      landing offers — machine, category, workspace — are two thirds of
      what this column already is (`mine` is one machine under one
      directory), and a category axis alone is not worth a heading.
      Built by the helper the landing calls, so the two cannot come back
      with different words for one state. */
  const wordOptions = useMemo(() => sessionOptions(mine, words, ["state:"]), [mine, words]);
  const axes = useMemo(() => sessionAxes(wordOptions), [wordOptions]);

  useLayoutEffect(() => {
    memory.bindNavigationCheckpoint(() => navigationStore.save({ at: memory.read("at"), content: memory.read("content"), historyOpen: memory.read("sidebarView") === "history" }));
  }, [memory, navigationStore]);
  useEffect(() => { memory.checkpointNavigation(); }, [memory, at, content, sidebarView]);

  const openTarget = useCallback((target: WorkspaceTarget, pinned: boolean) => {
    let next = openWorkspaceTab(memory.read("content"), target, pinned);
    const active = workspaceTabId(target);
    const gone = stalestTab(next.items.map(workspaceTabId), limit, touched.current, (id) => {
      const item = next.items.find((item) => workspaceTabId(item) === id);
      return id === active || item?.kind === "file" && memory.read("drafts").has(item.path);
    });
    if (gone !== null) next = closeWorkspaceTab(next, gone);
    setContent(next);
    setScreen("file");
  }, [memory, setContent, limit, setScreen]);
  const open = useCallback((path: string, pinned: boolean) => {
    if (diffOf(path) === null) setRecent(rememberOpened(machine, root, path));
    openTarget(targetOf(path), pinned);
  }, [machine, root, setRecent, openTarget]);
  const openCommit = (value: GitCommitSummary, pinned = false) => openTarget({ kind: "commit", commit: value }, pinned);

  useEffect(() => {
    if (content.active !== null) touched.current.set(content.active, Date.now());
  }, [content.active]);

  /** git's letter was pressed: open that file **against HEAD** in a
      tab of its own (工作区第五轮 第 38 条).
   *
   * **A preview tab, like a single click on a file**, and for the same
   * reason: a diff is something you glance at on the way to deciding
   * something, and a strip that grew a permanent tab every time
   * somebody pressed a letter would fill up with reading nobody kept.
   * Pinning it is the same gesture as pinning any other — open it
   * twice, or type in the file it is about.
   *
   * It takes the preview seat off whatever is in it, which is what
   * makes「press M, press M on the next file」 not leave a trail. */
  const openDiff = useCallback(
    (path: string) => {
      open(diffKey(path), false);
    },
    [open],
  );

  /** A name moved on the disk (工作区第五轮 第 36 条).
   *
   * **Every tab about that file follows it, and what was typed in it
   * follows too.** A rename is not a close: dropping the tab would
   * throw away a draft for a file that still exists, under a name the
   * person themselves chose, which is the worst moment to lose work.
   *
   * **A directory takes its whole subtree.** Renaming `src` moves every
   * open file under it, and a tab left pointing at `src/old/a.rs` is a
   * tab over a path that is not there — the state the tree is not
   * allowed to leave behind (`onGone`, from the other end).
   *
   * And the marks are stale the moment a path changes, so git is asked
   * again — the same counter a save bumps.
   */
  const moved = useCallback(
    (from: string, to: string) => {
      const after = (p: string) =>
        p === from ? to : p.startsWith(`${from}/`) ? `${to}${p.slice(from.length)}` : p;
      const next = renameWorkspaceFiles(memory.read("content"), after);
      setContent(next);
      setDrafts((was) => {
        const now = new Map<string, Draft>();
        for (const [p, d] of was) now.set(after(p), d);
        return now;
      });
      setClosing((was) => (was === null ? was : after(was)));
      setRecent((was) => was.map(after));
      setGitAt((n) => n + 1);
    },
    [memory, setContent],
  );

  /** Take a tab off the strip. **Never asks here** — whether there is
      anything to ask about is `shut`'s business one level up, and this
      is what both of its answers end in. */
  const drop = useCallback(
    (path: string) => {
      const next = closeWorkspaceTab(memory.read("content"), workspaceTabId(targetOf(path)));
      setContent(next);
      setDrafts((was) => {
        if (!was.has(path)) return was;
        // **Thrown away on purpose** (不保存): the write the file's view
        // makes on its way out must not bring it back (save-a).
        discard(machine, path);
        const now = new Map(was);
        now.delete(path);
        return now;
      });
      setClosing((was) => (was === path ? null : was));
    },
    [machine, memory, setContent],
  );

  /** Close a tab. **Something typed in it is saved on the way out**
      (save-a, 「编辑文件要实时保存」): the tab closes when the write
      lands. It asks first only when the file cannot be saved — a clash
      or a failure already on it, or the save on the way out not landing
      (`waiting`'s `ask`) — and the dialog is raised rather than the close
      refused: a refusal would leave a person pressing × over and over
      with nothing telling them why (docs/UX.md 做了但没变化). */
  const shut = useCallback(
    (path: string) => {
      if (!memory.read("drafts").has(path)) return drop(path);
      const r = roadOf(machine, path);
      // The front file's view writes what it would have written at the
      // end of the pause; a tab behind wrote its text as it was left,
      // and that write may still be under way. Neither — a merge not
      // looked at, a write that was thrown away — is a question.
      if (r.trouble === null && (flushFile(machine, path) !== null || r.busy)) {
        setWaiting({ path, ask: true });
        return;
      }
      setClosing(path);
    },
    [memory, drop, machine],
  );

  // The save the sheet asked for, made by the view that holds the
  // buffer — so the tab is brought to the front and asked on the beat
  // after, when that view is the one mounted.
  useEffect(() => {
    if (wantSave === null || file !== wantSave) return;
    setWantSave(null);
    setSaveAt((n) => n + 1);
  }, [wantSave, file]);

  // …and it closes when it has nothing waiting in it any more, which is
  // the only honest signal that the save landed: a refusal leaves the
  // draft where it is, and the tab with it.
  useEffect(() => {
    if (waiting === null) return;
    if (!drafts.has(waiting.path)) {
      setWaiting(null);
      drop(waiting.path);
      return;
    }
    // The save on the way out did not land: ask, rather than close over
    // the edits or leave the tab standing with nothing said.
    const r = roadOf(machine, waiting.path);
    if (waiting.ask && r.trouble !== null && !r.busy) {
      setWaiting(null);
      setClosing(waiting.path);
    }
  }, [waiting, drafts, drop, machine, roads]);

  // ⌘W, ⌃⇥ / ⌃⇧⇥ on this strip: **lent to the app's keyboard**
  // (`lib/shortcuts`, 2026-09-28) rather than a listener of its own — the
  // map, the menu bar's 关闭标签页 and 「not under a sheet」 are decided
  // there once for every strip. ⌘W still asks first when something is
  // typed in the tab (`shut`). **Only while a file is in front** for ⌘W,
  // and since 2026-09-16 that is no longer the same question as 「the
  // editor is not open」: the editor has a column of its own (ruling 45)
  // and the file is still on screen beside it. A press inside the
  // editor's own window never reaches this page — its ⌘W is VS Code's.
  const closeTab = (id: string) => {
    const target = memory.read("content").items.find((item) => workspaceTabId(item) === id);
    if (!target) return;
    if (target.kind === "commit") setContent((was) => closeWorkspaceTab(was, id));
    else shut(keyOf(target));
  };
  useTabHost(content.items.length > 0 ? {
    count: content.items.length,
    front: content.active !== null,
    close: () => { if (content.active !== null) closeTab(content.active); },
    step: (by) => {
      const at = content.items.findIndex((item) => workspaceTabId(item) === content.active);
      const target = content.items[(at + by + content.items.length) % content.items.length];
      if (target) setContent((was) => selectWorkspaceTab(was, workspaceTabId(target)));
    },
  } : null);

  // **`⌘B` folds and unfolds the tree** (VS Code's key for its side
  // bar), and it is the one way back that no other fold can hide: the
  // chevron that folds the tree sits on the list's row, and the list
  // folds too (the rail's own icon), so with both away nothing on the
  // glass restores the tree — the trap named when this was designed
  // (2026-09-16). Live for as long as the workspace is, file or no file.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && (e.key === "b" || e.key === "B")) {
        e.preventDefault();
        // **On the narrow face it means「the tree」, not「fold the
        // tree」** (2026-09-16 ruling 33): there is no column to fold
        // there, and the key's whole job on both faces is the same one
        // — put the tree in front of me, from wherever I am.
        if (narrow) setScreen("tree");
        else side?.set(!side.on);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [side, narrow]);

  // 会话里的路径与网址可点 (ruling 35). What the conversations in the
  // right-hand column are allowed to turn into links, and what happens
  // when one is pressed.
  //
  // **Pressing a path opens a preview tab**, which is a single click in
  // the tree spelled from somewhere else — the same gesture, the same
  // seat, so a person reading a conversation and a person walking the
  // tree end up with the same strip.
  const { known: fileKnown, see: seeFile } = useKnownFiles(machine, root);
  const linkHome = useMemo<LinkHome>(
    () => ({
      machine,
      root,
      known: fileKnown,
      see: seeFile,
      open: (abs, line) => {
        open(abs, false);
        setGoLine(line === null ? null : { path: abs, line, at: Date.now() });
      },
      beside: true,
    }),
    [machine, root, fileKnown, seeFile, open],
  );

  // A file the shell asked for on the way in (2026-09-18 ruling 54).
  // **The same two moves a press inside this pane makes** — a preview
  // tab and, where a line was named, the scroll — because it is the
  // same gesture spelled from the session landing, and a second road
  // into the middle column would be a second arrangement of it.
  useEffect(() => {
    if (reveal === null) return;
    open(reveal.path, false);
    setGoLine(
      reveal.line === null
        ? null
        : { path: reveal.path, line: reveal.line, at: reveal.at },
    );
    // On the narrow face the middle column is a screen, and arriving
    // on the tree with the file open behind it is the press going
    // nowhere (33).
    if (narrow) setScreen("file");
    onRevealSeen();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [reveal]);

  // **The last file closed from the page's tabs sheet leaves the page**
  // (UX-25): on the narrow face the file screen with no file is the
  // tree's turn, as the session page's last × is the list's.
  useEffect(() => {
    if (narrow && screen === "file" && file === null && commit === null) setScreen("tree");
  }, [narrow, screen, file, commit]);

  /** What the strip draws. **Built here because the facts are this
      pane's** — which files are open, which one is in front, which one
      has something typed in it — and `TabStrip` paints and decides
      nothing (its own head says so). */
  const tabFaces: TabFace[] = useMemo(
    () =>
      content.items.map((target) => {
        const id = workspaceTabId(target);
        if (target.kind === "commit") return {
          id, label: target.commit.subject, title: `${target.commit.subject} · ${target.commit.id}`,
          face: machineFace, path: target.commit.id, machine,
          preview: preview === id,
          onPin: () => pin(id),
          mark: { key: "history", glyph: (className: string) => <IconHistory className={className} /> },
          menu: contentMenu(id),
        };
        const key = keyOf(target);
        // A diff tab is a tab about a file, so everything below that
        // reads a path reads the **file's** — the icon, the second
        // line, the hover title. Only the name says which face of it is
        // open (`diffLabel`).
        const path = fileOf(key);
        const isDiff = diffOf(key) !== null;
        return {
        id,
        onPin: () => pin(id),
        label: isDiff ? diffLabel(basename(path)) : basename(path),
        face: machineFace,
        path,
        machine,
        // **One line, the name** (UX-20, UX-25: one tab language — the
        // session tabs' pill). ⚠️ 墓碑 (F4.7; W17): `sub`, the folder the
        // file sits in under its name — the absolute path is the hover
        // title's, and both are on the menu.
        // What the open file has left to whisper — today, a diff that
        // stopped short (第十四轮 第 2 条). Only the front tab has a
        // view to report it, so only the front tab wears it.
        note: key === file ? fileNote : null,
        // A diff has nothing to type in, so it never carries a dot.
        dirty: !isDiff && drafts.has(key),
        preview: preview === id,
        // The file's own kind on the tab too (2026-09-16:「标签页打开了,
        // 最上方也要有 icon」) — the same icon the tree drew for it, as
        // the mark on the machine's face, the way every pill's seat is
        // assembled (UX-32; `fileKind`). ⚠️ 墓碑 (UX-25 → UX-32): the
        // glyph alone, at `--tab-face`, in the seat instead of a face.
        mark: fileKind(basename(path)),
        // 预览, on the tab it is about (2026-09-17 ruling 47). Only
        // where there is a second face to switch to — a `.rs` has one
        // face and grows no eye, exactly as the strip's far end drew
        // none for it before this moved.
        // ⚠️ 墓碑 (round UX-3): `look`, the 预览 eye on the tab (ruling 47).
        // A file with a second face opens on it now, and 编辑 at the
        // strip's right end is the way to its source (`fileTail`).
        edit: !isDiff && key === file ? readFirst.item : null,
        menu: tabMenu(key),
        // Save confirms the pending draft; its landed state is briefly ink.
        // A tab not in front is brought forward before saving (`wantSave`).
        // Clean files follow the existing watcher; explicit reread stays in
        // the workspace menu rather than a permanent strip button.
        act: isDiff
          ? undefined
          : drafts.has(key)
            ? {
                kind: "save" as const,
                label: gui.watch_save,
                onPress: () => {
                  if (key === file) setSaveAt((n) => n + 1);
                  else {
                    setFile(key);
                    setWantSave(key);
                  }
                },
              }
            : roadOf(machine, key).confirmed
              ? { kind: "saved" as const, label: gui.file_saved, onPress: () => {} }
              : undefined,
        };
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [content, machine, machineFace, root, drafts, preview, shut, file, fileNote, readFirst.item?.label, readFirst.item === null, roads],
  );

  function closeOthers(id: string) {
    for (const item of memory.read("content").items) {
      const other = workspaceTabId(item);
      if (other !== id) closeTab(other);
    }
    setContent((was) => selectWorkspaceTab(was, id));
  }
  function contentMenu(id: string): RowAction[] {
    return [
      { key: "close", label: gui.close, run: () => closeTab(id) },
      { key: "close-others", label: gui.tab_close_others, run: () => closeOthers(id) },
    ];
  }
  function tabMenu(key: string): RowAction[] {
    const path = fileOf(key);
    return [
      ...contentMenu(workspaceTabId(targetOf(key))),
      { key: "copy-path", label: gui.tab_copy_path, run: () => navigator.clipboard.writeText(path) },
      { key: "copy-rel", label: gui.tab_copy_rel, run: () => navigator.clipboard.writeText(under(root, path)) },
    ];
  }

  useFullScreen(narrow && screen === "file");
  /** What the view hands back about its unsaved copy, for one file.
      **Nothing is written when nothing changed** (reported from an effect,
      `FileView`); typing pins a preview tab (「在里面打了字,就钉住」). */
  const draftOf = (path: string) => (d: Draft | null) => {
    setDrafts((was) => {
      const had = was.get(path);
      if (d === null ? had === undefined : had === d) return was;
      const now = new Map(was);
      if (d === null) now.delete(path);
      else now.set(path, d);
      return now;
    });
    if (d !== null) pin(workspaceTabId({ kind: "file", path }));
  };
  /** 编辑 at the same place, and ✓ back to reading there (round UX-3). */
  const editHere = () => {
    restore.current = topLine(mid.current);
    flushSync(() => setFace("source"));
    beginEditing(mid.current);
  };
  const readHere = () => {
    (document.activeElement as HTMLElement | null)?.blur?.();
    // Back to the rendered face as it is: a draft stays a draft (the
    // pill's ✓ saves it, UX-33). ⚠️ 墓碑: a save on the way back.
    restore.current = topLine(mid.current);
    setFace("preview");
  };
  const shortcuts = root !== "" && codeWin.offered ? <GlassCapsule size="head" data-ws-code-control>
    <CodeButton code={codeWin} className="h-head-item-h w-head-item-h rounded-full data-[on=true]:glass-sel" />
  </GlassCapsule> : null;
  const workspaceActions = [
    ...(!narrow ? [{ key: "tree", label: gui.workspace_tree, run: () => side?.set(!tree) }] : []),
    { key: "refresh", label: gui.file_reread, run: () => { setReadAt(n => n + 1); setGitAt(n => n + 1); memory.repositoryHistory.invalidate(); } },
    { key: "close-others", label: gui.tab_close_others, run: () => { const id = memory.read("content").active; if (id !== null) closeOthers(id); } },
  ];
  /** **The file's verbs at the strip's right end** (round UX-3, the Mac):
      分享, 「…」, and 编辑 where the file has a face to read it on — the
      phone's file page's circles, beside the tab they are about. */
  const fileTail =
    file !== null && diffOf(file) === null && rev !== null ? (
      // A revision from git 历史 where the file was (round UX-4): back to
      // the file as it is, and the compare switch.
      <div data-ws-file-tail className="flex flex-none items-center gap-in px-2">
        {/* Which revision this is, where the phone's name pill says it. */}
        <span data-file-rev-chip title={`${rev.author} · ${rev.subject}`} className="glass rounded-full px-row py-1 font-mono text-callout text-foreground">
          {shortId(rev)}
        </span>
        <GlassCircle aria-label={gui.file_rev_back} tip={gui.file_rev_back} data-file-rev-back onClick={() => { setRev(null); setFace(defaultFaceOf(file)); }}>
          <IconBack />
        </GlassCircle>
        {!revCompare && revisionRenders(file) && (
          <GlassCircle
            aria-label={gui.file_face_preview}
            aria-pressed={face === "preview"}
            tip={gui.file_face_preview}
            data-file-face-toggle={face}
            onClick={() => setFace((f) => f === "preview" ? "source" : "preview")}
            className={cn(face === "preview" && "glass-sel text-[var(--rail-on-ink)]")}
          >
            <IconEye />
          </GlassCircle>
        )}
        <GlassCircle
          aria-label={revCompare ? gui.file_rev_alone : gui.file_rev_compare}
          aria-pressed={revCompare}
          tip={revCompare ? gui.file_rev_alone : gui.file_rev_compare}
          data-file-rev-compare
          onClick={() => setRevCompare(!revCompare)}
          className={cn(revCompare && "glass-sel text-[var(--rail-on-ink)]")}
        >
          <IconBranch />
        </GlassCircle>
      </div>
    ) : file !== null && diffOf(file) === null ? (
      <div data-ws-file-tail className="flex flex-none items-center gap-in px-2">
        <FileVerbs
          handoff={handoff}
          machine={machine}
          path={file}
          here={here}
          code={codeWin}
          onGone={(p) => {
            drop(p);
            setGitAt((n) => n + 1);
          }}
          onMoved={moved}
          onLocate={() => { requestFileLocation(machine, file); setSidebarView("files"); setHunt(""); if(root === "")setAt(directoryOf(file)); setScreen("tree"); side?.set(true); }}
          history={git !== null}
          previewing={face === "preview"}
          faceToggle={rendererOf(file) !== null ? {face, run: face === "preview" ? editHere : readHere} : undefined}
          toolbar={{ leading: shortcuts, actions: workspaceActions }}
          onRevision={(c) => {
            setRev(c);
            setRevCompare(false);
            setFace(defaultFaceOf(file));
          }}
        />
      </div>
    ) : null;

  const switches = <div data-ws-switches className="flex flex-none items-center gap-in">
    {status && <span data-region-caption className="min-w-0 truncate text-caption text-tertiary">{status}</span>}
    {file !== null && diffOf(file) === null && rev === null ? fileTail : <>{fileTail}{shortcuts}
      <DropdownMenu>
        <DropdownMenuTrigger asChild><GlassCircle aria-label={gui.rail_more} data-ws-more><IconMore /></GlassCircle></DropdownMenuTrigger>
        <DropdownMenuContent align="end">{workspaceActions.map(action => <DropdownMenuItem key={action.key} onSelect={action.run}>{action.label}</DropdownMenuItem>)}</DropdownMenuContent>
      </DropdownMenu>
    </>}
    {!(picked!==null&&sessionDetailOpen)&&<WindowControls />}
  </div>;

  const workspaceSessionList = (
    <div data-ws-sessions-list className="min-h-0 flex-1 overflow-y-auto">
      <SessionsList rows={mine} devices={devices} query={query} words={words} selected={picked ?? made}
        onSelect={(row) => { sessionTabs.open(row.id); showSessionDetail(true); }}
        onChildSession={(parent,child)=>{const id=delegatedSessionId(parent,child);rememberDelegatedSession(id,child);sessionTabs.open(parent.id);sessionTabs.open(id);showSessionDetail(true);}}
        onPin={onPin} onClose={onClose} pinFailed={pinFailed} closeRefused={closeRefused} showVia={false} />
    </div>
  );

  const workspaceTitle = (
    <div data-ws-side-head className="flex min-w-0 items-center gap-in">
      {!narrow && <GlassCircle aria-label={gui.back} data-ws-list-back onClick={onBack}><IconBack /></GlassCircle>}
      <PaneBar inline seat="side" title={root === "" ? machine : basename(root)}
        searchLabel={sidebarView === "sessions" ? gui.search_sessions : historyOpen ? gui.git_history_search_page : gui.tree_search}
        query={sidebarView === "sessions" ? query : historyOpen ? historyQuery : hunt}
        onQuery={sidebarView === "sessions" ? setQuery : historyOpen ? setHistoryQuery : setHunt}
        filterLabel={sidebarView === "sessions" ? gui.filter : undefined}
        filter={sidebarView === "sessions" ? { options: wordOptions, chosen: words, onToggle: toggleWord } : undefined}
        axes={sidebarView === "sessions" ? axes : undefined}
        arrange={sidebarView === "sessions" ? { ...arrange, options: [...arrange.options] } : undefined}
        create={sidebarView === "sessions" ? { label: gui.new_session, mark: "data-ws-new-session", onSelect: () => onNewSession(channel ?? machine, root) } : undefined}
        actions={root === "" || sidebarView === "sessions" ? undefined : [
          { key: "file", label: gui.file_new, afterClose: true, onSelect: () => { setHistoryOpen(false); dropHunt(); setMake({ kind: "file", at: Date.now() }); } },
          { key: "dir", label: gui.dir_new, afterClose: true, onSelect: () => { setHistoryOpen(false); dropHunt(); setMake({ kind: "dir", at: Date.now() }); } },
        ]} />
    </div>
  );
  const workspaceMetadata = <div ref={band} data-band data-ws-sidebar-heading>
    <WorkspaceNavigation branch={branch} repo={repository} view={sidebarView} onView={setSidebarView} withSessions/>
  </div>;


  return (
    // **`h-full`, not `flex-1`.** The slot this lands in (App.tsx) is a
    // block, not a flex container, so `flex-1` resolves to nothing and
    // the pane sizes to its own content — which for a long file meant a
    // 3700px pane inside a 1026px window, the whole app scrolling as one
    // page and the head and tree scrolling away with it. Measured on a
    // real file, 2026-09-12. Every other detail pane already wears
    // `h-full` for this reason (`DetailPane`, `BrowseLanding`).
    // Everything painted inside this pane knows which project it is in
    // — which is what lets a conversation in the right-hand column turn
    // a path into a press on the middle one (ruling 35, `linkHome`).
    <LinkHomeProvider value={linkHome}>
    <RegionLead.Provider value={null}>
    {narrow && screen !== "file" && screen !== "sessions" && <NarrowHead>{() => screen === "tree" ? <>
      <GlassCircle aria-label={gui.back} data-back onClick={onBack}><IconBack /></GlassCircle>{workspaceTitle}
    </> : <>
      <GlassCircle aria-label={gui.back} data-ws-back onClick={() => screen === "sessions" ? showSessionDetail(false) : setScreen("tree")}><IconBack /></GlassCircle>
      <span className="flex-1" />
    </>}</NarrowHead>}
    <DeeperHeads>
    <div className="flex h-full min-h-0 flex-col">
      {/* **The head row is gone** (工作区批⑧). It held the workspace's
          name, its branch, how far the branch was from its remote, a
          count of unsaved files and a breadcrumb — and every one of
          those moved somewhere that already answers for it: the name
          and the branch are two corners of this workspace's row in the
          list beside it (docs/WORKSPACE.md 一行的四个角), 「N 个文件
          未存」 became a dot on each tab that has one, and the crumbs
          were cut by the ruling. What is left of the head is the strip.
          (2026-09-28: the name, the branch, the count and `机器:路径`
          are said once more, in R0's two title lines — the window's own
          head, not a row of this pane's; see `subtitle`.)

          **Except on the browsing face.** `root === ""` is what
          看它的文件 hands over — a whole machine's disk, where 「where
          am I」 is the question and the address bar is the answer — so
          that face keeps a bar with the machine's name and its crumbs,
          and a project gets none. */}
      {root === "" && (
        <div
          className="flex items-center gap-2 border-b px-3 py-2"
        >
          <span className="shrink-0 truncate text-body font-title text-foreground">
            {machine}
          </span>
          <PathCrumbs at={at} onGo={setAt} onEdit={() => setAt(at)} />
        </div>
      )}

      <div className="flex min-h-0 flex-1">
        {/* **The list column's head while it holds the tree** (UX-23 §2):
            「‹ 工作区名」 where the list's title stands; ‹ is the way back to
            the workspaces (the head of a folder in Files). */}
        {!narrow && tree && (
          <SideSeat>{workspaceTitle}</SideSeat>
        )}
        {(narrow ? screen === "tree" : tree) && (
          <ListPane
            mark="data-ws-tree"
            // **A region's name, not a control's** — and that
            // distinction is why `workspace-tree` came back into the
            // catalog after being retired in 2026-09-16 (the key's own
            // note says which of the two it is now).
            name={gui.workspace_tree}
            // **The shell, like every other secondary sidebar**
            // (2026-09-20 第 6 条). The ground, the head's height, the
            // seam and the scroll rule all come from there; the tree
            // keeps its own scroller inside, so `scroll` is off.
            // **Wide, the list column's own seat and seam** (UX-23 §2): the
            // tree takes the workspaces list's place in the side block,
            // sized by the list's drag.
            pane={narrow ? null : (side?.pane ?? null)}
            narrow={narrow}
            seat={narrow ? undefined : "side"}
            seam={{ "data-list-seam": "" }}
            scroll={false}
            // **A ceiling on the share, not only a width** (工作区批⑨).
            // The dragged width is a number of pixels and the pane it
            // sits in is not: on a 390 phone the remembered 288 left
            // the file column two characters wide. Nothing said so
            // while the sheet did not wrap — it simply grew past the
            // column and the pane scrolled sideways — and the first
            // thing to notice was `check-face`'s touch floor, which
            // measured a 24-wide editor.
            // **The whole screen on narrow, a column on wide.** A
            // remembered column width applied to a screen would leave a
            // phone's tree 288 wide in a 390 window with a dead strip
            // beside it (ruling 33).
            //
            // **The share is the file's floor now, not 45%**
            // (2026-09-17 ruling 52). 45% of the row was a ceiling for
            // two columns, and this page has three: with the list and
            // the tree folded the sessions column stopped at 601 of
            // 1400 — measured — while the drag believed it had 1096.
            // Two ceilings and the tighter one silently won. The one
            // that is left says the same thing the seam says
            // (`roomBeside`): everything except the middle column's
            // floor. It is still worth having, because a width
            // remembered from a wide window is applied unread in a
            // narrow one, and that is the case the seam never sees.
            className={cn(narrow && "min-w-0 flex-1")}
            // ── the tree's own head: a box and ＋ ──────────────────
            // (2026-09-17 ruling 47:「在文件树的侧边上加一个搜索框(可搜
            // 索文件名与内容)」— the user).
            //
            // **Every list's anatomy since 2026-09-28**: the search box,
            // then ＋ (新建文件 / 新建目录, `treeNew`). The two switches
            // stood where ＋ is from ruling 47 (「框的右侧是session和
            // vscode icon」) because the narrow face had no other door
            // to them before a file was open; they are the page's own
            // buttons now — R0's right end, and on narrow the first
            // row's (`NarrowHead`), which every screen has.
            head={workspaceMetadata}
          >
            {/* **Two trees, one ruling apart** (工作区批⑦). A project
                gets `FileTree`: rooted, expandable, with no way above
                itself. A machine's whole disk (`root === ""`, what
                看它的文件 opens) keeps `FilesPane`, where 上一级 and the
                address bar are the point — docs/WORKSPACE.md 第二轮 says
                so in the user's own terms. */}
            {sidebarView === "sessions" ? workspaceSessionList : historyOpen ? (
              <WorkspaceHistory machine={machine} root={root} query={historyQuery} memory={memory.repositoryHistory}
                    liveHead={liveHead} active={commit?.id ?? null} onOpen={openCommit} />
            ) : hunt.trim() !== "" ? (
              <Hits
                found={found}
                error={huntError}
                query={hunt.trim()}
                dir={huntDir}
                here={here}
                onOpen={(path, line) => {
                  open(path, false);
                  setGoLine(line === null ? null : { path, line, at: Date.now() });
                }}
              />
            ) : root === "" ? (
              <FilesPane
                machine={machine}
                path={at}
                onPath={setAt}
                onLanded={setAt}
                onMoved={moved}
                onGone={path=>{for(const tab of tabs)if(movedFilePath(fileOf(tab),path,path)!==null)drop(tab);}}
                // The browsing face gets the same strip: a tab is a
                // file wherever the file was found, and a second rule
                // for「what happens when you press a file」 is the kind
                // of thing a person has to learn twice. It opens no
                // preview because there is no second click to replace
                // one here — `FilesPane` has one gesture, not two.
                onFile={(path) =>
                  path === null
                    ? // The file somebody was reading was deleted, so
                      // its tab goes with it: a tab over a file that is
                      // not there is the state neither tree may leave
                      // behind, and the strip would keep it on screen.
                      file !== null && drop(file)
                    : open(path, true)
                }
                open={file}
                here={here}
                marks={marks}
                onDiff={openDiff}
              />
            ) : (
              <FileTree
                // Remounted per workspace, which is what lets the tree
                // read its remembered branches straight out of storage
                // at birth rather than resetting itself afterwards.
                key={`${machine} ${root}`}
                machine={machine}
                root={root}
                here={here}
                marks={marks}
                open={file}
                onOpen={open}
                previewOnClick
                observation={observation}
                onWatchEntries={setTreeWatchEntries}
                onDiff={openDiff}
                onGone={path=>{for(const tab of tabs)if(movedFilePath(fileOf(tab),path,path)!==null)drop(tab);}}
                onMoved={moved}
                onMade={() => setGitAt((n) => n + 1)}
                // A new-item request is consumed once by the actual tree.
                make={make}
                onMakeSeen={() => setMake(null)}
              />
            )}
          </ListPane>
        )}
        {(!narrow || screen === "file") && (
        <div
          ref={mid}
          data-ws-mid
          // **Named so the seams can ask how much room it has**
          // (2026-09-17 ruling 52). A column being dragged may take
          // whatever this one can spare above its floor
          // (`--ws-mid-min`), which is a live measurement and not a
          // ratio — `use-list-width`'s `roomBeside` reads it here.
          //
          // **The floor is the drag's and not this box's.** Written as
          // a `min-width` it would be a wedge instead of a stop: the
          // three columns would simply overflow the window, and the
          // face would scroll sideways rather than the seam refusing.
          className="relative flex min-w-0 flex-1 flex-col"
        >
          <div ref={readFirst.box} className="contents">
          <WorkspaceContentFrame
            narrow={narrow}
            onBack={() => setScreen("tree")}
            // **The session tabs' capsule** (UX-25, the owner 2026-10-05:
            // one tab language): pills in one glass capsule, the file's
            // kind in the icon seat, one line of name, × on the one in
            // front and under the pointer. ⚠️ 墓碑: the VS Code-style strip.
            faces={tabFaces}
            head={regionLead}
            active={content.active ?? ""}
            onFocus={(id) => setContent((was) => selectWorkspaceTab(was, id))}
            onClose={closeTab}
            stripData={{ ...WINDOW_DRAG, "data-ws-tab-strip": "" }}
            tail={switches}
            frontAct
            bodyClass="relative isolate flex flex-col"
            // This strip heads the middle column, between the tree and
            // the sessions column — not the region (第十三轮 第 5 条's
            // board is the sessions landing's), so the file lies flat
            // under it on the card's colour.
            frame={false}
          >
          {commit !== null ? (
            <CommitPage key={commit.id} machine={machine} root={root} commit={commit} presentation="tab" state={commitState(commit.id)} onBack={() => {}} />
          ) : file === null ? (
            // 中栏没有标签时 (ruling 37). Not a blank: what was open last
            // time. Where this is and what branch it is on are R0's
            // since 2026-09-28 — said once (`WorkspaceEmpty`'s head).
            <WorkspaceEmpty root={root} recent={recent} onOpen={(p) => open(p, false)} />
          ) : diffOf(file) !== null ? (
            // 点状态字母开出来的那一张 (ruling 38). Read-only, and it
            // is remounted per file the way `FileView` is.
            <DiffView
              key={`${machine} ${file}`}
              machine={machine}
              path={diffOf(file) ?? ""}
              narrow={narrow}
            />
          ) : rev !== null ? (
            <RevisionView machine={machine} path={file} rev={rev} compare={revCompare} face={face} narrow={narrow} />
          ) : (
            <FileView
              handoff={handoff}
              key={`${machine} ${file}`}
              machine={machine}
              path={file}
              face={face}
              // A save changes which files differ from HEAD, and the
              // tree's marks are held here rather than in the view that
              // did the saving (工作区批4). Asked again rather than
              // guessed at: one edit can move a file from M to nothing,
              // and a mark this pane patched by hand would be a second
              // opinion about a fact git already has.
              onSaved={() => {
                setGitAt((n) => n + 1);
                readFirst.saved();
              }}
              reading={readFirst.reading}
              onCanEdit={readFirst.onCanEdit}
              // The one sentence that outlived the rows under the strip
              // (第十四轮 第 2 条), worn by this file's own tab.
              onNote={setFileNote}
              // **Asked to save from outside**, by the sheet that
              // catches a tab being closed over a draft. A counter,
              // because two asks are two saves.
              saveAt={saveAt}
              // **Asked to read itself again on the way back into the
              // window** (工作区批⑩), the same trigger the marks ride.
              readAt={readAt}
              // A line a press in a conversation asked for (ruling
              // 35), only ever for the file it named.
              goLine={goLine !== null && goLine.path === file ? goLine : null}
              draft={drafts.get(file)}
              editorHistory={historyOf(file)}
              onDraft={draftOf(file)}
            />
          )}
          </WorkspaceContentFrame>
          </div>
        </div>
        )}
        {picked !== null && (narrow ? screen === "sessions" : sessionDetailOpen) && (
          <ListPane mark="data-ws-sessions" name={gui.workspace_sessions} mountSeamFirst side="right"
            pane={narrow ? null : sidePane} narrow={narrow} seam={{ "data-ws-seam": "sessions" }} scroll={false}
            className={cn(narrow ? "min-w-0 flex-1" : "max-w-[calc(100%-var(--ws-mid-min))] shrink-0",!narrow&&contentSide.open&&"min-w-[min(calc(2*var(--session-side-min)),calc(100%-var(--ws-mid-min)))]")}>
            <SessionFrame mobile={narrow} contentMode={narrow&&contentSide.open} grouped faces={sessionFaces} active={picked} onFocus={selectSession} onClose={closeSessionTab}
              onNew={newSession} onReorder={sessionTabs.reorder} stripData={{'data-session-tabs':'','data-workspace-session-tabs':''}}
              onBack={()=>{showSessionDetail(false);setSidebarView('sessions');if(narrow)setScreen('tree');}}
              onTitle={()=>{if(picked)openSessionContent(picked);}}
              aside={!narrow&&selectedSession&&sessionContent&&contentSide.open?<SessionContentColumn row={selectedSession}
                context={sessionContent} faces={sessionFaces} onSelectSession={selectSession} onCloseSession={closeSessionTab} onNewSession={newSession}/>:undefined}>
              <SessionLinkScope row={selectedSession}><DetailPane row={selectedSession} narrow={narrow} openIds={sessionTabs.tabs} sessions={rows}
                onSession={id=>{sessionTabs.open(id);showSessionDetail(true);}}
                onBack={() => { showSessionDetail(false); setSidebarView("sessions"); if (narrow) setScreen("tree"); }} /></SessionLinkScope>
              {narrow&&contentSide.open&&sessionContent&&contentScope&&<MobileContentSheet onClose={contentScope.onClose}>
                <SessionContentSeat dock={sessionContent.dock} scope={contentScope}/>
              </MobileContentSheet>}
            </SessionFrame>
          </ListPane>
        )}
      </div>
      {/* The editor's consent and refusal (`components/CodeWindow`). */}
      {codeWin.dialog}
      {/* One sheet for the whole strip, not one per tab: what is being
          asked about is state, and a dialog per tab would be one
          mounted component per open file. */}
      <CloseDraftDialog
        name={closing === null ? null : basename(closing)}
        onKeep={() => setClosing(null)}
        onDrop={() => closing !== null && drop(closing)}
        onSave={() => {
          if (closing === null) return;
          // **Asks the view that holds the buffer to save**, rather
          // than writing from here: the save is pinned to a digest that
          // view is holding, and a second writer with its own idea of
          // that digest is exactly the clash the save contract exists
          // to avoid.
          //
          // **The sheet goes now and the tab does not.** What happens
          // next has a face of its own — the file's own strip says
          // 存好了 or says why not, and carries the three ways out of a
          // refusal — and a modal held open over that would be khor
          // asking a second time about something it is already showing.
          setFile(closing);
          setWantSave(closing);
          setWaiting({ path: closing, ask: false });
          setClosing(null);
        }}
      />

    </div>
    </DeeperHeads>
    </RegionLead.Provider>
    </LinkHomeProvider>
  );
}

function targetOf(key: string): WorkspaceTarget {
  const path = diffOf(key);
  return path === null ? { kind: "file", path: key } : { kind: "diff", path };
}
function keyOf(target: Exclude<WorkspaceTarget, { kind: "commit" }>): string {
  return target.kind === "file" ? target.path : diffKey(target.path);
}

/** How long the box waits after the last keystroke before it asks
    (2026-09-17 ruling 47). **A walk of somebody's project per
    keystroke is three walks for two answers nobody reads**, and a
    quarter of a second is under the threshold where a person notices
    waiting at all. */
const HUNT_WAIT = 300;

/** What the tree's column shows while there is something in the box
    (2026-09-17 ruling 47).
 *
 * **Names first, then lines**, because they answer two different
 * questions and the first one is the one a person asks most: 「where is
 * the file called this」 comes before 「where is this written」.
 *
 * It replaces the tree rather than standing beside it: the column is
 * one screen wide on a phone and a third of one on a desktop, and a
 * result list under a tree would be a scroller inside a scroller.
 *
 * **On the tree's lines** (UX-40 §5): a name's kind icon and name stand
 * where the tree's first level draws them (`--tree-inset`, the tree's own
 * icon box), and a line hit and the sentences stand on the page's left
 * line (`--page-inset`) — the box above starts there too. ⚠️ 墓碑: 12 in
 * from the column's edge, 8 short of the list's line.
 */
export function Hits({
  found,
  error,
  query,
  dir,
  here,
  onOpen,
}: {
  found: Found | null;
  error: string | null;
  query: string;
  /** What the paths are shown relative to — the whole point of showing
      them at all is telling two `mod.rs` apart. */
  dir: string;
  here: boolean;
  /** A row was pressed: open that file, and on a line hit go to that
      line — the road a path in a conversation already takes (ruling
      35). */
  onOpen: (path: string, line: number | null, keep?:boolean) => void;
}) {
  const actions = useFileObjectActions(here, dir);
  const short = (p: string) => (p.startsWith(`${dir}/`) ? p.slice(dir.length + 1) : p);
  return (
    <div data-tree-results className="min-h-0 flex-1 overflow-y-auto">
      {error !== null && (
        // The machine's own sentence, at the size of something meant to
        // be read — `FileTree`'s rule for a branch that refused.
        <div data-hits-refused className="px-(--page-inset) py-4 text-body">
          {error}
        </div>
      )}
      {found !== null && found.names.length === 0 && found.lines.length === 0 && (
        <div data-hits-none className="px-(--page-inset) py-4 text-callout text-muted-foreground">
          {gui.search_none}
        </div>
      )}
      {found?.names.map((path: string) => (
        <RowMenu key={path} items={actions(path)}>
        <button
          type="button"
          data-hit-name={path}
          onClick={() => onOpen(path, null)}
          onDoubleClick={() => onOpen(path,null,true)}
          className="flex min-h-[var(--tree-row)] w-full items-center gap-2 ps-(--tree-inset) pe-(--page-inset) text-left text-callout hover:bg-muted/50"
        >
          <FileKindIcon
            name={basename(path)}
            className="h-[var(--tree-slot)] w-[var(--tree-slot)] flex-none items-center ps-[calc(var(--tree-slot)/2-var(--affordance)/2+var(--tree-icon-nudge))] [&>svg]:!size-4"
          />
          <span className="min-w-0 flex-1 truncate">{marked(short(path), query)}</span>
        </button>
        </RowMenu>
      ))}
      {found?.lines.map((hit: SearchHit) => (
        <RowMenu key={`${hit.path}:${hit.line}`} items={actions(hit.path)}>
        <button
          type="button"
          data-hit-line={`${short(hit.path)}:${hit.line}`}
          onClick={() => onOpen(hit.path, hit.line)}
          onDoubleClick={() => onOpen(hit.path,hit.line,true)}
          className="flex min-h-[var(--tree-row)] w-full flex-col justify-center gap-0.5 px-(--page-inset) py-1 text-left hover:bg-muted/50"
        >
          {/* 「显示 `path:line` 与那一行文字」 — where it is above what
              it says, because the second line is what a person reads to
              decide and the first is what they read to place it. */}
          <span data-hit-where className="truncate text-caption text-muted-foreground">
            {short(hit.path)}:{hit.line}
          </span>
          <span data-hit-text className="truncate font-mono text-caption">
            {marked(hit.text.trim(), query)}
          </span>
        </button>
        </RowMenu>
      ))}
      {found?.more === true && (
        <div data-hits-more className="px-(--page-inset) py-2 text-caption text-muted-foreground">
          {cli.search_more}
        </div>
      )}
    </div>
  );
}

/** One line with the words somebody typed picked out of it.
 *
 * **Case-insensitive and plain text, which is what was searched for**
 * (`khor_node::files::search`): a highlight that used a pattern where
 * the search used a substring would draw a mark in a place the answer
 * did not come from.
 */
function marked(text: string, query: string): ReactNode {
  const at = text.toLowerCase();
  const needle = query.toLowerCase();
  if (needle === "") return text;
  const out: ReactNode[] = [];
  let from = 0;
  for (;;) {
    const hit = at.indexOf(needle, from);
    if (hit < 0) break;
    if (hit > from) out.push(text.slice(from, hit));
    out.push(
      <mark key={hit} data-hit-mark className="bg-accent text-foreground">
        {text.slice(hit, hit + needle.length)}
      </mark>,
    );
    from = hit + needle.length;
  }
  if (out.length === 0) return text;
  if (from < text.length) out.push(text.slice(from));
  return out;
}

/** What this column calls itself in this face's storage — the pane
    name its ticked filter words are filed under (工作区批⑩). One name
    for every workspace, not one per workspace: see `words`. */
const WS_PANE = "workspace-sessions";

/** What a diff tab is called on the strip (工作区第五轮 第 38 条).
 *
 * **A tab is a string here, so a diff needs a spelling no path can
 * have.** A NUL cannot be in a unix path — the kernel ends the name at
 * one — so a key that starts with one is a key nothing on any disk can
 * collide with. The alternative was a second list beside `tabs`, and
 * two lists that have to stay in step is the shape that goes wrong the
 * first time somebody closes the third tab.
 */
const DIFF = "\u0000diff:";

/** The strip key for「this file against HEAD」. */
export const diffKey = (path: string) => `${DIFF}${path}`;

/** The file a diff tab is about, or `null` for an ordinary tab. */
export function diffOf(key: string): string | null {
  return key.startsWith(DIFF) ? key.slice(DIFF.length) : null;
}

/** The file a tab is about, whichever kind it is — for everything that
    cares about the file and not about which face of it is open: the
    stored-tab ceiling, the kind icon, the second line. */
export const fileOf = (key: string) => diffOf(key) ?? key;

/** Where the open tabs are kept, per workspace (工作区批⑧).
    **Keyed by the pair that is the workspace's identity**
    (docs/WORKSPACE.md 身份与名字), the same key shape the tree's open
    branches use: the same checkout on two machines is two projects, and
    one's tabs under the other's name would point at another disk. */
function workspaceStorage(): Pick<Storage, "getItem" | "setItem"> | null {
  try { return window.localStorage; } catch { return null; }
}

/** The tabs this workspace was last left with. Paths outside the root
    are dropped whatever storage holds — the ceiling is the whole ruling
    (工作区批⑦), and a tab from somewhere else is one the tree cannot
    explain. Storage refusing reads as「nothing was open」, which is the
    shape a workspace opens in anyway. */
export function storedTabs(machine: string, root: string): string[] {
  return new WorkspaceNavigationStore(workspaceStorage, machine, root, targetOf).restore().content.items.filter((item) => item.kind !== "commit").map(keyOf);
}

/** Where a file sits relative to the workspace root — the tab's second
    line and its 复制相对路径. Falls back to the absolute path where
    there is no root to be relative to (the browsing face), because a
    path is better than a blank line. */
function under(root: string, path: string): string {
  return root !== "" && path.startsWith(`${root}/`) ? path.slice(root.length + 1) : path;
}

/** Absolute path → its mark, for every marked file **and every
    directory above it** inside the workspace.

    **A directory wears `·`, not a letter.** git has no opinion about
    directories — it tracks files — so any letter on a folder would be
    khor inventing a claim. What the dot says is the only thing that is
    true and the only thing worth walking for: something under here
    differs from HEAD. The letter waits until you get to the file.

    **The letter stored against a directory is the heaviest one below
    it** (工作区批⑦: C > D > M > A/U, `fileRow.heavier`); the row turns
    that into the dot and paints the dot in its ink. Stored rather than
    painted here because two different trees draw these rows — this
    hands over the fact, and 「a directory draws it as a dot」 stays one
    decision in one place.

    Null when there is no answer, which the tree paints as no marks at
    all. Since 工作区批⑦ that is the whole of it: no sentence either. */
function folded(root: string, git: GitStatus | null): Map<string, string> | null {
  if (git === null) return null;
  const out = new Map<string, string>();
  for (const r of git.rows) {
    const full = `${root}/${r.path}`;
    out.set(full, r.mark);
    // **An ignored path is the one mark that does not travel up**
    // (2026-09-16). The fold exists so a shut directory can say
    // 「something under here differs」; `.gitignore` covering a file
    // says the opposite, and a `src` whose only marked child was a
    // build output would go grey — a claim about the directory that
    // nothing made. The node folds the same way
    // (`khor_node::git::fold`), so the two ends agree.
    if (r.mark === IGNORED) continue;
    // Every step between the file and the root, exclusive of both: the
    // root itself is the head of the pane and never a row in it.
    let cut = full.lastIndexOf("/");
    while (cut > root.length) {
      const dir = full.slice(0, cut);
      const had = out.get(dir);
      out.set(dir, had === undefined ? r.mark : heavier(had, r.mark));
      cut = full.lastIndexOf("/", cut - 1);
    }
  }
  return out;
}
