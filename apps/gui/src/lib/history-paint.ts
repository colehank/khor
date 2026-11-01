import type { HistoryRecordRef } from '@/gen/bindings/HistoryRecordRef';
import type { HistoryEntry } from '@/gen/bindings/HistoryEntry';
import { applyAcpUpdate, type AcpItem, type AcpUpdate } from './acp-view';
import { gui } from '@/gen/catalog';
export type HistoryTextSpan = {entry: string; from: number; to: number};
export type HistoryContributionPoint = {entry: string; offset: number};
export type HistoryItem = (AcpItem | { who: 'sys'; text: string }) & {
  historyKey: string;
  displayKey?: string;
  /** Contributions belong only to the currently retained bounded window. */
  historyEntries: string[];
  historyRecords: { entry: string; record: HistoryRecordRef | null }[];
  /** UTF16 offsets into this bounded painted body, not raw-file byte ranges.
   * null means this optional anchor metadata exceeded its contribution budget. */
  historyTextSpans?: HistoryTextSpan[] | null;
};
/** Ordering comes from checked physical ranges plus the parser item slot.
 * An overlapping or whole-record reference without a comparable slot is unknown. */
export function historyReferenceBetween(reference: HistoryRecordRef, left?: HistoryRecordRef | null, right?: HistoryRecordRef | null): boolean {
  const same = (a: HistoryRecordRef, b: HistoryRecordRef) => a.start === b.start && a.end === b.end
    && typeof a.item === 'number' && a.item === b.item;
  const before = (a: HistoryRecordRef, b: HistoryRecordRef) => a.end <= b.start
    || (a.start === b.start && a.end === b.end && typeof a.item === 'number' && typeof b.item === 'number' && a.item < b.item);
  if (left && (before(reference, left) || same(reference, left))) return false;
  if (right && (before(right, reference) || same(right, reference))) return false;
  return true;
}

export type HistoryPaintStep = HistoryItem | AcpItem | { who: 'boundary' };
/** The caller owns this temporary fold. Streaming readers use the exact same
 * normalization and grouping step, then retire no-longer-needed contributions. */
export function appendHistoryEntry(out: HistoryPaintStep[], entry: HistoryEntry,
  references: HistoryRecordRef[] = [], previousRecord: HistoryRecordRef | null = null): HistoryRecordRef | null {
  const record = entry.record ?? null;
  // A referenced raw record may contain text we have not projected. Never
  // concatenate the visible fragments across it into an invented message.
  if (references.length && (!record || (previousRecord && references.some(reference =>
    historyReferenceBetween(reference, previousRecord, record)))))
    out.push({who:'boundary'});
  const nextRecord = record ?? previousRecord;
  const frame = entry.frame;
  if (frame.kind === 'capture_gap') {
    out.push({who: 'sys', text: gui.history_capture_gap(1), historyKey: `${entry.key}:capture`, historyEntries: [entry.key], historyRecords: [{entry: entry.key, record}]});
    return nextRecord;
  }
  if (frame.kind === 'turn') { out.push({who: 'boundary'}); return nextRecord; }
  if (frame.kind !== 'note')
    return nextRecord;
  const carried = frame.update as {
    update?: AcpUpdate;
  } | null;
  const update = carried?.update;
  if (!update || typeof update !== 'object')
    return nextRecord;
  // The reader distinguishes complete native messages/prompts from streamed
  // ACP chunks. Record offsets alone cannot tell those two meanings apart.
  if (entry.starts_message) out.push({ who: 'boundary' });
  const before = out.slice();
  const start = out.length;
  const orphan = update.sessionUpdate === 'tool_call_update'
    && !out.some(item => item.who === 'tool' && item.id === update.toolCallId);
  applyAcpUpdate(out, orphan ? { ...update, sessionUpdate: 'tool_call', title: update.title ?? update.toolCallId } : update);
  for (let index = 0; index < out.length; index += 1) {
    if (out[index].who === 'boundary' || out[index] === before[index]) continue;
    const prior = before[index] as HistoryItem | undefined, next = out[index];
    const from=prior && 'text' in prior ? prior.text.length : 0;
    const spans='text' in next
      ? prior?.historyTextSpans===null || (prior?.historyTextSpans?.length ?? 0)>=512 ? null
        : [...(prior?.historyTextSpans ?? []),{entry:entry.key,from,to:next.text.length}]
      : undefined;
    out[index] = { ...next,
      ...(spans===undefined ? {} : {historyTextSpans:spans}),
      historyKey: prior?.historyKey ?? `${entry.key}:${index - start}`,
      historyEntries: [...(prior?.historyEntries ?? []), entry.key],
      historyRecords: [...(prior?.historyRecords ?? []), {entry: entry.key, record}],
    } as HistoryItem;
  }
  return nextRecord;
}

