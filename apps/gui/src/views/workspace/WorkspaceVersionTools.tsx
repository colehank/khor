import { useRef, useState } from "react";
import { exploreGit, said, type GitExploreResult } from "@/api";
import { CommitDiff } from "@/components/CommitDiff.lazy";
import { FileKindIcon } from "@/components/FileKindIcon";
import { IconBranch, IconChevronDown, IconChevronRight } from "@/components/icons";
import { RegionState } from "@/components/RegionState";
import { Button } from "@/components/ui/button";
import { Dialog, DialogBody, DialogHeader, DialogTitle, TaskActions, TaskOverlay } from "@/components/ui/dialog";
import { PopupButton, type PopupChoice } from "@/components/ui/popup-button";
import { gui } from "@/gen/catalog";
import type { GitCommitSummary } from "@/gen/bindings/GitCommitSummary";
import { basename } from "@/words";
import { WorkspaceRemoteRefs } from "./WorkspaceRemoteRefs";
import { useGitResource, versionChoices, versionName, type GitLocation, type GitTool, type Version } from "./git-versions";

type Selectable = GitLocation & { version: Version | null };
type Comparison = Extract<GitExploreResult, {kind: "compare"}>;
type Merge = Extract<GitExploreResult, {kind: "merge"}>;

export function WorkspaceVersionTools({ tool, onClose, onWorktree, onVersion, onCommit, ...location }: Selectable & {
  tool: GitTool;
  onClose: () => void;
  onWorktree?: (path: string) => void;
  onVersion: (version: Version) => void;
  onCommit: (commit: GitCommitSummary) => void;
}) {
  const title = { compare: gui.git_compare, merge: gui.git_merge_preview, worktrees: gui.git_worktrees, remotes: gui.git_remote_panel }[tool];
  return <Dialog open onOpenChange={open => { if (!open) onClose(); }}>
    <TaskOverlay purpose="selection" initialHeight={tool === "worktrees" ? "fit" : undefined} scrollable aria-describedby={undefined} data-git-tool={tool}>
      <DialogHeader><DialogTitle>{title}</DialogTitle></DialogHeader>
      <DialogBody>
        {tool === "worktrees" ? <Worktrees {...location} onOpen={onWorktree ? path => { onClose(); onWorktree(path); } : undefined} />
          : tool === "remotes" ? <WorkspaceRemoteRefs {...location} onVersion={version => { onClose(); onVersion(version); }} />
            : <CompareVersions {...location} kind={tool} onCommit={commit => { onClose(); onCommit(commit); }} />}
      </DialogBody>
    </TaskOverlay>
  </Dialog>;
}

function CompareVersions({ kind, version, onCommit, ...location }: Selectable & {
  kind: "compare" | "merge"; onCommit: (commit: GitCommitSummary) => void;
}) {
  const references = useGitResource(location, { kind: "references" }, "references");
  const [left, setLeft] = useState("HEAD");
  const [right, setRight] = useState(version?.commit ?? "");
  const [busy, setBusy] = useState(false);
  const running = useRef(false);
  const [answer, setAnswer] = useState<Comparison | Merge | null>(null);
  const [error, setError] = useState<string | null>(null);
  const ready = references.value;
  if (!ready) return <RegionState state={references.error ? "error" : "loading-never"} message={references.error} onRetry={references.retry} />;
  const choices: PopupChoice[] = [
    { value: "HEAD", label: ready.current ? versionName(ready.current) : gui.git_branch_head, note: gui.git_branch_current },
    ...versionChoices(ready.refs.filter(ref => ref !== ready.current && ref !== version?.reference)),
  ];
  if (version) choices.splice(1, 0, { value: version.commit, label: versionName(version.reference), note: version.commit.slice(0, 7) });
  const clear = () => { setAnswer(null); setError(null); };
  const start = async () => {
    if (running.current || !right) return;
    running.current = true;
    setBusy(true);
    setError(null);
    try {
      setAnswer(await exploreGit(location.machine, location.root, { kind, base: left, head: right }, kind));
    } catch (error) {
      setError(said(error));
    } finally {
      running.current = false;
      setBusy(false);
    }
  };
  return <div className="flex min-w-0 flex-col gap-row">
    <div className="grid gap-row sm:grid-cols-2">
      <div className="flex min-w-0 flex-col gap-in">
        <span className="text-caption text-muted-foreground">{gui.git_base_version}</span>
        <PopupButton look="field" label={gui.git_base_version} value={left} choices={choices} disabled={busy}
          mark={{ "data-git-compare-base": true }} onPick={value => { setLeft(value); clear(); }} />
      </div>
      <div className="flex min-w-0 flex-col gap-in">
        <span className="text-caption text-muted-foreground">{gui.git_target_version}</span>
        <PopupButton look="field" label={gui.git_target_version} value={right} choices={choices} disabled={busy}
          mark={{ "data-git-compare-head": true }} placeholder={gui.git_select_version} onPick={value => { setRight(value); clear(); }} />
      </div>
    </div>
    <TaskActions primary={<Button onClick={() => void start()} disabled={!right || busy} aria-busy={busy} data-git-run>
      {kind === "compare" ? gui.git_compare_run : gui.git_merge_run}
    </Button>} />
    {busy && <RegionState state="loading-never" />}
    {error && <RegionState state="error" message={error} onRetry={() => void start()} />}
    {answer?.kind === "compare" && <ComparisonResult value={answer} onCommit={onCommit} />}
    {answer?.kind === "merge" && <div data-git-merge-result role="status" className="flex flex-col gap-in">
      {answer.total === 0 ? <p className="text-body">{gui.git_merge_clean}</p> : <>
        <h3 className="text-body font-label">{gui.git_merge_conflicts} · {answer.total}</h3>
        <ul className="divide-y divide-border">
          {answer.conflicts.map(path => <li key={path} className="flex items-center gap-in py-in text-callout">
            <FileKindIcon name={basename(path)} className="size-affordance flex-none" /><span className="min-w-0 break-all">{path}</span>
          </li>)}
        </ul>
        {answer.conflicts.length < answer.total && <p className="text-caption text-muted-foreground">{gui.git_compare_more}</p>}
      </>}
    </div>}
  </div>;
}

