// What a page's head does when its content scrolls (`app.css`
// `[data-band]`): the round HEADS rule, 2026-10-03 (the owner:「各种上滑
// 但需要保持标题的…和 apple 一样，标题那块毛玻璃」).
//
// **A head is a band.** It stays where it is; the scroller under it is
// pulled up to the top of the band's parent, so its first rows start
// under the band and pass beneath it, and once the scroller is off its
// top the band wears the glass bar (the ground, translucent, blurred,
// with a hairline under it). At rest the band is the plain ground.
//
// A head opts in with `ref={band}` and `data-band`; the element right
// after it is the box that is pulled up (`under`). Which scroller is the
// band's is found, not declared: the scrollable box standing at the very
// top of `under`, reached through boxes that start there too. A screen
// whose top is not a scroller (a toolbar, a terminal) is not pulled up
// and its head simply stays plain — nothing passes under it.
// Transparent desktop windows keep the rows below the head instead: the
// shared CSS cancels both overlap and padding, retaining one tint plane.
//
// Scrollers with no band over them keep the old fade (`EDGED`, the audit
// fix plan F4.11): their top edge fades once scrolled.

/** Scrollers whose top edge fades once scrolled, where no band covers
    them (`app.css` `[data-scrolled]:not([data-band-scroll])`). */
export const EDGED = "[data-nav-clear], [data-pane-body], [data-files-list]";

/** How far a scroller goes before its band turns to glass: a few px, so
    a stray touch at the top does not flicker the bar. */
const EDGE = 2;

type Band = {
  head: HTMLElement;
  scroller: HTMLElement | null;
  /** Whether the scroller is off its top. */
  sync: () => void;
};

/** What a band never reaches into: a terminal's own scroller. */
const TERMINAL = "[data-terminal], .xterm";

const owner = new WeakMap<HTMLElement, Band>();

/** Every band on screen, by its `find`. */
const live = new Set<() => void>();

/**
 * Brings every band up to date now: its height, its scroller, the
 * scroller's padding, its scrolled state. For a caller that is about to
 * set a scroller's position in a layout effect (the per-place scroll
 * memory, `useKeptScroll`): a position set before the padding exists is
 * moved by it when it arrives, which is the list that came back shifted
 * by the band's height.
 */
export function settleBands(): void {
  for (const find of live) find();
}

function scrolls(el: HTMLElement): boolean {
  const y = getComputedStyle(el).overflowY;
  return y === "auto" || y === "scroll";
}

/**
 * The scroller standing at `under`'s top: `under` itself, or down
 * through the boxes that start where it starts, the first one that
 * scrolls. A box that is
 * a band's own parent belongs to that band, never to this one.
 */
function topScroller(under: HTMLElement): HTMLElement | null {
  const top = under.getBoundingClientRect().top;
  let at: HTMLElement | null = under;
  for (let depth = 0; at && depth < 12; depth++) {
    if (scrolls(at)) return at;
    let next: HTMLElement | null = null;
    for (const c of at.children) {
      if (!(c instanceof HTMLElement)) continue;
      const r = c.getBoundingClientRect();
      if (r.height === 0 || Math.abs(r.top - top) > 1) continue;
      // A box kept mounted but hidden is not what stands there: a
      // machine's listing stays under an open file (`MachineFiles`,
      // invisible), and the band padded that listing and pulled the
      // whole screen up, so the file's own name slid under the head
      // (2026-10-03, the owner's phone).
      if (getComputedStyle(c).visibility === "hidden") continue;
      // Another band's group: its scroller is that band's. A terminal
      // scrolls its own lines (xterm's viewport) and is never padded.
      if (c.querySelector(":scope > [data-band]") || c.matches(TERMINAL)) return null;
      next = c;
      break;
    }
    at = next;
  }
  return null;
}

/**
 * Makes `head` a band (a React 19 ref callback: the cleanup is the
 * return). Its height is written on its parent as `--band-h`, which the
 * pulled-up box and its scroller read.
 */
export function band(head: HTMLElement | null): (() => void) | undefined {
  if (!head) return;
  const parent = head.parentElement;
  if (!parent) return;
  const b: Band = {
    head,
    scroller: null,
    sync: () => {
      const on = !!b.scroller && b.scroller.scrollTop > EDGE;
      if (on !== head.hasAttribute("data-scrolled")) head.toggleAttribute("data-scrolled", on);
    },
  };
  // **The band's height is on its parent before the first paint**
  // (2026-10-03, the owner's phone: a page opened with its facts row under
  // the head and no ground, then settled). Read here, during the commit
  // that attaches the head, and again by every `find`; the observer only
  // follows later changes (a font, folded verbs).
  const measure = () => {
    const h = `${head.offsetHeight}px`;
    if (parent.style.getPropertyValue("--band-h") !== h) parent.style.setProperty("--band-h", h);
  };
  measure();
  const size = new ResizeObserver(measure);
  size.observe(head);

  const release = (el: HTMLElement) => {
    el.removeAttribute("data-band-scroll");
    el.style.removeProperty("--band-pad");
    if (owner.get(el) === b) owner.delete(el);
  };
  const find = () => {
    measure();
    const under = head.nextElementSibling;
    const found = under instanceof HTMLElement ? topScroller(under) : null;
    if (found !== b.scroller) {
      if (b.scroller) release(b.scroller);
      if (found) {
        // Its own top padding, read before the band's is added to it.
        found.style.setProperty("--band-pad", getComputedStyle(found).paddingTop);
        found.setAttribute("data-band-scroll", "");
        owner.set(found, b);
      }
      b.scroller = found;
      parent.toggleAttribute("data-band-over", !!found);
    }
    b.sync();
  };
  // Screens come and go under a head (a list's swap, a page opened):
  // look again **in the same frame**. A mutation observer answers in the
  // microtask after React's commit, before the browser paints, so the new
  // screen's scroller has its padding in its first frame. ⚠️ 墓碑: this
  // looked again 60 ms later, so every page opened and every list came
  // back for three or four frames without the band's padding — content
  // under the head on the way in, a list shifted up by the band's height
  // on the way back. Changes inside a terminal are none of a band's
  // business and are skipped (xterm rewrites its rows all the time).
  const tree = new MutationObserver((records) => {
    if (records.every((r) => r.target instanceof Element && r.target.closest(TERMINAL))) return;
    find();
  });
  tree.observe(parent, { childList: true, subtree: true });
  find();
  live.add(find);
  return () => {
    live.delete(find);
    size.disconnect();
    tree.disconnect();
    if (b.scroller) release(b.scroller);
    parent.removeAttribute("data-band-over");
    parent.style.removeProperty("--band-h");
    head.removeAttribute("data-scrolled");
  };
}

export function followScrollEdge(): void {
  document.addEventListener(
    "scroll",
    (e) => {
      const el = e.target;
      if (!(el instanceof HTMLElement)) return;
      owner.get(el)?.sync();
      // Every scroller under a head (V12): a page's, a list's body, a
      // machine's listing.
      if (!el.matches(EDGED)) return;
      const up = el.scrollTop > 0;
      if (up !== el.hasAttribute("data-scrolled")) el.toggleAttribute("data-scrolled", up);
    },
    true,
  );
}
