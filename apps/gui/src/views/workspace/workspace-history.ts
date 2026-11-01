import type { GitHistoryPage } from "@/gen/bindings/GitHistoryPage";
import type { KeptScrollSlot } from "@/hooks/use-kept-scroll";

const PAGE_LIMIT = 40;
const CACHE_PAGES = 8;
const PAGE_BYTES = 256 * 1024;

type Page = { index: number; data: GitHistoryPage; scroll: KeptScrollSlot };
type Action = { kind: "refresh" } | { kind: "page"; index: number };
type Request = (cursor: string | null, limit: number) => Promise<GitHistoryPage>;
type Operation = { lease: symbol; stale: boolean };
export type WorkspaceHistorySnapshot = {
  current: Page | null;
  generation: number;
  busy: boolean;
  error: string | null;
  refreshRequested: boolean;
};

/** Only bounded page data survives the view. Requests belong to its active lease. */
export class WorkspaceHistoryMemory {
  private pages = new Map<number, Page>();
  private listeners = new Set<() => void>();
  private lease: symbol | null = null;
  private operation: Operation | null = null;
  private retryAction: Action | null = null;
  private view: WorkspaceHistorySnapshot = {
    current: null, generation: 0, busy: false, error: null, refreshRequested: true,
  };

  readonly subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };
  readonly snapshot = () => this.view;

  attach(): () => void {
    const lease = Symbol("workspace-history-view");
    if (this.operation) this.operation.stale = true;
    this.lease = lease;
    if (this.view.current === null) this.publish({ refreshRequested: true });
    return () => {
      if (this.lease !== lease) return;
      this.lease = null;
      if (this.operation) this.operation.stale = true;
      if (this.view.current === null) this.publish({ refreshRequested: true });
    };
  }

  invalidate(): void {
    if (this.operation) this.operation.stale = true;
    this.retryAction = null;
    this.publish({ refreshRequested: true, error: null });
  }

  refresh(request: Request): Promise<void> {
    return this.run({ kind: "refresh" }, request);
  }

  go(index: number, request: Request): Promise<void> {
    if (!Number.isSafeInteger(index) || index < 0) return Promise.resolve();
    if (index > (this.view.current?.index ?? 0) && this.view.current?.data.next === null) return Promise.resolve();
    return this.run({ kind: "page", index }, request);
  }

  retry(request: Request): Promise<void> {
    return this.retryAction ? this.run(this.retryAction, request) : Promise.resolve();
  }

  private publish(update: Partial<WorkspaceHistorySnapshot>): void {
    this.view = { ...this.view, ...update };
    for (const listener of this.listeners) listener();
  }

  private owns(operation: Operation): boolean {
    return this.operation === operation && !operation.stale && this.lease === operation.lease;
  }

  private remember(page: Page): void {
    this.pages.delete(page.index);
    this.pages.set(page.index, page);
    // Page zero is the fixed snapshot's restart anchor. Protect the visible
    // page while a failed navigation might still need to leave it on screen.
    for (const [index] of this.pages) {
      if (this.pages.size <= CACHE_PAGES) break;
      if (index !== 0 && index !== this.view.current?.index) this.pages.delete(index);
    }
  }

  private check(data: GitHistoryPage, head?: string | null, cursor?: string | null): void {
    if (head !== undefined && data.head !== head) throw new Error("Repository history snapshot changed");
    if (data.commits.length > PAGE_LIMIT || new TextEncoder().encode(JSON.stringify(data)).length > PAGE_BYTES) {
      throw new Error("Repository history page exceeds its response budget");
    }
    if (new Set(data.commits.map(commit => commit.id)).size !== data.commits.length) {
      throw new Error("Repository history page repeats a commit");
    }
    if (data.next !== null && (data.next === cursor || data.commits.length === 0)) {
      throw new Error("Repository history cursor did not advance");
    }
    if (data.head === null && (data.commits.length !== 0 || data.next !== null)) {
      throw new Error("Repository history has commits without a snapshot HEAD");
    }
  }

  private async run(action: Action, request: Request): Promise<void> {
    if (this.lease === null || this.operation !== null || action.kind === "page" && this.view.refreshRequested) return;
    const operation: Operation = { lease: this.lease, stale: false };
    this.operation = operation;
    this.publish({ busy: true, error: null, refreshRequested: action.kind === "refresh" ? false : this.view.refreshRequested });
    try {
      let target: Page;
      if (action.kind === "refresh") {
        const data = await request(null, PAGE_LIMIT);
        if (!this.owns(operation)) return;
        this.check(data);
        target = { index: 0, data, scroll: {} };
        this.pages.clear();
      } else {
        const cached = this.pages.get(action.index);
        if (cached) target = cached;
        else {
          let previous = [...this.pages.values()]
            .filter(page => page.index < action.index)
            .sort((a, b) => b.index - a.index)[0];
          if (!previous) throw new Error("Repository history has no snapshot anchor");
          while (previous.index < action.index) {
            const cursor = previous.data.next;
            if (cursor === null) throw new Error("Repository history has no earlier page");
            const data = await request(cursor, PAGE_LIMIT);
            if (!this.owns(operation)) return;
            this.check(data, previous.data.head, cursor);
            previous = { index: previous.index + 1, data, scroll: {} };
            this.remember(previous);
          }
          target = previous;
        }
      }
      if (!this.owns(operation)) return;
      this.remember(target);
      this.retryAction = null;
      this.publish({ current: target, generation: this.view.generation + (action.kind === "refresh" ? 1 : 0) });
    } catch (error) {
      if (!this.owns(operation)) return;
      this.retryAction = action;
      this.publish({ error: error instanceof Error ? error.message : String(error) });
    } finally {
      if (this.operation === operation) {
        this.operation = null;
        this.publish({ busy: false });
      }
    }
  }
}
