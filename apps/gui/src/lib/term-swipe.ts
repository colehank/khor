// **Any downward swipe lowers the keyboard** (item 49; item 28 made the
// swipe the one rule that lowers it). While the keyboard is up, a
// one-finger touch that leaves the tap's dead zone (`SLOP`, the pane's
// `TAP_SLOP`) more down than across lowers it at once — no speed, no
// edge, no travel beyond the dead zone. The content may scroll under the
// same drag; the owner accepted that.
//
// ⚠️ Tombstone (item 42, then item 28): the keyboard went down only when
// the finger crossed its top edge (`lib/viewport`'s `keyboardEdge`), and
// before that only for 48 pt at an average of 0.3 px/ms judged at the
// lift. Both were too hard on the iPhone (the owner, 2026-10-06 10:50,
// on the build carrying item 42: still 「需要很用力滑」).
//
// A touch held in the dead zone for `PRESS_MS` is a press — the pane's
// selection (`pressSelects`) — and a drag after it never lowers: the
// grid refitting under a selection's finger would move what it selects.
//
// Native listeners on the terminal's own scroller, so a touch on the key
// row, the deck or the key panel (portals into the bar, `BarSeat`) never
// reaches it; a drag the program takes as its wheel counts as well: under
// tmux's mouse the swipe lowers too.
import { keyboardUp, lowerKeyboard } from "@/lib/soft-keyboard";

/** The tap's dead zone, px (the pane's `TAP_SLOP`): past it a touch is a
    drag. */
const SLOP = 10;
/** Held this long inside the dead zone, a touch is a press (the pane's
    `LONG_PRESS_MS`). */
const PRESS_MS = 450;

export function swipeLowers(
  scroller: HTMLElement,
  field: HTMLElement | null,
  /** The gates' seams, and the key panel's (UX-31): what is up and how
      it goes down. */
  seams: { up?: () => boolean; lower?: () => void } = {},
): () => void {
  if (!field) return () => {};
  const up = seams.up ?? (() => keyboardUp(field));
  const lower = seams.lower ?? (() => lowerKeyboard(field));
  let x0 = 0;
  let y0 = 0;
  let t0 = 0;
  /** The touch has left the dead zone. */
  let away = false;
  /** This touch is still being judged: one finger, not from the bar, not
      a press, the keyboard not yet lowered by it. */
  let one = false;
  const start = (e: TouchEvent) => {
    one = e.touches.length === 1 && !(e.target as Element | null)?.closest?.("[data-session-bar]");
    away = false;
    x0 = e.touches[0]?.clientX ?? 0;
    y0 = e.touches[0]?.clientY ?? 0;
    t0 = performance.now();
  };
  const move = (e: TouchEvent) => {
    if (e.touches.length !== 1) one = false;
    if (!one) return;
    const t = e.touches[0];
    const dx = t.clientX - x0;
    const dy = t.clientY - y0;
    if (!away) {
      if (Math.hypot(dx, dy) <= SLOP) return;
      if (performance.now() - t0 >= PRESS_MS) {
        one = false;
        return;
      }
      away = true;
    }
    if (dy <= Math.abs(dx) || !up()) return;
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
