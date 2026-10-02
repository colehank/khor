// Whether a finger is what presses this page (`(pointer: coarse)`).
//
// **The pointer, never the width** (docs/UX.md, 2026-10-02): an iPad is
// wide and has fingers, a Mac window dragged narrow has a mouse. The
// stylesheet asks the same question for the floors (`tokens.css`, the
// coarse block); this is the same question for the things that exist
// only under a finger — the keys a soft keyboard lacks and the switch
// for the keyboard (`views/TermKeys`).
import { useEffect, useState } from "react";

const COARSE = "(pointer: coarse)";

export function useCoarse(): boolean {
  const [coarse, setCoarse] = useState(() => window.matchMedia(COARSE).matches);
  useEffect(() => {
    const m = window.matchMedia(COARSE);
    const on = (e: MediaQueryListEvent) => setCoarse(e.matches);
    setCoarse(m.matches);
    m.addEventListener("change", on);
    return () => m.removeEventListener("change", on);
  }, []);
  return coarse;
}
