import { useEffect, useRef, useState } from 'react';
import type { HistoryRecordRef } from '@/gen/bindings/HistoryRecordRef';
import type { HistoryRecordChunk } from '@/gen/bindings/HistoryRecordChunk';
import { Button } from '@/components/ui/button';
import { gui } from '@/gen/catalog';
import { decodeRecordSection } from '@/lib/history-record';
type ReadRecord = (cursor: string, from: number, signal?: AbortSignal) => Promise<HistoryRecordChunk | null>;
/** Only one raw section is retained. Next advances a streaming UTF-8 decoder;
 * reopening a reference starts a fresh stream rather than concatenating a file. */
export function HistoryRecords({ references, read }: {
  references: HistoryRecordRef[];
  read: ReadRecord;
}) {
  const [selected, setSelected] = useState<HistoryRecordRef | null>(null);
  const [text, setText] = useState('');
  const [next, setNext] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const decoder = useRef(new TextDecoder());
  const pending = useRef<AbortController | null>(null);
  useEffect(() => () => { pending.current?.abort(); }, []);
  const load = async (reference: HistoryRecordRef, from: number, restart: boolean) => {
    pending.current?.abort();
    const controller = new AbortController();
    pending.current = controller;
    if (restart) {
      decoder.current = new TextDecoder();
      setText('');
      setNext(null);
    }
    setSelected(reference);
    setBusy(true);
    setError(null);
    try {
      const chunk = await read(reference.cursor, from, controller.signal);
      if (!chunk || controller.signal.aborted || pending.current !== controller) return;
      setText(decodeRecordSection(chunk, decoder.current));
      setNext(chunk.next);
    } catch (failure) {
      if (!controller.signal.aborted) setError(String(failure));
    } finally {
      if (pending.current === controller && !controller.signal.aborted) setBusy(false);
    }
  };
  if (!references.length && !selected) return null;
  return (
    <section className="space-y-in py-row" data-history-records>
      <p className="text-caption text-muted-foreground">{gui.history_record_note}</p>
      <div className="flex flex-wrap gap-in">
        {references.map((reference, index) => (
          <Button key={`${reference.start}:${reference.item ?? 'raw'}`} size="sm" variant="secondary"
            onClick={() => void load(reference, reference.start, true)}>
            {gui.history_record} · {index + 1}
          </Button>
        ))}
      </div>
      {selected && (
        <div className="space-y-in">
          <div className="flex flex-wrap items-center gap-in">
            <Button size="sm" disabled={busy} onClick={() => void load(selected, selected.start, true)}>{gui.history_record_restart}</Button>
            <Button size="sm" disabled={busy || next === null} onClick={() => {
              if (next !== null) void load(selected, next, false);
            }}>{gui.history_record_next}</Button>
          </div>
          {error && <p role="alert" className="text-caption text-destructive">{error}</p>}
          <pre data-selectable className="max-h-[50vh] overflow-auto whitespace-pre-wrap break-all text-caption">{text}</pre>
        </div>
      )}
    </section>
  );
}
