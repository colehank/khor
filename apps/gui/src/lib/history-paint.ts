import type { HistoryRecordRef } from '@/gen/bindings/HistoryRecordRef';
import type { HistoryEntry } from '@/gen/bindings/HistoryEntry';
import { applyAcpUpdate, type AcpItem, type AcpUpdate } from './acp-view';
import { gui } from '@/gen/catalog';
export type HistoryItem = (AcpItem | { who: 'sys'; text: string }) & {
  historyKey: string;
  displayKey?: string;
  /** Contributions belong only to the currently retained bounded window. */
  historyEntries: string[];
  historyRecords: { entry: string; record: HistoryRecordRef | null }[];
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

export function paintHistory(entries: HistoryEntry[], references: HistoryRecordRef[] = []): HistoryItem[] {
  const out: (HistoryItem | AcpItem | {
    who: 'boundary';
  })[] = [];
  let previousRecord: HistoryRecordRef | null = null;
  for (const entry of entries) {
    const record = entry.record ?? null;
    // A referenced raw record may contain text we have not projected. Never
    // concatenate the visible fragments across it into an invented message.
    if (references.length && (!record || (previousRecord && references.some(reference =>
      historyReferenceBetween(reference, previousRecord, record)))))
      out.push({who:'boundary'});
    if (record) previousRecord = record;
    const frame = entry.frame;
    if (frame.kind === 'capture_gap') {
      out.push({who: 'sys', text: gui.history_capture_gap(1), historyKey: `${entry.key}:capture`, historyEntries: [entry.key], historyRecords: [{entry: entry.key, record}]});
      continue;
    }
    if (frame.kind === 'turn') { out.push({who: 'boundary'}); continue; }
    if (frame.kind !== 'note')
      continue;
    const carried = frame.update as {
      update?: AcpUpdate;
    } | null;
    const update = carried?.update;
    if (!update || typeof update !== 'object')
      continue;
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
      const prior = before[index] as HistoryItem | undefined;
      out[index] = { ...out[index],
        historyKey: prior?.historyKey ?? `${entry.key}:${index - start}`,
        historyEntries: [...(prior?.historyEntries ?? []), entry.key],
        historyRecords: [...(prior?.historyRecords ?? []), {entry: entry.key, record}],
      } as HistoryItem;
    }
  }
  return out.filter((item): item is HistoryItem => item.who !== 'boundary');
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
