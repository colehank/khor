import type { HistoryPage } from '@/gen/bindings/HistoryPage';
import type { HistoryRecordChunk } from '@/gen/bindings/HistoryRecordChunk';
import type { HistoryProjection } from '@/gen/bindings/HistoryProjection';
type Transport = {
  begin: () => Promise<string>;
  cancel: (key: string) => Promise<unknown>;
  page: (id: string, key: string, cursor: string | null, newer: boolean) => Promise<HistoryPage>;
  record: (id: string, key: string, cursor: string, from: number) => Promise<HistoryRecordChunk>;
  project?: (id: string, key: string, cursor: string, start: number, end: number, scaffold: string | null) => Promise<HistoryProjection>;
};
/** One mounted history reader owns every lease it starts, including a begin
 * that answers after unmount. The transport handshake precedes all file IO. */
export function historySource(id: string, transport: Transport) {
  let closed = false;
  const active = new Map<string, () => Promise<void>>();
  const jobs = new Set<Promise<unknown>>();
  const cancelling = new Map<string, Promise<void>>();
  let cleanupCount = 0;
  let lastCleanupError: string | null = null;
  const cancel = (key: string): Promise<void> => {
    const existing = cancelling.get(key);
    if (existing)
      return existing;
    const job = Promise.resolve().then(() => transport.cancel(key)).then(() => { }, error => {
      cleanupCount += 1;
      lastCleanupError = String(error).slice(0, 512);
    }).finally(() => { cancelling.delete(key); });
    cancelling.set(key, job);
    return job;
  };
  function execute<T>(work: (key: string) => Promise<T>, signal?: AbortSignal, releaseOnSuccess = false): Promise<T | null> {
    const run = async (): Promise<T | null> => {
      if (closed || signal?.aborted)
        return null;
      const key = await transport.begin();
      if (closed || signal?.aborted) {
        await cancel(key);
        return null;
      }
      let releasing: Promise<void> | null = null;
      const release = () => releasing ??= cancel(key);
      active.set(key, release);
      const abort = () => { void release(); };
      signal?.addEventListener('abort', abort, { once: true });
      let answer: T | undefined;
      let failed = false;
      let failure: unknown;
      try {
        answer = await work(key);
      }
      catch (error) {
        // A failed transport may never have claimed the acknowledged lease.
        await release();
        failed = true;
        failure = error;
      }
      finally {
        // Pure scaffold projection claims the near-side worker only. Its
        // unused home admission must be released even after a successful reply.
        if (releaseOnSuccess) await release();
        signal?.removeEventListener('abort', abort);
        active.delete(key);
      }
      // Cleanup can itself await an ACK. Retirement during that await must
      // reject the answer just as retirement during IO does.
      if (closed || signal?.aborted) return null;
      if (failed) throw failure;
      return answer as T;
    };
    const job = run();
    jobs.add(job);
    job.then(() => jobs.delete(job), () => jobs.delete(job));
    return job;
  }
  return {
    page: (cursor: string | null, newer = false, signal?: AbortSignal) => execute(key => transport.page(id, key, cursor, newer), signal),
    record: (cursor: string, from: number, signal?: AbortSignal) => execute(key => transport.record(id, key, cursor, from), signal),
    project: (cursor: string, start: number, end: number, scaffold: string | null, signal?: AbortSignal) => execute(key => {
      if (!transport.project) throw new Error('history projection transport unavailable');
      return transport.project(id, key, cursor, start, end, scaffold);
    }, signal, true),
    async dispose() {
      closed = true;
      for (const release of active.values())
        void release();
      await Promise.allSettled([...jobs]);
      await Promise.all([...cancelling.values()]);
      return { failures: cleanupCount, lastError: lastCleanupError };
    },
  };
}
