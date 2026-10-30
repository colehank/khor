import type { HistoryRecordRef } from '@/gen/bindings/HistoryRecordRef';
import type { historySource } from './history-source';
import type { FileHandoff } from '@/gen/bindings/FileHandoff';
type Writer = Pick<FileSystemWritableFileStream, 'write' | 'close' | 'abort'>;

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
