// **A drawing's ink on the page's left line** (the redesign, 2026-10-02,
// the owner: one left line for a page; 「content aligns, the block does
// not have to」). A dial's box carries room around its arc for marks that
// may stand out of it, so the box sitting on the line leaves the ink a
// few pixels in. This measures the drawing's ink as drawn — the svg's
// content box, in its own units, scaled to the box — and pulls the box
// left by exactly that much, so the first mark stands on the line and
// nothing draws past it. Measured, not computed from the dial's
// geometry, so a change to a pip's length cannot leave it stale.
import { useLayoutEffect, useRef, type ReactNode } from "react";

export function useFlushInk<T extends HTMLElement>() {
  const box = useRef<T>(null);
  useLayoutEffect(() => {
    const el = box.current;
    const svg = el?.querySelector("svg");
    if (!el || !svg) return;
    const ink = svg.getBBox();
    const view = svg.viewBox.baseVal;
    const width = svg.getBoundingClientRect().width;
    if (!view || view.width === 0 || width === 0) return;
    const scale = width / view.width;
    el.style.marginInlineStart = `${-Math.max(0, ink.x - view.x) * scale}px`;
    // Every render: the marks move with the reading, and one bounding
    // box is cheaper than a reason to skip it.
  });
  return box;
}

/** The hook, as a wrapper: the drawing inside is pulled onto the line. */
export function FlushInk({ children }: { children: ReactNode }) {
  const box = useFlushInk<HTMLSpanElement>();
  return (
    <span ref={box} data-flush-ink className="flex flex-none">
      {children}
    </span>
  );
}
