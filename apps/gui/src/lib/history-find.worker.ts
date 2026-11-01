import type { HistoryPage } from '@/gen/bindings/HistoryPage';
import type { HistoryRecordRef } from '@/gen/bindings/HistoryRecordRef';
import type { historySource } from './history-source';
import { historyFindBatch, type FindHit, type FindQuery, type FindWorkerInput, type FindWorkerOutput } from './history-find-model';
import { findRecord } from './history-find-record';
import { historyFindAction, HistoryActionFailure } from './history-find-action';
import { speak } from '@/gen/catalog';
import { replayHistoryQuery, historyPageContext } from './history-find-stream';

let batch: ReturnType<typeof historyFindBatch> | null = null;
let query: FindQuery;
let page: HistoryPage | null = null;
let normalDone = true;
let references: HistoryRecordRef[] = [];
let fragment: HistoryRecordRef | null = null;
let raw: ReturnType<typeof findRecord> | null = null;
let textReplay: ReturnType<typeof replayHistoryQuery> | null = null;
let heldHit: FindHit | null = null;
let running = false;
let serial = 0;
let waiting: { id: number; resolve: (value: unknown) => void; reject: (error: Error) => void } | null = null;
const utf8 = new TextEncoder();
const send = (value: FindWorkerOutput) => self.postMessage(value);
const request = (ask: Omit<Extract<FindWorkerOutput, { kind: 'read' }>, 'id'>
  | Omit<Extract<FindWorkerOutput, { kind: 'project' }>, 'id'>
  | Omit<Extract<FindWorkerOutput, { kind: 'read_page' }>, 'id'>) => new Promise<unknown>((resolve, reject) => {
  if (waiting) throw new Error('history search already has pending IO');
  const id = ++serial;
  waiting = { id, resolve, reject };
  send({ ...ask, id });
});
type IO = Pick<ReturnType<typeof historySource>, 'record' | 'project' | 'page'>;
const io: IO = {
  page: async (before, newer = false) => await request({ kind: 'read_page', before: before!, newer }) as Awaited<ReturnType<IO['page']>>,
  record: async (cursor, from) => await request({ kind: 'read', cursor, from }) as Awaited<ReturnType<IO['record']>>,
  project: async (cursor, start, end, scaffold) => await request({ kind: 'project', cursor, start, end, scaffold }) as Awaited<ReturnType<IO['project']>>,
};

async function next() {
  if (!batch || !page) throw new Error('history search has no pending page');
  if (!normalDone) {
    const answer = batch.next();
    normalDone = answer.pageDone;
    send({ ...answer, unsearchedRanges: answer.unsearchedRanges - (answer.pageDone ? page.references.length : 0),
      pageDone: normalDone && !references.length && !raw && !textReplay && !batch.hasReplay() && !(fragment && page.older === null) });
    return;
  }
  const hits: FindHit[] = [];
  let bytes = 0;
  let unsearchedRanges = 0;
  while (hits.length < 32) {
    if (!textReplay && batch.hasReplay()) textReplay = replayHistoryQuery(batch.takeReplay()!,query,io);
    if (textReplay) {
      const result = heldHit ? {done:false as const,value:heldHit} : await textReplay.next();
      if (result.done) {unsearchedRanges+=result.value.unsearched;textReplay=null;continue;}
      const size=utf8.encode(JSON.stringify(result.value)).length+1;
      if (bytes+size>48*1024) {heldHit=result.value;break;}
      heldHit=null;bytes+=size;hits.push(result.value);continue;
    }
    if (raw) {
      const result = heldHit ? { done: false as const, value: heldHit } : await raw.next();
      if (result.done) {
        if (!result.value) unsearchedRanges++;
        raw = null;
        continue;
      }
      const size = utf8.encode(JSON.stringify(result.value)).length + 1;
      if (bytes + size > 48 * 1024) { heldHit = result.value; break; }
      heldHit = null;
      bytes += size;
      hits.push(result.value);
      continue;
    }
    const ref = references.shift();
    if (!ref) break;
    if (fragment) {
      if (ref.end === fragment.start && ref.missing_end) {
        fragment = { ...fragment, start: ref.start, missing_start: ref.missing_start };
        if (fragment.missing_start) continue;
        if (fragment.missing_end) { unsearchedRanges++; fragment = null; continue; }
        raw = findRecord(query, fragment, page.newer, io); fragment = null;
        continue;
      }
      unsearchedRanges++; fragment = null;
    }
    if (ref.missing_start) { fragment = ref; continue; }
    if (ref.missing_end) { unsearchedRanges++; continue; }
    raw = findRecord(query, ref, page.newer, io);
  }
  const pageDone = !references.length && !raw && !heldHit && !textReplay && !batch.hasReplay();
  if (pageDone && page.older === null && fragment) { fragment = null; unsearchedRanges++; }
  const answer: Extract<FindWorkerOutput, { kind: 'result' }> = {
    kind: 'result', hits, older: page.older, pageDone, unsearchedRanges,
  };
  if (utf8.encode(JSON.stringify(answer)).length > 64 * 1024) throw new Error('history search result exceeds its byte budget');
  send(answer);
}

self.onmessage = async (event: MessageEvent<FindWorkerInput>) => {
  const message = event.data;
  if (message.kind === 'io') {
    if (!waiting || waiting.id !== message.id) return;
    const completion = waiting; waiting = null;
    if (message.error) completion.reject(new Error(message.error)); else completion.resolve(message.result);
    return;
  }
  try {
    if (running) throw new Error('history search already has pending work');
    running = true;
    if (message.kind === 'query') {
      speak(message.language);
      query = message.query;
      batch = historyFindBatch(query);
      return;
    }
    if (!batch) throw new Error('history search query is missing');
    if (message.kind === 'action') {
      try {
        const answer = await historyFindAction(message.action, message.hit, io);
        if (utf8.encode(JSON.stringify(answer)).length > 512 * 1024) throw new HistoryActionFailure('too_large');
        send({ kind: 'action_result', id: message.id, answer });
      } catch (error) {
        send({ kind: 'action_error', id: message.id, code: error instanceof HistoryActionFailure ? error.code : 'read_failed', error: String(error) });
      }
      return;
    }
    if (message.kind === 'page') {
      if (raw || references.length || heldHit || textReplay || batch.hasReplay() || !normalDone) throw new Error('previous search page has not been consumed');
      page = message.page;
      if (page.references.length > 256 || utf8.encode(JSON.stringify(page)).length > 512 * 1024)
        throw new Error('history search page exceeds the reader budget');
      // Normal text still needs the raw-reference positions as barriers.
      // This worker accounts for raw projection coverage separately below.
      const answer = batch.page(page,await historyPageContext(page,io));
      normalDone = answer.pageDone;
      references = [...page.references].sort((a, b) => b.start - a.start || (a.item ?? -1) - (b.item ?? -1));
      send({ ...answer, unsearchedRanges: answer.unsearchedRanges - (answer.pageDone ? page.references.length : 0),
        pageDone: normalDone && !references.length && !textReplay && !batch.hasReplay() && !(fragment && page.older === null) });
      return;
    }
    await next();
  } catch (failure) {
    send({ kind: 'error', error: String(failure) });
  } finally { running = false; }
};
