// **The swipe that lowers the keyboard is iOS's own: the finger reaches
// the keyboard's top edge** (item 42; item 28 made the swipe the one rule
// that lowers it). iOS's gesture for a scrolling view is
// `keyboardDismissMode = .interactive`: a drag that reaches the keyboard
// takes it down with the finger, at any speed, after any pause. A web
// view cannot ask for it, and the rule judged at the lift — 48 pt down,
// more down than across, an *average* speed over the whole touch of
// 0.3 px/ms — failed a slow or paused drag, which had scrolled the
// content instead (the owner: 「需要很用力滑才能收起」). Now: while the
// keyboard is up, a drag that is more down than across lowers it the
// moment the finger crosses the keyboard's top edge, as the visual
// viewport places it (`lib/viewport`'s `keyboardEdge`). No speed, no
// travel beyond reaching the edge.
//
// Native listeners on the terminal's own scroller, so a touch on the key
// row or the deck (a portal into the bar, `BarSeat`) never reaches it; a
// drag the program takes as its wheel counts as well: under tmux's mouse
// the swipe lowers too.
import { keyboardUp, lowerKeyboard } from "@/lib/soft-keyboard";
import { keyboardEdge } from "@/lib/viewport";

export function swipeLowers(
  scroller: HTMLElement,
  field: HTMLElement | null,
  /** The gates' seams: what the keyboard's state, edge and lowering are. */
  seams: { up?: () => boolean; edge?: () => number | null; lower?: () => void } = {},
): () => void {
  if (!field) return () => {};
  const up = seams.up ?? (() => keyboardUp(field));
  const edge = seams.edge ?? keyboardEdge;
  const lower = seams.lower ?? (() => lowerKeyboard(field));
  let x0 = 0;
  let y0 = 0;
  /** This touch is still being judged: one finger, not from the bar, the
      keyboard not yet lowered by it. */
  let one = false;
  const start = (e: TouchEvent) => {
    one = e.touches.length === 1 && !(e.target as Element | null)?.closest?.("[data-session-bar]");
    x0 = e.touches[0]?.clientX ?? 0;
    y0 = e.touches[0]?.clientY ?? 0;
  };
  const move = (e: TouchEvent) => {
    if (e.touches.length !== 1) one = false;
    if (!one) return;
    const t = e.touches[0];
    const dy = t.clientY - y0;
    if (dy <= 0 || dy <= Math.abs(t.clientX - x0)) return;
    const top = edge();
    if (top === null || t.clientY < top || !up()) return;
    one = false;
    lower();
  };
  const end = () => {
    one = false;
  };
  scroller.addEventListener("touchstart", start, { passive: true });
  scroller.addEventListener("touchmove", move, { passive: true });
  scroller.addEventListener("touchend", end, { passive: true });
  scroller.addEventListener("touchcancel", end, { passive: true });
  return () => {
    scroller.removeEventListener("touchstart", start);
    scroller.removeEventListener("touchmove", move);
    scroller.removeEventListener("touchend", end);
    scroller.removeEventListener("touchcancel", end);
  };
}
