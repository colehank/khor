// **Pull to refresh** (the redesign, 2026-10-02): a list under a finger,
// pulled down from its top, asks again. The gesture is the platform's —
// the pane follows the finger, past a threshold it is armed, let go and
// a spinner turns above the title while the pane holds just the
// spinner's room, then the pane settles back — and only under a finger
// (`pointer: coarse`): a mouse has no pull, and its list re-reads on its
// own beat anyway.
//
// **Only a spinner** (the audit fix plan F6.5, narrow P-H): no word on
// screen, as on iOS; the four moments are said to a screen reader
// (`ListPane`). ⚠️ 墓碑: a 46-tall strip that wrote 下拉同步 / 松开同步 /
// 正在同步 / 已同步 and pushed the list down by its height.
//
// What 「asks again」 does is the caller's (`onRefresh`): the shell runs a
// sync round with every machine (`syncNow`, the node's `sync_now`), then
// re-reads the rows and the machines.
import { useEffect, useRef, useState } from "react";

import { useCoarse } from "@/hooks/use-coarse";

export type PullState = "idle" | "pulling" | "armed" | "syncing" | "done";

/** How far a finger has to pull, in CSS pixels of strip, to arm it. */
const ARM = 64;
const SLOP = 8;
/** The room the pane holds open above itself while it works: the
    spinner's, and no more. */
export const PULL_HOLD = 36;
/** How long 「已同步」 stays said (to a screen reader) once the pane has
    settled. */
const DONE_MS = 600;

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
    let live = true, completion = 0;
    let from: {x: number; y: number} | null = null;
    let claimed = false;
    let now = 0;
    busy.current = false;
    setPull(0); setState("idle");
    const cancel = () => {
      if (!from) return;
      from = null; claimed = false; now = 0;
      setPull(0); setState("idle");
    };
    const start = (e: TouchEvent) => {
      cancel();
      from = null; claimed = false; now = 0;
      if (busy.current || scroller.scrollTop > 0 || e.touches.length !== 1) return;
      from = {x: e.touches[0].clientX, y: e.touches[0].clientY};
    };
    const move = (e: TouchEvent) => {
      if (from === null) return;
      if (e.touches.length !== 1 || !e.cancelable) {cancel(); return;}
      const dx = e.touches[0].clientX - from.x;
      const dy = e.touches[0].clientY - from.y;
      if (!claimed) {
        if (Math.max(Math.abs(dx), Math.abs(dy)) < SLOP) return;
        if (scroller.scrollTop > 0 || dy <= 0 || Math.abs(dx) >= dy) {
          cancel();
          return;
        }
        claimed = true;
      }
      // The list follows the finger at half its speed: the rubber band
      // that says this is a pull, not a scroll.
      e.preventDefault();
      now = Math.max(0, dy / 2);
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
      setPull(PULL_HOLD);
      setState("syncing");
      void Promise.resolve().then(go)
        .catch(() => {})
        .finally(() => {
          if (!live) return;
          setState("done");
          setPull(0);
          completion = window.setTimeout(() => {
            busy.current = false;
            setState("idle");
          }, DONE_MS);
        });
    };
    scroller.addEventListener("touchstart", start, { passive: true });
    // Not passive: a pull holds the page still instead of scrolling it.
    scroller.addEventListener("touchmove", move, { passive: false });
    scroller.addEventListener("touchend", end);
    scroller.addEventListener("touchcancel", cancel);
    return () => {
      live = false;
      busy.current = false;
      window.clearTimeout(completion);
      scroller.removeEventListener("touchstart", start);
      scroller.removeEventListener("touchmove", move);
      scroller.removeEventListener("touchend", end);
      scroller.removeEventListener("touchcancel", cancel);
    };
  }, [scroller, coarse]);

  return { state, pull };
}
