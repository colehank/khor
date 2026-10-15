/**
 * **The 附栏's 工作区 seat is the workspace's own tree** (UX-39 §5, the owner
 * 2026-10-06 22:00: 「工作区就不能出工作区外了，顶级不可返回目录就是该
 * 工作区」, and 「是不是有两套绘制目录的方法」— yes, there were: the
 * machine page's listing (`FilesPane`, folder icons, no git, 2026-08-17)
 * and the workspace landing's tree (`FileTree`, twisties, git's letters,
 * 工作区批⑦). The seat drew the first; it is the workspace, so it draws
 * the second). The ceiling is the workspace that holds the session's
 * directory (`holding`), or that directory itself when no workspace
 * does; git's letters come from one status read, folded up the tree the
 * way the landing folds them. A file opens in place as the file page
 * with the main area's deck (`FilePage`, UX-40 §4; it folds as the main
 * area's does, UX-43 §4), and ‹ there is
 * the way back to the tree — or to 最近, for the file a row of 最近 opened.
 * Where it stands — the file open — is said as it moves (`onSpot`) and
 * handed back when the 附栏 opens again (`spot`, UX-47).
 *
 * **Find and make** (UX-40 §1): the find row under the 附栏's head is the
 * landing tree's own box and ＋ — names and lines under the workspace
 * (`Hits`), 新建文件 / 新建目录 into the tree (`FileTree`'s `make`).
 *
 * ⚠️ 墓碑 (item 60 part 5 → STORE-BRIEF phase 1, the owner 2026-10-07:
 * 「工作区不显示"放进来的"」): **「放进来的」**, a row above the tree
 * opening the machine's `~/khor-drop/` as a tree of its own, the one door
 * out of the ceiling. Received files are 更多 → 存储's (`StorePage`).
 *
 * ⚠️ 墓碑 (UX-37 §6 → UX-40 §4): the bare file page, ‹ and the name in a
 * head at its top. ⚠️ 墓碑 (UX-37 §1): `MachineFiles` here, climbing to
 * the root and on to every workspace.
 */
import { useEffect, useRef, useState, type ReactNode } from "react";

import { fetchGitLog, fetchGitStatus, searchIn, type Found, type GitLog } from "@/api";
import { GlassCircle } from "@/components/Glass";
import { IconPlus } from "@/components/icons";
import { SideFind } from "@/components/SideFind";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { SeatHead } from "@/components/SeatHead";
import { gui } from "@/gen/catalog";
import type { GitCommit } from "@/gen/bindings/GitCommit";
import type { GitStatus } from "@/gen/bindings/GitStatus";
import type { WsSpot } from "@/hooks/use-session-side";
import { holds, onTreeChanged, sameMachine } from "@/lib/tree-changed";
import { heavier, IGNORED } from "@/views/fileRow";
import { FilePage } from "@/views/FilePage";
import { CommitPage } from "@/views/CommitPage";
import { HistorySheet } from "@/views/FileHistory";
import { FileTree } from "@/views/FileTree";
import { Hits } from "@/views/WorkspacePane";
import type { Draft } from "@/views/FileView";

/** Absolute path → git's letter, folded so a directory carries the
    heaviest letter under it (`WorkspacePane`'s rule, written again here
    rather than exported from a 2 500-line pane). */
