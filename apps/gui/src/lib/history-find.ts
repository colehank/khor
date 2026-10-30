import type { historySource } from './history-source';
import type { FindHit, FindQuery, FindWorkerInput, FindWorkerOutput } from './history-find-model';

export type HistoryFindState = {
  busy: boolean;
  done: boolean;
  complete: boolean;
  reset: boolean;
  hits: FindHit[];
  scannedPages: number;
  unsearchedRanges: number;
  error: string | null;
};
type Source = Pick<ReturnType<typeof historySource>, 'page' | 'dispose'>;
/** The UI only forwards bounded RPC replies. Projection and matching run
 * in a dedicated worker; this source owns separate cancellable read leases. */
export function historyFind(source: Source, query: FindQuery, changed: (state: HistoryFindState) => void) {
  const worker = new Worker(new URL('./history-find.worker.ts', import.meta.url), { type: 'module' });
  let state: HistoryFindState = { busy: false, done: false, complete: false, reset: false,
    hits: [], scannedPages: 0, unsearchedRanges: 0, error: null };
  let closed = false;
  let started = false;
  let before: string | null = null;
  let pageDone = true;
  let requestedBefore: string | null = null;
  let pending: AbortController | null = null;
  const publish = () => { if (!closed) changed(state); };
  const post = (message: FindWorkerInput) => worker.postMessage(message);
  const fail = (error: unknown) => {
    if (closed) return;
    pending?.abort();
    worker.terminate();
    state = { ...state, busy: false, error: String(error) };
    publish();
    void source.dispose();
  };
  const load = async () => {
    if (closed) return;
    state = { ...state, busy: true };
    publish();
    const controller = new AbortController();
    pending = controller;
    requestedBefore = before;
    try {
      const page = await source.page(before, false, controller.signal);
      if (closed || pending !== controller || !page) return;
      pending = null;
      if (page.reset) {
        worker.terminate();
        state = { ...state, busy: false, hits: [], reset: true };
        publish();
        void source.dispose();
        return;
      }
      state = { ...state, scannedPages: state.scannedPages + 1 };
      post({ kind: 'page', page });
    } catch (error) { fail(error); }
  };
  worker.onmessage = (event: MessageEvent<FindWorkerOutput>) => {
    if (closed) return;
    const answer = event.data;
    if (answer.kind === 'error') { fail(answer.error); return; }
    if (answer.pageDone && answer.older !== null && answer.older === requestedBefore) {
      fail('history search source cursor did not advance');
      return;
    }
    pageDone = answer.pageDone;
    before = answer.older;
    state = { ...state, hits: answer.hits, busy: false,
      done: pageDone && before === null,
      complete: pageDone && before === null && state.unsearchedRanges + answer.unsearchedRanges === 0,
      unsearchedRanges: state.unsearchedRanges + answer.unsearchedRanges };
    publish();
    if (!state.hits.length && !state.done) {
      if (pageDone) void load();
      else { state = { ...state, busy: true }; post({ kind: 'next' }); }
    }
  };
  worker.onerror = event => fail(event.message);
  post({ kind: 'query', query });
  return {
    start: () => {
      if (started || closed) return;
      started = true;
      return load();
    },
    next() {
      if (closed || state.busy || state.done || state.reset || state.error) return;
      state = { ...state, hits: [], busy: true }; publish();
      if (pageDone) void load();
      else post({ kind: 'next' });
    },
    async dispose() {
      closed = true;
      pending?.abort();
      worker.terminate();
      return source.dispose();
    },
  };
}
