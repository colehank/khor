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
// A terminal's grid resizes with the fold, as it does with the keyboard
// (item 44, 2026-10-06; the owner withdrew 10-05's 「网格不重排」): the
// whole height the deck frees is the content's, the terminal's box grows
// by it and refits, and the pty is told. The band of history the fold
// once uncovered (term's item 32) is gone with the reserve it needed.

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

/** The folded deck's ink, by what is behind it. */
export type DeckInk = "light" | "dark";

/**
 * **The capsule's text adapts to what is behind it** (UX-29, the owner's
 * Safari shots): white over a dark ground, black over a light one. The
 * ground as a CSS colour — `r,g,b` as the page's sampler says it, or
 * anything a canvas reads (`#rrggbb`, `rgb()`, a name) — to the ink that
 * reads on it, by the ground's relative luminance (sRGB, the WCAG
 * weights). Nothing known (`""`, or a colour nothing can read) is a white
 * page's ground: dark ink.
 */
export function inkOver(ground: string): DeckInk {
  const rgb = rgbOf(ground);
  if (!rgb) return "dark";
  const lin = (c: number) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  const l = 0.2126 * lin(rgb[0]) + 0.7152 * lin(rgb[1]) + 0.0722 * lin(rgb[2]);
  return l > 0.36 ? "dark" : "light";
}

let pen: CanvasRenderingContext2D | null = null;
function rgbOf(color: string): [number, number, number] | null {
  const bare = /^\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*$/.exec(color);
  if (bare) return [Number(bare[1]), Number(bare[2]), Number(bare[3])];
  if (!color.trim() || typeof document === "undefined") return null;
  pen ??= document.createElement("canvas").getContext("2d");
  if (!pen) return null;
  pen.fillStyle = "#000000";
  pen.fillStyle = "#010203";
  pen.fillStyle = color;
  const got = String(pen.fillStyle);
  // A colour the canvas could not read leaves the sentinel in place.
  if (got === "#010203" && color.trim() !== "#010203") return null;
  const hex = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(got);
  if (hex) return [parseInt(hex[1], 16), parseInt(hex[2], 16), parseInt(hex[3], 16)];
  const fn = /rgba?\(([\d.]+),\s*([\d.]+),\s*([\d.]+)/.exec(got);
  return fn ? [Number(fn[1]), Number(fn[2]), Number(fn[3])] : null;
}

/**
 * **What the app's own content paints under a point**: the first painted
 * background up from the elements at `(x, y)`, skipping anything inside
 * `skip` (the deck itself, over the content) — a terminal's theme, a
 * preview's ground, the app's ground. `""` where nothing paints.
 */
export function groundUnder(x: number, y: number, skip: Element | null): string {
  if (typeof document === "undefined") return "";
  for (const hit of document.elementsFromPoint(x, y)) {
    if (skip && skip.contains(hit)) continue;
    for (let e: Element | null = hit; e; e = e.parentElement) {
      const bg = getComputedStyle(e).backgroundColor;
      const m = /rgba?\(([\d.]+),\s*([\d.]+),\s*([\d.]+)(?:,\s*([\d.]+))?\)/.exec(bg);
      if (m && (m[4] === undefined || Number(m[4]) > 0.5)) return `${m[1]},${m[2]},${m[3]}`;
    }
  }
  return "";
}
