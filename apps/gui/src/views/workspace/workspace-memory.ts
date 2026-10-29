import type { GitStatus } from "@/gen/bindings/GitStatus";
import { WorkspaceRefresh } from "./workspace-refresh";
import type { GitCommit } from "@/gen/bindings/GitCommit";
import type { GitCommitSummary } from "@/gen/bindings/GitCommitSummary";
import type { EditorHistorySlot } from "@/lib/code/history";
import type { Draft } from "@/views/FileView";
import type { CommitPageState } from "@/views/CommitPage";
import type { WorkspaceTabs } from "./workspace-tabs";
import { WorkspaceHistoryMemory } from "./workspace-history";

export type WorkspaceMetadata = {
  status: GitStatus | null;
  headKnown: boolean;
  head: string | null;
  commit: GitCommitSummary | null;
};

export type WorkspaceIdentity = { device: string; path: string };
export type WorkspaceFields = {
  at: string;
  content: WorkspaceTabs;
  recent: string[];
  historyOpen: boolean;
  historyQuery: string;
  branch: string;
  headCommit: GitCommitSummary | null;
  screen: "tree" | "file" | "sessions";
  face: "source" | "preview";
  rev: GitCommit | null;
  revCompare: boolean;
  drafts: Map<string, Draft>;
};
type Update<T> = T | ((current: T) => T);

export function workspaceIdentity(identity: WorkspaceIdentity): string {
  return JSON.stringify([identity.device, identity.path]);
}

/** Retained data has no DOM or timers; pending reads cannot continue after view detach. */
export class WorkspaceMemory {
  readonly repositoryHistory = new WorkspaceHistoryMemory();
  readonly metadata = new WorkspaceRefresh<WorkspaceMetadata>();
  readonly commits = new Map<string, CommitPageState>();
  readonly histories = new Map<string, EditorHistorySlot>();
  private values = new Map<keyof WorkspaceFields, unknown>();
  private listeners = new Set<() => void>();

  readonly subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };

  ensure<K extends keyof WorkspaceFields>(key: K, initialize: () => WorkspaceFields[K]): void {
    if (!this.values.has(key)) this.values.set(key, initialize());
  }

  read<K extends keyof WorkspaceFields>(key: K): WorkspaceFields[K] {
    if (!this.values.has(key)) throw new Error(`Workspace field is not initialized: ${key}`);
    return this.values.get(key) as WorkspaceFields[K];
  }

  update<K extends keyof WorkspaceFields>(key: K, update: Update<WorkspaceFields[K]>): void {
    const current = this.read(key);
    const next = typeof update === "function" ? update(current) : update;
    if (Object.is(current, next)) return;
    this.values.set(key, next);
    for (const listener of this.listeners) listener();
  }

  draftCount = (): number => {
    this.ensure("drafts", () => new Map());
    return this.read("drafts").size;
  };
}

export class WorkspaceMemories {
  private owners = new Map<string, WorkspaceMemory>();

  retain(identity: WorkspaceIdentity): WorkspaceMemory {
    const key = workspaceIdentity(identity);
    let memory = this.owners.get(key);
    if (!memory) {
      memory = new WorkspaceMemory();
      this.owners.set(key, memory);
    }
    return memory;
  }

  entries(): ReadonlyArray<readonly [string, WorkspaceMemory]> {
    return [...this.owners.entries()];
  }
}
