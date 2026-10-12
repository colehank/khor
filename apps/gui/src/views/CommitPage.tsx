// **One commit, as GitHub's commit page says it** (COMMITS-BRIEF,
// 2026-10-12, the owner: 「提交历史应该如 github，点开后显示这个 commit 的
// 更改」). Opened from a row of the repository's 提交历史; a page, not a
// sheet — it scrolls, and its head has ‹. On the wide face it stands where
// the file does; on the phone it is a screen.
//
// Top to bottom: who wrote it and when, the subject, the body; 「已更改 N
// 个文件」 with the lines added and removed; then each file — a bar with
// its letter and path, and under it the commit's own hunks with both
// sides' line numbers.
//
// **The hunks are drawn here, not by `DiffView`.** That one lays one file
// out whole beside HEAD in CodeMirror's merge view, from the file as it is
// on disk; a commit is up to fifty files, neither side of which is on
// disk, and fifty merge editors is not a page. The node answers `git
// show`'s unified hunks (`khor_node::git::CommitHunk`) and the ink is the
// merge view's own: `git-deleted` / `git-added` at 10 % behind the line.
//
// **「跳转到文件」** floats at the foot, GitHub's pill: the changed files
// as a menu, a press scrolls that file's bar to the top. Drawn only where
// there is somewhere to jump — two files or more.
import { useEffect, useMemo, useRef, useState } from "react";

import { fetchGitCommit, said } from "@/api";
import { GlassCircle } from "@/components/Glass";
import { IconBack } from "@/components/icons";
import { Waiting } from "@/components/RegionState";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { gui } from "@/gen/catalog";
import type { GitCommit } from "@/gen/bindings/GitCommit";
import type { GitCommitFile } from "@/gen/bindings/GitCommitFile";
import type { GitCommitHunk } from "@/gen/bindings/GitCommitHunk";
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

/** One file's bar and its hunks. The bar stays at the top while its hunks
    scroll under it, as GitHub's does, so a long file still says whose
    lines these are. */
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
        <div className="px-row py-2 text-callout text-muted-foreground">{gui.file_not_text}</div>
      ) : (
        f.hunks.length > 0 && (
          <div data-commit-diff data-selectable className="overflow-x-auto font-mono text-mono">
            <div className="w-max min-w-full">
              {f.hunks.map((h, i) => (
                <Hunk key={i} h={h} />
              ))}
            </div>
          </div>
        )
      )}
    </section>
  );
}

const TINT = {
  "+": "color-mix(in srgb, var(--git-added) 10%, transparent)",
  "-": "color-mix(in srgb, var(--git-deleted) 10%, transparent)",
} as const;

/** `@@ −a +b @@`, then each line with the old and the new number (blank on
    the side it is not on), its sign, and its text. */
function Hunk({ h }: { h: GitCommitHunk }) {
  let o = h.old_at;
  let n = h.new_at;
  return (
    <>
      <div data-commit-hunk className="bg-fill px-row text-tertiary select-none">
        @@ −{h.old_at} +{h.new_at} @@
      </div>
      {h.lines.map((l, i) => {
        const k = l[0] === "+" || l[0] === "-" ? l[0] : " ";
        const old = k === "+" ? null : o++;
        const now = k === "-" ? null : n++;
        return (
          <div
            key={i}
            data-diff-line={k === " " ? "same" : k === "+" ? "added" : "removed"}
            className="flex"
            style={k === " " ? undefined : { background: TINT[k] }}
          >
            <span className="w-[5ch] flex-none pe-1 text-right text-tertiary tabular-nums select-none">{old}</span>
            <span className="w-[5ch] flex-none pe-1 text-right text-tertiary tabular-nums select-none">{now}</span>
            <span
              className={cn(
                "w-[2ch] flex-none text-center select-none",
                k === "+" ? "text-git-added" : k === "-" ? "text-git-deleted" : "",
              )}
            >
              {k === "+" ? "+" : k === "-" ? "−" : ""}
            </span>
            <span className="whitespace-pre pe-row">{l.slice(1)}</span>
          </div>
        );
      })}
    </>
  );
}
