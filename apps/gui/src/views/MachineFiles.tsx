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
import { useState } from "react";

import { GlassCircle } from "@/components/Glass";
import { IconBack } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { useNarrow } from "@/hooks/use-narrow";
import { FilesPane } from "@/views/FilesPane";
import { FileView } from "@/views/FileView";

type Draft = Parameters<NonNullable<Parameters<typeof FileView>[0]["onDraft"]>>[0];

export function MachineFiles({ machine, here }: { machine: string; here: boolean }) {
  const narrow = useNarrow();
  // `""` is the machine's home, the listing's own contract.
  const [at, setAt] = useState("");
  const [file, setFile] = useState<string | null>(null);
  // What was typed into a file and not saved, kept when another file is
  // opened — the workspace pane's rule (`FileView`'s `draft`).
  const [drafts, setDrafts] = useState<Map<string, NonNullable<Draft>>>(() => new Map());

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
      <div data-machine-files className="flex min-h-0 flex-1 flex-col">
        {file === null ? (
          <div className="flex min-h-0 flex-1 flex-col ps-files-inset">{listing}</div>
        ) : (
          <>
            <div data-machine-file-head className="flex flex-none items-center gap-in px-page pb-in">
              <GlassCircle aria-label={gui.back} data-back onClick={() => setFile(null)}>
                <IconBack />
              </GlassCircle>
              <span className="min-w-0 truncate text-body font-title">{file.split("/").pop()}</span>
            </div>
            <div className="min-h-0 flex-1">{view}</div>
          </>
        )}
      </div>
    );
  }
  return (
    <div data-machine-files className="flex min-h-0 flex-1 border-t">
      <div className="flex min-h-0 w-list flex-none flex-col border-r ps-files-inset">{listing}</div>
      <div className="min-h-0 min-w-0 flex-1">{view}</div>
    </div>
  );
}
