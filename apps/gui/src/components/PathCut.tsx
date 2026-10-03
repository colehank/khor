import { useLayoutEffect, useState } from "react";
import { cn } from "@/lib/utils";

/**
 * **A path cut at its start, by measure** (2026-10-03, the owner on a
 * phone: the workspace rows' 「…」 was drawn over the path, and other rows
 * lost their first characters at the left edge with no ellipsis at all).
 *
 * The end of a path is what tells two repositories apart, so a path that
 * does not fit keeps its tail: `…/working/khor`. The cut falls on a `/`
 * where one leaves room, so the line starts on a whole segment; only a
 * last segment wider than the line is cut inside itself.
 *
 * ⚠️ 墓碑: this was `direction: rtl` with the text in an LTR `<bdi>` and
 * `text-overflow: ellipsis`. Chrome draws that right; **WebKit does not**
 * — it lays the ellipsis over the isolated run without clipping the
 * glyphs under it, or clips at the left with no ellipsis (measured in
 * Simulator Safari, `ux-03-wk-ws-list-before`). Measuring in JS is the
 * same answer on every engine, and it is what the workspace head already
 * did for its middle cut.
 *
 * One rule for every path shown in a line that can be too short for it:
 * the workspace rows, the machine page's workspace list, a host. The
 * whole value stays the title (a pointer's tooltip) and is what a screen
 * reader reads; the cut is only drawn.
 */
export function PathCut({
  text,
  className,
  ...data
}: {
  text: string;
  className?: string;
  [mark: `data-${string}`]: unknown;
}) {
  const [box, setBox] = useState<HTMLSpanElement | null>(null);
  const [shown, setShown] = useState(text);
  useLayoutEffect(() => {
    if (box === null) return;
    const fit = () => {
      const room = box.clientWidth;
      if (room <= 0) return;
      const cs = getComputedStyle(box);
      const wide = measurer(`${cs.fontStyle} ${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`);
      // A pixel spare: the canvas and the line can round a kern apart.
      setShown(cutHead(text, (s) => wide(s) <= room - 1));
    };
    fit();
    // Fires on the column's width and on the type size (the line's
    // height changes with it).
    const watch = new ResizeObserver(fit);
    watch.observe(box);
    return () => watch.disconnect();
  }, [box, text]);
  return (
    <span
      {...(data as Record<string, unknown>)}
      ref={setBox}
      title={text}
      className={cn("block min-w-0 overflow-hidden whitespace-nowrap", className)}
    >
      <span aria-hidden>{shown}</span>
      <span className="sr-only">{text}</span>
    </span>
  );
}

/** The longest tail of `text`, behind 「…」, that `fits`: the whole text
    when it fits; else from the earliest `/` that leaves room; else (one
    segment wider than the line) as many of its last characters as fit. */
export function cutHead(text: string, fits: (s: string) => boolean): string {
  if (fits(text)) return text;
  for (let at = text.indexOf("/", 1); at > 0; at = text.indexOf("/", at + 1)) {
    const tail = `…${text.slice(at)}`;
    if (fits(tail)) return tail;
  }
  // Characters, counted from the end, by halves.
  const chars = [...text];
  let lo = 0;
  let hi = chars.length;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if (fits(`…${chars.slice(chars.length - mid).join("")}`)) lo = mid;
    else hi = mid - 1;
  }
  return `…${chars.slice(chars.length - lo).join("")}`;
}

let ruler: CanvasRenderingContext2D | null = null;
/** One canvas for every cut on screen; `font` is set per measure. */
function measurer(font: string): (s: string) => number {
  ruler ??= document.createElement("canvas").getContext("2d");
  return (s) => {
    if (ruler === null) return 0;
    ruler.font = font;
    return ruler.measureText(s).width;
  };
}
