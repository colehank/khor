import type { HistoryEntry } from '@/gen/bindings/HistoryEntry';
import { applyAcpUpdate, type AcpItem, type AcpUpdate } from './acp-view';
import { gui } from '@/gen/catalog';
export type HistoryItem = (AcpItem | { who: 'sys'; text: string }) & {
  historyKey: string;
};
export function paintHistory(entries: HistoryEntry[]): HistoryItem[] {
  const out: (HistoryItem | AcpItem | {
    who: 'boundary';
  })[] = [];
  for (const entry of entries) {
    const frame = entry.frame;
    if (frame.kind === 'capture_gap') {
      out.push({who: 'sys', text: gui.history_capture_gap(1), historyKey: `${entry.key}:capture`});
      continue;
    }
    if (frame.kind !== 'note')
      continue;
    const carried = frame.update as {
      update?: AcpUpdate;
    } | null;
    const update = carried?.update;
    if (!update || typeof update !== 'object')
      continue;
    // Keep record keys and paragraph boundaries stable when an older page lands.
    // Tool updates still reconcile by call ID across the retained window.
    out.push({ who: 'boundary' });
    const start = out.length;
    const orphan = update.sessionUpdate === 'tool_call_update'
      && !out.some(item => item.who === 'tool' && item.id === update.toolCallId);
    applyAcpUpdate(out, orphan ? { ...update, sessionUpdate: 'tool_call', title: update.title ?? update.toolCallId } : update);
    for (let index = start; index < out.length; index += 1) {
      out[index] = { ...out[index], historyKey: `${entry.key}:${index - start}` } as HistoryItem;
    }
  }
  return out.filter((item): item is HistoryItem => item.who !== 'boundary');
}
