import { useEffect, useRef, useState } from "react";

import type { PillNotice } from "@/lib/pill-notices";
import { YIELD_MS } from "@/lib/tempo";

type Signal = "rejoin" | "upload" | "failed" | "settling";

/** The current pill's perimeter, from the same facts as its words.
 * Progress/seconds do not restart it; changing tabs never signals completion.
 * A transfer ending only fades the activity light: cancellation is not success.
 */
export function usePillSignal(id: string, face?: { notices?: PillNotice[]; drop?: unknown } | null) {
  const now = face?.notices?.[0]?.kind ?? (face?.drop ? "upload" : undefined);
  const previous = useRef({ id, kind: now });
  const [tail, setTail] = useState<string | null>(null);
  useEffect(() => {
    const before = previous.current;
    previous.current = { id, kind: now };
    if (before.id === id && !now && (before.kind === "rejoin" || before.kind === "upload")) {
      setTail(id);
      const timer = window.setTimeout(() => setTail(null), YIELD_MS);
      return () => window.clearTimeout(timer);
    }
    setTail(null);
  }, [id, now]);
  return (now ?? (tail === id ? "settling" : undefined)) as Signal | undefined;
}
