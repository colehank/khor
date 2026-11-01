import { useLayoutEffect, useRef, useState } from 'react';
import { inApp, startHistoryExport, fileHandoffState, fileHandoffCancel, fileHandoffSave } from '@/api';
import { gui } from '@/gen/catalog';
import type { HistoryRecordRef } from '@/gen/bindings/HistoryRecordRef';
import { historyApi } from '@/lib/history-api';
import { completeHistoryRecord, exportHistoryRecord, exportHistoryToApp } from '@/lib/history-export';

type Picker = (options: { suggestedName: string }) => Promise<FileSystemFileHandle>;
const picker = () => (window as Window & { showSaveFilePicker?: Picker }).showSaveFilePicker?.bind(window);
type Operation = { cancel: () => void; finished: Promise<void> };

/** One export owner per conversation. Search and raw records share it;
 * a caller's cancel capability can only cancel the operation it started. */
export function useHistoryExport(id: string, binding: string | null) {
  const current = useRef<{ stop: AbortController; source: ReturnType<typeof historyApi> } | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const stop = () => {
    const operation = current.current; current.current = null;
    operation?.stop.abort();
    if (operation) void operation.source.dispose();
    setBusy(false); setNotice(null);
  };
  useLayoutEffect(() => {
    setBusy(false); setNotice(null);
    return () => {
      const operation = current.current; current.current = null;
      operation?.stop.abort();
      if (operation) void operation.source.dispose();
    };
  }, [id, binding]);
  const run = (reference: HistoryRecordRef): Operation | null => {
    const choose = picker();
    if (!binding || current.current || (!inApp && !choose)) return null;
    const operation = { source: historyApi(id, binding), stop: new AbortController() };
    current.current = operation; setBusy(true); setNotice(gui.file_open_preparing);
    const finished = (async () => {
      try {
        // Browser file panels require the original click activation. Native
        // panels are presented later, after the app-owned original is staged.
        const handle = inApp ? null : await choose!({ suggestedName: 'history-record.jsonl' });
        if (operation.stop.signal.aborted) return;
        const complete = await completeHistoryRecord(operation.source, reference, operation.stop.signal);
        let saved = true;
        if (inApp) {
          const token = crypto.randomUUID();
          saved = await exportHistoryToApp({
            start: () => startHistoryExport(token, id, binding, complete.cursor, complete.start, complete.end),
            state: () => fileHandoffState(token), cancel: () => fileHandoffCancel(token), save: () => fileHandoffSave(token),
          }, complete, operation.stop.signal);
        } else {
          const writer = await handle!.createWritable();
          await exportHistoryRecord(operation.source, complete, writer, operation.stop.signal);
        }
        if (current.current === operation) setNotice(saved ? gui.history_exported : null);
      } catch (error) {
        if (current.current === operation && !operation.stop.signal.aborted) {
          setNotice(error instanceof DOMException && error.name === 'AbortError' ? null : gui.history_export_failed);
          console.warn('History export failed', String(error).slice(0, 512));
        }
      } finally {
        const cleanup = await operation.source.dispose();
        if (cleanup.failures) console.warn('History export read cleanup failed', cleanup);
        if (current.current === operation) { current.current = null; setBusy(false); }
      }
    })();
    return { cancel: () => {
      operation.stop.abort(); void operation.source.dispose();
      if (current.current === operation) { current.current = null; setBusy(false); setNotice(null); }
    }, finished };
  };
  return { busy, notice, available: !!binding && (inApp || !!picker()), run, stop,
    clearNotice: () => { if (!current.current) setNotice(null); } };
}
export type HistoryExportControl = ReturnType<typeof useHistoryExport>;
