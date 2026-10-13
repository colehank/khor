// A buffer's shape is not an input capability. A tmux client can hold the
// outer mouse while the program in its pane never requested mouse reports.
import type { Terminal } from "@xterm/xterm";
import { SETTLE_MS } from "./term-scroller";

export type PaneScrollContext = { mouse: boolean; alternate: boolean };
export type ScrollTarget = "history" | "mouse" | "pages";

export function scrollTarget(term: Terminal, pane: PaneScrollContext | null = null): ScrollTarget {
  const alternate = term.buffer.active.type === "alternate";
  const mouse = alternate && pane ? pane.mouse : term.modes.mouseTrackingMode !== "none";
  if (mouse) return "mouse";
  if (alternate && (pane?.alternate ?? true)) return "pages";
  return "history";
}

/** A page is the fallback for a full-screen program that did not take the
    mouse. Send the first page promptly, then accumulate one viewport of
    travel per page. A new gesture or reversed direction starts fresh. */
export function pageScroll(send: (bytes: string) => void, height: () => number, now = () => performance.now()) {
  let last = -Infinity;
  let direction = 0;
  let carried = 0;
  return (rows: number) => {
    if (!Number.isFinite(rows) || rows === 0) return;
    const at = now();
    const next = Math.sign(rows);
    const fresh = at - last > SETTLE_MS || direction !== next;
    last = at;
    direction = next;
    if (fresh) carried = 0;
    carried += Math.abs(rows);
    let pages = fresh ? 1 : 0;
    if (fresh) carried = Math.max(0, carried - 1);
    const step = Math.max(1, height() - 1);
    pages += Math.floor(carried / step);
    carried %= step;
    const key = next < 0 ? "\x1b[5~" : "\x1b[6~";
    if (pages) send(key.repeat(pages));
  };
}

/** Intercept only the standard paging fallback; application mouse reports
    and ordinary history retain their existing owners. The same binding
    handles a desktop wheel and the native touch scroller's notches. */
export function wheelsToPages(box: HTMLElement, term: Terminal, pane: () => PaneScrollContext | null): () => void {
  const page = pageScroll(bytes => term.input(bytes, true), () => term.rows);
  const wheel = (e: WheelEvent) => {
    if (e.ctrlKey || e.metaKey || scrollTarget(term, pane()) !== "pages") return;
    e.preventDefault();
    e.stopPropagation();
    const row = (term.element?.querySelector(".xterm-screen")?.clientHeight ?? 0) / Math.max(1, term.rows) || 16;
    const rows = e.deltaMode === WheelEvent.DOM_DELTA_LINE ? e.deltaY : e.deltaMode === WheelEvent.DOM_DELTA_PAGE ? e.deltaY * term.rows : e.deltaY / row;
    page(rows);
  };
  box.addEventListener("wheel", wheel, { capture: true, passive: false });
  return () => box.removeEventListener("wheel", wheel, true);
}
