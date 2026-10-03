// The terminal key row's cursor pad (2026-10-03; the owner on two passes:
// 「触控板有点太灵敏了，而且位置有点不好按」, then 「方向键仍然过于灵敏了，而且还存在
// 手指已经离开了但方向键一直在往某个方向移动的情况」).

// ── The numbers to tune ───────────────────────────────────────────────
/** Travel that counts as a flick: crossing it sends the one arrow (pt). */
export const PAD_DEAD = 12;
/** A finger that stays within this of where it settled is still (pt). */
export const PAD_STILL = 4;
/** Still this long after a flick, and the arrow repeats (ms). */
export const PAD_HOLD_MS = 400;
/** The repeat's first rate, arrows per second… */
export const PAD_RATE = 6;
/** …its rate once it has run `PAD_RAMP_MS`. */
export const PAD_RATE_FAST = 12;
/** How long the repeat runs at its first rate (ms). */
export const PAD_RAMP_MS = 1000;
// ──────────────────────────────────────────────────────────────────────

/** With no pointer event this long and no finger on the glass, a repeat
    is over whatever else was or was not heard (ms). */
const PAD_WATCHDOG_MS = 600;

import { useEffect, useRef, useState, type PointerEvent, type RefObject } from "react";
import type { Terminal } from "@xterm/xterm";

import { IconArrow, IconMove } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { cursorKey, type Mods } from "@/lib/term-keys";
import { cn } from "@/lib/utils";

type Way = "left" | "right" | "up" | "down";
const LETTER: Record<Way, "A" | "B" | "C" | "D"> = { up: "A", down: "B", right: "C", left: "D" };
const NO_MODS: Mods = { ctrl: false, alt: false };

/** Fingers on the glass, counted from the document's own touch events,
    and a mouse button held: the watchdog's word on whether anybody is
    still holding the pad. */
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

/** One gesture on the pad, from the finger's landing. */
type Gesture = {
  pointer: number;
  /** Where the finger landed. */
  x0: number;
  y0: number;
  /** The way the flick locked, once it crossed the dead zone. */
  way: Way | null;
  /** Where the finger settled last, for the stillness test. */
  sx: number;
  sy: number;
  rest: number;
  repeat: number;
  /** When the repeat began, for its two rates. */
  repeatFrom: number;
  /** When the last pointer event for this gesture arrived. */
  heard: number;
  /** The latches as they were at the landing: the whole gesture uses them. */
  mods: Mods;
  freed: boolean;
};

/**
 * **The cursor pad: a joystick, one arrow per flick** (the owner, after the
 * per-distance pad: 「轻滑一下就是移动一下，连着一个方向不动一会再连跳」).
 * Pinned second from the row's right end. A finger lands on it and the
 * whole screen is its pad until it lifts (pointer capture).
 *
 * - Nothing until the finger has travelled `PAD_DEAD`; a tap sends nothing.
 * - Crossing it locks the axis that dominates and the way along it, and
 *   sends **exactly one arrow**. Further travel sends nothing: three
 *   columns is three flicks, or a hold.
 * - A finger still (within `PAD_STILL`) for `PAD_HOLD_MS` repeats the locked
 *   arrow, `PAD_RATE` a second, `PAD_RATE_FAST` after `PAD_RAMP_MS`. Moving
 *   stops the repeat; the next stillness starts it again. Lift ends it all.
 *
 * Termius's space-bar cursor works the same way — move to step, hold for
 * a series in that direction (docs.termius.com/terminal/mobile-terminal) —
 * without its three speed gears: one ramp, no acceleration by distance.
 *
 * **The repeat dies with the gesture, however it ends**: `pointerup`,
 * `pointercancel`, `lostpointercapture`, the window losing focus, the page
 * going to the background, a second press replacing the first, the pad
 * leaving the screen — and a watchdog for anything none of those catches:
 * no pointer event for `PAD_WATCHDOG_MS` with no finger or button down.
 *
 * Arrows go through `cursorKey`, so an editor's application mode and a
 * latched Ctrl or Alt (word jumps) hold; the latches let go at the first
 * arrow, as on every key. While a gesture is locked the cap shows that
 * way's arrow in the held grey; repeating, the arrow wears the accent.
 */
