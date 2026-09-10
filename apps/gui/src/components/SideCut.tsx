// A sideways scroller that says when it has cut something off
// (用户 2026-09-10 选「丁」:一个字都不写,边缘渐隐).
//
// **The measuring is here and the drawing is in `app.css`**, split that
// way because they answer different questions: whether anything is cut
// off is a fact about this element right now, and what a cut edge looks
// like is a fact about this app. The stylesheet keys off the two marks
// this sets and nothing else.
//
// **Both edges, not just the right one.** The option that was chosen
// named the right edge, because that is the one a table starts cut on —
// but the moment somebody scrolls, the left is cut too, and a mark that
// appeared on one side and never the other would be reporting half of
// what it can see.
import { useEffect, useRef, type ReactNode } from "react";

import { cn } from "@/lib/utils";

export function SideCut({
  children,
  className,
  ...rest
}: {
  children: ReactNode;
  /** Goes on the scroller, not the frame — the frame exists only to
      hold the veils still while the content moves under them. */
  className?: string;
} & React.HTMLAttributes<HTMLDivElement>) {
  const frame = useRef<HTMLDivElement>(null);
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = box.current;
    const on = frame.current;
    if (!el || !on) return;
    const mark = () => {
      // **A pixel of slack on both ends.** Sub-pixel layout leaves
      // `scrollWidth` a hair over `clientWidth` on tables that fit
      // exactly, and a veil drawn for that hair is a veil that never
      // goes away — which is the one failure this option cannot afford,
      // since it has no words to correct itself with.
      const room = el.scrollWidth - el.clientWidth;
      on.toggleAttribute("data-cut-left", el.scrollLeft > 1);
      on.toggleAttribute("data-cut-right", room - el.scrollLeft > 1);
    };
    mark();
    el.addEventListener("scroll", mark, { passive: true });
    // Two things change whether anything is cut off, and they are not
    // the same event: the frame getting narrower, and the content
    // getting wider (a column appearing when a provider starts
    // reporting cache figures). The observer watches both.
    const watch = new ResizeObserver(mark);
    watch.observe(el);
    for (const kid of Array.from(el.children)) watch.observe(kid);
    return () => {
      el.removeEventListener("scroll", mark);
      watch.disconnect();
    };
  }, []);

  return (
    <div ref={frame} data-side-cut {...rest}>
      <div ref={box} className={cn("overflow-x-auto", className)}>
        {children}
      </div>
    </div>
  );
}
