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
 * way the landing folds them. A file opens in place as the bare file
 * page (UX-37 §6) and ‹ there is the way back to the tree.
 *
 * ⚠️ 墓碑 (UX-37 §1): `MachineFiles` here, climbing to the root and on to
 * every workspace.
 */
import { useEffect, useState } from "react";

import { fetchGitStatus } from "@/api";
import { SeatHead } from "@/components/SeatHead";
import { gui } from "@/gen/catalog";
import type { GitStatus } from "@/gen/bindings/GitStatus";
import { heavier, IGNORED } from "@/views/fileRow";
import { FilePage } from "@/views/FilePage";
import { FileTree } from "@/views/FileTree";
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
}: {
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
  const [drafts, setDrafts] = useState<Map<string, Draft>>(() => new Map());
  useEffect(() => {
    if (file !== null) onRead?.(file);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [file]);
  return (
    <div data-side-workspace data-root={root} className="relative flex min-h-0 flex-1 flex-col">
      {/* The tree stays mounted under an open file, so ‹ finds it where
          it was scrolled. */}
      <div className={file !== null ? "invisible absolute inset-0 flex flex-col" : "flex min-h-0 flex-1 flex-col overflow-y-auto"}>
        {onLeave && <SeatHead title={root.split("/").filter(Boolean).pop() ?? root} back={onLeave} backLabel={gui.group_recent} />}
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
        />
      </div>
      {file !== null && (
        <div data-files-page className="relative flex min-h-0 flex-1 flex-col bg-background">
          <FilePage
            key={file}
            machine={machine}
            path={file}
            here={here}
            git
            bare
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
