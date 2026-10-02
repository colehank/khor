// The one breakpoint, and the only place 719 is written.
//
// **The shell stamps `data-narrow` on the root from this**, and
// stylesheets key off that attribute (`app.css`, the machine card's
// part rows). A `@media` repeating 719 would be a second copy that can
// drift with nothing to catch it — and the px gate refuses it out loud,
// which is how this came back: the claim was written here before
// anything implemented it, sat false for a while, and was made true the
// day a stylesheet actually needed the number (2026-09-04).
import { useEffect, useState } from "react";

export const NARROW_QUERY = "(max-width: 759px)";

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
