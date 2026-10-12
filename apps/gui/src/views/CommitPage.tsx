// **One commit, as GitHub's commit page says it** (COMMITS-BRIEF,
// 2026-10-12, the owner: 「提交历史应该如 github，点开后显示这个 commit 的
// 更改」). Opened from a row of the repository's 提交历史; a page, not a
// sheet — it scrolls, and its head has ‹. On the wide face it stands where
// the file does; on the phone it is a screen.
//
// Top to bottom: who wrote it and when, the subject, the body; 「已更改 N
// 个文件」 with the lines added and removed; then each file — a bar with
// its letter and path, and under it the change with both sides' line
// numbers.
//
// **Each file's change is CodeMirror's unified merge view** (cm-d item 1,
// `components/CommitDiff`), not `DiffView`: that one is the whole file
// against the disk, and neither side of a commit is on disk. The node
// answers both sides' texts (`khor_node::git::CommitFile`) and the page
// diffs them — syntax ink on both sides, the unchanged stretches folded
// behind 「展开未更改的 N 行」, the ink the diff tab's: `git-deleted` /
// `git-added` at 10 % behind the line. Each view is as tall as its lines,
// so the page is still one scroll on a phone. A side past the node's byte
// cap (`big`) says so in one line, as a binary does.
//
// **「跳转到文件」** floats at the foot, GitHub's pill: the changed files
// as a menu, a press scrolls that file's bar to the top. Drawn only where
// there is somewhere to jump — two files or more.
import { useEffect, useMemo, useRef, useState } from "react";

import { fetchGitCommit, said } from "@/api";
import { CommitDiff, preloadCommitDiff } from "@/components/CommitDiff.lazy";
import { GlassCircle } from "@/components/Glass";
import { IconBack } from "@/components/icons";
import { Waiting } from "@/components/RegionState";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { gui } from "@/gen/catalog";
import type { GitCommit } from "@/gen/bindings/GitCommit";
import type { GitCommitFile } from "@/gen/bindings/GitCommitFile";
import type { GitCommitShow } from "@/gen/bindings/GitCommitShow";
import { cn } from "@/lib/utils";
import { authorLine, bodyOf, shortId } from "@/views/FileHistory";
import { markInk } from "@/views/fileRow";
import { ago, basename } from "@/words";

