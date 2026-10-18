import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { markSeen, type SessionRow } from "@/api";
import { SessionReads, type ReadReceipt } from "@/lib/session-read";

/** Read here now; persist independently of the list/network heartbeat. Failed
 * writes remain queued and retry while this face is alive, including after Back. */
export function useSessionReads(raw: SessionRow[]) {
  const [receipts] = useState(() => new SessionReads());
  const [version, setVersion] = useState(0);
  const sending = useRef(new Set<string>());
  const mounted = useRef(true);
  const flush = useCallback(() => {
    for (const receipt of receipts.pending()) {
      if (sending.current.has(receipt.id)) continue;
      sending.current.add(receipt.id);
      void markSeen(receipt.id, receipt.at_ms).then(
        () => receipts.confirmed(receipt),
        () => { /* Preserve the read intent for the next retry. */ },
      ).finally(() => { sending.current.delete(receipt.id); });
    }
  }, [receipts]);
  useEffect(() => {
    mounted.current = true;
    flush();
    const timer = window.setInterval(flush, 2000);
    return () => { mounted.current = false; window.clearInterval(timer); };
  }, [flush]);
  const read = useCallback((row: ReadReceipt & { unread: number }) => {
    if (!mounted.current || !receipts.read(row)) return;
    setVersion((v) => v + 1);
    flush();
  }, [receipts, flush]);
  const rows = useMemo(() => receipts.project(raw), [raw, receipts, version]);
  return { rows, read };
}
