import type { WorkspaceFields } from "./workspace-memory";
import { emptyWorkspaceTabs, workspaceTabId, type WorkspaceTabs, type WorkspaceTarget } from "./workspace-tabs";

export type WorkspaceNavigation = Pick<WorkspaceFields, "at" | "content" | "historyOpen">;
type StoragePort = Pick<Storage, "getItem" | "setItem">;
const LIMIT = 512 * 1024;

export class WorkspaceNavigationStore {
  private readonly key: string;
  private unreadable = false;

  constructor(private storage: () => StoragePort | null, machine: string, private root: string, private legacy: (key: string) => WorkspaceTarget) {
    this.key = `khor.ws-tabs.${machine}:${root}`;
  }

  restore(): WorkspaceNavigation {
    const empty = { at: this.root, historyOpen: false, content: emptyWorkspaceTabs() };
    try {
      const raw = this.storage()?.getItem(this.key);
      if (!raw) return empty;
      if (raw.length > LIMIT) throw new Error("workspace navigation exceeds its limit");
      const value: unknown = JSON.parse(raw);
      if (Array.isArray(value)) {
        const items = value.filter((key): key is string => typeof key === "string").map(this.legacy).filter(target => this.validTarget(target));
        return { ...empty, content: this.tabs(items, null, null) };
      }
      if (!record(value) || value.schema !== 1 || !record(value.content) || !Array.isArray(value.content.items)) throw new Error("unknown workspace navigation record");
      const items = value.content.items.filter((target): target is WorkspaceTarget => this.validTarget(target));
      return {
        at: typeof value.at === "string" && (value.at === this.root || this.inside(value.at)) ? value.at : this.root,
        historyOpen: value.historyOpen === true,
        content: this.tabs(items, value.content.active, value.content.preview),
      };
    } catch { this.unreadable = true; return empty; }
  }

  save(navigation: WorkspaceNavigation): boolean {
    if (this.unreadable) return false;
    try {
      const storage = this.storage(); if (!storage) return false;
      const raw = JSON.stringify({ schema: 1, ...navigation });
      if (raw.length > LIMIT) return false;
      storage.setItem(this.key, raw);
      return true;
    } catch { return false; }
  }

  private inside(path: string): boolean {
    const prefix = this.root.endsWith("/") ? this.root : this.root + "/";
    return path.startsWith(prefix) && !path.includes("\0") && !path.split("/").some(part => part === "." || part === "..");
  }

  private validTarget(value: unknown): value is WorkspaceTarget {
    if (!record(value)) return false;
    if (value.kind === "file" || value.kind === "diff") return typeof value.path === "string" && this.inside(value.path);
    if (value.kind !== "commit" || !record(value.commit)) return false;
    const commit = value.commit;
    return typeof commit.id === "string" && /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(commit.id)
      && typeof commit.subject === "string" && typeof commit.author === "string"
      && typeof commit.when_ms === "number" && Number.isFinite(commit.when_ms) && typeof commit.cut === "boolean";
  }

  private tabs(items: WorkspaceTarget[], active: unknown, preview: unknown): WorkspaceTabs {
    const unique = new Map(items.map(target => [workspaceTabId(target), target]));
    return {
      items: [...unique.values()],
      active: typeof active === "string" && unique.has(active) ? active : unique.keys().next().value ?? null,
      preview: typeof preview === "string" && unique.has(preview) ? preview : null,
    };
  }
}

function record(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null && !Array.isArray(value); }
