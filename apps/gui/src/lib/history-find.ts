import { speaking } from '@/gen/catalog';
import type { HistoryFindAction, HistoryActionAnswer } from './history-find-action';
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
type Source = Pick<ReturnType<typeof historySource>, 'page' | 'record' | 'project' | 'dispose'>;
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
  let actionId = 0;
  let action: { id: number; resolve: (value: HistoryActionAnswer) => void; reject: (error: Error) => void } | null = null;
  const publish = () => { if (!closed) changed(state); };
  const post = (message: FindWorkerInput) => worker.postMessage(message);
  const fail = (error: unknown) => {
    if (closed) return;
    pending?.abort();
    action?.reject(new Error(String(error))); action = null;
    worker.terminate();
    state = { ...state, hits: [], complete: false, busy: false, error: String(error) };
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
    if (answer.kind === 'action_result' || answer.kind === 'action_error') {
      if (!action || action.id !== answer.id) return;
      const job = action; action = null;
      state = { ...state, busy: false }; publish();
      if (answer.kind === 'action_result') job.resolve(answer.answer);
      else {
        const error = new Error(answer.code); job.reject(error);
        if (answer.code === 'source_changed' || answer.code === 'read_failed') fail(answer.error ?? answer.code);
      }
      return;
    }
    if (answer.kind === 'read' || answer.kind === 'project' || answer.kind === 'read_page') {
      const controller = new AbortController();
      if (pending) { fail('history search requested concurrent IO'); return; }
      pending = controller;
      const work = answer.kind === 'read_page' ? source.page(answer.before, false, controller.signal) : answer.kind === 'read'
        ? source.record(answer.cursor, answer.from, controller.signal)
        : source.project(answer.cursor, answer.start, answer.end, answer.scaffold, controller.signal);
      void work.then(result => {
        if (closed || pending !== controller) return;
        pending = null;
        post({ kind: 'io', id: answer.id, result });
      }, error => {
        if (closed || pending !== controller) return;
        pending = null;
        post({ kind: 'io', id: answer.id, error: String(error) });
      });
      return;
    }
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
  post({ kind: 'query', query, language: speaking() });
  return {
    start: () => {
      if (started || closed || state.error || state.reset) return;
      started = true;
      return load();
    },
    next() {
      if (closed || state.busy || state.done || state.reset || state.error) return;
      state = { ...state, hits: [], busy: true }; publish();
      if (pageDone) void load();
      else post({ kind: 'next' });
    },
    inspect(hit: FindHit, kind: HistoryFindAction): Promise<HistoryActionAnswer> {
      if (closed || state.busy || state.error || state.reset || !state.hits.includes(hit))
        return Promise.reject(new Error('history search result is no longer active'));
      state = { ...state, busy: true }; publish();
      const id = ++actionId;
      const promise = new Promise<HistoryActionAnswer>((resolve, reject) => { action = { id, resolve, reject }; });
      post({ kind: 'action', id, action: kind, hit });
      return promise;
    },
    async dispose() {
      closed = true;
      action?.reject(new Error('history search action cancelled')); action = null;
      pending?.abort();
      worker.terminate();
      return source.dispose();
    },
  };
}
