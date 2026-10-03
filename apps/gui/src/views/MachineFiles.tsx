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
 */
import { useEffect, useState } from "react";

import { EmptyState } from "@/components/RegionState";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { useNarrow } from "@/hooks/use-narrow";
import { FilesPane } from "@/views/FilesPane";
import { FileView } from "@/views/FileView";

type Draft = Parameters<NonNullable<Parameters<typeof FileView>[0]["onDraft"]>>[0];

export function MachineFiles({
  machine,
  here,
  onOpenFile,
}: {
  machine: string;
  here: boolean;
  /** **One back button** (the audit fix plan F4.2; N07): while a file is
      open on the narrow face, the page head's ‹ leaves the file rather
      than the page — handed up as the way out, `null` when there is no
      file to leave. */
  onOpenFile?: (back: (() => void) | null) => void;
}) {
  const narrow = useNarrow();
  // `""` is the machine's home, the listing's own contract.
  const [at, setAt] = useState("");
  const [file, setFile] = useState<string | null>(null);
  // What was typed into a file and not saved, kept when another file is
  // opened — the workspace pane's rule (`FileView`'s `draft`).
  const [drafts, setDrafts] = useState<Map<string, NonNullable<Draft>>>(() => new Map());
  useEffect(() => {
    if (!narrow) return;
    onOpenFile?.(file !== null ? () => setFile(null) : null);
    return () => onOpenFile?.(null);
  }, [narrow, file, onOpenFile]);

  const listing = (
    <FilesPane
      machine={machine}
      path={at}
      onPath={setAt}
      onLanded={setAt}
      onFile={(path) => setFile(path)}
      open={file}
      here={here}
    />
  );
  const view = file !== null && (
    <FileView
      key={`${machine} ${file}`}
      machine={machine}
      path={file}
      git={false}
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
          // Only the file moves under a back swipe; the head both share
          // stays put.
          <div data-files-page className="relative flex min-h-0 flex-1 flex-col bg-background">
            {/* The file's name; the way back is the page head's ‹. */}
            <div data-machine-file-head className="flex flex-none items-center px-page pb-in">
              <span className="min-w-0 truncate text-body font-title">{file.split("/").pop()}</span>
            </div>
            {/* The line numbers start on the page's left line (N07): the
                view's gutter pads 8 of its own. **A flex column**, so the
                view is held to the screen and its own scroller scrolls
                (2026-10-03, the owner's phone: 「从机器进入工作区文件不可
                拖动」). As a block it let the view grow to the whole file's
                height — 19 336 px for a 605-line lock file — inside a
                screen that clips, so there was nothing a finger could
                scroll and the file's name slid under the page head. */}
            <div className="flex min-h-0 flex-1 flex-col ps-[calc(var(--page-inset)-0.5rem)]">{view}</div>
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
