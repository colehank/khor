// A workspace retains each commit's presentation data; standalone pages
// keep their own navigation. Neither mode retains hidden editor views.
import { useCallback, useEffect, useMemo, useRef, useState, useReducer } from "react";

import { fetchGitCommit, said } from "@/api";
import { CommitDiff, preloadCommitDiff } from "@/components/CommitDiff.lazy";
import { HistoryDeck } from "@/components/HistoryDeck";
import { useNarrow } from "@/hooks/use-narrow";
import { GlassCircle } from "@/components/Glass";
import { IconBack, IconChevronDown, IconChevronRight } from "@/components/icons";
import { Waiting } from "@/components/RegionState";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { gui } from "@/gen/catalog";
import type { GitCommit } from "@/gen/bindings/GitCommit";
import type { GitCommitSummary } from "@/gen/bindings/GitCommitSummary";
import type { GitCommitFile } from "@/gen/bindings/GitCommitFile";
import type { GitCommitShow } from "@/gen/bindings/GitCommitShow";
import { cn } from "@/lib/utils";
import { authorLine, bodyOf } from "@/views/FileHistory";
import { markInk } from "@/views/fileRow";
import { ago, basename } from "@/words";
import { useKeptScrollRef, type KeptScrollSlot } from "@/hooks/use-kept-scroll";
import {band,stickyGroups} from "@/lib/scroll-edge";

export type CommitPageState = { collapsed: Set<string>; scroll: KeptScrollSlot };

