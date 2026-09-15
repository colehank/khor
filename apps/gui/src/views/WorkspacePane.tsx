// One workspace, open (工作区批2). Three columns under one head: the
// tree, the file, and the sessions living in this project.
//
// **Both side columns are switched from the head, never from
// themselves.** A column that is collapsed cannot carry the control
// that brings it back, so the pair lives in a place that does not move.
// Left switches the tree, right switches the sessions — the two ends
// they act on.
//
// **Why the sessions column is the one that yields.** Counted across
// the shell: rail, workspace list, tree, file, sessions is five columns
// side by side, and nothing readable survives that. The sessions have a
// whole landing of their own on the rail; the tree and the file have no
// second address. So when the workspace list is open, this opens with
// the sessions folded, and opening them is one press away.
import { useCallback, useEffect, useMemo, useState } from "react";

import {
  onWebFace,
  fetchCodeHere,
  fetchGitStatus,
  openCode,
  stopCode,
  type GitStatus,
  type SessionRow,
} from "@/api";
import { CloseDraftDialog } from "@/components/CloseDraftDialog";
import { IconCode, IconFile, IconFiles, IconSessions } from "@/components/icons";
import { PathCrumbs } from "@/components/PathCrumbs";
import { type RowAction } from "@/components/RowMenu";
import { Button } from "@/components/ui/button";
import { TabStrip, type TabFace } from "@/browse/TabStrip";
import { gui } from "@/gen/catalog";
import { useHidden } from "@/hooks/use-hidden";
import { useListWidth } from "@/hooks/use-list-width";
import { cn } from "@/lib/utils";
import { basename } from "@/words";
import { BrowserPane } from "@/views/BrowserPane";
import { DetailPane } from "@/views/DetailPane";
import { SessionsList } from "@/views/SessionsList";
import { FilesPane } from "@/views/FilesPane";
import { FileTree } from "@/views/FileTree";
import { heavier } from "@/views/fileRow";
import { FileView, type Draft } from "@/views/FileView";
import type { Editor } from "@/gen/bindings/Editor";

