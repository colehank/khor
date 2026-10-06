// **A tab pill's third seat is 「和来源对齐」** (UX-33, the owner 2026-10-06):
// a session's re-attach, a page's reload (× while it loads), a file's
// re-read when clean and ✓ save when changed — on every pill, in front or
// not. The strip draws the seat; what it does belongs to the view that
// holds the thing (a terminal's `reopen`, a conversation's retry). Those
// views are mounted only for open tabs and sit three levels under the
// strip, so they **offer** their act here by tab id and the strip
// **reads** it; a pill whose view has not offered one brings the tab to
// the front, and the view offers its act as it mounts.
import { useSyncExternalStore } from "react";

export type TabActKind = "reload" | "stop" | "save";

export type TabAct = {
  kind: TabActKind;
  /** The act is under way (a join, a load): the pill's line runs. */
  busy?: boolean;
  /** Something with a known size is landing for this tab (item 60, a
      file dropped on a terminal): the pill's line is filled this far,
      0 to 1, while `busy` is not. */
  share?: number;
  run: () => void;
};

const acts = new Map<string, TabAct>();
const listeners = new Set<() => void>();
let version = 0;

function changed() {
  version += 1;
  for (const l of listeners) l();
}

/** The view that holds tab `id` offers its act; returns the withdrawal.
    Offered again with a new `busy`, the strip re-reads it. */
export function offerTabAct(id: string, act: TabAct): () => void {
  acts.set(id, act);
  changed();
  return () => {
    if (acts.get(id) === act) {
      acts.delete(id);
      changed();
    }
  };
}

export function tabAct(id: string): TabAct | null {
  return acts.get(id) ?? null;
}

/** The acts as the strip reads them: a version that moves on every offer
    and withdrawal, so a pill re-renders when its view's act changes. */
export function useTabActs(): number {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => version,
    () => version,
  );
}
