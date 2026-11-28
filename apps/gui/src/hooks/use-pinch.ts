// **Two fingers to look closer at a picture** (round CHAT, 2026-10-03:
// 「images full-screen with pinch zoom on the phone」). The page itself
// does not zoom — `index.html`'s viewport holds it at 1, so a stray pinch
// never leaves the whole app enlarged — and so a picture that is worth
// reading closer needs its own.
//
// Two fingers scale it about its middle (1× to 6×); one finger moves it
// while it is enlarged; a double tap goes to 2.5× and back. At 1× a single
// finger is left alone, so whatever scrolls around the picture still does.
// With a zoom control, a mouse can move the enlarged picture too. The
// picture's layout box never changes size, and nothing around it moves.
import { useEffect, useState, type RefObject } from "react";
import { pinchPoint } from "@/lib/preview-pinch";

const MAX = 6;
const TAP_SCALE = 2.5;
/** Two taps this close together are one double tap (iOS's own window). */
const DOUBLE_MS = 300;

/** Hands back the ref callback for the element to zoom. */
export function usePinch(zoom?: RefObject<((factor: number) => void) | null>): (el: HTMLElement | null) => void {
  const [el, setEl] = useState<HTMLElement | null>(null);
  useEffect(() => {
    if (el === null) return;
    const surface = zoom ? el.parentElement ?? el : el;
    let scale = 1;
    let x = 0;
    let y = 0;
    let pinch: { distance: number; scale: number; anchor?: { x: number; y: number } } | null = null;
    let pan: { px: number; py: number; x: number; y: number } | null = null;
    let drag: { id: number; x: number; y: number } | null = null;
    let moved = false;
    let lastTap = 0;
    const apply = () => {
      if (scale <= 1) {
        scale = 1;
        x = 0;
        y = 0;
      }
      if (zoom) {
        // Keep the original fitted frame covered as the enlarged image moves.
        const maxX = el.offsetWidth * (scale - 1) / 2;
        const maxY = el.offsetHeight * (scale - 1) / 2;
        x = Math.min(maxX, Math.max(-maxX, x));
        y = Math.min(maxY, Math.max(-maxY, y));
        if (scale === 1) delete el.dataset.imagePan;
        else el.dataset.imagePan = drag ? "dragging" : "ready";
      }
      el.style.transform = scale === 1 ? "" : `translate(${x}px, ${y}px) scale(${scale})`;
    };
    if (zoom) zoom.current = factor => {
      scale = Math.min(MAX, Math.max(1, scale * factor));
      apply();
    };
    const startDrag = (event: PointerEvent) => {
      if (event.pointerType !== "mouse" || event.button !== 0 || scale === 1) return;
      event.preventDefault();
      drag = { id: event.pointerId, x: event.clientX, y: event.clientY };
      el.setPointerCapture(event.pointerId);
      el.dataset.imagePan = "dragging";
    };
    const moveDrag = (event: PointerEvent) => {
      if (drag?.id !== event.pointerId) return;
      x += event.clientX - drag.x;
      y += event.clientY - drag.y;
      drag.x = event.clientX;
      drag.y = event.clientY;
      apply();
    };
    const endDrag = (event: PointerEvent) => {
      if (drag?.id !== event.pointerId) return;
      drag = null;
      apply();
    };
    const preventNativeDrag = (event: DragEvent) => {
      if (scale > 1) event.preventDefault();
    };
    const down = (e: TouchEvent) => {
      if (e.target instanceof Element && e.target.closest("[data-preview-zoom]")) return;
      moved = false;
      if (e.touches.length === 2) {
        if (zoom && ![...e.touches].every(touch => touch.target instanceof Node && surface.contains(touch.target))) return;
        const point = pinchPoint(e.touches);
        pinch = { distance: point.distance, scale };
        if (zoom) {
          const box = el.getBoundingClientRect();
          pinch.anchor = { x: (point.x - box.left - box.width / 2) / scale,
            y: (point.y - box.top - box.height / 2) / scale };
        }
        lastTap = 0;
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
        const point = pinchPoint(e.touches);
        const next = Math.min(MAX, Math.max(1, pinch.scale * point.distance / pinch.distance));
        if (pinch.anchor) {
          const box = el.getBoundingClientRect();
          x += point.x - box.left - box.width / 2 - pinch.anchor.x * next;
          y += point.y - box.top - box.height / 2 - pinch.anchor.y * next;
        }
        scale = next;
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
      if (zoom && pinch && scale > 1 && e.touches.length === 1) {
        pan = { px: e.touches[0].clientX, py: e.touches[0].clientY, x, y };
      }
      if (e.touches.length < 2) pinch = null;
      if (e.touches.length > 0) return;
      pan = null;
      if (moved || e.changedTouches.length !== 1) return;
      if (zoom && !(e.target instanceof Node && el.contains(e.target))) return;
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
    const cancel = () => { pinch = pan = null; lastTap = 0; };
    surface.addEventListener("touchstart", down, { passive: false });
    surface.addEventListener("touchmove", move, { passive: false });
    surface.addEventListener("touchend", up, { passive: false });
    surface.addEventListener("touchcancel", cancel);
    const resize = zoom ? new ResizeObserver(apply) : null;
    if (resize) {
      resize.observe(el);
      el.addEventListener("pointerdown", startDrag);
      el.addEventListener("pointermove", moveDrag);
      el.addEventListener("pointerup", endDrag);
      el.addEventListener("pointercancel", endDrag);
      el.addEventListener("lostpointercapture", endDrag);
      el.addEventListener("dragstart", preventNativeDrag);
    }
    return () => {
      if (zoom) {
        zoom.current = null;
        resize?.disconnect();
        el.removeEventListener("pointerdown", startDrag);
        el.removeEventListener("pointermove", moveDrag);
        el.removeEventListener("pointerup", endDrag);
        el.removeEventListener("pointercancel", endDrag);
        el.removeEventListener("lostpointercapture", endDrag);
        el.removeEventListener("dragstart", preventNativeDrag);
        delete el.dataset.imagePan;
      }
      surface.removeEventListener("touchstart", down);
      surface.removeEventListener("touchmove", move);
      surface.removeEventListener("touchend", up);
      surface.removeEventListener("touchcancel", cancel);
      el.style.transform = "";
    };
  }, [el, zoom]);
  return setEl;
}