function ComparisonResult({ value, onCommit }: { value: Comparison; onCommit: (commit: GitCommitSummary) => void }) {
  const [file, setFile] = useState<string | null>(null);
  const groups: [string, number, GitCommitSummary[]][] = [
    [gui.git_compare_base_only, value.behind, value.base_only],
    [gui.git_compare_head_only, value.ahead, value.head_only],
  ];
  return <section data-git-compare-result className="flex min-w-0 flex-col gap-row">
    <p className="text-caption text-muted-foreground">{value.ancestor
      ? <>{gui.git_compare_ancestor} <span className="font-mono">{value.ancestor.slice(0, 7)}</span></>
      : gui.git_compare_no_ancestor}</p>
    {groups.map(([label, count, commits]) => {
      return <details key={label}>
        <summary className="min-h-touch rounded-sm py-in text-callout hover:bg-fill">{label} · {count}{value.commits_cut ? "+" : ""}</summary>
        {commits.map(commit => <button key={commit.id} type="button" onClick={() => onCommit(commit)}
          className="flex min-h-touch w-full min-w-0 flex-col justify-center rounded-sm px-in text-start text-callout hover:bg-fill">
          <span className="max-w-full truncate">{commit.subject}</span>
          <span className="text-caption text-muted-foreground">{commit.author} · {commit.id.slice(0, 7)}</span>
        </button>)}
        {commits.length < count && <p className="px-in text-caption text-muted-foreground">{gui.git_compare_more}</p>}
      </details>;
    })}
    <div className="flex min-w-0 flex-col gap-in">
      <h3 className="text-body font-label">{gui.git_compare_changes} · {value.total}</h3>
      {value.total === 0 && <p className="text-callout text-muted-foreground">{gui.git_compare_same}</p>}
      {value.files.map(change => {
        const open = file === change.path;
        const Chevron = open ? IconChevronDown : IconChevronRight;
        return <div key={change.path} className="min-w-0 border-b border-border">
          <button type="button" aria-expanded={open} data-git-compare-file={change.path}
            onClick={() => setFile(open ? null : change.path)}
            className="flex min-h-touch w-full min-w-0 items-center gap-in rounded-sm text-start text-callout hover:bg-fill">
            <Chevron className="size-affordance flex-none text-muted-foreground" />
            <FileKindIcon name={basename(change.path)} className="size-affordance flex-none" />
            <span className="min-w-0 flex-1 break-all">{change.from ? `${change.from} → ${change.path}` : change.path}</span>
            <span className="font-mono text-caption text-muted-foreground">{change.status}</span>
          </button>
          {open && (change.binary || change.big || change.capped
            ? <p className="px-in py-in text-caption text-muted-foreground">{change.binary ? gui.git_snapshot_file_binary : gui.git_snapshot_file_large}</p>
            : <CommitDiff path={change.path} before={change.before} after={change.after} />)}
        </div>;
      })}
      {value.more && <p className="text-caption text-muted-foreground">{gui.git_compare_more}</p>}
    </div>
  </section>;
}

function Worktrees({ onOpen, ...location }: GitLocation & { onOpen?: (path: string) => void }) {
  const request = useGitResource(location, { kind: "worktrees" }, "worktrees");
  const value = request.value;
  if (!value) return <RegionState state={request.error ? "error" : "loading-never"} message={request.error} onRetry={request.retry} />;
  return <div data-git-worktrees className="divide-y divide-border">
    {value.items.map(item => <div key={item.path} data-git-worktree={item.path} className="flex min-h-touch items-center gap-in py-in">
      <IconBranch className="size-affordance flex-none text-muted-foreground" />
      <div className="min-w-0 flex-1">
        <p className="break-all text-body">{item.reference ? versionName(item.reference) : gui.git_branch_head}
          {item.current && <span className="ms-in text-caption text-muted-foreground">{gui.git_branch_current}</span>}</p>
        <p className="break-all text-caption text-muted-foreground">{item.path}</p>
        {!item.available && <p className="text-caption text-muted-foreground">{gui.git_worktree_unavailable}</p>}
      </div>
      {!item.current && item.available && onOpen && <Button variant="secondary" onClick={() => onOpen(item.path)}>{gui.git_worktree_open}</Button>}
    </div>)}
    {value.more && <p className="py-in text-caption text-muted-foreground">{gui.git_compare_more}</p>}
  </div>;
}
