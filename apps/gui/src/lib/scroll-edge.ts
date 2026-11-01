// The band owns its scroller and the fading content beneath its title.
// Window colour belongs to the window surface, never to the band.
// Explicit sticky groups keep their headings separate from their bodies.

/** Scrollers whose top edge fades once scrolled, where no band covers
    them (`app.css` `[data-scrolled]:not([data-band-scroll])`). */
export const EDGED = "[data-nav-clear], [data-pane-body], [data-files-list]";

/** How far a scroller goes before its band turns to glass: a few px, so
    a stray touch at the top does not flicker the bar. */
const EDGE = 2;

type Band = {
  head: HTMLElement | null;
  scroller: HTMLElement | null;
  /** Whether the scroller is off its top. */
  sync: () => void;
};

/** What a band never reaches into: a terminal's own scroller. */
const TERMINAL = "[data-terminal], .xterm";

const owner = new WeakMap<HTMLElement, Band>();
const fadeOwner = new WeakMap<HTMLElement, Band>();

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
 * pulled-up box and its scroller read. A visually reordered toolbar can
 * explicitly locate its DOM body; native content supplies no body.
 */
export function band(head: HTMLElement | null, locateUnder?: () => HTMLElement | null): (() => void) | undefined {
  if (!head?.parentElement) return;
  let under: HTMLElement | null = null;
  return watchBand(head, head.parentElement, () => {
    const candidate = locateUnder ? locateUnder() : head.nextElementSibling;
    under = candidate instanceof HTMLElement ? candidate : null;
    return under instanceof HTMLElement ? topScroller(under) : null;
  }, locateUnder ? () => under : undefined);
}

/** A standalone scroll port with sticky groups but no fixed outer title.
 * Existing owners win; a later registration never takes their port. */
export function stickyGroups(scroller: HTMLElement | null): (() => void) | undefined {
  if (!scroller || owner.has(scroller)) return;
  return watchBand(null, scroller, () => scroller);
}

