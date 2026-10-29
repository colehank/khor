import { Fragment, useEffect, useState } from "react";
import { fetchGitLog, said, type GitLog } from "@/api";
import { Waiting } from "@/components/RegionState";
import { gui } from "@/gen/catalog";
import type { GitCommit } from "@/gen/bindings/GitCommit";
import { ago } from "@/words";

export function WorkspaceHistory({ machine, root, query, refresh, active, onOpen }: {
  machine: string;
  root: string;
  query: string;
  refresh: number;
  active: string | null;
  onOpen: (commit: GitCommit, pinned: boolean) => void;
}) {
  const [log, setLog] = useState<GitLog | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [since, setSince] = useState(Date.now);
  useEffect(() => {
    let live = true;
    setLog(null);
    setError(null);
    setSince(Date.now());
    fetchGitLog(machine, root).then(
      value => { if (live) setLog(value); },
      failure => { if (live) setError(said(failure)); },
    );
    return () => { live = false; };
  }, [machine, root, refresh]);
  const needle = query.trim().toLocaleLowerCase();
  const commits = log?.commits.filter(commit => `${commit.id} ${commit.subject} ${commit.author}`.toLocaleLowerCase().includes(needle)) ?? [];
  let previous = "";
  return <div data-workspace-log data-files-list className="min-h-0 flex-1 overflow-y-auto">
    {error !== null ? <p role="alert" className="px-page py-in text-callout text-state-failed">{error}</p>
      : log === null ? <Waiting since={since} />
      : <div data-workspace-log-list>{commits.map(commit => {
        const day = new Date(commit.when_ms).toLocaleDateString();
        const startsDay = day !== previous;
        previous = day;
        return <Fragment key={commit.id}>
          {startsDay && <div data-workspace-log-day className="text-caption text-muted-foreground">{day}</div>}
          <button type="button" data-workspace-log-commit={commit.id} data-open={active === commit.id}
            onClick={() => onOpen(commit, false)} onDoubleClick={() => onOpen(commit, true)}
            className="workspace-log-row w-full min-w-0 text-left text-callout hover:bg-fill data-[open=true]:bg-fill-strong">
            <span className="block truncate text-foreground">{commit.subject}</span>
            <span className="flex min-w-0 gap-in text-caption text-muted-foreground">
              <span className="min-w-0 flex-1 truncate">{commit.author} · {commit.id.slice(0, 7)}</span><span className="flex-none">{ago(commit.when_ms)}</span>
            </span>
          </button>
        </Fragment>;
      })}</div>}
    {log?.more && <p data-workspace-log-more className="px-page py-in text-caption text-muted-foreground">{gui.file_history_more}</p>}
  </div>;
}
