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
 * with the main area's deck (`FilePage docked`, UX-40 §4), and ‹ there is
 * the way back to the tree — or to 最近, for the file a row of 最近 opened.
 *
 * **Find and make** (UX-40 §1): the find row under the 附栏's head is the
 * landing tree's own box and ＋ — names and lines under the workspace
 * (`Hits`), 新建文件 / 新建目录 into the tree (`FileTree`'s `make`).
 *
 * **「放进来的」** (item 60 part 5): while the machine's `~/khor-drop/`
 * holds something, a row above the tree opens it in the seat, as its own
 * tree with ‹ back to the workspace — where files dropped onto this
 * machine's terminals land. It is outside the workspace on purpose: the
 * one door out of the ceiling, and only to that directory.
 *
 * ⚠️ 墓碑 (UX-37 §6 → UX-40 §4): the bare file page, ‹ and the name in a
 * head at its top. ⚠️ 墓碑 (UX-37 §1): `MachineFiles` here, climbing to
 * the root and on to every workspace.
 */
import { useEffect, useRef, useState } from "react";

import { fetchGitStatus, fetchLs, searchIn, type Found } from "@/api";
import { GlassCircle } from "@/components/Glass";
import { IconChevronRight, IconFiles, IconPlus } from "@/components/icons";
import { SideFind } from "@/components/SideFind";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { SeatHead } from "@/components/SeatHead";
import { gui } from "@/gen/catalog";
import type { GitStatus } from "@/gen/bindings/GitStatus";
import { heavier, IGNORED } from "@/views/fileRow";
import { FilePage } from "@/views/FilePage";
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
  drops = true,
}: {
  /** What ‹ goes back to, said on its head (`onLeave`). */
  backLabel?: string;
  /** Whether the 「放进来的」 row is looked for (not inside it). */
  drops?: boolean;
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
  const [file, setFile] = useState<string | null>(start?.path ?? null);
  /** A line hit's line, for the file it opened (`FilePage`'s `goLine`). */
  const [goLine, setGoLine] = useState<{ path: string; line: number; at: number } | null>(null);
  useEffect(() => {
    if (start !== null) setFile(start.path);
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
  // **「放进来的」**: the machine's home, then its `khor-drop` when it is
  // there and holds something. Asked once per seat.
  const [dropDir, setDropDir] = useState<string | null>(null);
  const [inDrops, setInDrops] = useState(false);
  useEffect(() => {
    if (!drops) return;
    let live = true;
    void fetchLs(machine, "")
      .then((home) => {
        if (!home.entries.some((e) => e.dir && e.name === "khor-drop")) return null;
        return fetchLs(machine, `${home.path.replace(/\/$/, "")}/khor-drop`);
      })
      .then((d) => live && d && d.entries.length > 0 && d.path !== root && setDropDir(d.path))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [machine, root, drops]);
  const [drafts, setDrafts] = useState<Map<string, Draft>>(() => new Map());
  // The hunt: asked 300 ms after the typing stops, a late answer to an
  // old question thrown away (`WorkspacePane`'s, the same rule).
  const [hunt, setHunt] = useState("");
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
  useEffect(() => {
    if (file !== null) onRead?.(file);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [file]);
  if (inDrops && dropDir !== null) {
    return (
      <SideWorkspace
        machine={machine}
        here={here}
        root={dropDir}
        drops={false}
        onLeave={() => setInDrops(false)}
        backLabel={gui.workspaces_tab}
        onRead={onRead}
        fileActs={fileActs}
      />
    );
  }
  return (
    <div data-side-workspace data-root={root} className="relative flex min-h-0 flex-1 flex-col">
      {/* The tree stays mounted under an open file, so ‹ finds it where
          it was scrolled. */}
      <div className={file !== null ? "invisible absolute inset-0 flex flex-col" : "flex min-h-0 flex-1 flex-col overflow-y-auto"}>
        {onLeave && <SeatHead title={root.split("/").filter(Boolean).pop() ?? root} back={onLeave} backLabel={backLabel} />}
        <SideFind label={gui.tree_search} query={hunt} onQuery={setHunt} plus={plus} />
        {dropDir !== null && hunt.trim() === "" && (
          <button
            type="button"
            data-side-drops={dropDir}
            onClick={() => setInDrops(true)}
            className="flex min-h-touch w-full flex-none items-center gap-row px-row text-left text-body"
          >
            <IconFiles className="size-kind-mark flex-none text-muted-foreground" />
            <span className="min-w-0 flex-1 truncate">{gui.drops_row}</span>
            <IconChevronRight className="size-4 flex-none text-tertiary" />
          </button>
        )}
        {hunt.trim() !== "" ? (
          <Hits
            found={found}
            error={huntError}
            query={hunt.trim()}
            dir={root}
            onOpen={(path, line) => {
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
            onOpen={(path) => setFile(path)}
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
          />
        )}
      </div>
      {file !== null && (
        <div data-files-page className="relative flex min-h-0 flex-1 flex-col bg-background">
          <FilePage
            key={file}
            machine={machine}
            path={file}
            here={here}
            git
            docked
            goLine={goLine?.path === file ? goLine : null}
            draft={drafts.get(file)}
            onDraft={(d) =>
              setDrafts((was) => {
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
