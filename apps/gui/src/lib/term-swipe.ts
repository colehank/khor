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
// **At the bottom before, at the bottom after** (item 50's rest, the lead
// 2026-10-06, from term-49's walk): the swipe scrolls the history a row
// or two before the keyboard goes, and the view then stayed that far up —
// the prompt just under the screen and the ↓ 新输出 float on its row, so
// the next tap on the prompt pressed the float. A touch that began at the
// bottom and lowered the keyboard puts the view back at the bottom once
// the finger is up and the keyboard has gone (`atRest` / `toRest`).
//
// Native listeners on the terminal's own scroller, so a touch on the key
// row, the deck or the key panel (portals into the bar, `BarSeat`) never
// reaches it; a drag the program takes as its wheel counts as well: under
// tmux's mouse the swipe lowers too.
import { FOLD_AT, tellDeck, UNFOLD_BY } from "@/lib/deck-fold";
import { keyboardUp, lowerKeyboard } from "@/lib/soft-keyboard";
import { footCovered } from "@/lib/viewport";

/** The tap's dead zone, px (the pane's `TAP_SLOP`): past it a touch is a
    drag. */
const SLOP = 10;
/** Held this long inside the dead zone, a touch is a press (the pane's
    `LONG_PRESS_MS`). */
const PRESS_MS = 450;
/** How often, and how long at most, the settle waits for the keyboard
    to have gone. */
const SETTLE_POLL_MS = 50;
const SETTLE_MAX_MS = 1500;
/** After the keyboard has gone, the grid's refit. */
const REFIT_MS = 150;

export function swipeLowers(
  scroller: HTMLElement,
  field: HTMLElement | null,
  /** The gates' seams, and the key panel's (UX-31): what is up and how
      it goes down; whether the view is at the bottom, and putting it
      back there. */
  seams: { up?: () => boolean; lower?: () => void; atRest?: () => boolean; toRest?: () => void } = {},
): () => void {
  if (!field) return () => {};
  const up = seams.up ?? (() => keyboardUp(field));
  const lower = seams.lower ?? (() => lowerKeyboard(field));
  /** This touch began at the bottom. */
  let rested = false;
  /** …and lowered the keyboard: the lift settles it back. */
  let settles = false;
  let waiting = 0;
  const settle = () => {
    const from = performance.now();
    const poll = () => {
      if (up() && performance.now() - from < SETTLE_MAX_MS) {
        waiting = window.setTimeout(poll, SETTLE_POLL_MS);
        return;
      }
      waiting = window.setTimeout(() => {
        waiting = 0;
        seams.toRest?.();
      }, REFIT_MS);
    };
    poll();
  };
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
    // A new touch is the person's again: no settle under it.
    window.clearTimeout(waiting);
    waiting = 0;
    settles = false;
    rested = one && (seams.atRest?.() ?? false);
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
    settles = rested;
    lower();
  };
  const end = () => {
    one = false;
    if (settles) settle();
    settles = false;
  };
  scroller.addEventListener("touchstart", start, { passive: true });
  scroller.addEventListener("touchmove", move, { passive: true });
  scroller.addEventListener("touchend", end, { passive: true });
  scroller.addEventListener("touchcancel", end, { passive: true });
  return () => {
    window.clearTimeout(waiting);
    scroller.removeEventListener("touchstart", start);
    scroller.removeEventListener("touchmove", move);
    scroller.removeEventListener("touchend", end);
    scroller.removeEventListener("touchcancel", end);
  };
}

/**
 * **The finger folds the deck, not the scroller** (item 50, the owner on
 * tmux + Claude Code, 2026-10-06: a drag down often did not fold). The
 * deck's rule (UX-13, `SessionBar`) reads the content scroller's `scroll`
 * events; over a program that holds the mouse the drag is the program's
 * wheel (item 36) and the scroller's travel says nothing about reading
 * away from rest. So the pane measures the finger: with the keyboard and
 * the key panel down when the touch began, a one-finger drag more down
 * than across past `FOLD_AT` folds the deck, one more up than across past
 * `UNFOLD_BY` unfolds it, and after either a turn is measured from the
 * finger's furthest point. The drag still goes to the program or the
 * buffer as before; the fold is a side effect, never a capture. A touch
 * that began with the keyboard up lowers it (`swipeLowers`) and folds
 * nothing — the deck folds on a later drag. The scroll rule stays beside
 * this one (reaching the bottom unfolds; a Mac's wheel folds a narrow
 * window); the two agree on direction.
 */
export function swipeFolds(
  scroller: HTMLElement,
  /** The gates' seams: whether the foot is covered, and the telling. */
  seams: { covered?: () => boolean; tell?: (fold: boolean) => void } = {},
): () => void {
  const covered = seams.covered ?? footCovered;
  const tell = seams.tell ?? ((fold: boolean) => tellDeck(scroller, fold));
  let x0 = 0;
  let y0 = 0;
  let t0 = 0;
  /** Where a turn is measured from: the start, then the furthest point
      in the direction last told. */
  let ax = 0;
  let ay = 0;
  let away = false;
  let said: boolean | null = null;
  let one = false;
  const start = (e: TouchEvent) => {
    one = e.touches.length === 1 && !covered() && !(e.target as Element | null)?.closest?.("[data-session-bar]");
    away = false;
    said = null;
    x0 = ax = e.touches[0]?.clientX ?? 0;
    y0 = ay = e.touches[0]?.clientY ?? 0;
    t0 = performance.now();
  };
  const move = (e: TouchEvent) => {
    if (e.touches.length !== 1) one = false;
    if (!one) return;
    const t = e.touches[0];
    if (!away) {
      if (Math.hypot(t.clientX - x0, t.clientY - y0) <= SLOP) return;
      // A press first is a selection (`pressSelects`): nothing folds.
      if (performance.now() - t0 >= PRESS_MS) {
        one = false;
        return;
      }
      away = true;
    }
    const dx = Math.abs(t.clientX - ax);
    const dy = t.clientY - ay;
    if (said !== true && dy > FOLD_AT && dy > dx) said = true;
    else if (said !== false && -dy > UNFOLD_BY && -dy > dx) said = false;
    else {
      if ((said === true && dy > 0) || (said === false && dy < 0)) {
        ax = t.clientX;
        ay = t.clientY;
      }
      return;
    }
    ax = t.clientX;
    ay = t.clientY;
    tell(said);
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
