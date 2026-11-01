import { useLayoutEffect, useRef, useState } from 'react';
import { gui, speaking } from '@/gen/catalog';
import { historyApi } from '@/lib/history-api';
import { historyFind, type HistoryFindState } from '@/lib/history-find';
import { findQuery, type FindHit } from '@/lib/history-find-model';
import type { HistoryActionAnswer } from '@/lib/history-find-action';
import type { HistoryExportControl } from '@/hooks/use-history-export';
import { imeKey } from '@/lib/ime';
import { Button } from '@/components/ui/button';
import { SideFind } from '@/components/SideFind';
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuLabel, ContextMenuTrigger } from '@/components/ui/context-menu';

/** A source/query owns one worker and its read leases. Results are one
 * bounded batch; closing, changing query or changing source retires them. */
export function HistorySearch({ id, binding, open, stop, exporter, onClose, onJump }: {
  exporter: HistoryExportControl; id: string; binding: string; open: boolean; stop: { current: (() => void) | null }; onClose: () => void; onJump: (hit: FindHit) => void;
}) {
  const language = speaking();
  const root = useRef<HTMLDivElement>(null);
  const current = useRef<ReturnType<typeof historyFind> | null>(null);
  const exporting = useRef<ReturnType<HistoryExportControl['run']>>(null);
  const [query, setQuery] = useState('');
  const [matchCase, setMatchCase] = useState(false);
  const [state, setState] = useState<HistoryFindState | null>(null);
  const [excerpt, setExcerpt] = useState<{ hit: FindHit; answer: HistoryActionAnswer } | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const copying = useRef<ReturnType<typeof historyFind> | null>(null);
  const [copyBusy, setCopyBusy] = useState(false);
  const retire = () => {
    const owner = current.current; current.current = null; copying.current = null;
    if (owner) void owner.dispose().then(result => { if (result.failures) console.warn('History search cleanup failed', result); });
    const transfer = exporting.current; exporting.current = null;
    transfer?.cancel();
  };
  useLayoutEffect(() => {
    if (!open) return;
    stop.current = retire;
    setState(null); setExcerpt(null); setNotice(null); setCopyBusy(false);
    const before = document.activeElement as HTMLElement | null;
    root.current?.querySelector<HTMLInputElement>('input')?.focus();
    return () => {
      if (stop.current === retire) stop.current = null;
      retire();
      if (before?.isConnected) before.focus({ preventScroll: true });
    };
  }, [id, binding, open, language]);
  const clear = () => { retire(); exporter.clearNotice(); setState(null); setExcerpt(null); setNotice(null); copying.current = null; setCopyBusy(false); };
  const begin = () => {
    clear();
    try {
      const parsed = findQuery({ text: query, caseSensitive: matchCase });
      const owner = historyFind(historyApi(id, binding), parsed, next => {
        if (current.current === owner) { setState(next); if (next.error || next.reset) setExcerpt(null); }
      });
      current.current = owner;
      void owner.start();
    } catch (error) { console.warn('History query rejected', String(error).slice(0, 512)); setNotice(gui.history_find_invalid); }
  };
  const failure = (error: unknown, action: 'read' | 'copy' | 'export' = 'read') => {
    const code = error instanceof Error ? error.message : '';
    setNotice(code === 'too_large' ? gui.history_copy_large : code === 'incomplete' ? gui.history_copy_incomplete : action === 'copy' ? gui.history_copy_failed : action === 'export' ? gui.history_export_failed : gui.history_action_failed);
    console.warn('History action failed', String(error).slice(0, 512));
  };
  const inspect = async (hit: FindHit) => {
    const owner = current.current;
    if (!owner) return;
    exporter.clearNotice();
    setNotice(null);
    if (!hit.record) { onJump(hit); return; }
    try {
      const answer = await owner.inspect(hit, 'context');
      if (current.current === owner) setExcerpt({ hit, answer });
    } catch (error) { if (current.current === owner) failure(error); }
  };
  const copy = async (hit: FindHit) => {
    const owner = current.current;
    if (!owner || copying.current) return;
    exporter.clearNotice();
    copying.current = owner; setCopyBusy(true); setNotice(null);
    const body = owner.inspect(hit, 'copy').then(answer => { if (!answer.complete) throw new Error('incomplete'); return answer.text; });
    try {
      // Delayed ClipboardItem data preserves Safari's click activation
      // while the selected original body is read asynchronously.
      if (navigator.clipboard?.write && typeof ClipboardItem !== 'undefined')
        await navigator.clipboard.write([new ClipboardItem({ 'text/plain': body.then(text => new Blob([text], { type: 'text/plain' })) })]);
      else if (navigator.clipboard?.writeText) await navigator.clipboard.writeText(await body);
      else { void body.catch(() => {}); throw new Error('clipboard unavailable'); }
      if (current.current === owner) setNotice(gui.history_copied);
    } catch (error) {
      // ClipboardItem may wrap a rejected body in a generic browser error.
      let cause = error;
      try { await body; } catch (readError) { cause = readError; }
      if (current.current === owner) failure(cause, 'copy');
    }
    finally { if (copying.current === owner) { copying.current = null; setCopyBusy(false); } }
  };
  const exportRecord = (hit: FindHit) => {
    if (!hit.record) return;
    const operation = exporter.run(hit.record);
    if (operation) exporting.current = operation;
  };
  if (!open) return null;
  const busy = state?.busy || exporter.busy || copyBusy;
  return <div ref={root} data-history-search className="flex max-h-[50%] min-h-0 flex-none flex-col gap-in px-row py-in" onKeyDownCapture={event => {
    if ((event.key === 'Enter' || event.key === 'Escape') && imeKey(event)) event.stopPropagation();
  }} onKeyDown={event => {
    if (event.key === 'Escape' && !imeKey(event)) { event.preventDefault(); event.stopPropagation(); clear(); onClose(); }
  }}>
    <SideFind className="px-0" label={gui.history_find_placeholder} query={query}
      onQuery={next => { clear(); setQuery(next); }} onEnter={begin}
      plus={<div className="flex items-center gap-in">
        <Button size="sm" variant={matchCase ? 'secondary' : 'ghost'} aria-label={gui.match_case} aria-pressed={matchCase}
          onClick={() => { clear(); setMatchCase(value => !value); }}>Aa</Button>
        <Button size="sm" disabled={!query || !!busy} onClick={begin}>{gui.search_open}</Button>
        <Button size="sm" variant="ghost" onClick={() => { clear(); onClose(); }}>{gui.close}</Button>
      </div>} />
    <div className="min-h-0 flex-1 space-y-in overflow-y-auto text-caption">
      <p className="text-muted-foreground">{gui.history_find_scope}</p>
      {state?.busy && <p role="status">{gui.history_searching}</p>}
      {state?.error && <p role="alert" className="text-destructive">{gui.history_action_failed}</p>}
      {state?.reset && <p role="status">{gui.history_find_changed}</p>}
      {state && !state.busy && !state.hits.length && state.done && <p role="status">{gui.search_none}</p>}
      {state?.hits.map((hit, index) => <ContextMenu key={`${hit.entry}:${hit.part}:${hit.from}`}>
        <ContextMenuTrigger asChild><Button data-history-find-hit size="sm" variant="ghost" disabled={!!busy}
          className="h-auto w-full justify-start whitespace-pre-wrap break-all text-left" onClick={() => void inspect(hit)}>
          <span className="min-w-0"><span className="tabular-nums text-muted-foreground">{index + 1} · </span>{hit.snippet}</span>
        </Button></ContextMenuTrigger>
        <ContextMenuContent>
          <ContextMenuItem disabled={!!busy} onSelect={() => void copy(hit)}>{gui.history_copy}</ContextMenuItem>
          {hit.record && <>
            <ContextMenuItem disabled={!!busy || !exporter.available} onSelect={() => void exportRecord(hit)}>{gui.history_export}</ContextMenuItem>
            {!exporter.available && <ContextMenuLabel>{gui.history_export_unavailable}</ContextMenuLabel>}
          </>}
        </ContextMenuContent>
      </ContextMenu>)}
      {state && !state.done && !state.error && !state.reset && <Button size="sm" disabled={!!busy} onClick={() => { setExcerpt(null); setNotice(null); current.current?.next(); }}>{gui.history_find_more}</Button>}
      {!!state?.unsearchedRanges && <p role="status" className="text-muted-foreground">{gui.history_find_partial}</p>}
      {(notice || exporter.notice) && <p role="status">{exporter.notice ?? notice}</p>}
      {excerpt && <section data-history-find-context className="space-y-in">
        <p className="text-muted-foreground">{gui.history_match_excerpt}</p>
        <pre data-selectable className="overflow-auto whitespace-pre-wrap break-all">{excerpt.answer.text.slice(0, excerpt.hit.from - excerpt.answer.from)}<mark className="bg-brand-wash text-brand-ink">{excerpt.answer.text.slice(excerpt.hit.from - excerpt.answer.from, excerpt.hit.to - excerpt.answer.from)}</mark>{excerpt.answer.text.slice(excerpt.hit.to - excerpt.answer.from)}</pre>
        <Button size="sm" disabled={!!busy} onClick={() => void copy(excerpt.hit)}>{gui.history_copy}</Button>
      </section>}
    </div>
  </div>;
}