export function WorkspacePane({
  machine,
  device,
  here,
  root,
  rows,
  channel,
  onPin,
  onClose,
  pinFailed,
  closeRefused,
  narrow,
  listOpen,
}: {
  machine: string;
  device: string;
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
  narrow: boolean;
  /** Whether the workspace list is holding a column of its own. The
      fifth column's existence is what decides the sessions' default,
      and only the shell knows about it. */
  listOpen: boolean;
}) {
  // Where the tree is standing. Starts at the workspace and walks down;
  // `root` changing is a different workspace, not a step.
  const [at, setAt] = useState(root);
  /** Every file open in the middle column, in the order they were
      opened, by absolute path. **Remembered per workspace in this
      face's own storage** (工作区批⑧) — a set of open tabs is a habit
      about one screen, the same class of thing as a column's width, and
      it belongs nowhere near the replicated documents.

      The tab's id on the strip is `机器:绝对路径`; what is kept here is
      the path alone, because the machine is this pane's and a stored id
      that disagreed with it would be a tab pointing at another disk. */
  const [tabs, setTabs] = useState<string[]>(() => storedTabs(machine, root));
  const [file, setFile] = useState<string | null>(() => tabs[0] ?? null);
  /** Which tab is the **preview**: opened by one click, and replaced by
      the next one rather than joined by it (VS Code's arrangement, and
      the user asked for VS Code's). `null` when every open tab is
      pinned. A double click, or typing in it, pins it. */
  const [preview, setPreview] = useState<string | null>(null);
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
      only thing that knows a save landed is the draft going away. */
  const [waiting, setWaiting] = useState<string | null>(null);
  /** Every file in this workspace with something typed in it and not
      saved, by absolute path.
      **Held here because the view is remounted on every file.** Until
      2026-09-12 clicking another row in the tree threw the typing away
      with no warning; now walking away and back finds it where it was,
      and the head says how many are waiting. */
  const [drafts, setDrafts] = useState<Map<string, Draft>>(new Map());
  /** What each file held before the last save made from this face, by
      absolute path. **Beside the drafts and for their reason**: the
      view is remounted on every file, and an undo that evaporates when
      you glance at the file next door is one nobody can lean on
      (`FileView`'s `undo`). */
  const [undos, setUndos] = useState<Map<string, string>>(new Map());
  // **Two seams of the workspace's own** (工作区批 收尾, 2026-09-12),
  // the same hook the list pane's seam uses — parameterised rather than
  // copied, so「drag, snap away, come back to the width it had, and the
  // arrow keys walk the same stops」is one implementation and not three
  // that drift.
  //
  // The head switches stay: a collapsed column cannot carry the control
  // that brings it back, and a seam at width zero is a two-pixel strip.
  // **Both ways of putting a column away write the same `collapsed`**,
  // so they are one state with two gestures rather than two states.
  const treePane = useListWidth(narrow, "ws-tree", "left", "[data-ws-tree]");
  const sidePane = useListWidth(narrow, "ws-sessions", "right", "[data-ws-sessions]");
  const tree = !treePane.collapsed;
  const sessions = !sidePane.collapsed;
  const setTree = (on: boolean) => (on === treePane.collapsed ? treePane.toggle() : undefined);
  const setSessions = (on: boolean) =>
    on === sidePane.collapsed ? sidePane.toggle() : undefined;
  /** Which session this column is standing in, by id, or `null` for
      the list. **Dropped when the workspace changes** along with the
      tree's position and the open file: a session belongs to one
      project, and carrying it across would leave somebody looking at a
      conversation about a directory that is no longer on screen. */
  const [picked, setPicked] = useState<string | null>(null);
  const [git, setGit] = useState<GitStatus | null>(null);
  /** Bumped to ask the marks again. A counter for the same reason the
      file view's is one: two saves are two questions. */
  const [gitAt, setGitAt] = useState(0);
  const hidden = useHidden();
  /** What that machine says about its editor. `version` empty means it
      has none **or could not be asked** — the two lead to the same next
      move, which is to draw no switch (`khor_gui_core::code::here`). */
  const [code, setCode] = useState<Editor>({ version: "", ready: false });
  /** The URL to point a webview at, once one has been asked for.
      `null` while khor's own file view is what the middle column shows,
      which is always how a workspace opens. */
  const [codeAt, setCodeAt] = useState<string | null>(null);
  /** The consent card is up. Asked once per machine and remembered in
      this face's own storage — **the thing consented to happens on the
      far machine, so it is keyed by that machine**, and a second face
      asking again is right rather than a bug: the person at that face
      has not agreed to anything yet. */
  const [asking, setAsking] = useState(false);
  const [codeError, setCodeError] = useState<string | null>(null);

  // **Asked once per workspace, not once per step of the tree.** The
  // answer covers the whole checkout and does not change while a person
  // walks it; re-asking on every directory would spend 54 ms of somebody
  // else's machine to redraw marks that did not move.
  //
  // **A refusal is not kept and not said** (工作区批⑦). 「文件打开不是
  // git 就不用显示,不必显示这个『这个路径不在 git 仓库里』」— the user,
  // 2026-09-15. Until then the sentence the node sent back lived beside
  // the branch, which meant a directory that is simply not a checkout
  // wore a line of apology every time it was opened. Both roads end in
  // `git === null`, the tree paints no marks, and that is the whole of
  // what khor has to say: 「不是 checkout」 and 「读不了」 look the same
  // on the glass because neither of them is a fact about a file.
  useEffect(() => {
    setGit(null);
    if (root === "") return;
    let live = true;
    fetchGitStatus(machine, root)
      .then((g) => live && setGit(g))
      .catch(() => live && setGit(null));
    return () => {
      live = false;
    };
  }, [machine, root, gitAt]);

  // **Asked again on the way back into the window, and never on a
  // timer** (工作区批7). The marks answer 「what differs from HEAD on
  // that machine」, and they are re-asked when the pane opens, when the
  // tree walks, and after khor's own save — which covers every change
  // khor made. What it did not cover is the change somebody *else*
  // made while this window sat in the background, and that is the one
  // a person comes back to and reads as current.
  //
  // A poll was the other road and it is the worse one: every beat is a
  // dial plus a `git status` walk on somebody else's machine, for an
  // answer that is almost always the one already on screen. Coming
  // back to the window is the moment the staleness starts to mislead,
  // so it is the moment to ask. A repository still changes under a
  // pane somebody is staring at without touching; that is left
  // unasked on purpose rather than paid for once every few seconds.
  useEffect(() => {
    if (hidden) return;
    setGitAt((n) => n + 1);
  }, [hidden]);

  // **Given back when this pane stops pointing at that machine.** A
  // web face is a `node` process with an extension host on somebody
  // else's box; leaving the workspace used to leave it running with
  // nothing on any screen pointed at it, which is the shape
  // `held.ts` was written for on the borrow side. Here the ledger is
  // one entry deep — one pane, one machine — so the cleanup is the
  // effect's own.
  useEffect(() => {
    if (codeAt === null) return;
    return () => {
      stopCode(machine).catch(() => {
        // The pipe on this side is already dropped by the node; a far
        // machine that could not be told is a server that outlives this
        // by one `khor code --stop`, not a hang.
      });
    };
  }, [machine, codeAt]);

  useEffect(() => {
    let live = true;
    setCode({ version: "", ready: false });
    setCodeAt(null);
    setAsking(false);
    setCodeError(null);
    fetchCodeHere(machine)
      .then((e) => live && setCode(e))
      // Already folded to the default on the way here; this arm is for
      // the call itself failing, and it means the same thing.
      .catch(() => live && setCode({ version: "", ready: false }));
    return () => {
      live = false;
    };
  }, [machine]);

  // **Only the sessions column's default lives here now** (工作区批⑧).
  // It used to also throw away the open file, both draft maps and the
  // tree's position, on `[root, machine, narrow, listOpen]` — and the
  // first two of those never change, because the shell remounts this
  // pane per workspace (`App`'s `key`). What did change was the other
  // two: dragging the window across the narrow breakpoint, or opening
  // the workspace list, silently discarded every unsaved draft in the
  // project. Nothing reported it, because the count that would have
  // said so was in the same render that had just been reset.
  useEffect(() => {
    // Narrow has no room for a second column at all, so the default
    // there is not about the fifth one.
    setSessions(!narrow && !listOpen);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [narrow, listOpen]);

  /** Remembered per machine, in this face's own storage. */
  const agreedKey = `khor.code.agreed.${machine}`;
  const agreed = (): boolean => {
    try {
      return window.localStorage.getItem(agreedKey) === "1";
    } catch {
      // Storage refused (a private window, a locked profile). **Then
      // nobody has agreed** — the safe answer is to ask again, not to
      // assume the yes we cannot read.
      return false;
    }
  };
  /** Pressing the switch: ask first if this machine has never been
      agreed to, otherwise go straight there.
      **The server already being over there is the other way past the
      card**, and it is a stronger reason than the remembered yes: the
      card's whole sentence is「第一次要下约 640 MB」, and on a machine
      that has the server that sentence is false. It was shown anyway
      until 2026-09-13, because this face only ever consulted its own
      storage — so a fresh app profile asked to download a server that
      had been on disk for two days, and the licence the card also names
      was accepted when those bytes landed. */
  const want = () => {
    setCodeError(null);
    if (code.ready || agreed()) {
      go();
    } else {
      setAsking(true);
    }
  };
  const go = () => {
    setAsking(false);
    setCodeError(null);
    try {
      window.localStorage.setItem(agreedKey, "1");
    } catch {
      // The launch still happens; only the memory of the yes is lost,
      // and the cost of that is being asked again.
    }
    openCode(machine, root)
      .then((a) => setCodeAt(a.url))
      .catch((e: unknown) => setCodeError(String(e instanceof Error ? e.message : e)));
  };

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
  // the accident of the fold having stopped exactly there. `place` is
  // the raw cwd now precisely so this question can be asked
  // (`khor_node::live::place_of`). The separator is what keeps
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
    const under = root.endsWith("/") ? root : `${root}/`;
    return rows
      .filter(
        (r) =>
          r.home === channel &&
          r.place !== null &&
          (r.place === root || r.place.startsWith(under)),
      )
      .map((r) => ({ ...r, group: "" }));
  }, [root, rows, channel]);
  const marks = useMemo(() => folded(root, git), [root, git]);

  /** Remember which tabs are open, for this workspace. Storage refusing
      costs the memory and nothing else — the tabs are on screen either
      way (`use-list-width`'s bargain). */
  const remember = useCallback(
    (open: string[]) => {
      try {
        window.localStorage.setItem(tabsKey(machine, root), JSON.stringify(open));
      } catch {
        /* the tabs are still open; only finding them again tomorrow is lost */
      }
    },
    [machine, root],
  );

  /** A file was pressed in the tree.
   *
   * **One click opens a preview, the next one replaces it** (工作区批⑧,
   * VS Code's arrangement, which is what was asked for). The preview is
   * a real tab in every way but one: it is the tab the next glance is
   * allowed to take. A double click pins it, and so does typing in it —
   * both are a person saying they are staying.
   */
  const open = useCallback(
    (path: string, pinned: boolean) => {
      setFile(path);
      setTabs((was) => {
        if (was.includes(path)) return was;
        // The preview seat, if there is one, is where a fresh glance
        // goes; otherwise the tab joins the end.
        const now =
          !pinned && preview !== null && was.includes(preview)
            ? was.map((p) => (p === preview ? path : p))
            : [...was, path];
        remember(now);
        return now;
      });
      setPreview((was) => (pinned ? (was === path ? null : was) : path));
    },
    [preview, remember],
  );

  /** Take a tab off the strip. **Never asks here** — whether there is
      anything to ask about is `shut`'s business one level up, and this
      is what both of its answers end in. */
  const drop = useCallback(
    (path: string) => {
      setTabs((was) => {
        const at = was.indexOf(path);
        if (at < 0) return was;
        const now = was.filter((p) => p !== path);
        remember(now);
        // **The neighbour, not the first tab.** Closing the third of
        // five and landing on the first is a jump a person has to
        // undo; the one that slid into this seat is where the eye
        // already is.
        setFile((open) => (open === path ? (now[at] ?? now[at - 1] ?? null) : open));
        return now;
      });
      setPreview((was) => (was === path ? null : was));
      setDrafts((was) => {
        if (!was.has(path)) return was;
        const now = new Map(was);
        now.delete(path);
        return now;
      });
      setUndos((was) => {
        if (!was.has(path)) return was;
        const now = new Map(was);
        now.delete(path);
        return now;
      });
      setClosing((was) => (was === path ? null : was));
    },
    [remember],
  );

  /** Close a tab, asking first when something is typed in it.
      **The dialog is the only place the draft can be saved from**, and
      it is raised rather than the close being refused: a refusal would
      leave a person pressing × over and over with nothing telling them
      why (docs/UX.md 做了但没变化). */
  const shut = useCallback(
    (path: string) => {
      if (drafts.has(path)) setClosing(path);
      else drop(path);
    },
    [drafts, drop],
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
    if (waiting === null || drafts.has(waiting)) return;
    setWaiting(null);
    drop(waiting);
  }, [waiting, drafts, drop]);

  // ⌘W / Ctrl+W, the one shortcut this strip has. **Both modifiers**,
  // because one build runs on a mac app, a phone's browser and a
  // Windows desktop (`FileView`'s save shortcut says the same).
  //
  // **Only while a file is in front.** The editor and the browsing
  // shell are other things with other tabs, and a shortcut that fired
  // over the top of them would close something nobody was looking at.
  useEffect(() => {
    if (file === null || codeAt !== null) return;
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && (e.key === "w" || e.key === "W")) {
        e.preventDefault();
        shut(file);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [file, codeAt, shut]);

  /** What the strip draws. **Built here because the facts are this
      pane's** — which files are open, which one is in front, which one
      has something typed in it — and `TabStrip` paints and decides
      nothing (its own head says so). */
  const tabFaces: TabFace[] = useMemo(
    () =>
      tabs.map((path) => ({
        id: `${machine}:${path}`,
        label: basename(path),
        face: null,
        path,
        machine,
        // Relative to the workspace root, which is the half that tells
        // two tabs called `mod.rs` apart. The absolute path is the
        // hover title's, and both are on the menu.
        sub: under(root, path),
        dirty: drafts.has(path),
        preview: preview === path,
        glyph: <IconFile />,
        menu: tabMenu(path),
      })),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [tabs, machine, root, drafts, preview, shut],
  );

  /** 关闭 / 关闭其他 / 复制路径 / 复制相对路径.
      **Two copy items and not one with a switch**: the absolute path is
      what a command line and another machine take, the relative one is
      what this project's own documents take, and neither stands in for
      the other. A clipboard refusal rejects, which is how `RowMenu`
      keeps the menu open over the press that failed. */
  function tabMenu(path: string): RowAction[] {
    return [
      { key: "close", label: gui.close, run: () => shut(path) },
      {
        key: "close-others",
        label: gui.tab_close_others,
        run: () => {
          for (const p of tabs) if (p !== path) shut(p);
          setFile(path);
        },
      },
      {
        key: "copy-path",
        label: gui.tab_copy_path,
        run: () => navigator.clipboard.writeText(path),
      },
      {
        key: "copy-rel",
        label: gui.tab_copy_rel,
        run: () => navigator.clipboard.writeText(under(root, path)),
      },
    ];
  }

  return (
    // **`h-full`, not `flex-1`.** The slot this lands in (App.tsx) is a
    // block, not a flex container, so `flex-1` resolves to nothing and
    // the pane sizes to its own content — which for a long file meant a
    // 3700px pane inside a 1026px window, the whole app scrolling as one
    // page and the head and tree scrolling away with it. Measured on a
    // real file, 2026-09-12. Every other detail pane already wears
    // `h-full` for this reason (`DetailPane`, `BrowseLanding`).
    <div className="flex h-full min-h-0 flex-col">
      {/* **The head row is gone** (工作区批⑧). It held the workspace's
          name, its branch, how far the branch was from its remote, a
          count of unsaved files and a breadcrumb — and every one of
          those moved somewhere that already answers for it: the name
          and the branch are two corners of this workspace's row in the
          list beside it (docs/WORKSPACE.md 一行的四个角), 「N 个文件
          未存」 became a dot on each tab that has one, and the crumbs
          were cut by the ruling. What is left of the head is the strip.

          **Except on the browsing face.** `root === ""` is what
          看它的文件 hands over — a whole machine's disk, where 「where
          am I」 is the question and the address bar is the answer — so
          that face keeps a bar with the machine's name and its crumbs,
          and a project gets none. */}
      {root === "" && (
        <div className="flex items-center gap-2 border-b px-3 py-2">
          <span className="shrink-0 truncate text-fact font-title text-foreground">
            {machine}
          </span>
          <PathCrumbs at={at} onGo={setAt} onEdit={() => setAt(at)} />
        </div>
      )}

      <div className="flex min-h-0 flex-1">
        {tree && (
          <div
            data-ws-tree
            className="flex shrink-0 flex-col border-r"
            style={{ width: treePane.width }}
          >
            {/* **Two trees, one ruling apart** (工作区批⑦). A project
                gets `FileTree`: rooted, expandable, with no way above
                itself. A machine's whole disk (`root === ""`, what
                看它的文件 opens) keeps `FilesPane`, where 上一级 and the
                address bar are the point — docs/WORKSPACE.md 第二轮 says
                so in the user's own terms. */}
            {root === "" ? (
              <FilesPane
                machine={machine}
                device={device}
                path={at}
                narrow={narrow}
                onPath={setAt}
                onLanded={setAt}
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
                onGone={drop}
              />
            )}
          </div>
        )}
        {/* The seam, drawn the way the list pane's is (App.tsx): a
            two-pixel strip with a hit margin overhanging the border,
            reachable by keyboard, lighting up under a pointer so it can
            be found before it is grabbed. */}
        {tree && !narrow && (
          <div
            data-ws-seam="tree"
            role="separator"
            aria-orientation="vertical"
            tabIndex={0}
            {...treePane.aria}
            {...treePane.handleProps}
            className="relative z-10 -mx-1 w-2 shrink-0 cursor-col-resize transition-colors outline-none hover:bg-border focus-visible:bg-ring"
          />
        )}
        <div className="flex min-w-0 flex-1 flex-col">
          {/* **One tab per open file, and the three switches at the far
              end of it** (工作区批⑧, the user's own layout: 「标签页的
              最右侧,显示现有的文件、session 和 vscode 切换」).

              The strip is `browse/TabStrip` itself, not a second
              drawing of one: a tab is a tab, and two components that
              both paint tabs is the failure `PaneBar` warns about one
              landing over. What this face hands it that a browsing tab
              does not have is the second line, the dot and the preview
              slope; what it does not hand it is 「+」, because a tab
              here is a file somebody pressed in the tree and there is
              nothing a bare new tab could open.

              **Drawn even with nothing open**, because the switches
              live on it: a strip that disappeared when the last tab
              closed would take the only way back to the tree with it —
              exactly the trap the old head was written to avoid ("a
              column that is collapsed cannot carry the control that
              brings it back"). */}
          <div data-ws-tab-strip className="flex flex-none flex-col">
          <TabStrip
            faces={tabFaces}
            active={file === null ? "" : `${machine}:${file}`}
            onFocus={(id) => setFile(id.slice(machine.length + 1))}
            onClose={(id) => shut(id.slice(machine.length + 1))}
            tail={
              <div data-ws-switches className="flex flex-none items-center gap-0.5">
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={gui.workspace_tree}
                  data-on={tree}
                  onClick={() => setTree(!tree)}
                  // The floor is worn here rather than by the variant:
                  // an icon button is 30 square by default and a finger
                  // needs `--touch` (tokens.css). Extended hit area,
                  // same drawing.
                  className={cn("min-h-touch min-w-touch", !tree && "text-muted-foreground")}
                >
                  <IconFiles />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={gui.workspace_sessions}
                  data-ws-sessions-switch
                  data-on={sessions}
                  onClick={() => setSessions(!sessions)}
                  className={cn("min-h-touch min-w-touch", !sessions && "text-muted-foreground")}
                >
                  <IconSessions />
                </Button>
                {/* **Only where there is an editor to open.** A machine
                    without one, and a machine that could not be asked,
                    both answer empty and both get no switch — offering
                    a button that fails is worse than offering nothing
                    (`khor_gui_core::code`).

                    **Never on the web face**, and that is a fact about
                    where the pipe lands rather than a missing feature.
                    The tunnel in front of somebody's editor is bound to
                    127.0.0.1 on the machine running the resident serve;
                    a phone reading khor's web face is not that machine,
                    so the link would name a loopback it cannot reach.

                    **A glyph now, not two words** (「vscode 换成 icon
                    显示」— the user, 2026-09-15). It sits beside two
                    other glyphs that switch a column, so a word here
                    read as a different kind of control; the two words
                    it wore are its name out loud instead, which is
                    where the other two keep theirs as well. */}
                {code.version !== "" && root !== "" && !onWebFace && (
                  <Button
                    variant="ghost"
                    size="icon"
                    data-code={codeAt === null ? "off" : "on"}
                    data-on={codeAt !== null}
                    aria-label={codeAt === null ? gui.code_tab : gui.code_off}
                    title={code.version}
                    onClick={() => (codeAt === null ? want() : setCodeAt(null))}
                    className={cn(
                      "min-h-touch min-w-touch",
                      codeAt === null && "text-muted-foreground",
                    )}
                  >
                    <IconCode />
                  </Button>
                )}
              </div>
            }
          />
          </div>
          {asking ? (
            // **Two sentences and two words.** Both sentences are about
            // what happens to the far machine; neither is about how to
            // undo it, because a person reading a consent card is not
            // looking for that yet (zh.toml `code-consent`).
            <div
              data-code-consent
              className="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center"
            >
              <span className="max-w-prose text-fact whitespace-pre-line text-foreground">
                {gui.code_consent}
              </span>
              <div className="flex items-center gap-2">
                <Button variant="outline" size="sm" data-code-no onClick={() => setAsking(false)}>
                  {gui.code_consent_no}
                </Button>
                <Button size="sm" data-code-go onClick={go}>
                  {gui.code_consent_go}
                </Button>
              </div>
            </div>
          ) : codeError !== null ? (
            <div className="flex-1 p-4 text-aux text-muted-foreground">{codeError}</div>
          ) : codeAt !== null ? (
            // **A page window of khor's own, laid over this column.**
            // Not a browsing tab: every tab in that shell names an exit
            // machine and an editor has none (`BrowserPane`'s `direct`).
            <BrowserPane
              // **Keyed by the machine, not by its name.** The tab id is
              // this page's handle on the app side and a handle has to
              // hold still: `device` is the name the user can retype
              // (`EditableName`), and renaming a machine with its editor
              // open would mint a second page webview and leave the
              // first one hidden for the life of the app.
              tab={`code:${machine}`}
              machine={machine}
              device={device}
              url={codeAt}
              direct
              onOpened={() => {}}
              onWent={() => {}}
              onPopped={() => {}}
            />
          ) : file === null ? (
            <div className="flex-1" />
          ) : (
            <FileView
              key={`${machine} ${file}`}
              machine={machine}
              path={file}
              // A save changes which files differ from HEAD, and the
              // tree's marks are held here rather than in the view that
              // did the saving (工作区批4). Asked again rather than
              // guessed at: one edit can move a file from M to nothing,
              // and a mark this pane patched by hand would be a second
              // opinion about a fact git already has.
              onSaved={() => setGitAt((n) => n + 1)}
              // **Asked to save from outside**, by the sheet that
              // catches a tab being closed over a draft. A counter,
              // because two asks are two saves.
              saveAt={saveAt}
              draft={drafts.get(file)}
              onDraft={(d) => {
                setDrafts((was) => {
                  // A draft that matches the disk is not a draft — the
                  // view says so by handing back `null`, and that is
                  // what puts the tab's dot out again.
                  //
                  // **Nothing is written when nothing changed.** This
                  // is reported from an effect now (`FileView`), so a
                  // new Map every time would be a render every time,
                  // and a render that re-runs the effect that caused
                  // it (工作区批⑧).
                  const had = was.get(file);
                  if (d === null ? had === undefined : had === d) return was;
                  const now = new Map(was);
                  if (d === null) now.delete(file);
                  else now.set(file, d);
                  return now;
                });
                // 「在里面打了字,就钉住」— typing is a person saying
                // they are staying, exactly as a double click is.
                if (d !== null) setPreview((was) => (was === file ? null : was));
              }}
              undo={undos.get(file)}
              onUndo={(t) =>
                setUndos((was) => {
                  const now = new Map(was);
                  if (t === null) now.delete(file);
                  else now.set(file, t);
                  return now;
                })
              }
            />
          )}
        </div>
        {sessions && !narrow && (
          <div
            data-ws-seam="sessions"
            role="separator"
            aria-orientation="vertical"
            tabIndex={0}
            {...sidePane.aria}
            {...sidePane.handleProps}
            className="relative z-10 -mx-1 w-2 shrink-0 cursor-col-resize transition-colors outline-none hover:bg-border focus-visible:bg-ring"
          />
        )}
        {sessions && (
          <div
            data-ws-sessions
            // **Scrolls as a list, does not as a pane.** `DetailPane`
            // is `h-full` with its own scroller inside; wrapped in a
            // second one it sizes to its content and the header stops
            // being sticky.
            className={cn(
              "flex shrink-0 flex-col border-l",
              picked === null ? "overflow-y-auto" : "min-h-0",
            )}
            style={{ width: sidePane.width }}
          >
            {/* **A place to work, not a list of what exists**
                (2026-09-13). Until now this column named the sessions
                running on this project and did nothing with them — you
                read a title and a state word and then went to the
                sessions landing to actually say anything, which put the
                project's files on one screen and its conversations on
                another. Pressing a row now opens that session here,
                beside its own files.

                **Master and detail rather than a strip of tabs**, and
                that is a fact about the column's width: this seam drags
                as narrow as a person wants and its default is a third
                of what the tree and the file leave over, which seats
                one title comfortably and four never. A strip that
                cannot show its tabs is a switcher you have to switch
                to. The back arrow is the way out, the shell's own
                gesture on its narrow face. */}
            {picked === null ? (
              // **The sessions list itself, not a second drawing of
              // it** (工作区批③). This column used to hand-draw a title
              // and a state word, which is three of the five questions
              // missing and a second answer to 「what does a session
              // row look like」 — the failure `PaneBar`'s head warns
              // about, one list over. The only difference a project's
              // column earns is that it does not repeat the machine
              // (`showVia`), because the header above it just said it.
              //
              // Every row action comes with it. A column where a row
              // can be opened but not pinned or closed would be the
              // same rows with fewer verbs, which is the thing a person
              // has to learn twice.
              <SessionsList
                rows={mine}
                query=""
                words={[]}
                selected={picked}
                onSelect={(r) => setPicked(r.id)}
                onPin={onPin}
                onClose={onClose}
                pinFailed={pinFailed}
                closeRefused={closeRefused}
                showVia={false}
              />
            ) : (
              <DetailPane
                row={mine.find((r) => r.id === picked) ?? null}
                // **`narrow` is a fact about this column, not about the
                // window** (工作区批③). `DetailPane` draws its back
                // arrow and its own face only where 「the list is
                // genuinely off-screen」 — which in a single column is
                // true at every width, because this pane shows the list
                // *or* the detail and never both. Passing the shell's
                // `narrow` through left the wide face with a session
                // open and no way out of it.
                narrow
                onBack={() => setPicked(null)}
              />
            )}
          </div>
        )}
      </div>
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
          setWaiting(closing);
          setClosing(null);
        }}
      />
    </div>
  );
}

/** Where the open tabs are kept, per workspace (工作区批⑧).
    **Keyed by the pair that is the workspace's identity**
    (docs/WORKSPACE.md 身份与名字), the same key shape the tree's open
    branches use: the same checkout on two machines is two projects, and
    one's tabs under the other's name would point at another disk. */
const tabsKey = (machine: string, root: string) => `khor.ws-tabs.${machine}:${root}`;

/** The tabs this workspace was last left with. Paths outside the root
    are dropped whatever storage holds — the ceiling is the whole ruling
    (工作区批⑦), and a tab from somewhere else is one the tree cannot
    explain. Storage refusing reads as「nothing was open」, which is the
    shape a workspace opens in anyway. */
function storedTabs(machine: string, root: string): string[] {
  try {
    const raw = window.localStorage.getItem(tabsKey(machine, root));
    const list: unknown = raw === null ? [] : JSON.parse(raw);
    if (!Array.isArray(list)) return [];
    return list.filter(
      (p): p is string =>
        typeof p === "string" && (root === "" ? p.startsWith("/") : p.startsWith(`${root}/`)),
    );
  } catch {
    return [];
  }
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
