// **Each place remembers how far down it was** (the audit fix plan F7.1;
// N11). A seat of the narrow bar, or a landing on the wide face, is a
// place a person leaves and comes back to; coming back to the top of a
// list they had scrolled is the web's tell, not an app's.
//
// The page's scrollers are recorded as they scroll (one capturing
// listener, so no component has to report), under the key of the place
// on screen; when the key comes back, the same scrollers — found by the
// same selector, in document order — are put back where they were. The
// content may still be arriving (a machine's readings, a list's rows),
// so the putting back is tried over a few frames until each scroller is
// tall enough to hold its old position.
import { useEffect, useLayoutEffect, useRef } from "react";

/** What counts as a place's scroller: the list body, a page's own
    scroller, a listing, and any board that scrolls. Terminals keep their
    own scrollback and are never touched. */
const SCROLLERS = "[data-pane-body], [data-nav-clear], [data-files-list], [data-region] .overflow-y-auto";

function scrollers(): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>(SCROLLERS)].filter(
    (e) => e.getClientRects().length > 0 && !e.closest(".xterm, [data-screen-under]"),
  );
}

export function useKeptScroll(key: string) {
  const kept = useRef(new Map<string, { top: number; height: number }[]>());
  const now = useRef(key);
  now.current = key;

  useEffect(() => {
    const on = (e: Event) => {
      const t = e.target;
      if (!(t instanceof HTMLElement) || !t.matches(SCROLLERS)) return;
      kept.current.set(
        now.current,
        scrollers().map((s) => ({ top: s.scrollTop, height: s.scrollHeight })),
      );
    };
    document.addEventListener("scroll", on, true);
    return () => document.removeEventListener("scroll", on, true);
  }, []);

  useLayoutEffect(() => {
    const want = kept.current.get(key);
    if (!want || want.every((w) => w.top === 0)) return;
    // **Put back once the content is back** (V06): a scroller set while
    // its page is still arriving is pushed again by the browser's scroll
    // anchoring as the rest lands above it (450 came back as 599). So
    // each waits until it is as tall as it was when it was left — or
    // until the wait runs out — with anchoring off meanwhile.
    let frames = 0;
    let raf = 0;
    const held = new Set<HTMLElement>();
    const put = () => {
      const got = scrollers();
      let done = got.length >= want.length;
      want.forEach((w, i) => {
        const s = got[i];
        if (!s) return;
        if (!held.has(s)) {
          held.add(s);
          s.style.overflowAnchor = "none";
        }
        const ready = s.scrollHeight >= w.height - 1 || frames > 45;
        if (ready && s.scrollHeight - s.clientHeight >= w.top) s.scrollTop = w.top;
        else done = false;
      });
      if (!done && ++frames < 60) raf = requestAnimationFrame(put);
      else for (const s of held) s.style.overflowAnchor = "";
    };
    put();
    return () => {
      cancelAnimationFrame(raf);
      for (const s of held) s.style.overflowAnchor = "";
    };
  }, [key]);

  /** Forget a place's scroll: its root was asked for again. */
  return (k: string) => kept.current.delete(k);
}
