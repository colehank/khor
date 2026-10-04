// **A held key repeats, the way a hardware keyboard's does** (item 34; the
// owner: 「长按删除是不是也得支持持续删的操作」). The numbers are the cursor
// stick's (`views/CursorPad`), which repeats its arrows the same way; both
// read them from here so the two never drift apart.
//
// macOS's own key repeat is the reference: about 400 ms before the first
// repeat at the default delay, then up to ~30 a second. Here the rate stays
// at 6 a second for the first second and 12 after it, because every key is
// a write across the network to a far terminal.
import { useEffect, useRef, type MouseEvent, type PointerEvent } from "react";

import { inApp, joyHaptic } from "@/api";
import { termJoyHaptic } from "@/lib/term-joy-haptic";

/** Held this long, a key (or the stick's arrow) repeats (ms). */
export const PAD_HOLD_MS = 400;
/** The repeat's first rate, keys per second… */
export const PAD_RATE = 6;
/** …its rate once it has run `PAD_RAMP_MS`. */
export const PAD_RATE_FAST = 12;
/** How long the repeat runs at its first rate (ms). */
export const PAD_RAMP_MS = 1000;
/** With no pointer event this long and no finger on the glass, a repeat
    is over whatever else was or was not heard (ms). */
export const PAD_WATCHDOG_MS = 600;
/** A finger that travels this far before the hold is scrolling the row,
    not holding a key (px). */
const KEY_SLOP = 8;

/**
 * **A repeat, felt** (item 23): a selection tick for each key a repeat
 * sends, a light impact when the stick takes a way. Prepared at touch-down,
 * let go at the lift with nothing felt. Only in the app (the bridge is
 * UIKit's), only with 「摇杆震动」 on (外观 → 终端).
 */
export function felt(kind: "prepare" | "tick" | "turn" | "end") {
  if (!inApp || !termJoyHaptic()) return;
  joyHaptic(kind).catch(() => {});
}

/** Fingers on the glass, counted from the document's own touch events,
    and a mouse button held: the watchdog's word on whether anybody is
    still holding a key. */
let touching = 0;
let buttons = 0;
if (typeof document !== "undefined") {
  const count = (e: TouchEvent) => {
    touching = e.touches.length;
  };
  document.addEventListener("touchstart", count, { capture: true, passive: true });
  document.addEventListener("touchend", count, { capture: true, passive: true });
  document.addEventListener("touchcancel", count, { capture: true, passive: true });
  const held = (e: globalThis.PointerEvent) => {
    if (e.pointerType === "mouse") buttons = e.buttons;
  };
  document.addEventListener("pointerdown", held, true);
  document.addEventListener("pointerup", held, true);
}

/** Whether nothing has been heard of a gesture for the watchdog's time
    and nobody is touching or pressing: its repeat must stop. */
export function letGo(heard: number): boolean {
  return performance.now() - heard > PAD_WATCHDOG_MS && touching === 0 && buttons === 0;
}

/** The next beat's delay for a repeat that began at `from`. */
export function beatMs(from: number): number {
  return 1000 / (performance.now() - from < PAD_RAMP_MS ? PAD_RATE : PAD_RATE_FAST);
}

/**
 * **A cap that repeats while held.** A tap sends once, at the click, as
 * every cap does — so a finger that scrolls the row sideways sends
 * nothing. Held still for `PAD_HOLD_MS`, the key goes out and then
 * repeats at `PAD_RATE`, `PAD_RATE_FAST` after `PAD_RAMP_MS`, with a tick
 * each time; the click that ends a hold sends nothing more. `send` is told
 * whether this is the gesture's first key, so the latches it went out
 * under can hold for the whole repeat, as the stick's do.
 *
 * The repeat never outlives the finger: it stops on the lift, a cancel,
 * lost capture, travel past the slop before it began, the window losing
 * focus, the page hidden, the cap leaving the screen, and on the
 * watchdog (`letGo`) for anything none of those catches. `repeats` false
 * (Esc, the latches, paste) gives the plain click.
 */
export function useHeldKey(send: (first: boolean) => void, repeats: boolean) {
  const hold = useRef<{ pointer: number; x: number; y: number; heard: number; timer: number; from: number; fired: boolean; sent: number } | null>(null);
  const sendRef = useRef(send);
  sendRef.current = send;
  /** A hold just ended by repeating: its click is not a tap. */
  const swallow = useRef(false);
  const stop = () => {
    const h = hold.current;
    if (!h) return;
    window.clearTimeout(h.timer);
    if (h.fired) felt("end");
    hold.current = null;
  };
  const stopRef = useRef(stop);
  stopRef.current = stop;
  useEffect(() => {
    const over = () => stopRef.current();
    const hidden = () => {
      if (document.visibilityState !== "visible") stopRef.current();
    };
    window.addEventListener("blur", over);
    document.addEventListener("visibilitychange", hidden);
    return () => {
      window.removeEventListener("blur", over);
      document.removeEventListener("visibilitychange", hidden);
      stopRef.current();
    };
  }, []);
  const beat = (h: NonNullable<typeof hold.current>) => {
    if (hold.current !== h) return;
    if (letGo(h.heard)) return stop();
    felt("tick");
    sendRef.current(h.sent === 0);
    h.sent += 1;
    h.timer = window.setTimeout(() => beat(h), beatMs(h.from));
  };
  if (!repeats) return { onClick: () => send(true) };
  return {
    onPointerDown: (e: PointerEvent<HTMLElement>) => {
      stop();
      swallow.current = false;
      felt("prepare");
      const cap = e.currentTarget;
      const h = { pointer: e.pointerId, x: e.clientX, y: e.clientY, heard: performance.now(), timer: 0, from: 0, fired: false, sent: 0 };
      hold.current = h;
      h.timer = window.setTimeout(() => {
        if (hold.current !== h) return;
        // From here the finger holds a key: the row must not take it.
        try {
          cap.setPointerCapture(h.pointer);
        } catch {
          // The pointer is already gone; the watchdog ends the repeat.
        }
        h.fired = true;
        swallow.current = true;
        h.from = performance.now();
        beat(h);
      }, PAD_HOLD_MS);
    },
    onPointerMove: (e: PointerEvent<HTMLElement>) => {
      const h = hold.current;
      if (!h || e.pointerId !== h.pointer) return;
      h.heard = performance.now();
      if (!h.fired && Math.hypot(e.clientX - h.x, e.clientY - h.y) > KEY_SLOP) stop();
    },
    onPointerUp: () => stop(),
    onPointerCancel: () => stop(),
    onLostPointerCapture: () => stop(),
    // A long press is the key held, not a menu.
    onContextMenu: (e: MouseEvent<HTMLElement>) => e.preventDefault(),
    onClick: () => {
      if (swallow.current) {
        swallow.current = false;
        return;
      }
      send(true);
    },
  };
}
