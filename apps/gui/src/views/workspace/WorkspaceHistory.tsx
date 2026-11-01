import { Fragment, useCallback, useEffect, useState, useSyncExternalStore } from "react";
import { fetchGitHistory, said } from "@/api";
import { Waiting } from "@/components/RegionState";
import { IconReload } from "@/components/icons";
import { gui } from "@/gen/catalog";
import type { GitCommitSummary } from "@/gen/bindings/GitCommitSummary";
import { useKeptScrollRef } from "@/hooks/use-kept-scroll";
import { ago } from "@/words";
import type { WorkspaceHistoryMemory } from "./workspace-history";

export function WorkspaceHistory({ machine, root, query, memory, active, onOpen, liveHead }: {
  machine: string;
  root: string;
  query: string;
  memory: WorkspaceHistoryMemory;
  active: string | null;
  onOpen: (commit: GitCommitSummary, pinned: boolean) => void;
  liveHead?: { known: boolean; id: string | null };
}) {
  const view = useSyncExternalStore(memory.subscribe, memory.snapshot, memory.snapshot);
  const [since, setSince] = useState(Date.now);
  const request = useCallback(async (cursor: string | null, limit: number) => {
    try { return await fetchGitHistory(machine, root, cursor, limit); }
    catch (error) { throw new Error(said(error)); }
  }, [machine, root]);
  useEffect(() => memory.attach(), [memory]);
  useEffect(() => {
    if (!view.busy && view.refreshRequested) void memory.refresh(request);
  }, [memory, request, view.busy, view.refreshRequested]);
  useEffect(() => { if (view.busy) setSince(Date.now()); }, [view.busy]);
  const current = view.current;
  const [emptyScroll] = useState(() => ({}));
  const place = JSON.stringify([machine, root, view.generation, current?.data.head, current?.index, query]);
  const reading = useKeptScrollRef(
    place,
    current?.scroll ?? emptyScroll,
  );
  const needle = query.trim().toLocaleLowerCase();
  const commits = current?.data.commits.filter(commit => `${commit.id} ${commit.subject} ${commit.author}`.toLocaleLowerCase().includes(needle)) ?? [];
  let previous = "";
  return <div key={place} ref={reading} data-workspace-log data-files-list aria-busy={view.busy} className="min-h-0 flex-1 overflow-y-auto">
    {current !== null && liveHead?.known && liveHead.id !== current.data.head && <div data-workspace-log-updated className="flex items-center justify-between gap-in px-page py-in text-caption text-muted-foreground">
      <span>{gui.git_history_updated}</span>
      <button type="button" disabled={view.busy} onClick={() => memory.invalidate()} className="min-h-touch text-foreground disabled:opacity-50">{gui.file_reread}</button>
    </div>}
    {current === null && view.busy ? <Waiting since={since} /> : current !== null && <div data-workspace-log-list>{commits.map(commit => {
      const day = new Date(commit.when_ms).toLocaleDateString();
      const startsDay = day !== previous;
      previous = day;
      return <Fragment key={commit.id}>
        {startsDay && <div data-workspace-log-day className="text-caption text-muted-foreground">{day}</div>}
        <button type="button" data-workspace-log-commit={commit.id} data-open={active === commit.id}
          onClick={() => onOpen(commit, false)} onDoubleClick={() => onOpen(commit, true)}
          className="workspace-log-row w-full min-w-0 text-left text-callout hover:bg-fill data-[open=true]:bg-fill-strong">
          <span className="block truncate text-foreground">{commit.subject}{commit.cut && "…"}</span>
          <span className="flex min-w-0 gap-in text-caption text-muted-foreground">
            <span className="min-w-0 flex-1 truncate">{commit.author} · {commit.id.slice(0, 7)}</span><span className="flex-none">{ago(commit.when_ms)}</span>
          </span>
        </button>
      </Fragment>;
    })}{commits.length === 0 && <p className="px-page py-in text-callout text-muted-foreground">{gui.no_matches}</p>}</div>}
    {view.error !== null && <div data-workspace-log-error className="px-page py-in text-callout">
      <p role="alert" className="text-state-failed">{view.error}</p>
      <button type="button" disabled={view.busy} onClick={() => { void memory.retry(request); }} className="min-h-touch text-foreground disabled:opacity-50">{gui.region_retry}</button>
    </div>}
    {current !== null && <nav aria-label={gui.commit_history} data-workspace-log-pages className="flex items-center justify-between gap-in px-page py-in text-callout">
      <button type="button" data-workspace-log-newer disabled={view.busy || view.refreshRequested || current.index === 0}
        onClick={() => { void memory.go(current.index - 1, request); }} className="min-h-touch rounded-sm px-in hover:bg-fill disabled:opacity-50">{gui.git_history_newer}</button>
      <span className="flex items-center gap-in text-caption tabular-nums text-muted-foreground">{view.busy && <IconReload aria-hidden className="size-affordance motion-safe:animate-spin" />}{current.index + 1}</span>
      <button type="button" data-workspace-log-older disabled={view.busy || view.refreshRequested || current.data.next === null}
        onClick={() => { void memory.go(current.index + 1, request); }} className="min-h-touch rounded-sm px-in hover:bg-fill disabled:opacity-50">{gui.git_history_older}</button>
    </nav>}
  </div>;
}
