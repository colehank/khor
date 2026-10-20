import { useCallback, useEffect, useLayoutEffect, useState } from "react";
import type { ReadReceipt } from "@/lib/session-read";

export type SessionReading = (id: string, reading: boolean) => void;

/** The renderer owns loading/history; the shell owns focus and occlusion.
 * Neither selection nor a network update alone is evidence of a new look. */
export function useSessionAttention(
  row: (ReadReceipt & { unread: number }) | null,
  visible: boolean,
  read: (row: ReadReceipt & { unread: number }) => void,
): SessionReading {
  const [focused, setFocused] = useState(() => document.hasFocus());
  const [readers, setReaders] = useState<Set<string>>(() => new Set());
  useEffect(() => {
    const sync = () => setFocused(document.hasFocus());
    window.addEventListener("focus", sync);
    window.addEventListener("blur", sync);
    sync();
    return () => {
      window.removeEventListener("focus", sync);
      window.removeEventListener("blur", sync);
    };
  }, []);
  const observe = useCallback<SessionReading>((id, reading) => {
    setReaders((was) => {
      if (was.has(id) === reading) return was;
      const next = new Set(was);
      if (reading) next.add(id);
      else next.delete(id);
      return next;
    });
  }, []);
  useLayoutEffect(() => {
    // Recheck live focus too: a completion and blur can share a frame.
    if (row && visible && focused && document.hasFocus() && readers.has(row.id)) read(row);
  }, [row, visible, focused, readers, read]);
  return observe;
}

/** Kept terminals explicitly stop reading while hidden or up in history. */
export function useReadingSession(id: string, reading: boolean, observe?: SessionReading): void {
  useLayoutEffect(() => {
    observe?.(id, reading);
    return () => observe?.(id, false);
  }, [id, reading, observe]);
}
