// **Back is a swipe from the left edge** (the audit fix plan F7.3; narrow
// P-A). On a phone the page follows the finger over the one it came
// from, and lets go into it or springs back by how far and how fast it
// was thrown — iOS's navigation, on every narrow screen that has a back.
//
// **Only from the edge.** A touch that starts more than `EDGE` in from
// the screen's left side is the content's: a terminal's selection, a tab
// row's scroll, a table's sideways pan. And the direction is decided
// before anything moves — a touch that goes down first is a scroll, and
// this lets it be.
//
// The page is moved by a variable on the host (`--swipe`, `app.css`
// `[data-swipe-host]`), never by React state: a finger is sixty moves a
// second, and the screen under it must not re-render for any of them.
import { useEffect, useRef } from "react";
import { flushSync } from "react-dom";

/** How far in from the screen's edge a back swipe may start. */
const EDGE = 20;
/** How far the finger must move before the direction is decided. */
const SLOP = 8;
/** A throw this fast (px/ms) completes whatever the distance. */
const FLING = 0.5;

export function useEdgeSwipe(host: HTMLElement | null, back: (() => void) | null) {
  const run = useRef(back);
  run.current = back;

  useEffect(() => {
    if (!host) return;
    let start: { x: number; y: number; t: number } | null = null;
    let decided: "swipe" | "scroll" | null = null;
    let dx = 0;
    let lastX = 0;
    let lastT = 0;
    let speed = 0;
    // On the root as well as the host: the page's head row is drawn in
    // the shell's first row (`SideHead`), outside the host, and it is
    // part of the page that is being pulled away.
    const root = document.documentElement;
    const set = (px: number) => {
      host.style.setProperty("--swipe", `${px}px`);
      root.style.setProperty("--swipe", `${px}px`);
    };
    const mark = (name: string, on: boolean) => {
      for (const el of [host, root]) {
        if (on) el.setAttribute(name, "");
        else el.removeAttribute(name);
      }
    };
    const clear = () => {
      mark("data-swiping", false);
      mark("data-settling", false);
      host.style.removeProperty("--swipe");
      root.style.removeProperty("--swipe");
    };
    const down = (e: TouchEvent) => {
      if (!run.current || e.touches.length !== 1) return;
      const t = e.touches[0];
      const left = host.getBoundingClientRect().left;
      if (t.clientX - left > EDGE) return;
      start = { x: t.clientX, y: t.clientY, t: e.timeStamp };
      decided = null;
      dx = 0;
      lastX = t.clientX;
      lastT = e.timeStamp;
      speed = 0;
    };
    const move = (e: TouchEvent) => {
      if (!start) return;
      const t = e.touches[0];
      const mx = t.clientX - start.x;
      const my = t.clientY - start.y;
      if (decided === null) {
        if (Math.abs(mx) < SLOP && Math.abs(my) < SLOP) return;
        decided = mx > 0 && Math.abs(mx) > Math.abs(my) ? "swipe" : "scroll";
        if (decided === "scroll") {
          start = null;
          return;
        }
        mark("data-swiping", true);
      }
      // The content under the finger never sees a swipe's moves.
      e.preventDefault();
      e.stopPropagation();
      dx = Math.max(0, mx);
      const dt = e.timeStamp - lastT;
      if (dt > 0) speed = (t.clientX - lastX) / dt;
      lastX = t.clientX;
      lastT = e.timeStamp;
      set(dx);
    };
    const up = () => {
      if (!start || decided !== "swipe") {
        start = null;
        return;
      }
      start = null;
      const width = host.getBoundingClientRect().width;
      const go = dx > width * 0.35 || speed > FLING;
      mark("data-settling", true);
      set(go ? width : 0);
      const done = () => {
        if (go) {
          // The page under is already where it will be: the screen that
          // arrives must not play its own entrance over it.
          document.documentElement.setAttribute("data-swiped", "");
          // **Committed before the page lets go of the finger's offset**
          // (2026-10-03, the owner on the phone: every swipe back showed
          // the page it left once more after arriving). React renders a
          // state change from a timer later, on its own beat; `clear()`
          // below drops `--swipe` at once — so for a frame or more the
          // page that was swiped away stood back at 0 over the list it
          // had uncovered. Flushed, the destination is in the DOM first,
          // and the page that was left is never painted again.
          const back = run.current;
          if (back) flushSync(back);
          // …and the screen that arrived never plays its entrance: the
          // root's `data-swiped` comes off two frames later, and an
          // animation switched back on starts from its first frame — the
          // list slid in 16px after it had already arrived. Marked on the
          // element itself, which the next navigation replaces.
          for (const el of host.querySelectorAll(":scope > [data-screen]")) el.setAttribute("data-arrived", "");
          requestAnimationFrame(() =>
            requestAnimationFrame(() => document.documentElement.removeAttribute("data-swiped")),
          );
        }
        clear();
      };
      const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
      if (reduce) done();
      else window.setTimeout(done, 240);
    };
    host.addEventListener("touchstart", down, { capture: true, passive: true });
    host.addEventListener("touchmove", move, { capture: true, passive: false });
    host.addEventListener("touchend", up, { capture: true });
    host.addEventListener("touchcancel", up, { capture: true });
    return () => {
      host.removeEventListener("touchstart", down, { capture: true });
      host.removeEventListener("touchmove", move, { capture: true });
      host.removeEventListener("touchend", up, { capture: true });
      host.removeEventListener("touchcancel", up, { capture: true });
      clear();
    };
  }, [host]);
}
