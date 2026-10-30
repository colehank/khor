import { beginHistoryRequest, cancelHistoryRequest, fetchHistoryPage, fetchHistoryRecord, fetchHistoryProjection } from '@/api';
import { historySource } from './history-source';

/** Each mounted reader/search owns an instance; all use the same transport
 * and cancellation protocol rather than sharing another view's leases. */
export function historyApi(id: string, binding: string) {
  return historySource(id, {
    begin: () => beginHistoryRequest(id, binding),
    cancel: key => cancelHistoryRequest(id, binding, key),
    page: (view, key, cursor, newer) => fetchHistoryPage(view, binding, key, cursor, newer),
    record: (view, key, cursor, from) => fetchHistoryRecord(view, binding, key, cursor, from),
    project: (view, key, cursor, start, end, scaffold) => fetchHistoryProjection(view, binding, key, cursor, start, end, scaffold),
  });
}
