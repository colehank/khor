// **One file's git 历史** (round UX-4, the file page's 「…」): the commits
// that changed it, as a sheet; a row opens that revision read-only where
// the file was, and 「与当前比较」 is the diff view the file already has,
// against that revision instead of HEAD.
//
// The questions go to the machine holding the file (`git_log`,
// `git_show`, `khor_node::git`), as the gutter's do.
import { useEffect, useMemo, useState } from "react";

import { fetchGitLog, fetchGitShow, said, type GitLog, type GitShow } from "@/api";
import { Markdown } from "@/components/Markdown";
import { Waiting } from "@/components/RegionState";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import type { GitCommit } from "@/gen/bindings/GitCommit";
import { groundAttr, usePreviewGround } from "@/lib/preview-ground";
import { DiffView } from "@/views/DiffView";
import { CodeEditor } from "@/components/CodeEditor.lazy";
import { rendererOf } from "@/views/FileView";
import { ago } from "@/words";

/** A commit as a person names it. */
export function shortId(c: GitCommit): string {
  return c.id.slice(0, 7);
}

/** The sheet: newest first, a row per commit, a press opens it. */
export function HistorySheet({
  open,
  onOpenChange,
  machine,
  path,
  onPick,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  machine: string;
  path: string;
  onPick: (c: GitCommit) => void;
}) {
  const [log, setLog] = useState<GitLog | null>(null);
  const [refused, setRefused] = useState<string | null>(null);
  const [since, setSince] = useState(() => Date.now());
  useEffect(() => {
    if (!open) return;
    let live = true;
    setLog(null);
    setRefused(null);
    setSince(Date.now());
    fetchGitLog(machine, path).then(
      (l) => live && setLog(l),
      (e: unknown) => live && setRefused(said(e)),
    );
    return () => {
      live = false;
    };
  }, [open, machine, path]);
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent form data-file-history className="max-h-[70dvh] !gap-0 overflow-y-auto">
        <DialogTitle className="px-in pb-in">{gui.file_history}</DialogTitle>
        {refused !== null ? (
          <div data-file-history-refused className="px-in text-callout text-state-failed">
            {refused}
          </div>
        ) : log === null ? (
          <Waiting since={since} />
        ) : log.commits.length === 0 ? (
          <div className="px-in text-callout text-muted-foreground">{gui.file_history_none}</div>
        ) : (
          <div className="flex min-w-0 flex-col">
            {log.commits.map((c) => (
              <button
                key={c.id}
                type="button"
                data-file-history-row={c.id}
                onClick={() => {
                  onOpenChange(false);
                  onPick(c);
                }}
                className="flex min-h-list-row min-w-0 flex-col justify-center rounded-md px-in text-left hover:bg-fill"
              >
                <span className="truncate text-body text-foreground">{c.subject}</span>
                <span className="truncate text-callout text-muted-foreground">
                  <span className="font-mono">{shortId(c)}</span> · {c.author} · {ago(c.when_ms)}
                </span>
              </button>
            ))}
            {log.more && <div className="px-in py-in text-callout text-muted-foreground">{gui.file_history_more}</div>}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}

/** The revision where the file was: read-only, rendered the way the file
    renders (Markdown, or its lines), or compared with the file as it is
    now. */
export function RevisionView({
  machine,
  path,
  rev,
  compare,
  narrow,
}: {
  machine: string;
  path: string;
  rev: GitCommit;
  compare: boolean;
  narrow: boolean;
}) {
  const [shown, setShown] = useState<GitShow | null>(null);
  const [refused, setRefused] = useState<string | null>(null);
  const [since, setSince] = useState(() => Date.now());
  const { ground } = usePreviewGround();
  useEffect(() => {
    let live = true;
    setShown(null);
    setRefused(null);
    setSince(Date.now());
    fetchGitShow(machine, path, rev.id).then(
      (s) => live && setShown(s),
      (e: unknown) => live && setRefused(said(e)),
    );
    return () => {
      live = false;
    };
  }, [machine, path, rev.id]);
  const against = useMemo(
    () => (shown === null ? undefined : { label: shortId(rev), hunks: shown.diff.hunks }),
    [shown, rev],
  );
  if (refused !== null) {
    return <div data-file-rev-refused className="p-3 text-callout text-state-failed">{refused}</div>;
  }
  if (shown === null) return <Waiting since={since} />;
  if (compare) return <DiffView machine={machine} path={path} narrow={narrow} against={against} />;
  const md = rendererOf(path) === "md";
  return (
    <div data-file-rev={rev.id} className="flex min-h-0 flex-1 flex-col">
      {md ? (
        <div data-file-preview="md" {...groundAttr(ground)} className="min-h-0 flex-1 overflow-auto">
          <Markdown text={shown.text} className="prose-khor px-6 py-4" />
        </div>
      ) : (
        // The revision in the file's own editor, read-only: numbers,
        // folding, ⌘F and the code's colours, the same as the file now.
        <CodeEditor path={path} text={shown.text} editable={false} />
      )}
      {shown.cut && <div className="px-3 py-1 text-callout text-muted-foreground">{gui.file_rev_cut}</div>}
    </div>
  );
}
