import type { HistoryRecordRef } from '@/gen/bindings/HistoryRecordRef';
import type { historySource } from './history-source';
import type { FileHandoff } from '@/gen/bindings/FileHandoff';
type Writer = Pick<FileSystemWritableFileStream, 'write' | 'close' | 'abort'>;

/** Locate physical JSONL boundaries from the same reader's window facts.
 * Keep one original cursor and scalar bounds, never whole records or a list
 * of fragment cursors. This does not claim the original is valid JSON. */
export async function completeHistoryRecord(source: Pick<ReturnType<typeof historySource>, 'page' | 'project'>, reference: HistoryRecordRef, signal: AbortSignal): Promise<HistoryRecordRef> {
  const utf8 = new TextEncoder();
  const valid = (ref: HistoryRecordRef) => Number.isSafeInteger(ref.start) && Number.isSafeInteger(ref.end)
    && ref.start >= 0 && ref.end > ref.start && typeof ref.missing_start === 'boolean' && typeof ref.missing_end === 'boolean'
    && typeof ref.cursor === 'string' && utf8.encode(ref.cursor).length <= 4096;
  if (!valid(reference)) throw new Error('invalid history export range');
  const check = () => { if (signal.aborted) throw new Error('history export cancelled'); };
  const page = async (cursor: string, newer: boolean) => {
    check();
    const value = await source.page(cursor, newer, signal);
    check();
    if (!value) throw new Error('history export cancelled');
    if (value.reset) throw new Error('history source changed');
    if (value.entries.length > 128 || value.references.length > 256 || value.encoded_bytes > 512 * 1024
      || utf8.encode(JSON.stringify(value)).length > 512 * 1024 || value.references.some(ref => !valid(ref)))
      throw new Error('invalid history export window');
    // Even immediate local replies yield the task queue for leave/cancel.
    await new Promise<void>(resolve => setTimeout(resolve, 0)); check();
    return value;
  };
  let start = reference.start, end = reference.end;
  let missingStart = reference.missing_start, missingEnd = reference.missing_end;
  let cursor = reference.cursor;
  while (missingStart) {
    const value = await page(cursor, false);
    const piece = value.references.find(ref => ref.end === start && ref.missing_end);
    if (!piece || piece.start >= start) throw new Error('incomplete history record boundary');
    start = piece.start; missingStart = piece.missing_start;
    cursor = piece.cursor;
  }
  cursor = reference.cursor;
  while (missingEnd) {
    const value = await page(cursor, true);
    const piece = value.references.find(ref => ref.start <= end && ref.end >= end && (ref.start < end || ref.missing_start));
    if (!piece) throw new Error('incomplete history record boundary');
    if (piece) { end = piece.end; missingEnd = piece.missing_end; }
    if (missingEnd && (!value.has_more || value.newer === cursor)) throw new Error('incomplete history record boundary');
    cursor = value.newer;
  }
  check();
  if (start === reference.start && end === reference.end && !reference.missing_start && !reference.missing_end) return reference;
  const joined = await source.project(reference.cursor, start, end, null, signal);
  check();
  if (!joined || joined.record.start !== start || joined.record.end !== end || !valid(joined.record))
    throw new Error('invalid joined history export range');
  // These false flags come from the observed window boundaries, not from
  // the pure widening response or a successful JSON parse.
  return { ...joined.record, missing_start: false, missing_end: false, item: reference.item };
}

/** Export one selected original record, byte for byte. The target commits
 * only on complete success; cancellation or read/write failure aborts it. */
export async function exportHistoryRecord(source: Pick<ReturnType<typeof historySource>, 'record'>, reference: HistoryRecordRef, target: Writer, signal: AbortSignal) {
  let committed = false;
  try {
    if (!Number.isSafeInteger(reference.start) || !Number.isSafeInteger(reference.end) || reference.start < 0 || reference.end <= reference.start)
      throw new Error('invalid history export range');
    for (let from = reference.start; from < reference.end;) {
      if (signal.aborted) throw new Error('history export cancelled');
      const chunk = await source.record(reference.cursor, from, signal);
      if (!chunk || signal.aborted) throw new Error('history export cancelled');
      if (chunk.offset !== from || chunk.data.length > 4 * Math.ceil(65536 / 3)) throw new Error('invalid history export chunk');
      const raw = atob(chunk.data);
      const bytes = Uint8Array.from(raw, char => char.charCodeAt(0));
      const end = from + bytes.length;
      if (!bytes.length || bytes.length > 65536 || end > reference.end || chunk.next !== (end < reference.end ? end : null))
        throw new Error('invalid history export continuation');
      await target.write(bytes);
      from = end;
    }
    if (signal.aborted) throw new Error('history export cancelled');
    await target.close();
    committed = true;
  } finally {
    if (!committed) await target.abort().catch(error => console.warn('History export abort failed', String(error).slice(0, 512)));
  }
}

type Handoff = {
  start: () => Promise<FileHandoff>;
  state: () => Promise<FileHandoff>;
  cancel: () => Promise<unknown>;
  save: () => Promise<boolean>;
};
/** Native staging uses the existing file job. An ACK received after leaving
 * still belongs to this operation and is cancelled before polling/presenting. */
export async function exportHistoryToApp(io: Handoff, reference: HistoryRecordRef, signal: AbortSignal): Promise<boolean> {
  let accepted = false;
  let attempted = false;
  let cancelled: Promise<unknown> | null = null;
  const cancel = () => {
    if (attempted && !cancelled) cancelled = io.cancel().catch(error => console.warn('History export cleanup failed', String(error).slice(0, 512)));
    return cancelled;
  };
  const abort = () => { if (accepted) void cancel(); };
  const check = () => { if (signal.aborted) throw new Error('history export cancelled'); };
  signal.addEventListener('abort', abort, { once: true });
  let finished = false;
  try {
    check();
    attempted = true;
    let state = await io.start();
    accepted = true;
    check();
    while (true) {
      check();
      if (state.error) throw new Error(state.error);
      if (state.cancelled) throw new Error('history export cancelled');
      const total = reference.end - reference.start;
      if (![state.total, state.moved].every(Number.isSafeInteger) || state.moved < 0 || state.moved > state.total || (state.total !== 0 && state.total !== total))
        throw new Error('invalid history export progress');
      if (state.ready) {
        if (state.total !== total || state.moved !== total) throw new Error('incomplete history export');
        const saved = await io.save();
        check(); finished = true;
        return saved;
      }
      await new Promise<void>((resolve, reject) => {
        const timer = setTimeout(() => { signal.removeEventListener('abort', stop); resolve(); }, 150);
        const stop = () => { clearTimeout(timer); reject(new Error('history export cancelled')); };
        signal.addEventListener('abort', stop, { once: true });
        if (signal.aborted) { signal.removeEventListener('abort', stop); stop(); }
      });
      state = await io.state();
    }
  } finally {
    signal.removeEventListener('abort', abort);
    if (!finished) await cancel();
    else if (cancelled) await cancelled;
  }
}
