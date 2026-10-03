// **Two fingers to look closer at a picture** (round CHAT, 2026-10-03:
// 「images full-screen with pinch zoom on the phone」). The page itself
// does not zoom — `index.html`'s viewport holds it at 1, so a stray pinch
// never leaves the whole app enlarged — and so a picture that is worth
// reading closer needs its own.
//
// Two fingers scale it about its middle (1× to 6×); one finger moves it
// while it is enlarged; a double tap goes to 2.5× and back. At 1× a single
// finger is left alone, so whatever scrolls around the picture still does.
// A transform and nothing else: the picture's box never changes size, and
// nothing around it moves.
import { useEffect, useState } from "react";

const MAX = 6;
const TAP_SCALE = 2.5;
/** Two taps this close together are one double tap (iOS's own window). */
const DOUBLE_MS = 300;

/** Hands back the ref callback for the element to zoom. */
export function usePinch(): (el: HTMLElement | null) => void {
  const [el, setEl] = useState<HTMLElement | null>(null);
  useEffect(() => {
    if (el === null) return;
    let scale = 1;
    let x = 0;
    let y = 0;
    let pinch: { d: number; scale: number } | null = null;
    let pan: { px: number; py: number; x: number; y: number } | null = null;
    let moved = false;
    let lastTap = 0;
    const apply = () => {
      if (scale <= 1) {
        scale = 1;
        x = 0;
        y = 0;
      }
      el.style.transform = scale === 1 ? "" : `translate(${x}px, ${y}px) scale(${scale})`;
    };
    const gap = (t: TouchList) => Math.hypot(t[0].clientX - t[1].clientX, t[0].clientY - t[1].clientY);
    const down = (e: TouchEvent) => {
      moved = false;
      if (e.touches.length === 2) {
        pinch = { d: gap(e.touches) || 1, scale };
        pan = null;
        e.preventDefault();
      } else if (e.touches.length === 1 && scale > 1) {
        pan = { px: e.touches[0].clientX, py: e.touches[0].clientY, x, y };
        e.preventDefault();
      }
    };
    const move = (e: TouchEvent) => {
      if (pinch && e.touches.length === 2) {
        moved = true;
        scale = Math.min(MAX, Math.max(1, (pinch.scale * gap(e.touches)) / pinch.d));
        apply();
        e.preventDefault();
      } else if (pan && e.touches.length === 1) {
        moved = true;
        x = pan.x + e.touches[0].clientX - pan.px;
        y = pan.y + e.touches[0].clientY - pan.py;
        apply();
        e.preventDefault();
      }
    };
    const up = (e: TouchEvent) => {
      if (e.touches.length < 2) pinch = null;
      if (e.touches.length > 0) return;
      pan = null;
      if (moved || e.changedTouches.length !== 1) return;
      const now = Date.now();
      if (now - lastTap < DOUBLE_MS) {
        scale = scale > 1 ? 1 : TAP_SCALE;
        apply();
        lastTap = 0;
        e.preventDefault();
      } else {
        lastTap = now;
      }
    };
    el.addEventListener("touchstart", down, { passive: false });
    el.addEventListener("touchmove", move, { passive: false });
    el.addEventListener("touchend", up, { passive: false });
    return () => {
      el.removeEventListener("touchstart", down);
      el.removeEventListener("touchmove", move);
      el.removeEventListener("touchend", up);
      el.style.transform = "";
    };
  }, [el]);
  return setEl;
}
