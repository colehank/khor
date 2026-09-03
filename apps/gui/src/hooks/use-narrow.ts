// The one breakpoint, and the only place 719 is written.
//
// **No stylesheet keys off it today.** This used to claim that CSS read
// a data attribute the shell set from here; nothing set one and nothing
// read one, so the comment sent readers looking for a mechanism that
// did not exist (checked 2026-09-04). Everything narrow is decided in
// TS, through this hook.
//
// If a stylesheet ever does need the breakpoint, publish it — stamp an
// attribute on the root from the shell and key off that — rather than
// repeating 719 in a `@media`, which is a second copy that can drift
// with nothing to catch it.
import { useEffect, useState } from "react";

export const NARROW_QUERY = "(max-width: 719px)";

export function useNarrow(): boolean {
  const [narrow, setNarrow] = useState(() => window.matchMedia(NARROW_QUERY).matches);
  useEffect(() => {
    const m = window.matchMedia(NARROW_QUERY);
    const onChange = (e: MediaQueryListEvent) => setNarrow(e.matches);
    m.addEventListener("change", onChange);
    return () => m.removeEventListener("change", onChange);
  }, []);
  return narrow;
}