export function CommitPage({
  machine,
  root,
  commit,
  onBack,
}: {
  machine: string;
  /** The repository, on that machine. */
  root: string;
  /** The row that was pressed: the head is drawn from it at once, before
      the files arrive. */
  commit: GitCommit;
  onBack: () => void;
}) {
  const [shown, setShown] = useState<GitCommitShow | null>(null);
  const [refused, setRefused] = useState<string | null>(null);
  const [since, setSince] = useState(() => Date.now());
  useEffect(() => {
    let live = true;
    // The diff's chunk travels while the node answers.
    preloadCommitDiff();
    setShown(null);
    setRefused(null);
    setSince(Date.now());
    fetchGitCommit(machine, root, commit.id).then(
      (s) => live && setShown(s),
      (e: unknown) => live && setRefused(said(e)),
    );
    return () => {
      live = false;
    };
  }, [machine, root, commit.id]);
  const body = bodyOf(commit);
  const scroller = useRef<HTMLDivElement>(null);
  const jump = (path: string) => {
    const box = scroller.current;
    const bar = [...(box?.querySelectorAll<HTMLElement>("[data-commit-file]") ?? [])].find((e) => e.dataset.commitFile === path);
    if (!box || !bar) return;
    box.scrollTop += bar.getBoundingClientRect().top - box.getBoundingClientRect().top;
  };
  const jumps = shown !== null && shown.files.length > 1;
  const sums = useMemo(
    () =>
      shown?.files.reduce((s, f) => ({ added: s.added + f.added, removed: s.removed + f.removed }), {
        added: 0,
        removed: 0,
      }),
    [shown],
  );
  return (
    <div data-commit-page={commit.id} className="relative flex min-h-0 flex-1 flex-col bg-card">
      <div data-commit-head className="flex flex-none items-center gap-in px-row py-in">
        <GlassCircle aria-label={gui.back} tip={gui.back} data-commit-back onClick={onBack}>
          <IconBack />
        </GlassCircle>
        <span className="min-w-0 flex-1" />
        <span className="flex-none font-mono text-callout text-tertiary">{shortId(commit)}</span>
      </div>
      <div
        ref={scroller}
        data-commit-scroll
        className={cn(
          "min-h-0 flex-1 overflow-y-auto",
          jumps ? "pb-[calc(var(--safe-bottom)+var(--glass-btn)+2*var(--gap-in))]" : "pb-[var(--safe-bottom)]",
        )}
      >
        <header data-selectable className="flex flex-col gap-1 px-row pb-in">
          <span data-commit-by className="text-callout text-muted-foreground">
            {authorLine(commit)} · {ago(commit.when_ms)}
          </span>
          <h1 data-commit-title className="text-title-sm text-foreground">
            {commit.subject}
          </h1>
          {body !== "" && (
            <p data-commit-message className="whitespace-pre-wrap text-body text-muted-foreground">
              {body}
            </p>
          )}
        </header>
        {refused !== null ? (
          <div data-commit-refused className="px-row text-callout text-state-failed">
            {refused}
          </div>
        ) : shown === null || sums === undefined ? (
          <Waiting since={since} />
        ) : (
          <>
            <div data-commit-stat className="border-y border-border px-row py-2 text-callout tabular-nums">
              {gui.commit_changed(shown.total)}
              {" · "}
              <span className="text-git-added">+{sums.added}</span> <span className="text-git-deleted">−{sums.removed}</span>
            </div>
            {shown.files.map((f) => (
              <FileChange key={f.path} f={f} />
            ))}
            {shown.more && (
              <div data-commit-more className="px-row py-in text-callout text-muted-foreground">
                {gui.commit_more}
              </div>
            )}
          </>
        )}
      </div>
      {jumps && (
        <div className="pointer-events-none absolute inset-x-0 bottom-[calc(var(--safe-bottom)+var(--gap-in))] flex justify-center">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                data-commit-jump
                className="glass pointer-events-auto flex h-glass-btn items-center rounded-full px-row text-body text-foreground"
              >
                {gui.commit_jump}
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent side="top" align="center" sideOffset={8} data-commit-jump-menu className="max-h-[50dvh] overflow-y-auto">
              {shown.files.map((f) => (
                <DropdownMenuItem key={f.path} data-commit-jump-item={f.path} onSelect={() => jump(f.path)}>
                  <span className={cn("w-[1ch] flex-none font-mono text-caption", markInk(f.status))}>{f.status}</span>
                  <span className="min-w-0 truncate" title={f.path}>
                    {basename(f.path)}
                  </span>
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      )}
    </div>
  );
}

/** One file's bar and its change. The bar stays at the top while the
    change scrolls under it, as GitHub's does, so a long file still says
    whose lines these are. */
function FileChange({ f }: { f: GitCommitFile }) {
  return (
    <section data-commit-file={f.path} className="border-b border-border">
      <div
        data-commit-file-bar
        className="sticky top-0 z-10 flex min-w-0 items-center gap-in border-b border-border bg-card px-row py-2"
      >
        <span className={cn("w-[1ch] flex-none font-mono text-caption", markInk(f.status))}>{f.status}</span>
        <span data-selectable className="min-w-0 flex-1 truncate font-mono text-callout text-foreground" title={f.path}>
          {f.from !== "" ? `${f.from} → ${f.path}` : f.path}
        </span>
        {!f.binary && (
          <span className="flex-none text-caption tabular-nums">
            <span className="text-git-added">+{f.added}</span> <span className="text-git-deleted">−{f.removed}</span>
          </span>
        )}
      </div>
      {f.binary ? (
        <div data-commit-binary className="px-row py-2 text-callout text-muted-foreground">
          {gui.file_not_text}
        </div>
      ) : f.big ? (
        <div data-commit-big className="px-row py-2 text-callout text-muted-foreground">
          {gui.commit_file_big}
        </div>
      ) : (
        f.before !== f.after && <CommitDiff path={f.path} before={f.before} after={f.after} />
      )}
    </section>
  );
}
