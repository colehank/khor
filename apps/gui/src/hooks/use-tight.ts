// Whether a box is too narrow for the layout it wants — asked of **the
// box**, not of the window.
//
// The machine card's part rows want three columns: a fixed name, a
// shared bar track, a fixed column of numbers. Whether those fit is a
// fact about the card, and the card is not the window: a rail and a
// list pane sit beside it, so at any given viewport the card is several
// hundred narrower than the screen. Keying the layout off the window's
// breakpoint put an overflow band right above it — measured 2026-09-04,
// 106 pixels of sideways overflow from 720 through 830, found by the
// full-bandwidth scan and by nothing else.
//
// So this measures the element. It is a container query written in TS,
// for the reason a container query would be right: the question is
// 「does this fit in here」 and only the box knows.
import { useEffect, useState, type RefObject } from "react";

export function useTight(ref: RefObject<HTMLElement | null>, min: number): boolean {
  const [tight, setTight] = useState(false);
  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    const ro = new ResizeObserver(([entry]) => {
      setTight(entry.contentRect.width < min);
    });
    ro.observe(node);
    return () => ro.disconnect();
  }, [ref, min]);
  return tight;
}
