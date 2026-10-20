// **One file's git 历史** (round UX-4, the file page's 「…」): the commits
// that changed it, as a sheet; a row opens that revision read-only where
// the file was, and 「与当前比较」 is the diff view the file already has,
// against that revision instead of HEAD.
//
// The questions go to the machine holding the file (`git_log`,
// `git_show`, `khor_node::git`), as the gutter's do.
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

import { fetchGitLog, fetchGitShow, said, type GitLog, type GitShow } from "@/api";
import { HistoryDeck } from "@/components/HistoryDeck";
import { useWindowNarrow } from "@/hooks/use-narrow";
import { basename } from "@/words";
import { Button } from "@/components/ui/button";
import { Markdown } from "@/components/Markdown";
import { Notebook } from "@/components/Notebook.lazy";
import { Waiting } from "@/components/RegionState";
import { Dialog, TaskOverlay, DialogTitle, DialogBody, DialogFooter } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import type { GitCommit } from "@/gen/bindings/GitCommit";
import { groundAttr, usePreviewGround } from "@/lib/preview-ground";
import { DiffView } from "@/views/DiffView";
import { CodeEditor } from "@/components/CodeEditor.lazy";
import { defaultFaceOf, rendererOf } from "@/views/FileView";
import { ago } from "@/words";

/** A commit as a person names it. */
export function shortId(c: GitCommit): string {
  return c.id.slice(0, 7);
}

/** `Co-Authored-By: name <mail>`, git's trailer for a second author. */
const CO_AUTHOR = /^co-authored-by:\s*(.*?)\s*(?:<[^>]*>)?\s*$/gim;

/** **Who wrote a commit** (COMMITS-BRIEF, GitHub's 「A 和 B 撰写」): its
    author, then each `Co-Authored-By:` trailer's name, once each. */
export function authorsOf(c: GitCommit): string[] {
  const names = c.author ? [c.author] : [];
  for (const m of c.body.matchAll(CO_AUTHOR)) {
    const n = m[1].trim();
    if (n && !names.includes(n)) names.push(n);
  }
  return names;
}

/** The author line: 「A 撰写」, 「A 和 B 撰写」 (a third and more ride
    with B, comma-joined). */
export function authorLine(c: GitCommit): string {
  const [a = "", ...rest] = authorsOf(c);
  return rest.length === 0 ? gui.commit_authored_one(a) : gui.commit_authored_two(a, rest.join(", "));
}

/** The body as a person wrote it: the co-author trailers are the author
    line's, not the prose's. */
export function bodyOf(c: GitCommit): string {
  return c.body.replace(CO_AUTHOR, "").trim();
}

/** The list row's second line: the body's first line with words on it. */
export function bodyLine(c: GitCommit): string {
  return bodyOf(c).split("\n").find((l) => l.trim() !== "")?.trim() ?? "";
}

