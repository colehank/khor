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

/** How long after the lift the deck is read again for the touch's line
    (item 61): the bar has taken what it was told by then. */
const READ_AGAIN_MS = 300;
/** The end speed is the finger's over this much of the touch's end. */
const END_MS = 50;
/** **A flick's end speed**, px/ms (item 61 b): a drag that lifts at
    least this fast folds or unfolds the deck however short it was, since
    the content goes on without the finger. Measured on the simulator
    (deck-61.md): drags lifting at 0.22–1.1 px/ms were carried on 120–450
    px by iOS's momentum, all but two of fifteen; under 0.2 about half
    were, and none faster than 0.1. A real flick lifts faster still. */
export const FLICK = 0.2;

/** One point of the finger's path. */
type Mark = { t: number; x: number; y: number };

/** The finger's speed down the screen over its last `END_MS`, px/ms
    (negative up): from the newest mark at least that old to the lift. A
    finger held still before the lift ends at 0. */
export function endSpeed(path: Mark[]): number {
  const last = path[path.length - 1];
  if (!last || path.length < 2) return 0;
  let from = path[0];
  for (const m of path) {
    if (last.t - m.t >= END_MS) from = m;
    else break;
  }
  const dt = last.t - from.t;
  return dt > 0 ? (last.y - from.y) / dt : 0;
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
 * window); the two agree on direction. A flick short of either
 * distance tells the same at its lift when it ends at `FLICK` or faster
 * (item 61 b): the content goes on without the finger.
 *
 * Each touch that left the dead zone is read for the doctor's terminal
 * row (item 61 a,
 * `judged`): which way and how far, how fast at the end, what it told
 * the deck or which rule kept it out, and the deck before and after.
 */
export function swipeFolds(
  scroller: HTMLElement,
  /** The gates' seams: whether the foot is covered, and the telling;
      whether the deck is folded, and where each touch's reading goes
      (item 61). */
  seams: {
    covered?: () => boolean;
    tell?: (fold: boolean) => void;
    folded?: () => boolean;
    judged?: (line: string) => void;
  } = {},
): () => void {
  const covered = seams.covered ?? footCovered;
  const tell = seams.tell ?? ((fold: boolean) => tellDeck(scroller, fold));
  const folded = seams.folded ?? (() => scroller.closest("[data-deck-folded]") !== null);
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
  /** **What each touch did, for the doctor's terminal row** (item 61 a:
      the owner's flicks did not always fold, and nothing said why). The
      first rule that took the touch out of judging, what it told, the
      finger's path and the deck before and after. Only a touch that
      left the dead zone is written: a tap is not a swipe. */
  let why = "";
  let told: boolean[] = [];
  let path: Mark[] = [];
  let deckWas = false;
  let left = false;
  const mark = (list: TouchList) => {
    const t = list[0];
    if (t) path.push({ t: performance.now(), x: t.clientX, y: t.clientY });
  };
  const start = (e: TouchEvent) => {
    // A second finger joins the touch already under way.
    if (e.touches.length > 1 && path.length) {
      one = false;
      why ||= "two fingers";
      return;
    }
    const bar = (e.target as Element | null)?.closest?.("[data-session-bar]");
    const keys = covered();
    one = e.touches.length === 1 && !keys && !bar;
    why = e.touches.length !== 1 ? "two fingers" : bar ? "from the bar" : keys ? "keyboard or key panel up" : "";
    away = false;
    left = false;
    said = null;
    told = [];
    path = [];
    deckWas = folded();
    x0 = ax = e.touches[0]?.clientX ?? 0;
    y0 = ay = e.touches[0]?.clientY ?? 0;
    t0 = performance.now();
    mark(e.touches);
  };
  const move = (e: TouchEvent) => {
    if (!path.length) return;
    mark(e.touches);
    const t = e.touches[0];
    if (t && Math.hypot(t.clientX - x0, t.clientY - y0) > SLOP) left = true;
    if (e.touches.length !== 1) {
      one = false;
      why ||= "two fingers";
    }
    if (!one) return;
    if (!away) {
      if (Math.hypot(t.clientX - x0, t.clientY - y0) <= SLOP) return;
      // A press first is a selection (`pressSelects`): nothing folds.
      if (performance.now() - t0 >= PRESS_MS) {
        one = false;
        why = `press first, ${Math.round(performance.now() - t0)} ms`;
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
    told.push(said);
    tell(said);
  };
  const end = (e: TouchEvent) => {
    if (e.touches.length > 0 || !path.length) return;
    if (e.type === "touchend") mark(e.changedTouches);
    const went = path;
    path = [];
    const last = went[went.length - 1];
    const speed = endSpeed(went);
    // **A flick folds** (item 61 b, the owner: not every swipe folded, a
    // held one did): a quick flick moves the finger less than
    // `FOLD_AT` while the content travels far on momentum, which only a
    // slow, held drag covered. At the lift, a drag past the dead zone
    // that went the fold way (or back) and ends at `FLICK` or faster
    // tells the deck what the distance would have; the same rules keep a
    // press, the keyboard, the bar and a second finger out.
    if (one && away) {
      const dx = Math.abs(last.x - ax);
      const dy = last.y - ay;
      const flick = said !== true && dy > dx && speed >= FLICK ? true : said !== false && -dy > dx && -speed >= FLICK ? false : null;
      if (flick !== null) {
        said = flick;
        told.push(flick);
        tell(flick);
      }
    }
    one = false;
    if (!left || !seams.judged) return;
    const first = went[0];
    const dy = last.y - first.y;
    const dx = Math.abs(last.x - first.x);
    // Judged to the end and told nothing: what the travel and the lift
    // lacked.
    const lift = `, end under ${FLICK}`;
    const short =
      dy > dx ? `down ${Math.round(dy)} of ${FOLD_AT}${lift}` : -dy > dx ? `up ${Math.round(-dy)} of ${UNFOLD_BY}${lift}` : "more across than along";
    const verdict = told.length ? `told ${told.map((f) => (f ? "fold" : "unfold")).join(", ")}` : `nothing: ${why || short}`;
    const line =
      `swipe ${dy >= 0 ? "down" : "up"} ${Math.round(Math.abs(dy))} px, across ${Math.round(dx)}, ` +
      `in ${Math.round(last.t - first.t)} ms, end ${speed.toFixed(2)} px/ms, ` +
      `${e.type === "touchcancel" ? "cancelled" : "lifted"}; ${verdict}`;
    const deck = (f: boolean) => (f ? "folded" : "open");
    const was = deckWas;
    window.setTimeout(() => seams.judged?.(`${line}; deck ${deck(was)} → ${deck(folded())}`), READ_AGAIN_MS);
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
