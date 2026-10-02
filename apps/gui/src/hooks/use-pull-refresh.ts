// **Pull to refresh** (the redesign, 2026-10-02): a list under a finger,
// pulled down from its top, asks again. The gesture is the platform's —
// drag past a threshold, let go, a strip says it is working, then that
// it is done — and only under a finger (`pointer: coarse`): a mouse has
// no pull, and its list re-reads on its own beat anyway.
//
// What 「asks again」 does is the caller's (`onRefresh`): today the shell
// re-reads the rows and the machines. A sync round with the network would
// be the honest version of the gesture, and no face can start one yet —
// the node has `sync_now`, the bridge and the app do not expose it.
import { useEffect, useRef, useState } from "react";

import { useCoarse } from "@/hooks/use-coarse";

export type PullState = "idle" | "pulling" | "armed" | "syncing" | "done";

/** How far a finger has to pull, in CSS pixels of strip, to arm it. */
const ARM = 64;
/** The strip's own resting height while it works. */
export const PULL_STRIP = 46;
/** How long 「已同步」 stays before the strip folds away. */
const DONE_MS = 900;

export function usePullRefresh(
  scroller: HTMLElement | null,
  onRefresh: (() => Promise<unknown>) | undefined,
): { state: PullState; pull: number } {
  const coarse = useCoarse();
  const [state, setState] = useState<PullState>("idle");
  const [pull, setPull] = useState(0);
  const run = useRef(onRefresh);
  run.current = onRefresh;
  const busy = useRef(false);

  useEffect(() => {
    if (!scroller || !coarse || !run.current) return;
    let from: number | null = null;
    let now = 0;
    const start = (e: TouchEvent) => {
      if (busy.current || scroller.scrollTop > 0 || e.touches.length !== 1) return;
      from = e.touches[0].clientY;
      now = 0;
    };
    const move = (e: TouchEvent) => {
      if (from === null) return;
      const dy = e.touches[0].clientY - from;
      if (dy <= 0) {
        if (now > 0) {
          now = 0;
          setPull(0);
          setState("idle");
        }
        return;
      }
      // The list follows the finger at half its speed: the rubber band
      // that says this is a pull, not a scroll.
      e.preventDefault();
      now = dy / 2;
      setPull(now);
      setState(now >= ARM ? "armed" : "pulling");
    };
    const end = () => {
      if (from === null) return;
      from = null;
      const go = run.current;
      if (now < ARM || !go) {
        now = 0;
        setPull(0);
        setState("idle");
        return;
      }
      busy.current = true;
      setPull(PULL_STRIP);
      setState("syncing");
      void go()
        .catch(() => {})
        .finally(() => {
          setState("done");
          window.setTimeout(() => {
            busy.current = false;
            setPull(0);
            setState("idle");
          }, DONE_MS);
        });
    };
    scroller.addEventListener("touchstart", start, { passive: true });
    // Not passive: a pull holds the page still instead of scrolling it.
    scroller.addEventListener("touchmove", move, { passive: false });
    scroller.addEventListener("touchend", end);
    scroller.addEventListener("touchcancel", end);
    return () => {
      scroller.removeEventListener("touchstart", start);
      scroller.removeEventListener("touchmove", move);
      scroller.removeEventListener("touchend", end);
      scroller.removeEventListener("touchcancel", end);
    };
  }, [scroller, coarse]);

  return { state, pull };
}
