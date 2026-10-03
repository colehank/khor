// The terminal key row's cursor pad (2026-10-03; the owner on two passes:
// 「触控板有点太灵敏了，而且位置有点不好按」, then 「方向键仍然过于灵敏了，而且还存在
// 手指已经离开了但方向键一直在往某个方向移动的情况」).

// ── The four numbers to tune ──────────────────────────────────────────
/** Travel before anything happens: a tap, or a finger settling (pt). */
export const PAD_DEAD = 12;
/** Travel per arrow along the locked axis, both axes (pt). */
export const PAD_STEP = 24;
/** A deflected finger still this long starts the repeat (ms). */
export const PAD_HOLD_MS = 400;
/** The repeat: this many arrows a second for its first second, then
    `PAD_RATE_FAST` (per s). */
export const PAD_RATE = 6;
export const PAD_RATE_FAST = 12;
// ──────────────────────────────────────────────────────────────────────

/** Movement below this is a resting finger's tremor, not a move (pt). */
const PAD_STILL = 3;
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
  /** Where the finger landed: deflection is measured from here. */
  x0: number;
  y0: number;
  /** Where the step count starts — the landing, or the point the finger
      rested at before it moved on from a repeat. */
  base: number;
  axis: "x" | "y" | null;
  /** Steps sent since `base`, signed: coming back takes them back. */
  sent: number;
  /** Where the finger last was along the axis, for the rest timer. */
  still: number;
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
 * **The cursor pad: drag for steps, hold for repeat**, pinned second from
 * the row's right end so there is room to drag both ways. A finger lands
 * on it and the whole screen is its pad until it lifts (pointer capture).
 *
 * - Nothing until the finger has travelled `PAD_DEAD`; a tap sends nothing
 *   (the most reachable spot on the row, and → would accept a shell's
 *   suggestion).
 * - Then the axis that dominates is the gesture's until the finger lifts.
 * - Along it, one arrow per `PAD_STEP` of travel from the landing; coming
 *   back sends the opposite arrow.
 * - A finger deflected past the dead zone and still for `PAD_HOLD_MS`
 *   repeats the arrow of its deflection, `PAD_RATE` a second, then
 *   `PAD_RATE_FAST` after a second, until it moves (the next step then needs
 *   a whole `PAD_STEP` from where it rested) or lifts.
 *
 * **The repeat dies with the gesture, however it ends**: `pointerup`,
 * `pointercancel`, `lostpointercapture`, the window losing focus, the page
 * going to the background, a second press replacing the first, the pad
 * leaving the screen — and a watchdog for anything none of those catches:
 * no pointer event for `PAD_WATCHDOG_MS` with no finger on the glass.
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
    const rate = performance.now() - s.repeatFrom < 1000 ? PAD_RATE : PAD_RATE_FAST;
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
      base: 0,
      axis: null,
      sent: 0,
      still: 0,
      rest: 0,
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
    const dx = e.clientX - s.x0;
    const dy = e.clientY - s.y0;
    if (!s.axis) {
      if (Math.hypot(dx, dy) < PAD_DEAD) return;
      s.axis = Math.abs(dx) >= Math.abs(dy) ? "x" : "y";
      s.still = s.axis === "x" ? dx : dy;
      setWay(wayOf(s.axis, s.still));
    }
    const at = s.axis === "x" ? dx : dy;
    if (Math.abs(at - s.still) < PAD_STILL && (s.rest || s.repeat)) return;
    // It moved: a repeat ends and the steps count from where it rested.
    if (s.repeat) {
      stopRepeat(s);
      setRepeating(false);
      s.base = s.still;
      s.sent = 0;
    }
    s.still = at;
    const want = Math.trunc((at - s.base) / PAD_STEP);
    while (s.sent !== want) {
      const forward = want > s.sent;
      s.sent += forward ? 1 : -1;
      send(s, wayOf(s.axis, forward ? 1 : -1));
    }
    window.clearTimeout(s.rest);
    s.rest = window.setTimeout(() => {
      s.rest = 0;
      if (g.current !== s || !s.axis || Math.abs(s.still) < PAD_DEAD) return;
      setRepeating(true);
      s.repeatFrom = performance.now();
      beat(s, wayOf(s.axis, Math.sign(s.still)));
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
