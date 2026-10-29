import type { HistoryPage } from '@/gen/bindings/HistoryPage';
import type { HistoryRecordChunk } from '@/gen/bindings/HistoryRecordChunk';
type Transport = {
  begin: () => Promise<string>;
  cancel: (key: string) => Promise<unknown>;
  page: (id: string, key: string, cursor: string | null, newer: boolean) => Promise<HistoryPage>;
  record: (id: string, key: string, cursor: string, from: number) => Promise<HistoryRecordChunk>;
};
/** One mounted history reader owns every lease it starts, including a begin
 * that answers after unmount. The transport handshake precedes all file IO. */
export function historySource(id: string, transport: Transport) {
  let closed = false;
  const active = new Set<string>();
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
  function execute<T>(work: (key: string) => Promise<T>, signal?: AbortSignal): Promise<T | null> {
    const run = async (): Promise<T | null> => {
      if (closed || signal?.aborted)
        return null;
      const key = await transport.begin();
      if (closed || signal?.aborted) {
        await cancel(key);
        return null;
      }
      active.add(key);
      const abort = () => { void cancel(key); };
      signal?.addEventListener('abort', abort, { once: true });
      try {
        const answer = await work(key);
        return closed || signal?.aborted ? null : answer;
      }
      catch (error) {
        // A failed transport may never have claimed the acknowledged lease.
        await cancel(key);
        if (closed || signal?.aborted)
          return null;
        throw error;
      }
      finally {
        signal?.removeEventListener('abort', abort);
        active.delete(key);
      }
    };
    const job = run();
    jobs.add(job);
    job.then(() => jobs.delete(job), () => jobs.delete(job));
    return job;
  }
  return {
    page: (cursor: string | null, newer = false, signal?: AbortSignal) => execute(key => transport.page(id, key, cursor, newer), signal),
    record: (cursor: string, from: number, signal?: AbortSignal) => execute(key => transport.record(id, key, cursor, from), signal),
    async dispose() {
      closed = true;
      for (const key of active)
        void cancel(key);
      await Promise.allSettled([...jobs]);
      await Promise.all([...cancelling.values()]);
      return { failures: cleanupCount, lastError: lastCleanupError };
    },
  };
}
