import { useEffect, useState } from "react";
import { fetchGitBranches, said, type GitBranches } from "@/api";
import { PopupButton, type PopupChoice } from "@/components/ui/popup-button";
import { gui } from "@/gen/catalog";

function branchName(reference: string): string {
  return reference.replace(/^refs\/(heads|remotes)\//, "");
}

export function WorkspaceBranchPicker({ machine, root, branch, reference, onPick }: {
  machine: string;
  root: string;
  branch: string | null;
  reference?: string;
  onPick: (reference?: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [branches, setBranches] = useState<GitBranches | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    let active = true;
    void fetchGitBranches(machine, root).then(
      result => { if (active) setBranches(result); },
      failure => { if (active) setError(said(failure)); },
    );
    return () => { active = false; };
  }, [open, machine, root]);

  const headLabel = branch === null ? gui.git_branch_history : branch || gui.git_branch_head;
  const currentLabel = branches === null || (!open && branch !== null)
    ? headLabel
    : branches.current ? branchName(branches.current) : gui.git_branch_head;
  const choices: PopupChoice[] = [{ value: "", label: currentLabel, note: gui.git_branch_current }];
  for (const ref of branches?.refs ?? []) {
    choices.push({
      value: ref,
      label: branchName(ref),
      group: ref.startsWith("refs/remotes/") ? gui.git_branch_remote : undefined,
    });
  }
  const notice = error ?? (branches === null ? gui.git_branch_loading : branches.more ? gui.git_branch_limited : null);
  if (notice !== null) choices.push({ value: "status", label: notice, disabled: true });

  return <PopupButton
    label={gui.git_branch_history}
    value={reference === branches?.current ? "" : reference ?? ""}
    placeholder={reference ? branchName(reference) : headLabel}
    choices={choices}
    onPick={value => onPick(value || undefined)}
    mark={{ "data-workspace-branch-picker": true }}
    onOpenChange={next => { setError(null); setOpen(next); }}
  />;
}
