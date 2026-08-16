// The one breakpoint. CSS keys off the data attribute the shell sets
// from here — defining it again in a @media would be a second copy.
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