function folded(root: string, git: GitStatus | null): Map<string, string> | null {
  if (git === null) return null;
  const out = new Map<string, string>();
  for (const r of git.rows) {
    const full = `${root}/${r.path}`;
    out.set(full, r.mark);
    if (r.mark === IGNORED) continue;
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

export function SideWorkspace({
  machine,
  here,
  root,
  start = null,
  onRead,
  fileActs,
  onLeave,
  backLabel = gui.group_recent,
  spot,
  onSpot,
  onOpenContent,
  findLeading,
  query,
  onQuery,
  onSearchEnter,
}: {
  /** A content-tab host may own file navigation and the search field's scope. */
  onOpenContent?: (path: string, line: number | null) => void;
  findLeading?: ReactNode;
  query?: string;
  onQuery?: (query: string) => void;
  onSearchEnter?: () => void;
  /** **Where the seat was left** (UX-47), read once at mount: the file
      open over the tree. Absent, the seat starts at the tree (or at
      `start`). */
  spot?: WsSpot | null;
  /** Where the seat stands now, said as it moves, so the 附栏 opened
      again comes back to it. */
  onSpot?: (spot: WsSpot) => void;
  /** What ‹ goes back to, said on its head (`onLeave`). */
  backLabel?: string;
  /** Entered from 最近 (UX-39 §3): the tree wears ‹ back there, and a file
      opened that way leaves to it rather than to the tree. */
  onLeave?: () => void;
  machine: string;
  here: boolean;
  /** The workspace's directory: the tree's ceiling. */
  root: string;
  /** A file to open, from outside (a path tapped in the terminal, a row
      of 最近). Stamped, so the same file asked twice opens twice. */
  start?: { path: string; at: number } | null;
  /** A file was opened here (the 附栏 remembers it as the session's). */
  onRead?: (path: string) => void;
  /** Acts for the open file's 「…」, by path. */
  fileActs?: (path: string) => { key: string; label: string; run: () => void }[];
}) {
  const [file, setFile] = useState<string | null>(spot ? spot.file : (start?.path ?? null));
  /** A line hit's line, for the file it opened (`FilePage`'s `goLine`). */
  const [goLine, setGoLine] = useState<{ path: string; line: number; at: number } | null>(null);
  // A seat come back to (`spot`) has already answered the `start` it
  // carries; only a new one opens its file.
  const startSeen = useRef(spot ? start : undefined);
  useEffect(() => {
    if (start !== null && start !== startSeen.current) setFile(start.path);
    startSeen.current = start;
  }, [start]);
  const [git, setGit] = useState<GitStatus | null>(null);
  const [gitAt, setGitAt] = useState(0);
  useEffect(() => {
    let live = true;
    setGit(null);
    fetchGitStatus(machine, root)
      .then((g) => live && setGit(g))
      .catch(() => live && setGit(null));
    return () => {
      live = false;
    };
  }, [machine, root, gitAt]);
  // **HEAD, for the tree's commit line** (WS-LAB item 1; the owner
  // 2026-10-13: 「mac 的 commit 有 ios 没有」— the lane wired
  // `WorkspacePane` and not this seat, which is the phone's whole
  // workspace). The same `gitAt` beat as the marks: a commit moves both.
  const [headCommit, setHeadCommit] = useState<GitLog["commits"][0] | null>(null);
  useEffect(() => {
    let live = true;
    fetchGitLog(machine, root)
      .then((l) => live && setHeadCommit(l.commits[0] ?? null))
      .catch(() => live && setHeadCommit(null));
    return () => {
      live = false;
    };
  }, [machine, root, gitAt]);
  /** The repository's 提交历史 sheet, and the commit a row of it opened —
      a screen in the file's place, ‹ back to the tree (`CommitPage`). */
  const [historyOpen, setHistoryOpen] = useState(false);
  const [commit, setCommit] = useState<GitCommit | null>(null);
  useEffect(() => setCommit(null), [machine, root]);
  // git's letters move with khor's acts anywhere in this checkout — the
  // main area's tree, a save there (UX-43 §1, `lib/tree-changed`).
  useEffect(
    () =>
      onTreeChanged((c) => {
        if (sameMachine(c, { machine, here }) && holds(root, c.dir)) setGitAt((n) => n + 1);
      }),
    [machine, here, root],
  );
  const [drafts, setDrafts] = useState<Map<string, Draft>>(() => new Map());
  // The hunt: asked 300 ms after the typing stops, a late answer to an
  // old question thrown away (`WorkspacePane`'s, the same rule).
  const [localHunt, setLocalHunt] = useState("");
  const hunt = query ?? localHunt;
  const setHunt = (value: string) => {
    if (onQuery) onQuery(value);
    else setLocalHunt(value);
  };
  const [found, setFound] = useState<Found | null>(null);
  const [huntError, setHuntError] = useState<string | null>(null);
  const huntAsked = useRef(0);
  useEffect(() => {
    const q = hunt.trim();
    if (q === "") {
      huntAsked.current += 1;
      setFound(null);
      setHuntError(null);
      return;
    }
    const mine = (huntAsked.current += 1);
    const wait = window.setTimeout(() => {
      searchIn(machine, root, q)
        .then((f) => {
          if (huntAsked.current !== mine) return;
          setFound(f);
          setHuntError(null);
        })
        .catch((e: unknown) => {
          if (huntAsked.current !== mine) return;
          setFound(null);
          setHuntError(String(e instanceof Error ? e.message : e));
        });
    }, 300);
    return () => window.clearTimeout(wait);
  }, [hunt, machine, root]);
  /** ＋'s choice, handed to the tree once the menu has given the focus
      back (the name box takes it next), the box cleared so the tree is
      there to take it. */
  const [make, setMake] = useState<{ kind: "file" | "dir"; at: number } | null>(null);
  const madeFromMenu = useRef<"file" | "dir" | null>(null);
  const plus = (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <GlassCircle aria-label={gui.new_item} data-side-new>
          <IconPlus />
        </GlassCircle>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="end"
        onCloseAutoFocus={(e) => {
          const kind = madeFromMenu.current;
          if (kind === null) return;
          madeFromMenu.current = null;
          e.preventDefault();
          setHunt("");
          setMake({ kind, at: Date.now() });
        }}
      >
        {(["file", "dir"] as const).map((kind) => (
          <DropdownMenuItem
            key={kind}
            data-side-new-item={kind}
            onSelect={() => {
              madeFromMenu.current = kind;
            }}
          >
            {kind === "file" ? gui.file_new : gui.dir_new}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
  // A file come back to was read when it was opened, not again now.
  const readSeen = useRef(spot ? file : null);
  useEffect(() => {
    if (file !== null && file !== readSeen.current) onRead?.(file);
    readSeen.current = file;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [file]);
  useEffect(() => {
    onSpot?.({ file });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [file]);
  return (
    <div data-side-workspace data-root={root} className="relative flex min-h-0 flex-1 flex-col">
      {/* The tree stays mounted under an open file, so ‹ finds it where
          it was scrolled. */}
      <div className={file !== null || commit !== null ? "invisible absolute inset-0 flex flex-col" : "flex min-h-0 flex-1 flex-col overflow-y-auto"}>
        {onLeave && <SeatHead title={root.split("/").filter(Boolean).pop() ?? root} back={onLeave} backLabel={backLabel} />}
        <SideFind onEnter={onSearchEnter} leading={findLeading} label={gui.tree_search} query={hunt} onQuery={setHunt} plus={plus} />
        {hunt.trim() !== "" ? (
          <Hits
            found={found}
            error={huntError}
            query={hunt.trim()}
            dir={root}
            onOpen={(path, line) => {
              if (onOpenContent) return onOpenContent(path, line);
              setFile(path);
              setGoLine(line === null ? null : { path, line, at: Date.now() });
            }}
          />
        ) : (
          <FileTree
            key={`${machine} ${root}`}
            machine={machine}
            root={root}
            here={here}
            marks={folded(root, git)}
            open={file}
            onOpen={(path) => onOpenContent ? onOpenContent(path, null) : setFile(path)}
            onGone={(path) => {
              if (file === path) setFile(null);
              setGitAt((n) => n + 1);
            }}
            onMoved={(from, to) => {
              if (file === from) setFile(to);
              setGitAt((n) => n + 1);
            }}
            onMade={() => setGitAt((n) => n + 1)}
            make={make}
            onMakeSeen={() => setMake(null)}
            headCommit={headCommit}
            onCommitLine={() => setHistoryOpen(true)}
          />
        )}
      </div>
      {commit !== null && (
        <div data-files-page className="relative flex min-h-0 flex-1 flex-col bg-background">
          <CommitPage key={commit.id} machine={machine} root={root} commit={commit} onBack={() => setCommit(null)} />
        </div>
      )}
      <HistorySheet
        open={historyOpen}
        onOpenChange={setHistoryOpen}
        machine={machine}
        path={root}
        repo
        onPick={(c) => {
          setFile(null);
          setCommit(c);
        }}
      />
      {file !== null && (
        <div data-files-page className="relative flex min-h-0 flex-1 flex-col bg-background">
          <FilePage
            key={file}
            machine={machine}
            path={file}
            here={here}
            git
            goLine={goLine?.path === file ? goLine : null}
            draft={drafts.get(file)}
            // **The same draft is no change** (`MachineFiles`' rule; fix-e,
            // 2026-10-08): `FileView` says its draft from an effect that
            // runs whenever this callback is new, which is every render —
            // a new Map each time made it an update a frame, for as long as
            // a file was open here, and the landings' switch (a transition)
            // never finished: 「侧边展开的时候, 不能通过导航到别的页面」.
            onDraft={(d) =>
              setDrafts((was) => {
                const had = was.get(file);
                if (d === null ? had === undefined : had === d) return was;
                const now = new Map(was);
                if (d === null) now.delete(file);
                else now.set(file, d);
                return now;
              })
            }
            onSaved={() => setGitAt((n) => n + 1)}
            onGone={() => {
              setFile(null);
              setGitAt((n) => n + 1);
            }}
            onMoved={(_from, to) => {
              setFile(to);
              setGitAt((n) => n + 1);
            }}
            moreActs={fileActs?.(file)}
            onBack={onLeave && start?.path === file ? onLeave : () => setFile(null)}
          />
        </div>
      )}
    </div>
  );
}
