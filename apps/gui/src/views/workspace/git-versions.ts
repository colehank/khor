import { useEffect, useState } from "react";
import { exploreGit, said, type GitExploreQuery, type GitExploreResult } from "@/api";
import { gui } from "@/gen/catalog";
import type { PopupChoice } from "@/components/ui/popup-button";

export type Version = { reference: string; commit: string };
export type GitTool = "compare" | "merge" | "worktrees" | "remotes";
export type GitLocation = { machine: string; root: string };

export function versionName(reference: string): string {
  return reference.startsWith("live:") ? reference.slice(5) : reference.replace(/^refs\/(heads|remotes|tags)\//, "");
}

export function versionChoices(refs: readonly string[]): PopupChoice[] {
  return refs.map(value => ({
    value,
    label: versionName(value),
    group: value.startsWith("refs/remotes/") ? gui.git_branch_remote
      : value.startsWith("refs/tags/") ? gui.git_tags : undefined,
    groupNote: value.startsWith("refs/remotes/") ? gui.git_branch_remote_note : undefined,
  }));
}

type Reply<K extends GitExploreResult["kind"]> = Extract<GitExploreResult, { kind: K }>;

export function useGitResource<K extends GitExploreResult["kind"]>(
  { machine, root }: GitLocation, query: GitExploreQuery | null, kind: K,
) {
  const key = JSON.stringify(query);
  const [tryCount, retry] = useState(0);
  const identity = JSON.stringify([machine, root, key, tryCount, kind]);
  const [answer, setAnswer] = useState<{ identity: string; value?: Reply<K>; error?: string }>();
  useEffect(() => {
    const selected: GitExploreQuery | null = JSON.parse(key);
    if (selected === null) return;
    let live = true;
    void exploreGit(machine, root, selected, kind).then(
      value => { if (live) setAnswer({ identity, value }); },
      error => { if (live) setAnswer({ identity, error: said(error) }); },
    );
    return () => { live = false; };
  }, [machine, root, identity, key, kind]);
  const current = answer?.identity === identity ? answer : undefined;
  return { value: current?.value, error: current?.error, retry: () => retry(at => at + 1) };
}