/** The sheet: newest first, a row per commit, a press opens it. */
export function HistorySheet({
  open,
  onOpenChange,
  machine,
  path,
  onPick,
  repo = false,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  machine: string;
  path: string;
  onPick: (c: GitCommit) => void;
  /** **The repository's commits, not one file's** (COMMITS-BRIEF): titled
      提交历史, and each row says what GitHub's does — subject and age, the
      body's first line, who wrote it. */
  repo?: boolean;
}) {
  const narrow = useWindowNarrow();
  const key = machine + "\0" + path + "\0" + repo;
  const [record, setRecord] = useState<{key:string;log:GitLog}|null>(null);
  const log = record?.key === key ? record.log : null;
  const scroller = useRef<HTMLDivElement>(null);
  const position = useRef({key,top:0});
  if(position.current.key !== key)position.current={key,top:0};
  useLayoutEffect(()=>{if(open && log && scroller.current)scroller.current.scrollTop=position.current.top;},[open,log,key]);
  const [refused, setRefused] = useState<string | null>(null);
  const [since, setSince] = useState(() => Date.now());
  useEffect(() => {
    if (!open) return;
    let live = true;
    setRefused(null);
    setSince(Date.now());
    fetchGitLog(machine, path).then(
      (l) => live && setRecord({key,log:l}),
      (e: unknown) => live && setRefused(said(e)),
    );
    return () => {
      live = false;
    };
  }, [open, machine, path, key]);
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <TaskOverlay purpose="selection" scrollable data-file-history data-history-overlay className="!gap-0">
        <DialogTitle className={narrow ? "sr-only" : "px-in pb-in"}>{repo ? gui.commit_history : gui.file_history}</DialogTitle>
        <DialogBody ref={scroller} data-history-scroll onScroll={e=>{position.current.top=e.currentTarget.scrollTop;}} className="gap-0 px-in">
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
            {log.commits.map((c) =>
              repo ? (
                <CommitRow
                  key={c.id}
                  c={c}
                  onPress={() => {
                    onOpenChange(false);
                    onPick(c);
                  }}
                />
              ) : (
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
              ),
            )}
            {log.more && <div className="px-in py-in text-callout text-muted-foreground">{gui.file_history_more}</div>}
          </div>
        )}
        </DialogBody>
        {narrow ? <HistoryDeck title={repo ? gui.commit_history : gui.file_history} subtitle={basename(path)} onBack={()=>onOpenChange(false)}/> : <DialogFooter><Button variant="ghost" onClick={()=>onOpenChange(false)}>{gui.back}</Button></DialogFooter>}
      </TaskOverlay>
    </Dialog>
  );
}

/** One commit in the repository's list, GitHub's three lines: the subject
    (truncated) with its age at the right; the body's first line, only when
    there is one; the author line. No avatar: khor has none for a git
    author. */
function CommitRow({ c, onPress }: { c: GitCommit; onPress: () => void }) {
  const line = bodyLine(c);
  return (
    <button
      type="button"
      data-commit-row={c.id}
      onClick={onPress}
      className="flex min-h-list-row min-w-0 flex-col justify-center gap-0.5 rounded-md px-in py-2 text-left hover:bg-fill"
    >
      <span className="flex min-w-0 items-baseline gap-in">
        <span data-commit-subject className="min-w-0 flex-1 truncate text-headline text-foreground">
          {c.subject}
        </span>
        <span data-commit-age className="flex-none text-callout text-tertiary tabular-nums">
          {ago(c.when_ms)}
        </span>
      </span>
      {line !== "" && (
        <span data-commit-body className="truncate text-callout text-muted-foreground">
          {line}
        </span>
      )}
      <span data-commit-author className="truncate text-callout text-muted-foreground">
        {authorLine(c)}
      </span>
    </button>
  );
}

/** Whether a past version has a rendered face: Markdown and notebooks
    (`RevisionView`). A page or a table read from git's text alone would
    be drawn without what sits beside it on disk. */
export const revisionRenders = (path: string): boolean => {
  const r = rendererOf(path);
  return r === "md" || r === "ipynb";
};

/** The revision where the file was: read-only, rendered the way the file
    renders (Markdown, a notebook, or its lines), or compared with the
    file as it is now. */
export function RevisionView({
  machine,
  path,
  rev,
  compare,
  narrow,
  face,
}: {
  machine: string;
  path: string;
  rev: GitCommit;
  compare: boolean;
  narrow: boolean;
  /** The same explicit preview choice as the current file. */
  face?: "source" | "preview";
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
  const renders = rendererOf(path);
  const preview = (face ?? defaultFaceOf(path)) === "preview";
  return (
    <div data-file-rev={rev.id} className="flex min-h-0 flex-1 flex-col">
      {renders === "md" && preview ? (
        <div data-file-preview="md" {...groundAttr(ground)} className="min-h-0 flex-1 overflow-auto">
          <Markdown text={shown.text} className="prose-khor px-6 py-4" machine={machine} path={path} />
        </div>
      ) : renders === "ipynb" && preview && !shown.cut ? (
        // A notebook's past version reads as the notebook does now: its
        // preview by default (`defaultFaceOf`).
        <div data-file-preview="ipynb" {...groundAttr(ground)} className="min-h-0 flex-1 overflow-auto">
          <Notebook text={shown.text} machine={machine} path={path} />
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
