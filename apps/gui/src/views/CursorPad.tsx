// The terminal key row's cursor pad (2026-10-03; the owner on two passes:
// 「触控板有点太灵敏了，而且位置有点不好按」, then 「方向键仍然过于灵敏了，而且还存在
// 手指已经离开了但方向键一直在往某个方向移动的情况」).

// ── The numbers to tune ───────────────────────────────────────────────
/** The dead zone round where the finger landed: outside it the stick
    points somewhere (pt). */
export const PAD_DEAD = 12;
/** Held this long in one direction, and the arrow repeats (ms). */
export const PAD_HOLD_MS = 400;
/** How far past the diagonal the finger must go to change axis: the other
    axis must outweigh the held one by this much (tan 55° ≈ 1.43, ten
    degrees of hysteresis either side of 45°). */
export const PAD_SWITCH = 1.43;
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
  /** Where the finger landed: the stick's centre. */
  x0: number;
  y0: number;
  /** The way the stick points now; null inside the dead zone. */
  way: Way | null;
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
 * **Which way a stick at (`dx`, `dy`) from its centre points**, given the
 * way it pointed before: the axis with the larger displacement, with a
 * hysteresis band round each diagonal so a finger resting on one does not
 * flicker between two arrows (`PAD_SWITCH`). The dead zone is radial, the
 * way game sticks do it (Josh Sutphin, "Doing Thumbstick Dead Zones Right",
 * 2013: a circle round the centre, not a square per axis).
 */
export function wayAt(dx: number, dy: number, was: Way | null): Way | null {
  if (Math.hypot(dx, dy) < PAD_DEAD) return null;
  const ax = Math.abs(dx);
  const ay = Math.abs(dy);
  const horizontal = (): Way => (dx > 0 ? "right" : "left");
  const vertical = (): Way => (dy > 0 ? "down" : "up");
  if (was === "left" || was === "right") return ay > ax * PAD_SWITCH ? vertical() : horizontal();
  if (was === "up" || was === "down") return ax > ay * PAD_SWITCH ? horizontal() : vertical();
  return ax >= ay ? horizontal() : vertical();
}

/**
 * **The cursor pad: a joystick** (the owner, after the per-distance pad:
 * 「轻滑一下就是移动一下，连着一个方向不动一会再连跳」; then item 19: a
 * stick must change direction while the finger is still down).
 * Pinned second from the row's right end. A finger lands on it and the
 * whole screen is its pad until it lifts (pointer capture); where it landed
 * is the stick's centre.
 *
 * - The way is **where the finger is now** relative to that centre
 *   (`wayAt`), re-read on every move: nothing inside the dead zone; outside
 *   it, the larger axis, with hysteresis round the diagonals.
 * - Pointing a new way — out of the dead zone, or across to another side
 *   without lifting — sends that arrow at once and starts its repeat over:
 *   `PAD_HOLD_MS`, then `PAD_RATE` a second, `PAD_RATE_FAST` after
 *   `PAD_RAMP_MS`. Moving within the same way sends nothing more.
 * - Back inside the dead zone the repeat stops; lifting ends it all.
 *
 * Termius's space-bar cursor repeats the same way while held
 * (docs.termius.com/terminal/mobile-terminal), and a game stick's d-pad
 * emulation re-reads its direction continuously rather than latching the
 * first one.
 *
 * **The repeat dies with the gesture, however it ends**: `pointerup`,
 * `pointercancel`, `lostpointercapture`, the window losing focus, the page
 * going to the background, a second press replacing the first, the pad
 * leaving the screen — and a watchdog for anything none of those catches:
 * no pointer event for `PAD_WATCHDOG_MS` with no finger or button down.
 *
 * Arrows go through `cursorKey`, so an editor's application mode and a
 * latched Ctrl or Alt (word jumps) hold; the latches let go at the first
 * arrow, as on every key. While the stick points the cap shows that way's
 * arrow in the held grey; repeating, the arrow wears the accent.
 */
export function CursorKey({ term, mods, onMods }: { term: RefObject<Terminal | null>; mods: Mods; onMods: (m: Mods) => void }) {
  const g = useRef<Gesture | null>(null);
  const [way, setWay] = useState<Way | null>(null);
  const [repeating, setRepeating] = useState(false);
  const stopRepeat = (s: Gesture) => {
    window.clearTimeout(s.repeat);
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
  /** One repeat beat, rescheduled each time: the rate steps up after a
      second, and the watchdog is asked on every beat. */
  const beat = (s: Gesture, w: Way) => {
    if (g.current !== s || s.way !== w) return;
    if (performance.now() - s.heard > PAD_WATCHDOG_MS && touching === 0 && buttons === 0) {
      end();
      return;
    }
    setRepeating(true);
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
      repeat: 0,
      repeatFrom: 0,
      heard: performance.now(),
      mods,
      freed: false,
    };
  };
  const move = (e: PointerEvent<HTMLButtonElement>) => {
    const s = g.current;
    if (!s || e.pointerId !== s.pointer) return;
    s.heard = performance.now();
    const now = wayAt(e.clientX - s.x0, e.clientY - s.y0, s.way);
    if (now === s.way) return;
    // A new way, or none: whatever repeated stops here.
    stopRepeat(s);
    setRepeating(false);
    s.way = now;
    if (!now) {
      setWay(null);
      return;
    }
    send(s, now);
    s.repeat = window.setTimeout(() => {
      s.repeatFrom = performance.now();
      beat(s, now);
    }, PAD_HOLD_MS);
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