export function CursorKey({ term, mods, onMods }: { term: RefObject<Terminal | null>; mods: Mods; onMods: (m: Mods) => void }) {
  const g = useRef<Gesture | null>(null);
  const [way, setWay] = useState<Way | null>(null);
  const [repeating, setRepeating] = useState(false);
  const stopRepeat = (s: Gesture) => {
    window.clearTimeout(s.rest);
    window.clearTimeout(s.repeat);
    s.rest = 0;
    s.repeat = 0;
  };
  const endRef = useRef<() => void>(() => {});
  const end = () => {
    if (g.current) stopRepeat(g.current);
    g.current = null;
    setWay(null);
    setRepeating(false);
  };
  endRef.current = end;
  // Endings the pad itself may never hear: the window losing focus, the
  // page hidden, and the pad leaving the screen.
  useEffect(() => {
    const over = () => endRef.current();
    const hidden = () => {
      if (document.visibilityState !== "visible") endRef.current();
    };
    window.addEventListener("blur", over);
    document.addEventListener("visibilitychange", hidden);
    return () => {
      window.removeEventListener("blur", over);
      document.removeEventListener("visibilitychange", hidden);
      endRef.current();
    };
  }, []);
  const send = (s: Gesture, w: Way) => {
    const t = term.current;
    if (!t || g.current !== s) return;
    setWay(w);
    if (!s.freed && (s.mods.ctrl || s.mods.alt)) {
      s.freed = true;
      onMods(NO_MODS);
    }
    t.input(cursorKey(t, LETTER[w], s.mods), true);
  };
  const wayOf = (axis: "x" | "y", sign: number): Way =>
    axis === "x" ? (sign > 0 ? "right" : "left") : sign > 0 ? "down" : "up";
  /** One repeat beat, rescheduled each time: the rate steps up after a
      second, and the watchdog is asked on every beat. */
  const beat = (s: Gesture, w: Way) => {
    if (g.current !== s) return;
    if (performance.now() - s.heard > PAD_WATCHDOG_MS && touching === 0 && buttons === 0) {
      end();
      return;
    }
    send(s, w);
    const rate = performance.now() - s.repeatFrom < PAD_RAMP_MS ? PAD_RATE : PAD_RATE_FAST;
    s.repeat = window.setTimeout(() => beat(s, w), 1000 / rate);
  };
  const down = (e: PointerEvent<HTMLButtonElement>) => {
    if (!term.current) return;
    // A press while a gesture is live replaces it — and stops its repeat.
    end();
    e.currentTarget.setPointerCapture(e.pointerId);
    g.current = {
      pointer: e.pointerId,
      x0: e.clientX,
      y0: e.clientY,
      way: null,
      sx: e.clientX,
      sy: e.clientY,
      rest: 0,
      repeat: 0,
      repeatFrom: 0,
      heard: performance.now(),
      mods,
      freed: false,
    };
  };
  /** The next stillness, from where the finger is now. */
  const settle = (s: Gesture, x: number, y: number) => {
    s.sx = x;
    s.sy = y;
    window.clearTimeout(s.rest);
    s.rest = window.setTimeout(() => {
      s.rest = 0;
      if (g.current !== s || !s.way) return;
      setRepeating(true);
      s.repeatFrom = performance.now();
      beat(s, s.way);
    }, PAD_HOLD_MS);
  };
  const move = (e: PointerEvent<HTMLButtonElement>) => {
    const s = g.current;
    if (!s || e.pointerId !== s.pointer) return;
    s.heard = performance.now();
    if (!s.way) {
      const dx = e.clientX - s.x0;
      const dy = e.clientY - s.y0;
      if (Math.hypot(dx, dy) < PAD_DEAD) return;
      s.way = Math.abs(dx) >= Math.abs(dy) ? wayOf("x", dx) : wayOf("y", dy);
      send(s, s.way);
      settle(s, e.clientX, e.clientY);
      return;
    }
    if (Math.hypot(e.clientX - s.sx, e.clientY - s.sy) <= PAD_STILL) return;
    // It moved: a repeat ends, and stillness is measured from here.
    if (s.repeat) {
      stopRepeat(s);
      setRepeating(false);
    }
    settle(s, e.clientX, e.clientY);
  };
  /** The gesture's own pointer ending, from wherever it is heard. */
  const ended = (e: PointerEvent<HTMLButtonElement>) => {
    if (!g.current || e.pointerId === g.current.pointer) end();
  };
  return (
    <button
      type="button"
      data-term-key="cursor"
      data-way={way ?? undefined}
      data-repeating={repeating || undefined}
      aria-label={gui.key_cursor}
      onMouseDown={(e) => e.preventDefault()}
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={ended}
      onPointerCancel={ended}
      onLostPointerCapture={ended}
      // A pad, not a cap: wider, the same fill. The finger owns it: no
      // page scroll, no sideways scroll of the row under it.
      className="flex h-key-cap w-key-pad-w flex-none touch-none items-stretch outline-none focus-visible:[&>span]:ring-2 focus-visible:[&>span]:ring-ring"
    >
      <span
        className={cn(
          "flex flex-1 items-center justify-center rounded-lg",
          way ? "bg-fill-strong" : "bg-fill",
          repeating ? "text-[var(--rail-on-ink)]" : "text-foreground",
        )}
      >
        {way ? <IconArrow to={way} className="size-5" /> : <IconMove className="size-5" />}
      </span>
    </button>
  );
}