export function CommitPage({
  machine,
  root,
  commit,
  onBack,
  edgeToEdge = false,
  presentation = "page",
  state,
  title,
}: {
  machine: string;
  /** The repository, on that machine. */
  root: string;
  /** The row that was pressed: the head is drawn from it at once, before
      the files arrive. */
  commit: GitCommit | GitCommitSummary | {id:string};
  title?: string;
  onBack: () => void;
  /** The main workspace owns the full phone viewport; inset sheets do not. */
  edgeToEdge?: boolean;
  presentation?: "page" | "tab";
  state?: CommitPageState;
}) {
  const narrow = useNarrow();
  const local = useRef<CommitPageState>({ collapsed: new Set(), scroll: {} });
  const retained = state ?? local.current;
  const [, redraw] = useReducer((n: number) => n + 1, 0);
  const setExpanded = (path: string, expanded: boolean) => {
    if (expanded) retained.collapsed.delete(path);
    else retained.collapsed.add(path);
    redraw();
  };
  const reading = useKeptScrollRef(JSON.stringify([machine, root, commit.id]), retained.scroll);
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
  const metadata = shown?.commit.id === commit.id ? shown.commit : "body" in commit ? commit : null;
  const subject = metadata?.subject ?? ('subject' in commit ? commit.subject : title || commit.id.slice(0,7));
  const body = metadata === null ? "" : bodyOf(metadata);
  const scroller = useRef<HTMLDivElement>(null);
  const selectedFile = useRef<string | null>(null);
  // Ref cleanup runs before a width-mode registration takes the same port.
  const attachScroller = useCallback((node:HTMLDivElement|null) => {
    scroller.current=node;
    const release=narrow || presentation === "tab" ? stickyGroups(node) : undefined;
    const releaseReading = shown !== null ? reading(node) : undefined;
    return()=>{releaseReading?.();release?.();if(scroller.current===node)scroller.current=null;};
  },[narrow, presentation, shown, reading]);
  const jump = (path: string) => {
    const box = scroller.current;
    const bar = [...(box?.querySelectorAll<HTMLElement>("[data-commit-file]") ?? [])].find((e) => e.dataset.commitFile === path);
    if (!box || !bar) return;
    setExpanded(path, true);
    bar.focus({ preventScroll: true });
    const inset = parseFloat(getComputedStyle(box).scrollPaddingTop) || 0;
    box.scrollTop += bar.getBoundingClientRect().top - box.getBoundingClientRect().top - inset;
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
    <div data-commit-page={commit.id} data-history-edge={narrow && edgeToEdge || undefined} className="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-card">
      {!narrow && presentation === "page" && <div ref={band} data-band data-commit-head className="flex flex-none items-center gap-in px-row py-in">
        <GlassCircle aria-label={gui.back} tip={gui.back} data-commit-back onClick={onBack}>
          <IconBack />
        </GlassCircle>
        <span className="min-w-0 flex-1" />
        <span className="flex-none font-mono text-callout text-tertiary">{commit.id.slice(0, 7)}</span>
      </div>}
      <div
        ref={attachScroller}
        data-commit-scroll
        className="min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto pb-in"
      >
        <header data-sticky-body data-selectable className="flex flex-col gap-1 px-row py-in">
          <h1 data-commit-title className="text-title-sm text-foreground">
            {subject}
          </h1>
          {body !== "" && (
            <p data-commit-message className="whitespace-pre-wrap text-body text-muted-foreground">
              {body}
            </p>
          )}
          <span data-commit-by className="text-callout text-muted-foreground">
            {metadata ? <>{authorLine(metadata)} · {ago(metadata.when_ms)} · </> : 'author' in commit ? <>{gui.commit_authored_one(commit.author)} · {ago(commit.when_ms)} · </> : null}<span className="font-mono">{commit.id}</span>
          </span>
        </header>
        {refused !== null ? (
          <div data-commit-refused className="px-row text-callout text-state-failed">
            {refused}
          </div>
        ) : shown === null || sums === undefined ? (
          <Waiting since={since} />
        ) : (
          <>
            <div data-sticky-body data-commit-stat className="flex flex-wrap items-center justify-between gap-in border-y border-border px-row py-2 text-callout tabular-nums"><span>
              {gui.commit_changed(shown.total)}
              {" · "}
              <span className="text-git-added">+{sums.added}</span> <span className="text-git-deleted">−{sums.removed}</span></span>
              {jumps && (
                      <div data-commit-file-index>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <button
                              type="button"
                              data-commit-jump
                              className="flex min-h-touch items-center gap-xxs rounded-sm px-in text-callout hover:bg-fill"
                            >
                              {gui.commit_jump}<IconChevronDown className="size-affordance"/>
                            </button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent
                            side="bottom" align="center" sideOffset={8} data-commit-jump-menu className="overflow-hidden"
                            onCloseAutoFocus={(event) => {
                              const path = selectedFile.current;
                              selectedFile.current = null;
                              if (path === null) return;
                              event.preventDefault();
                              jump(path);
                            }}
                          >
                            {shown.files.map((f) => (
                              <DropdownMenuItem key={f.path} data-commit-jump-item={f.path} onSelect={() => { selectedFile.current = f.path; }}>
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
            {shown.files.map((f) => (
              <FileChange key={f.path} f={f} full={presentation === "tab"} expanded={!retained.collapsed.has(f.path)} onToggle={() => setExpanded(f.path, retained.collapsed.has(f.path))} />
            ))}
            {shown.more && (
              <div data-commit-more className="px-row py-in text-callout text-muted-foreground">
                {gui.commit_more}
              </div>
            )}
          </>
        )}
      </div>
      {narrow && presentation === "page" && <HistoryDeck title={commit.id.slice(0, 7)} subtitle={subject} onBack={onBack} commit/>}
    </div>
  );
}

/** One file's bar and its change. The bar stays at the top while the
    change scrolls under it, as GitHub's does, so a long file still says
    whose lines these are. */
function FileChange({ f, expanded, onToggle, full }: { f: GitCommitFile; expanded: boolean; onToggle: () => void; full: boolean }) {
  return (
    <section data-sticky-group data-commit-file={f.path} tabIndex={-1} aria-label={f.path} className="border-b border-border">
      <div
        data-commit-file-bar
        data-sticky-head
        className="sticky top-0 z-10 flex min-w-0 pointer-coarse:min-h-touch items-center gap-in border-b border-border bg-card px-row py-2"
      >
        <button type="button" data-commit-collapse data-glass-hit aria-label={f.path} aria-expanded={expanded} onClick={onToggle} className="relative flex size-affordance flex-none items-center justify-center">
          {expanded ? <IconChevronDown className="size-affordance" /> : <IconChevronRight className="size-affordance" />}
        </button>
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
      <div data-sticky-body="flush" hidden={!expanded}>{expanded && (f.binary ? (
        <div data-commit-binary className="px-row py-2 text-callout text-muted-foreground">
          {gui.file_not_text}
        </div>
      ) : f.big ? (
        <div data-commit-big className="px-row py-2 text-callout text-muted-foreground">
          {gui.commit_file_big}
        </div>
      ) : f.capped ? (
        <div data-commit-capped className="px-row py-2 text-callout text-muted-foreground">
          {gui.commit_file_capped}
        </div>
      ) : (
        f.before !== f.after && <CommitDiff path={f.path} before={f.before} after={f.after} full={full} />
      ))}</div>
    </section>
  );
}
