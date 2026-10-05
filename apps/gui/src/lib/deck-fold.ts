// **One deck rule for every content page** (UX-13, the owner 2026-10-05
// 00:40 and the addendum at 02:10). At rest the deck is the full row under
// the content; reading away from rest folds it into the 24-pt title strip
// at the screen's foot; coming back toward rest, reaching it, or a tap on
// the strip unfolds it. Nothing folds while the soft keyboard is up, and a
// keyboard rising unfolds a folded deck.
//
// "Rest" is where a page's newest thing is: the top of a web page or a
// file, the bottom of a terminal or a conversation. The distances are the
// browser's since UX-4 (`page:scroll`), mirrored for the pages that rest
// at the bottom.
//
// The terminal keeps its grid (the owner: 「Claude Code 不能重绘」): the
// height the full deck takes stays reserved while folded, all but whole
// terminal rows of it, which go to the terminal's own box as the band
// term's item 32 draws history into (`audit/term-tmux-history.md`, the
// item 32 handoff): `data-folded` and `--fold-band` on the pane's root
// `[data-kept-terminal]`, set in the same frame the deck shrinks, so the
// mount term keeps at the box's height less the band never changes size.

/** How far from rest (points) before the deck folds. */
export const FOLD_AT = 48;
/** How far back toward rest (points) brings it back. */
export const UNFOLD_BY = 24;

/** **A deck brought back by hand stays until the next gesture** (the
    simulator, a real drag then a tap on the strip): the flick's momentum
    goes on scrolling after the finger lifts, and every one of those moves
    read as reading on and folded the deck straight back. After an unfold
    that was asked for, folding waits for the scroll to stop for a beat
    (the momentum ended) — the next drag may fold it again. */
export class UnfoldHold {
  private held = false;
  private last = -Infinity;
  /** The strip was pressed. */
  asked() {
    this.held = true;
    this.last = performance.now();
  }
  /** A scroll arrived; whether it may fold the deck. */
  allows(): boolean {
    const now = performance.now();
    if (this.held && now - this.last > QUIET_MS) this.held = false;
    this.last = now;
    return !this.held;
  }
}

/** A gap in the scroll events this long is the end of a gesture. */
const QUIET_MS = 250;

/** Scrollers whose rest is their bottom: a terminal's, a conversation's. */
export const RESTS_AT_BOTTOM = "[data-term-scroller], [data-chat-scroll], [data-machine-chat-scroll]";

/** Where the reader turned back toward rest, between steps. */
export type FoldMemo = { turned: number | null };

/**
 * One step of the rule: `away` is the distance from rest, `leaving`
 * whether this move went further from it, `rest` the distance that still
 * counts as at rest. Answers the deck's new state, or null for no change.
 */
export function foldStep(memo: FoldMemo, away: number, leaving: boolean, rest: number): boolean | null {
  if (away <= rest) {
    memo.turned = null;
    return false;
  }
  if (leaving) {
    memo.turned = null;
    return away > FOLD_AT ? true : null;
  }
  // Back by a few lines before it comes back, so a finger settling at the
  // end of a flick away from rest does not undo it.
  memo.turned ??= away;
  return memo.turned - away >= UNFOLD_BY ? false : null;
}

/** Whole terminal rows (at most two) that fit in what the fold frees
    over the strip: two on a phone (60 − 24 against 16-px rows), one in a
    Mac window dragged narrow, where the deck is shorter. */
export function bandRows(freed: number, row: number): number {
  if (!(row > 0)) return 0;
  return Math.max(0, Math.min(2, Math.floor(freed / row)));
}
