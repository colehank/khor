/**
 * **A machine's files, in place on its page** (the redesign, 2026-10-02,
 * the owner: 「文件按钮点击后，不要跳转到工作区，工作区只负责工作区」).
 * The machine head's 文件 is a switch now, like 会话: pressed, the page's
 * body is this — the machine's disk, from its home, the way the verb
 * used to root it — and the head and the open landing stay the machine's.
 * Nothing here adds a directory to the workspaces or opens that landing.
 *
 * It is the workspace pane's two pieces for a whole disk, and only them:
 * the listing with its address and 上一级 (`FilesPane`) and the file view
 * (`FileView`). Left out, because they are a workspace's: the tab strip
 * (one file at a time here), the sessions column and the tool row, git's
 * marks and diffs (a disk is not a checkout), the in-tree search, and the
 * source / preview switch (the view opens on the source).
 *
 * Wide: the listing in a column, the file beside it. Narrow: one screen
 * at a time — the listing, or the file under a way back to it.
 *
 * **The file is the file page** (round UX-3, `FilePage`): no title rows,
 * the name a pill at the top, ‹ · 分享 · 「…」 · 编辑 floating at the
 * bottom — on the narrow face the whole window (the machine's head steps
 * aside, `useFullScreen`), on the wide face the column beside the listing.
 */
import { useEffect, useRef, useState } from "react";

import { EmptyState } from "@/components/RegionState";
import { gui } from "@/gen/catalog";
import { useFullScreen } from "@/lib/full-screen";
import { cn } from "@/lib/utils";
import { useNarrow } from "@/hooks/use-narrow";
import { FilePage } from "@/views/FilePage";
import { FilesPane } from "@/views/FilesPane";
import type { Draft } from "@/views/FileView";

export function MachineFiles({
  machine,
  here,
  onOpenFile,
  start = null,
  startDir = null,
  fileActs,
  onTop,
  topLabel,
  onRead,
  bare = false,
}: {
  /** **Inside the session's 附栏** (UX-37 §6): a file opens as the bare
      file page (‹ · name · content, no deck) and takes nothing of the
      screen — the sheet is the screen it is in. */
  bare?: boolean;
  /** ‹ at the root goes on to here (UX-37 §1: the 附栏's 工作区 climbs
      the shell's directory to the root, then to every workspace). */
  onTop?: () => void;
  /** What ‹ at the root goes to, for the row's name. */
  topLabel?: string;
  /** A file was opened to be read here (the 附栏 remembers it as the
      session's, UX-37 §1). */
  onRead?: (path: string) => void;
  /** Acts for the open file's 「…」, by path (`FilePage` `moreActs`). */
  fileActs?: (path: string) => { key: string; label: string; run: () => void }[];
  /** The directory the listing opens at (UX-17: a session's 目录 sheet
      opens where the session is); the machine's home without it. */
  startDir?: string | null;
  machine: string;
  here: boolean;
  /** A file to open, from outside the page (a conversation's
      「在文件视图中打开」, round CHAT): the listing goes to its directory
      and the file opens. Stamped, so the same file asked twice opens
      twice. */
  start?: { path: string; at: number } | null;
  /** **One back button** (the audit fix plan F4.2; N07): while a file is
      open on the narrow face, the way back leaves the file rather than
      the page — handed up for the edge swipe, `null` when there is no
      file to leave. */
  onOpenFile?: (back: (() => void) | null) => void;
}) {
  const narrow = useNarrow();
  // `""` is the machine's home, the listing's own contract.
  const [at, setAt] = useState(() => (start ? start.path.slice(0, start.path.lastIndexOf("/")) || "/" : (startDir ?? "")));
  const [file, setFile] = useState<string | null>(start?.path ?? null);
  /** Bumped to read the listing again after the file page renamed or
      deleted what it showed. */
  const [reread, setReread] = useState(0);
  useEffect(() => {
    if (start === null) return;
    setAt(start.path.slice(0, start.path.lastIndexOf("/")) || "/");
    setFile(start.path);
  }, [start]);
  // What was typed into a file and not saved, kept when another file is
  // opened — the workspace pane's rule (`FileView`'s `draft`).
  const [drafts, setDrafts] = useState<Map<string, Draft>>(() => new Map());
  useEffect(() => {
    if (!narrow) return;
    onOpenFile?.(file !== null ? () => setFile(null) : null);
    return () => onOpenFile?.(null);
  }, [narrow, file, onOpenFile]);
  useFullScreen(narrow && file !== null && !bare);
  const read = useRef(onRead);
  read.current = onRead;
  useEffect(() => {
    if (file !== null) read.current?.(file);
  }, [file]);

  const listing = (
    <FilesPane
      key={reread}
      machine={machine}
      path={at}
      onPath={setAt}
      onLanded={setAt}
      onFile={(path) => setFile(path)}
      open={file}
      here={here}
      onTop={onTop}
      topLabel={topLabel}
    />
  );
  const view = file !== null && (
    <FilePage
      key={`${machine} ${file}`}
      machine={machine}
      path={file}
      here={here}
      moreActs={fileActs?.(file)}
      bare={bare}
      onBack={() => setFile(null)}
      onGone={() => {
        setFile(null);
        setReread((n) => n + 1);
      }}
      onMoved={(_, to) => {
        setFile(to);
        setReread((n) => n + 1);
      }}
      draft={drafts.get(file)}
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
    />
  );

  if (narrow) {
    return (
      <div data-machine-files className="relative flex min-h-0 flex-1 flex-col">
        {/* **The listing stays mounted under an open file** (F4.2; N07),
            so going back finds it where it was scrolled — and it is what
            a back swipe uncovers (V01): drawn only while a finger holds
            the file (`app.css` `[data-files-under]`). */}
        <div
          data-files-under={file !== null ? "" : undefined}
          className={cn(
            "flex min-h-0 flex-1 flex-col ps-files-inset",
            file !== null && "invisible absolute inset-0",
          )}
        >
          {listing}
        </div>
        {file !== null && (
          <div data-files-page className="relative flex min-h-0 flex-1 flex-col bg-background">
            {view}
          </div>
        )}
      </div>
    );
  }
  // No rule over the listing: a machine page's head has none in any of
  // its modes (F4.2; W16). An empty right side says what it is for.
  return (
    <div data-machine-files className="flex min-h-0 flex-1">
      <div className="flex min-h-0 w-list flex-none flex-col border-r ps-files-inset">{listing}</div>
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        {view || <EmptyState data-files-pick>{gui.files_pick}</EmptyState>}
      </div>
    </div>
  );
}
