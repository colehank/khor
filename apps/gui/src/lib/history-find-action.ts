import type { historySource } from './history-source';
import { historyText, historyPartComplete, type FindHit } from './history-find-model';
import { paintHistory } from './history-paint';
import { recordText } from './history-find-record';

export type HistoryFindAction = 'context' | 'copy';
export type HistoryActionAnswer = { text: string; from: number; complete: boolean };
export class HistoryActionFailure extends Error {
  constructor(readonly code: 'too_large' | 'incomplete' | 'source_changed') { super(code); }
}
type IO = Pick<ReturnType<typeof historySource>, 'page' | 'record' | 'project'>;
const utf8 = new TextEncoder();

/** One projected body part, independent of mounted DOM and retained pages.
 * Copy never returns an incomplete prefix as if it were the complete part. */
export async function historyFindAction(action: HistoryFindAction, hit: FindHit, io: IO): Promise<HistoryActionAnswer> {
  if (![hit.from, hit.to, hit.part].every(Number.isSafeInteger) || hit.from < 0 || hit.to < hit.from || hit.part < 0)
    throw new Error('invalid history text selection');
  if (action === 'copy' && hit.complete === false) throw new HistoryActionFailure('incomplete');
  let stream: AsyncIterable<string>;
  let complete = true;
  let finish = async () => {};
  if (hit.record) {
    const parts = recordText(hit.record, hit.before, io);
    finish = async () => { await parts.return(false); };
    while (true) {
      const next = await parts.next();
      if (next.done) { await finish(); throw new HistoryActionFailure('incomplete'); }
      if (next.value.entry === hit.entry && next.value.part === hit.part) {
        stream = next.value.text; complete = next.value.complete; break;
      }
    }
  } else {
    let page = await io.page(hit.before, false);
    if (!page) throw new Error('history text read was cancelled');
    if (page.reset) throw new HistoryActionFailure('source_changed');
    if (page.entries.length > 128 || page.encoded_bytes > 512 * 1024 || utf8.encode(JSON.stringify(page)).length > 512 * 1024)
      throw new Error('history text page exceeds its byte budget');
    let items = paintHistory(page.entries, page.references);
    let index = items.findIndex(item => item.historyEntries.includes(hit.entry));
    if (index < 0) throw new HistoryActionFailure('incomplete');
    let body = historyText(items[index]);
    let text = body.texts[hit.part];
    if (text === undefined) throw new HistoryActionFailure('incomplete');
    // A seam result is located in the older page, but its final characters
    // are in the newer page. Re-read through the same source fence and painter;
    // no message-body/ordering algorithm is duplicated in the action.
    let pages = 1, pageBytes = utf8.encode(JSON.stringify(page)).length;
    while (hit.continued && text.length < hit.to) {
      const next = await io.page(page.newer, true);
      if (!next) throw new Error('history text read was cancelled');
      if (next.reset) throw new HistoryActionFailure('source_changed');
      if (next.entries.length > 128 || next.encoded_bytes > 512 * 1024 || utf8.encode(JSON.stringify(next)).length > 512 * 1024)
        throw new Error('history text page exceeds its byte budget');
      const nextBytes = utf8.encode(JSON.stringify(next)).length;
      if (++pages > 8 || page.entries.length + next.entries.length > 512
        || pageBytes + nextBytes > 2 * 1024 * 1024
        || page.encoded_bytes + next.encoded_bytes > 2 * 1024 * 1024 || next.newer === page.newer)
        throw new HistoryActionFailure('incomplete');
      pageBytes += nextBytes;
      page = {...next, older: page.older, entries: [...page.entries, ...next.entries],
        references: [...page.references, ...next.references], encoded_bytes: page.encoded_bytes + next.encoded_bytes};
      items = paintHistory(page.entries, page.references);
      index = items.findIndex(item => item.historyEntries.includes(hit.entry));
      if (index < 0) throw new HistoryActionFailure('incomplete');
      body = historyText(items[index]); text = body.texts[hit.part];
      if (text === undefined) throw new HistoryActionFailure('incomplete');
      if (!next.has_more && text.length < hit.to) throw new HistoryActionFailure('incomplete');
    }
    complete = !hit.continued && body.complete && historyPartComplete(items, index, page, hit.atTail === true);
    const selectedText = text;
    stream = (async function* () { yield* selectedText; })();
  }
  try {
    if (action === 'copy' && !complete) throw new HistoryActionFailure('incomplete');
    const start = action === 'context' ? Math.max(0, hit.from - 160) : 0;
    const end = action === 'context' ? hit.to + 160 : Number.MAX_SAFE_INTEGER;
    let offset = 0, from = 0, bytes = 0, text = '';
    for await (const char of stream) {
      const next = offset + char.length;
      if (next > start && offset < end) {
        if (!text) from = offset;
        bytes += utf8.encode(char).length;
        if (bytes > 64 * 1024) throw new HistoryActionFailure('too_large');
        text += char;
      }
      offset = next;
      if (offset >= end) break;
    }
    if (offset < hit.to) throw new HistoryActionFailure('source_changed');
    return { text, from, complete: action === 'copy' && complete };
  } finally { await finish(); }
}
