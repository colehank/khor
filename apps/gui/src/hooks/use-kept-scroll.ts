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
    (e) => e.getClientRects().length > 0 && !e.closest(".xterm"),
  );
}

export function useKeptScroll(key: string) {
  const kept = useRef(new Map<string, number[]>());
  const now = useRef(key);
  now.current = key;

  useEffect(() => {
    const on = (e: Event) => {
      const t = e.target;
      if (!(t instanceof HTMLElement) || !t.matches(SCROLLERS)) return;
      kept.current.set(
        now.current,
        scrollers().map((s) => s.scrollTop),
      );
    };
    document.addEventListener("scroll", on, true);
    return () => document.removeEventListener("scroll", on, true);
  }, []);

  useLayoutEffect(() => {
    const want = kept.current.get(key);
    if (!want || want.every((t) => t === 0)) return;
    let frames = 0;
    let raf = 0;
    const put = () => {
      const got = scrollers();
      let done = got.length >= want.length;
      want.forEach((top, i) => {
        const s = got[i];
        if (!s) return;
        if (s.scrollHeight - s.clientHeight >= top) s.scrollTop = top;
        else done = false;
      });
      if (!done && ++frames < 30) raf = requestAnimationFrame(put);
    };
    put();
    return () => cancelAnimationFrame(raf);
  }, [key]);

  /** Forget a place's scroll: its root was asked for again. */
  return (k: string) => kept.current.delete(k);
}