export function paintHistory(entries: HistoryEntry[], references: HistoryRecordRef[] = []): HistoryItem[] {
  const out: HistoryPaintStep[] = [];
  let previousRecord: HistoryRecordRef | null = null;
  for (const entry of entries) previousRecord = appendHistoryEntry(out, entry, references, previousRecord);
  return out.filter((item): item is HistoryItem => item.who !== 'boundary');
}

/** A visible DOM sample earns a source-relative anchor only if it occurs
 * once in the actual shared projected body. Markdown syntax is not guessed. */
export function historyContributionPoint(item: HistoryItem, sample: string, sampleFrom: number): HistoryContributionPoint | null {
  if(!sample || sample.length>48 || !('text' in item) || !item.historyTextSpans)return null;
  const start=item.text.indexOf(sample);
  if(start<0 || item.text.indexOf(sample,start+1)>=0)return null;
  const at=start-sampleFrom, span=item.historyTextSpans.find(span=>at>=span.from && at<span.to);
  return span ? {entry:span.entry,offset:at-span.from} : null;
}

/** The exact contribution must still be retained and unchanged. A nearby
 * identical phrase in another record does not inherit this reading position. */
export function historyContributionPresent(item: HistoryItem, point: HistoryContributionPoint, sample: string, sampleFrom: number): boolean {
  if(!sample || sample.length>48 || !Number.isSafeInteger(point.offset) || !('text' in item) || !item.historyTextSpans)return false;
  const spans=item.historyTextSpans.filter(span=>span.entry===point.entry);
  if(spans.length!==1 || point.offset<0 || point.offset>=spans[0].to-spans[0].from)return false;
  const at=spans[0].from+point.offset+sampleFrom;
  return at>=0 && item.text.slice(at,at+sample.length)===sample;
}

/** Canonical body and live controls have separate owners. Only contributions
 * after the latest recorded turn/gap can belong to a current streaming body.
 * Callers still gate live/readonly/current-window eligibility independently. */
export function latestHistoryBody(past: HistoryItem[], entries: HistoryEntry[]): HistoryItem | null {
  let after = 0;
  for (let index = entries.length - 1; index >= 0; index--) {
    if (entries[index].frame.kind === 'turn' || entries[index].frame.kind === 'capture_gap') {
      after = index + 1;
      break;
    }
  }
  const current = new Set(entries.slice(after).map(entry => entry.key));
  for (let index = past.length - 1; index >= 0; index--) {
    const item = past[index];
    if (item.who !== 'sys' && item.historyEntries.some(key => current.has(key))) return item;
  }
  return null;
}

/** Reuse disclosure owners only through retained record contributions. Each
 * previous owner is assigned at most once; callers drop previous on reset. */
export function retainHistoryIdentity(previous: HistoryItem[], next: HistoryItem[]): HistoryItem[] {
  const owners = new Map<string, string>();
  for (const item of previous)
    for (const key of item.historyEntries)
      owners.set(`${item.who}:${key}`, item.displayKey ?? item.historyKey);
  const used = new Set<string>();
  return next.map((item, index) => {
    const inherited = item.historyEntries.map(key => owners.get(`${item.who}:${key}`))
      .find((key): key is string => key !== undefined && !used.has(key));
    let displayKey = inherited ?? item.historyKey;
    let collision = 0;
    while (used.has(displayKey)) displayKey = `${item.historyKey}:view:${index}:${collision++}`;
    used.add(displayKey);
    return {...item, displayKey};
  });
}
