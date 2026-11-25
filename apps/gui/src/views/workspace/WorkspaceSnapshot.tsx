import { useRef, useState } from "react";
import { exploreGit, said, type GitExploreResult } from "@/api";
import { CodeEditor } from "@/components/CodeEditor.lazy";
import { FileKindIcon } from "@/components/FileKindIcon";
import { IconChevronDown, IconChevronRight, IconFiles } from "@/components/icons";
import { RegionState } from "@/components/RegionState";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { useGitResource, type GitLocation, type Version } from "./git-versions";

type SnapshotProps = GitLocation & { version: Version };
type TreeProps = SnapshotProps & { selected: string | null; onOpen: (path: string) => void };
type DirectoryPage = Extract<GitExploreResult, {kind: "directory"}>;

export function WorkspaceSnapshotTree(props: TreeProps) {
  return <div data-git-snapshot-tree className="min-h-0 flex-1 overflow-y-auto">
    <SnapshotDirectory {...props} path="" depth={0} />
  </div>;
}

function SnapshotDirectory({ path, depth, ...props }: TreeProps & { path: string; depth: number }) {
  const { machine, root, version, selected, onOpen } = props;
  const query = { kind: "directory", commit: version.commit, path } as const;
  const page = useGitResource(props, { ...query, offset: 0 }, "directory");
  const [append, setAppend] = useState<DirectoryPage[]>([]);
  const [expanded, setExpanded] = useState(() => new Map<string, boolean>());
  const busy = useRef(false);
  const [loading, setLoading] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const first = page.value;
  if (!first) return <RegionState state={page.error ? "error" : "loading-never"} message={page.error} onRetry={page.retry} />;
  const last = append.at(-1) ?? first;
  const entries = [first, ...append].flatMap(value => value.entries);
  const next = last.next;
  const more = async () => {
    if (next === null || busy.current) return;
    busy.current = true;
    setLoading(true);
    setFailure(null);
    try {
      const result = await exploreGit(machine, root, { ...query, offset: next }, "directory");
      setAppend(was => [...was, result]);
    } catch (error) {
      setFailure(said(error));
    } finally {
      busy.current = false;
      setLoading(false);
    }
  };
  return <div>
    {entries.length === 0 && <p className="px-page py-in text-callout text-muted-foreground">{gui.git_snapshot_empty}</p>}
    {entries.map(entry => {
      const full = path ? `${path}/${entry.name}` : entry.name;
      const directory = entry.kind === "directory";
      const open = directory && (expanded.get(full) ?? selected?.startsWith(`${full}/`) ?? false);
      const Chevron = open ? IconChevronDown : IconChevronRight;
      return <div key={entry.name}>
        <button type="button" data-git-snapshot-path={full} data-git-snapshot-kind={entry.kind}
          aria-expanded={directory ? open : undefined} aria-current={selected === full ? "true" : undefined}
          title={entry.name}
          style={{ paddingInlineStart: `calc(var(--tree-inset) + ${depth} * var(--tree-step))` }}
          className={cn("flex min-h-touch w-full items-center gap-in rounded-sm pe-page text-start text-callout hover:bg-fill",
            selected === full && "bg-fill-strong")}
          onClick={() => directory ? setExpanded(was => new Map(was).set(full, !open)) : onOpen(full)}>
          <span className="flex size-affordance flex-none items-center justify-center text-muted-foreground" aria-hidden>
            {directory ? <Chevron className="size-affordance" /> : null}
          </span>
          {directory ? <IconFiles className="size-affordance flex-none text-muted-foreground" /> : <FileKindIcon name={entry.name} className="size-affordance flex-none" />}
          <span className="truncate">{entry.name}</span>
        </button>
        {open && <SnapshotDirectory {...props} path={full} depth={depth + 1} />}
      </div>;
    })}
    {[first, ...append].some(value => value.incomplete) && <p className="px-page py-in text-caption text-muted-foreground">{gui.git_snapshot_omitted}</p>}
    {failure && <p role="alert" className="px-page py-in text-callout text-muted-foreground">{failure}</p>}
    {next !== null && <button type="button" disabled={loading} onClick={() => void more()}
      className="min-h-touch px-page text-callout text-foreground disabled:opacity-50">{failure ? gui.region_retry : gui.git_more}</button>}
  </div>;
}

export function WorkspaceSnapshotFile({ path, ...location }: SnapshotProps & { path: string | null }) {
  const { version } = location;
  const request = useGitResource(location, path !== null ? { kind: "file", commit: version.commit, path } : null, "file");
  if (path === null) return <RegionState state="empty" message={gui.git_snapshot_pick_file} />;
  const file = request.value;
  if (!file) return <RegionState state={request.error ? "error" : "loading-never"} message={request.error} onRetry={request.retry} />;
  if (file.entry === null) return <RegionState state="empty" message={gui.git_snapshot_file_missing} />;
  if (file.text === null) return <RegionState state="empty" message={file.too_large ? gui.git_snapshot_file_large : gui.git_snapshot_file_binary} />;
  return <div data-git-snapshot-file={path} data-git-snapshot-commit={version.commit} className="flex min-h-0 flex-1 flex-col">
    {file.entry !== "file" && <p className="px-page py-in text-caption text-muted-foreground">
      {file.entry === "symlink" ? gui.git_snapshot_symlink : gui.git_snapshot_submodule}
    </p>}
    <CodeEditor path={path} text={file.text} editable={false} className="min-h-0 flex-1" />
  </div>;
}
