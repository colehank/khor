import type { HistoryPage } from '@/gen/bindings/HistoryPage';
import { historySource } from './history-source';
import { EMPTY_HISTORY_WINDOW, receiveHistoryPage, type HistoryDirection, type HistoryWindow } from './history-window';
export type HistoryReaderState = {
  window: HistoryWindow;
  loaded: boolean;
  busy: boolean;
  error: unknown;
  reset: boolean;
};
type Source = ReturnType<typeof historySource>;
type Clock = {
  schedule: (run: () => void, delay: number) => number;
  cancel: (timer: number) => void;
};
/** A reader has one page operation at a time. Following pauses while reading
 * away from the tail; explicit navigation never races an automatic page merge. */
export function historyReader(source: Source, changed: (state: HistoryReaderState) => void, canFollow: () => boolean, clock: Clock) {
  let state: HistoryReaderState = { window: EMPTY_HISTORY_WINDOW, loaded: false, busy: false, error: null, reset: false };
  let closed = false;
  let pending: AbortController | null = null;
  let timer: number | null = null;
  let failures = 0;
  let catchingUp = false;
  const publish = () => {
    if (!closed) changed(state);
  };
  const schedule = () => {
    if (timer !== null)
      clock.cancel(timer);
    if (!closed)
      timer = clock.schedule(() => {
        timer = null;
        if (!pending && canFollow() && (!state.loaded || state.window.latest || catchingUp))
          void load(state.loaded ? 'newer' : 'tail');
        else
          schedule();
      }, Math.min(5000, 700 * 2 ** Math.min(failures, 3)));
  };
  async function load(direction: HistoryDirection) {
    if (closed)
      return;
    if (pending && direction !== 'tail')
      return;
    const cursor = direction === 'tail' ? null : direction === 'older' ? state.window.older : state.window.newer;
    if (direction !== 'tail' && cursor === null)
      return;
    if (direction !== 'newer') catchingUp = false;
    pending?.abort();
    const controller = new AbortController();
    pending = controller;
    state = { ...state, busy: true, error: null };
    publish();
    try {
      const page: HistoryPage | null = await source.page(cursor, direction === 'newer', controller.signal);
      if (closed || pending !== controller || !page)
        return;
      failures = 0;
      catchingUp = direction === 'newer' && page.has_more;
      state = { ...state, window: receiveHistoryPage(state.window, page, direction), loaded: true,
        reset: page.reset, error: null };
    }
    catch (error) {
      if (closed || pending !== controller || controller.signal.aborted)
        return;
      failures += 1;
      state = { ...state, error };
    }
    finally {
      if (!closed && pending === controller) {
        pending = null;
        state = { ...state, busy: false };
        publish();
        schedule();
      }
    }
  }
  return {
    start: () => {
      if (canFollow()) return load('tail');
      schedule();
    },
    older: () => load('older'),
    newer: () => load('newer'),
    latest: () => load('tail'),
    record: source.record,
    pause() {
      pending?.abort();
      pending = null;
      state = { ...state, busy: false };
      publish();
      schedule();
    },
    resume() {
      if (!pending && canFollow() && (!state.loaded || state.window.latest || catchingUp))
        void load(state.loaded ? 'newer' : 'tail');
    },
    async dispose() {
      closed = true;
      pending?.abort();
      if (timer !== null)
        clock.cancel(timer);
      return source.dispose();
    },
  };
}
