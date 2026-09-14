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
import { useEffect, useMemo, useState } from "react";

import {
  onWebFace,
  fetchCodeHere,
  fetchGitStatus,
  openCode,
  stopCode,
  type GitStatus,
  type SessionRow,
} from "@/api";
import { IconFiles, IconSessions } from "@/components/icons";
import { PathCrumbs } from "@/components/PathCrumbs";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { useHidden } from "@/hooks/use-hidden";
import { useListWidth } from "@/hooks/use-list-width";
import { cn } from "@/lib/utils";
import { ago, basename } from "@/words";
import { BrowserPane } from "@/views/BrowserPane";
import { DetailPane } from "@/views/DetailPane";
import { SessionsList } from "@/views/SessionsList";
import { FilesPane } from "@/views/FilesPane";
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
  const [file, setFile] = useState<string | null>(null);
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
  // Kept apart from `git` rather than folded into a null: no marks and
  // no answer are two different facts, and painting an unread
  // repository as「clean」is the one reading that must never be a
  // guess (`khor_gui_core::git`).
  const [gitError, setGitError] = useState<string | null>(null);
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
  useEffect(() => {
    setGit(null);
    setGitError(null);
    if (root === "") return;
    let live = true;
    fetchGitStatus(machine, root)
      .then((g) => live && setGit(g))
      .catch((e: unknown) => live && setGitError(String(e instanceof Error ? e.message : e)));
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

  useEffect(() => {
    setAt(root);
    setFile(null);
    // A different workspace is a different set of files. **Dropped
    // rather than carried**: a draft is pinned to a digest on one
    // machine's disk, and carrying it into another project would be
    // holding a save aimed at a file nobody is looking at.
    setDrafts(new Map());
    setUndos(new Map());
    setPicked(null);
    // Narrow has no room for a second column at all, so the default
    // there is not about the fifth one.
    setSessions(!narrow && !listOpen);
  }, [root, machine, narrow, listOpen]);

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

  return (
    // **`h-full`, not `flex-1`.** The slot this lands in (App.tsx) is a
    // block, not a flex container, so `flex-1` resolves to nothing and
    // the pane sizes to its own content — which for a long file meant a
    // 3700px pane inside a 1026px window, the whole app scrolling as one
    // page and the head and tree scrolling away with it. Measured on a
    // real file, 2026-09-12. Every other detail pane already wears
    // `h-full` for this reason (`DetailPane`, `BrowseLanding`).
    <div className="flex h-full min-h-0 flex-col">
      <div className="flex items-center gap-2 border-b px-3 py-2">
        <span className="shrink-0 truncate text-fact font-title text-foreground">
          {root === "" ? machine : basename(root)}
        </span>
        {/* **The branch, or nothing at all.** A detached HEAD has no
            name to say and a repository with no commit has none either;
            both arrive as an empty string, and 「没有就不显示」 is the
            rule for both. The refusal takes the same spot because it is
            the answer to the same question — this is the one place on
            the face with room for a sentence about git. */}
        {/* **Only when something is waiting**, and it is a count of
            files rather than a state of this one: the file in front of
            you says 未存 for itself, and this is the half that would
            otherwise be invisible — a draft two directories away. */}
        {drafts.size > 0 && (
          <span data-drafts={drafts.size} className="shrink-0 text-aux text-muted-foreground">
            {gui.drafts_waiting(drafts.size)}
          </span>
        )}
        {gitError !== null ? (
          <span className="shrink-0 truncate text-aux text-muted-foreground">{gitError}</span>
        ) : (
          git !== null && (
            <>
              {git.branch !== "" && (
                <span
                  data-branch={git.branch}
                  className="shrink-0 truncate text-aux text-muted-foreground"
                >
                  {git.branch}
                </span>
              )}
              {/* **How far from the remote, and never without when.**
                  khor opens no connection nobody asked for, so it
                  never fetches — which makes 领先/落后 a fact about
                  the last time somebody else did, and a three-week-old
                  pair the same shape as a fresh one. That is why 批3
                  cut it and this is what brings it back: the number
                  and its age are one sentence or neither is said.

                  Nothing at all when `fetched_ms` is 0. A repository
                  that never fetched has no date to put on the pair,
                  and a branch with no upstream answers 0/0 — which is
                  also what being in step answers, and both lead to the
                  same next move, which is to draw nothing. */}
              {git.fetched_ms > 0 && (git.ahead > 0 || git.behind > 0) && (
                <span
                  data-track={`${git.ahead}/${git.behind}`}
                  className="shrink-0 truncate text-aux text-muted-foreground"
                >
                  {git.ahead > 0 && git.behind > 0
                    ? cli.git_track(git.ahead, git.behind, ago(git.fetched_ms))
                    : git.ahead > 0
                      ? cli.git_ahead(git.ahead, ago(git.fetched_ms))
                      : cli.git_behind(git.behind, ago(git.fetched_ms))}
                </span>
              )}
              {/* The cap said out loud, for `Response::Dir::truncated`'s
                  reason: a tree that was cut and a tree that is clean
                  below this point look exactly the same. */}
              {git.more && (
                <span className="shrink-0 truncate text-aux text-muted-foreground">
                  {cli.git_status_more}
                </span>
              )}
            </>
          )
        )}
        <PathCrumbs at={at} onGo={setAt} onEdit={() => setAt(at)} />
        <div className="flex shrink-0 items-center gap-0.5">
          <Button
            variant="ghost"
            size="icon"
            aria-label={gui.workspace_tree}
            data-on={tree}
            onClick={() => setTree(!tree)}
            // The floor is worn here rather than by the variant: an
            // icon button is 30 square by default and a finger needs
            // `--touch` (tokens.css). Extended hit area, same drawing.
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
        </div>
        {/* **Only where there is an editor to open.** A machine without
            one, and a machine that could not be asked, both answer
            empty and both get no switch — offering a button that fails
            is worse than offering nothing (`khor_gui_core::code`).

            Two words for one control, the pin's rule: it says what
            pressing it will do, not which side it is on. */}
        {/* **Never on the web face**, and that is a fact about where
            the pipe lands rather than a missing feature. The tunnel in
            front of somebody's editor is bound to 127.0.0.1 on the
            machine running the resident serve; a phone reading khor's
            web face is not that machine, so the link would name a
            loopback it cannot reach. Offering a button that resolves to
            somewhere else's localhost is worse than offering none. */}
        {code.version !== "" && root !== "" && !onWebFace && (
          <Button
            variant="ghost"
            size="sm"
            data-code={codeAt === null ? "off" : "on"}
            title={code.version}
            onClick={() => (codeAt === null ? want() : setCodeAt(null))}
            className="min-h-touch min-w-touch shrink-0"
          >
            {codeAt === null ? gui.code_tab : gui.code_off}
          </Button>
        )}
      </div>

      <div className="flex min-h-0 flex-1">
        {tree && (
          <div
            data-ws-tree
            className="flex shrink-0 flex-col border-r"
            style={{ width: treePane.width }}
          >
            <FilesPane
              machine={machine}
              device={device}
              path={at}
              narrow={narrow}
              onPath={setAt}
              onLanded={setAt}
              onFile={setFile}
              open={file}
              here={here}
              marks={marks}
            />
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
              draft={drafts.get(file)}
              onDraft={(d) =>
                setDrafts((was) => {
                  const now = new Map(was);
                  // A draft that matches the disk is not a draft. The
                  // view says so by handing back `null`, and keeping an
                  // entry for it would make the head count files that
                  // have nothing waiting.
                  if (d === null) now.delete(file);
                  else now.set(file, d);
                  return now;
                })
              }
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
    </div>
  );
}

/** Absolute path → its mark, for every marked file **and every
    directory above it** inside the workspace.

    **A directory wears `·`, not a letter.** git has no opinion about
    directories — it tracks files — so any letter on a folder would be
    khor inventing a claim. What the dot says is the only thing that is
    true and the only thing worth walking for: something under here
    differs from HEAD. The letter waits until you get to the file.

    Null when there is no answer, which the tree paints as no marks at
    all — the same distinction the head makes above. */
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
      out.set(full.slice(0, cut), DOT);
      cut = full.lastIndexOf("/", cut - 1);
    }
  }
  return out;
}

/** What a directory wears. Not a catalog string: it is a glyph, and the
    catalog holds sentences a person reads. */
const DOT = "\u00b7";
