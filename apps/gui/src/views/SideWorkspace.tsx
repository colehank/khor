import { WorkspaceGitNavigation } from '@/components/WorkspaceGitNavigation';
import { WorkspaceHistory } from '@/views/workspace/WorkspaceHistory';
import { WorkspaceHistoryMemory } from '@/views/workspace/workspace-history';
import type { GitCommitSummary } from '@/gen/bindings/GitCommitSummary';
import { useDirectoryWatch } from "@/hooks/use-directory-watch";
import { useHidden } from "@/hooks/use-hidden";
import { directoryScopeIsNative, directoryWatchScopes } from "@/lib/directory-watch";
import {useDraftState} from '@/hooks/use-draft-count';
/** Rooted files open locally or through the session's existing content-tab owner. */
import { useEffect, useRef, useState, type ReactNode } from "react";

import { fetchCheckout, fetchGitLog, fetchGitStatus, searchIn, type Found, type GitLog } from "@/api";
import { GlassCircle } from "@/components/Glass";
import { IconPlus } from "@/components/icons";
import { SideFind } from "@/components/SideFind";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { SeatHead } from "@/components/SeatHead";
import { gui } from "@/gen/catalog";
import type { GitStatus } from "@/gen/bindings/GitStatus";
import type { WsSpot } from "@/hooks/use-session-side";
import { holds, onTreeChanged, sameMachine } from "@/lib/tree-changed";
import { heavier, IGNORED } from "@/views/fileRow";
import { movedFilePath } from "@/lib/file-actions";
import { requestFileLocation } from "@/lib/file-location";
import { FilePage } from "@/views/FilePage";
import { useRepositoryHistory } from "@/hooks/use-repository-history";
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
  onLeave,
  backLabel = gui.group_recent,
  spot,
  onSpot,
  onOpenContent,
  onOpenCommit,
  historyView,
  onHistoryView,
  onContentMoved,
  onContentGone,
  findLeading,
  query,
  onQuery,
  onSearchEnter,
  makeRequest,
  treeHead,
  hideFind = false,
  active = true,
}: {
  active?: boolean;
  /** A content-tab host may own file navigation and the search field's scope. */
  onOpenContent?: (path: string, line: number | null, keep?:boolean) => void;
  onOpenCommit?: (commit:GitCommitSummary,keep:boolean)=>void;
  historyView?:boolean;
  onHistoryView?:(history:boolean)=>void;
  onContentMoved?: (from:string,to:string)=>void;
  onContentGone?: (path:string)=>void;
  findLeading?: ReactNode;
  query?: string;
  onQuery?: (query: string) => void;
  onSearchEnter?: () => void;
  makeRequest?: { kind: "file" | "dir"; at: number };
  hideFind?: boolean;
  treeHead?: (openHistory: () => void, repo: boolean) => ReactNode;
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
}) {
  const [file, setFile] = useState<string | null>(spot ? spot.file : (start?.path ?? null));
  const [readAt, setReadAt] = useState(0);
  const hidden = useHidden();
  const visible = active && !hidden;
  const fileParent = file === null ? null : file.slice(0, file.lastIndexOf("/")) || "/";
  const observation = useDirectoryWatch(machine, directoryWatchScopes(null, fileParent === null ? [] : [fileParent]).scopes, active);
  const wasVisible = useRef(visible);
  useEffect(() => {
    const returned = !wasVisible.current && visible;
    wasVisible.current = visible;
    if (returned && fileParent !== null) setReadAt(n => n + 1);
  }, [visible, fileParent]);
  const fileCovered = fileParent !== null && directoryScopeIsNative(observation, { kind: "entries", path: fileParent });
  useEffect(() => {
    if (!visible) return;
    const batch = observation.batch;
    if (fileParent !== null && observation.status === "watching" && batch !== null &&
        (batch.reset || batch.dirs.includes(fileParent))) setReadAt(n => n + 1);
  }, [observation.batch, observation.status, fileParent, visible]);
  useEffect(() => {
    if (fileParent === null || !visible || fileCovered) return;
    const refresh = () => { if (!document.hidden) setReadAt(n => n + 1); };
    const slow = window.setInterval(refresh, 60_000);
    window.addEventListener("focus", refresh);
    return () => { window.clearInterval(slow); window.removeEventListener("focus", refresh); };
  }, [fileParent, visible, fileCovered]);
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
  // Repository identity is a cheap independent answer, just as in the
  // main workspace list. A failed/slow status or a detached HEAD cannot
  // turn an existing checkout into a folder.
  const [checkout, setCheckout] = useState<{machine:string;root:string;repo:boolean}|null>(null);
  const [gitAt, setGitAt] = useState(0);
  useEffect(() => {
    let live = true;
    setGit(null);
    fetchCheckout(machine, root).then(repo => {
      if(live)setCheckout({machine,root,repo});
    }, () => {});
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
      a screen in the file's place, ‹ back to the retained history list (`CommitPage`). */
  const route = useRepositoryHistory(machine,root);
  const {commit,setCommit,backToHistory}=route;
  const historyOpen=historyView??route.historyOpen;
  const setHistoryOpen=onHistoryView??route.setHistoryOpen;
  const [historyMemory]=useState(()=>new WorkspaceHistoryMemory());
  const hosted=onOpenCommit!==undefined;
  const repo=checkout?.machine===machine&&checkout.root===root?checkout.repo:git!==null;

  // git's letters move with khor's acts anywhere in this checkout — the
  // main area's tree, a save there (UX-43 §1, `lib/tree-changed`).
  useEffect(
    () =>
      onTreeChanged((c) => {
        if (sameMachine(c, { machine, here }) && holds(root, c.dir)) setGitAt((n) => n + 1);
      }),
    [machine, here, root],
  );
  const [drafts, setDrafts] = useDraftState<Map<string, Draft>>(() => new Map(), drafts=>drafts.size);
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
    if (q === "" || hosted && historyOpen) {
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
  }, [hunt, machine, root, hosted, historyOpen]);
  /** ＋'s choice, handed to the tree once the menu has given the focus
      back (the name box takes it next), the box cleared so the tree is
      there to take it. */
  const [make, setMake] = useState<{ kind: "file" | "dir"; at: number } | null>(null);
  useEffect(() => { if (makeRequest) { setHunt(""); setMake(makeRequest); } }, [makeRequest]);
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
        {!hideFind && <SideFind onEnter={onSearchEnter} leading={findLeading} label={gui.tree_search} query={hunt} onQuery={setHunt} plus={plus} />}
        {hosted && <WorkspaceGitNavigation branch={git?.branch??''} repo={repo} history={historyOpen} onHistory={setHistoryOpen} seat="session"/>}
        {hosted && historyOpen ? <WorkspaceHistory machine={machine} root={root} query={hunt} memory={historyMemory} active={null} onOpen={(commit,keep)=>onOpenCommit?.(commit,keep)}/>:hunt.trim() !== "" ? (
          <Hits
            found={found}
            error={huntError}
            query={hunt.trim()}
            dir={root}
            here={here}
            onOpen={(path, line, keep) => {
              if (onOpenContent) return onOpenContent(path, line, keep);
              setFile(path);
              setGoLine(line === null ? null : { path, line, at: Date.now() });
            }}
          />
        ) : (
          <>{treeHead?.(() => setHistoryOpen(true), checkout?.machine===machine&&checkout.root===root?checkout.repo:git!==null)}<FileTree
            watchActive={active && file === null && commit === null}
            key={`${machine} ${root}`}
            machine={machine}
            root={root}
            here={here}
            marks={folded(root, git)}
            open={file}
            onOpen={(path,keep) => onOpenContent ? onOpenContent(path, null,keep) : setFile(path)}
            onGone={(path) => {
              if (file && movedFilePath(file,path,path)!==null) setFile(null);
              onContentGone?.(path);
              setGitAt((n) => n + 1);
            }}
            onMoved={(from, to) => {
              if (file) { const next=movedFilePath(file,from,to); if(next!==null)setFile(next); }
              onContentMoved?.(from,to);
              setGitAt((n) => n + 1);
            }}
            onMade={() => setGitAt((n) => n + 1)}
            make={make}
            onMakeSeen={() => setMake(null)}
            headCommit={treeHead || hosted ? null : headCommit}
            onCommitLine={() => setHistoryOpen(true)}
          /></>
        )}
      </div>
      {commit !== null && (
        <div data-files-page className="relative flex min-h-0 flex-1 flex-col bg-background">
          <CommitPage key={commit.id} machine={machine} root={root} commit={commit} onBack={backToHistory} />
        </div>
      )}
      {!hosted && <HistorySheet
        open={historyOpen}
        onOpenChange={setHistoryOpen}
        machine={machine}
        path={root}
        repo
        onPick={(c) => {
          setFile(null);
          setCommit(c);
        }}
      />}
      {file !== null && (
        <div data-files-page className="relative flex min-h-0 flex-1 flex-col bg-background">
          <FilePage
            key={file}
            machine={machine}
            path={file}
            readAt={readAt}
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
            onLocate={() => { requestFileLocation(machine, file); setHunt(""); setFile(null); }}
            onBack={onLeave && start?.path === file ? onLeave : () => setFile(null)}
          />
        </div>
      )}
    </div>
  );
}
