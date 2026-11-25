import { useEffect, useState } from "react";
import { exploreGit, said, type GitExploreResult } from "@/api";
import { IconBranch } from "@/components/icons";
import { PopupButton, type PopupChoice } from "@/components/ui/popup-button";
import { gui } from "@/gen/catalog";
import { versionChoices, versionName, type GitTool } from "./git-versions";

export function WorkspaceBranchPicker({ id, machine, root, branch, reference, busy, onPick, onTool }: {
  id: string;
  machine: string;
  root: string;
  branch: string | null;
  reference?: string;
  busy: boolean;
  onPick: (reference?: string) => void;
  onTool: (tool: GitTool) => void;
}) {
  const [open, setOpen] = useState(false);
  const [catalog, setCatalog] = useState<Extract<GitExploreResult, { kind: "references" }> | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    let active = true;
    void exploreGit(machine, root, { kind: "references" }, "references").then(
      result => { if (active) setCatalog(result); },
      failure => { if (active) setError(said(failure)); },
    );
    return () => { active = false; };
  }, [open, machine, root]);

  const headLabel = branch === null ? gui.git_branch_history : branch || gui.git_branch_head;
  const currentLabel = catalog === null || (!open && branch !== null)
    ? headLabel
    : catalog.current ? versionName(catalog.current) : gui.git_branch_head;
  const choices: PopupChoice[] = [
    { value: "", label: currentLabel, note: gui.git_branch_current },
    ...versionChoices((catalog?.refs ?? []).filter(ref => ref !== catalog?.current || ref === reference)),
  ];
  if (reference && !choices.some(choice => choice.value === reference)) {
    choices.push({ value: reference, label: versionName(reference), note: gui.git_snapshot });
  }
  const notice = error ?? (catalog === null ? gui.git_branch_loading : catalog.more ? gui.git_branch_limited : null);
  if (notice !== null) choices.push({ value: "status", label: notice, disabled: true });

  return <PopupButton
    label={gui.git_branch_history}
    leading={<span data-workspace-branch-icon aria-hidden><IconBranch /></span>}
    value={reference ?? ""}
    choices={choices}
    onPick={value => onPick(value || undefined)}
    mark={{ id, "data-workspace-branch-picker": true, "aria-busy": busy }}
    onOpenChange={next => { setError(null); setOpen(next); }}
    actions={[
      { label: gui.git_compare, run: () => onTool("compare") },
      { label: gui.git_merge_preview, run: () => onTool("merge") },
      { label: gui.git_worktrees, run: () => onTool("worktrees") },
      { label: gui.git_remotes, run: () => onTool("remotes") },
    ]}
  />;
}
