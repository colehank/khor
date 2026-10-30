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
    const page = await io.page(hit.before, false);
    if (!page) throw new Error('history text read was cancelled');
    if (page.reset) throw new HistoryActionFailure('source_changed');
    if (page.entries.length > 128 || page.encoded_bytes > 512 * 1024 || utf8.encode(JSON.stringify(page)).length > 512 * 1024)
      throw new Error('history text page exceeds its byte budget');
    const items = paintHistory(page.entries, page.references);
    const index = items.findIndex(item => item.historyEntries.includes(hit.entry));
    if (index < 0) throw new HistoryActionFailure('incomplete');
    const body = historyText(items[index]);
    const text = body.texts[hit.part];
    if (text === undefined) throw new HistoryActionFailure('incomplete');
    complete = body.complete && historyPartComplete(items, index, page, hit.atTail === true);
    stream = (async function* () { yield* text; })();
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
