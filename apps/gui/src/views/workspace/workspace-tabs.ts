import type { GitCommit } from "@/gen/bindings/GitCommit";

export type WorkspaceTarget =
  | { kind: "file"; path: string }
  | { kind: "diff"; path: string }
  | { kind: "commit"; commit: GitCommit };

export type WorkspaceTabs = {
  items: WorkspaceTarget[];
  active: string | null;
  preview: string | null;
};

export function workspaceTabId(target: WorkspaceTarget): string {
  return JSON.stringify([target.kind, target.kind === "commit" ? target.commit.id : target.path]);
}

export function emptyWorkspaceTabs(): WorkspaceTabs {
  return { items: [], active: null, preview: null };
}

export function selectedWorkspaceTarget(tabs: WorkspaceTabs): WorkspaceTarget | null {
  return tabs.items.find((target) => workspaceTabId(target) === tabs.active) ?? null;
}

export function selectWorkspaceTab(tabs: WorkspaceTabs, id: string): WorkspaceTabs {
  return tabs.active === id || !tabs.items.some((target) => workspaceTabId(target) === id)
    ? tabs : { ...tabs, active: id };
}

export function pinWorkspaceTab(tabs: WorkspaceTabs, id: string): WorkspaceTabs {
  return tabs.preview === id ? { ...tabs, preview: null } : tabs;
}

export function openWorkspaceTab(tabs: WorkspaceTabs, target: WorkspaceTarget, pinned: boolean): WorkspaceTabs {
  const id = workspaceTabId(target);
  if (tabs.items.some((item) => workspaceTabId(item) === id)) {
    const selected = selectWorkspaceTab(tabs, id);
    return pinned ? pinWorkspaceTab(selected, id) : selected;
  }
  const replace = !pinned && tabs.preview !== null;
  const items = replace
    ? tabs.items.map((item) => workspaceTabId(item) === tabs.preview ? target : item)
    : [...tabs.items, target];
  return { items, active: id, preview: pinned ? tabs.preview : id };
}

export function closeWorkspaceTab(tabs: WorkspaceTabs, id: string): WorkspaceTabs {
  const at = tabs.items.findIndex((target) => workspaceTabId(target) === id);
  if (at < 0) return tabs;
  const items = tabs.items.filter((target) => workspaceTabId(target) !== id);
  const neighbour = items[at] ?? items[at - 1];
  return {
    items,
    active: tabs.active === id ? neighbour ? workspaceTabId(neighbour) : null : tabs.active,
    preview: tabs.preview === id ? null : tabs.preview,
  };
}

export function renameWorkspaceFiles(tabs: WorkspaceTabs, rename: (path: string) => string): WorkspaceTabs {
  const ids = new Map<string, string>();
  const items = tabs.items.map((target) => {
    const next = target.kind === "commit" ? target : { ...target, path: rename(target.path) };
    ids.set(workspaceTabId(target), workspaceTabId(next));
    return next;
  });
  if (![...ids].some(([before, after]) => before !== after)) return tabs;
  return {
    items,
    active: tabs.active === null ? null : ids.get(tabs.active) ?? null,
    preview: tabs.preview === null ? null : ids.get(tabs.preview) ?? null,
  };
}
