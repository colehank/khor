import type { DirWatchScope } from "@/gen/bindings/DirWatchScope";
import type { DirWatchBatch } from "@/gen/bindings/DirWatchBatch";
import type { DirWatchOpened } from "@/gen/bindings/DirWatchOpened";

export type DirectoryWatchRequest = { machine: string; scopes: DirWatchScope[] };
export type DirectoryWatchState = {
  key: string | null;
  scopes: DirWatchScope[];
  status: "idle" | "pending" | "watching" | "failed";
  batch: DirWatchBatch | null;
  error: unknown;
};
type Lease = { opened: DirWatchOpened; key: string; closed: boolean };
type Transport = {
  begin: (machine: string, scopes: DirWatchScope[]) => Promise<DirWatchOpened>;
  wait: (machineId: string, id: string, cursor: string, waitMs: number) => Promise<DirWatchBatch>;
  leave: (machineId: string, id: string) => Promise<unknown>;
};
type Schedule = (run: () => void, delay: number) => () => void;

export const directoryWatchKey = ({ machine, scopes }: DirectoryWatchRequest): string => JSON.stringify([machine, scopes]);

/** One active view owns one lease; late begin replies must release their own id. */
export class DirectoryWatch {
  private request: DirectoryWatchRequest | null = null;
  private lease: Lease | null = null;
  private beginning = false;
  private closing = false;
  private cancelTimer: (() => void) | null = null;
  private failures = 0;
  private listeners = new Set<() => void>();
  private state: DirectoryWatchState = { key: null, scopes: [], status: "idle", batch: null, error: null };

  constructor(private readonly transport: Transport, private readonly waitMs: number, private readonly schedule: Schedule) {}

  readonly subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };
  readonly snapshot = () => this.state;

  set(request: DirectoryWatchRequest | null): void {
    const key = request === null ? null : directoryWatchKey(request);
    if (key === this.state.key) return;
    this.request = request;
    this.cancelTimer?.();
    this.cancelTimer = null;
    const previous = this.lease;
    this.lease = null;
    if (previous) this.release(previous);
    this.failures = 0;
    this.publish({ key, scopes: request?.scopes ?? [], status: key === null ? "idle" : "pending", batch: null, error: null });
    this.begin();
  }

  private publish(update: Partial<DirectoryWatchState>): void {
    this.state = { ...this.state, ...update };
    for (const listener of this.listeners) listener();
  }

  private later(run: () => void, delay: number): void {
    this.cancelTimer?.();
    this.cancelTimer = this.schedule(() => { this.cancelTimer = null; run(); }, delay);
  }

  private release(lease: Lease): void {
    if (lease.closed) return;
    lease.closed = true;
    this.closing = true;
    // Leave is independent of an outstanding held wait. An unreachable peer
    // still expires its lease; do not retain an unbounded queue of retries.
    void this.transport.leave(lease.opened.machine_id, lease.opened.id).catch(() => {}).finally(() => {
      this.closing = false;
      this.begin();
    });
  }

  private begin(): void {
    if (this.request === null || this.beginning || this.closing || this.lease || this.cancelTimer) return;
    const request = this.request;
    const key = directoryWatchKey(request);
    this.beginning = true;
    void this.transport.begin(request.machine, request.scopes).then(opened => {
      const lease: Lease = { opened, key, closed: false };
      if (this.state.key !== key || this.request !== request) {
        this.release(lease);
        return;
      }
      this.lease = lease;
      this.failures = 0;
      this.publish({ status: "watching", batch: opened.batch, error: null });
      this.next(lease, opened.batch);
    }, error => {
      if (this.state.key === key && this.request === request) this.failed(error);
    }).finally(() => {
      this.beginning = false;
      this.begin();
    });
  }

  private owns(lease: Lease): boolean {
    return this.lease === lease && !lease.closed && this.state.key === lease.key && this.request !== null;
  }

  private next(lease: Lease, batch: DirWatchBatch): void {
    if (!this.owns(lease)) return;
    const delay = this.waitMs === 0 ? Math.max(1000, batch.retry_ms) : batch.retry_ms;
    this.later(() => { void this.wait(lease, batch.cursor); }, delay);
  }

  private async wait(lease: Lease, cursor: string): Promise<void> {
    if (!this.owns(lease)) return;
    try {
      const batch = await this.transport.wait(lease.opened.machine_id, lease.opened.id, cursor, this.waitMs);
      if (!this.owns(lease)) return;
      this.failures = 0;
      this.publish({ status: "watching", batch, error: null });
      this.next(lease, batch);
    } catch (error) {
      if (!this.owns(lease)) return;
      this.lease = null;
      this.failed(error);
      this.release(lease);
    }
  }

  private failed(error: unknown): void {
    if (this.request === null) return;
    this.publish({ status: "failed", error });
    const delay = Math.min(30_000, 5000 * 2 ** Math.min(this.failures++, 3));
    this.later(() => this.begin(), delay);
  }
}

/** Entries beyond the backend budget remain explicit fallback levels. */
export function directoryWatchScopes(workspace: string | null, entries: string[]): { scopes: DirWatchScope[]; omitted: string[] } {
  const scopes: DirWatchScope[] = [];
  const omitted: string[] = [];
  let bytes = 0;
  const add = (scope: DirWatchScope) => {
    const path = scope.kind === "workspace" ? scope.root : scope.path;
    const size = new TextEncoder().encode(path).length;
    if (scopes.length >= 64 || bytes + size > 64 * 1024) { omitted.push(path); return; }
    scopes.push(scope);
    bytes += size;
  };
  if (workspace !== null) add({ kind: "workspace", root: workspace });
  for (const path of new Set(entries)) {
    if (path !== workspace) add({ kind: "entries", path });
  }
  return { scopes, omitted };
}

export function directoryScopeIsNative(state: DirectoryWatchState, scope: DirWatchScope): boolean {
  if (state.status !== "watching" || state.batch === null) return false;
  const key = JSON.stringify(scope);
  const index = state.scopes.findIndex(candidate => JSON.stringify(candidate) === key);
  return index >= 0 && state.batch.coverage.some(item => item.scope === index && item.mode === "native");
}

/** A listed link can lead outside the recursive checkout, or to another spelling within it. */
export function directoryAliases(root: string, listings: ReadonlyMap<string, { path: string }>, visible: string[]): string[] {
  const physical = listings.get(root)?.path;
  if (physical === undefined) return [];
  const prefix = physical === "/" ? "" : physical;
  return visible.filter(path => {
    const listed = listings.get(path);
    const relative = path === root ? "" : path.slice(root === "/" ? 0 : root.length);
    return listed !== undefined && listed.path !== `${prefix}${relative || (physical === "/" ? "/" : "")}`;
  });
}
