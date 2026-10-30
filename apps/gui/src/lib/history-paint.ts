import type { HistoryEntry } from '@/gen/bindings/HistoryEntry';
import { applyAcpUpdate, type AcpItem, type AcpUpdate } from './acp-view';
import { gui } from '@/gen/catalog';
export type HistoryItem = (AcpItem | { who: 'sys'; text: string }) & {
  historyKey: string;
  /** Contributions belong only to the currently retained bounded window. */
  historyEntries: string[];
};
export function paintHistory(entries: HistoryEntry[]): HistoryItem[] {
  const out: (HistoryItem | AcpItem | {
    who: 'boundary';
  })[] = [];
  for (const entry of entries) {
    const frame = entry.frame;
    if (frame.kind === 'capture_gap') {
      out.push({who: 'sys', text: gui.history_capture_gap(1), historyKey: `${entry.key}:capture`, historyEntries: [entry.key]});
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
      } as HistoryItem;
    }
  }
  return out.filter((item): item is HistoryItem => item.who !== 'boundary');
}