function watchBand(head: HTMLElement | null, parent: HTMLElement, locate: () => HTMLElement | null, locateOverlap?: () => HTMLElement | null): () => void {
  const b: Band = {
    head,
    scroller: null,
    sync: () => { if (!frame) frame = requestAnimationFrame(paint); },
  };
  let frame = 0, stylesDirty = true, portPadding = 0;
  let overlap: HTMLElement | null = null;
  const setOverlap = (next: HTMLElement | null) => {
    if (next === overlap) return;
    overlap?.removeAttribute('data-band-under');
    overlap = next;
    overlap?.setAttribute('data-band-under', '');
  };
  const stickyTops = new Map<HTMLElement,number>();
  let targets: { body: HTMLElement; heading: boolean; groupHead: HTMLElement | null }[] = [];
  const clearFades = () => {
    for (const {body} of targets) {
      size.unobserve(body);
      if (fadeOwner.get(body) !== b) continue;
      body.removeAttribute('data-sticky-fading');
      body.removeAttribute('data-sticky-held');
      body.style.removeProperty('--sticky-fade-start');
      body.style.removeProperty('--sticky-fade-length');
      fadeOwner.delete(body);
    }
    targets = [];
    stickyTops.clear();
    stylesDirty = true;
  };
  function paint() {
    frame = 0;
    const on = !!b.scroller && b.scroller.scrollTop > EDGE;
    if (stylesDirty) {
      stylesDirty = false;
      portPadding = b.scroller ? parseFloat(getComputedStyle(b.scroller).paddingTop) || 0 : 0;
      for (const {body,heading,groupHead} of targets) {
        for (const header of [heading ? body : null,groupHead])
          if (header && !stickyTops.has(header)) stickyTops.set(header,parseFloat(getComputedStyle(header).top) || 0);
      }
    }
    const edge = b.scroller?.getBoundingClientRect().top ?? 0;
    const stickyEdge = (header:HTMLElement) => edge+(b.scroller?.clientTop ?? 0)+portPadding+(stickyTops.get(header) ?? 0);
    const main = head?.getBoundingClientRect() ?? {top:edge,bottom:edge};
    const tail = parseFloat(getComputedStyle(parent!).getPropertyValue('--gap-pane')) || 0;
    const values = targets.map(({body,heading,groupHead}) => {
      const rect = body.getBoundingClientRect(), top = rect.top;
      const group = !heading && groupHead?.getBoundingClientRect();
      const boundary = groupHead ? stickyEdge(groupHead) : main.bottom;
      const held = group && group.top <= boundary + 1 && group.bottom > boundary;
      const flush = held && body.dataset.stickyBody === 'flush';
      // Process-column controls centre one line inside the touch target.
      // Keep direct row glyphs out of that line; the lower inset still fades
      // naturally to full body opacity at the same heading bottom.
      const line = flush && groupHead?.hasAttribute('data-top-heads') ? parseFloat(getComputedStyle(groupHead).lineHeight) : 0;
      const labelClear = line > 0 && group ? Math.min(group.height, (group.height + line) / 2) : 0;
      const start = (held ? group.top + labelClear : main.top) - top;
      const end = (held ? group.bottom : main.bottom) + (heading || flush ? 0 : tail) - top;
      return {body,start,length:end-start,active:on && end>0,material:heading && on && rect.top<=stickyEdge(body)+1 && rect.bottom>stickyEdge(body)};
    });
    head?.toggleAttribute('data-scrolled',on);
    for (const {body,start,length,active,material} of values) {
      if (fadeOwner.get(body) !== b) continue;
      body.toggleAttribute('data-sticky-fading',active);
      body.toggleAttribute('data-sticky-held',material);
      if (!active) continue;
      body.style.setProperty('--sticky-fade-start',`${start}px`);
      body.style.setProperty('--sticky-fade-length',`${length}px`);
    }
  }
  const discoverFades = () => {
    clearFades();
    const scroller = b.scroller;
    if (!scroller) return;
    const marker = '[data-sticky-head], [data-sticky-body]';
    const branches = marker+', [data-band]';
    const bodies:HTMLElement[] = [];
    const visit = (body:HTMLElement) => {
      // An inner primary band keeps its own content owner.
      if (body.matches('[data-band]') || body.querySelector(':scope > [data-band]')) return;
      if (body.matches(marker) || !body.querySelector(branches)) { bodies.push(body);return; }
      for (const child of body.children) if (child instanceof HTMLElement) visit(child);
    };
    if (scroller.querySelector(branches)) {
      for (const child of scroller.children) if (child instanceof HTMLElement) visit(child);
    } else if (head) bodies.push(scroller);
    targets = bodies.map(body => ({
      body,heading:body.hasAttribute('data-sticky-head'),
      groupHead:body.closest('[data-sticky-group]')?.querySelector<HTMLElement>(':scope > [data-sticky-head]') ?? null,
    }));
    for (const {body} of targets) { fadeOwner.set(body,b);size.observe(body); }
  };
  // **The band's height is on its parent before the first paint**
  // (2026-10-03, the owner's phone: a page opened with its facts row under
  // the head and no ground, then settled). Read here, during the commit
  // that attaches the head, and again by every `find`; the observer only
  // follows later changes (a font, folded verbs).
  const measure = () => {
    if (!head) return;
    const h = `${head.offsetHeight}px`;
    if (parent.style.getPropertyValue("--band-h") !== h) parent.style.setProperty("--band-h", h);
  };
  measure();
  const size = new ResizeObserver(() => { measure();stylesDirty=true;stickyTops.clear();b.sync(); });
  if (head) size.observe(head);

  const release = (el: HTMLElement) => {
    setOverlap(null);
    clearFades();
    size.unobserve(el);
    el.removeAttribute(head ? 'data-band-scroll' : 'data-sticky-scroll');
    if (head) el.style.removeProperty("--band-pad");
    if (owner.get(el) === b) owner.delete(el);
  };
  const find = () => {
    measure();
    let found = locate();
    if (found && owner.has(found) && owner.get(found) !== b) found = null;
    if (found !== b.scroller) {
      if (b.scroller) release(b.scroller);
      if (found) {
        // Its own top padding, read before the band's is added to it.
        if (head) found.style.setProperty("--band-pad", getComputedStyle(found).paddingTop);
        found.setAttribute(head ? 'data-band-scroll' : 'data-sticky-scroll', "");
        size.observe(found);
        owner.set(found, b);
      }
      b.scroller = found;
      if (head) parent.toggleAttribute("data-band-over", !!found);
    }
    setOverlap(found && locateOverlap ? locateOverlap() : null);
    discoverFades();
    cancelAnimationFrame(frame);
    paint();
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
    cancelAnimationFrame(frame);
    frame = 0;
    clearFades();
    size.disconnect();
    tree.disconnect();
    if (b.scroller) release(b.scroller);
    setOverlap(null);
    if (head) {
      parent.removeAttribute("data-band-over");
      parent.style.removeProperty("--band-h");
      head.removeAttribute("data-scrolled");
    }
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
