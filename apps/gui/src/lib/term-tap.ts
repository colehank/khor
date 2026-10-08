// **When a tap in the terminal raises the keyboard** (roadmap 2.2, Appearance
// → Terminal → keyboard wake; it replaces item 5's on/off switch).
//
// - `auto` (the default): a tap on the input area (`term-marks`' `inInput`:
//   the shell's input from `OSC 133 ; B`, a program's input box ± 2 rows,
//   a bare cursor row ± 1) raises it; a tap anywhere else on the terminal
//   puts it away — and khor's key panel (UX-31) — when it is up, and does
//   nothing when it is down (item 49). The input region is the one the
//   field's marks define (FinalTerm's B, the prompt line Kitty lets a
//   click move the cursor in with `click_events=1`, which fish 4
//   announces); a phone also has to decide the keyboard there.
//   ⚠️ Tombstone (10-05, item 28): 「点别处不动」, a tap never lowered.
//   **With no cursor on screen** (a full-screen program that hid it, as
//   `top` does: `?25l`) there is no input area to aim at, so a tap
//   anywhere raises the keyboard when it is down and lowers it when it is
//   up (item 74, the owner's ruling). A program that takes the mouse still
//   gets the tap.
// - `screen`: any tap raises it (item 5's "on"); a tap never lowers.
// - `button`: only the bar's keyboard switch moves it (item 5's "off").
//
// Only under a finger: a Mac's click focuses the terminal either way. This
// device's, in its `localStorage` like the terminal's other settings; the
// old switch's stored answer is carried over once.
import type { Terminal } from "@xterm/xterm";
import { useEffect, useState } from "react";

import { heardFrom } from "@/lib/other-windows";

export type Wake = "auto" | "screen" | "button";
export const WAKES: Wake[] = ["auto", "screen", "button"];

const KEY = "khor.term-wake";
/** Item 5's switch: "on" or "off". */
const OLD_KEY = "khor.term-tap-keyboard";
const CHANGED = "khor:term-wake";

/** This session's answer once it has been set, for a face whose storage
    refuses it. */
let session: Wake | null = null;

export function termWake(): Wake {
  if (session !== null) return session;
  try {
    const v = window.localStorage.getItem(KEY);
    if (WAKES.includes(v as Wake)) return v as Wake;
    const old = window.localStorage.getItem(OLD_KEY);
    if (old === "off" || old === "on") {
      const carried: Wake = old === "off" ? "button" : "screen";
      window.localStorage.setItem(KEY, carried);
      window.localStorage.removeItem(OLD_KEY);
      return carried;
    }
    return "auto";
  } catch {
    return "auto";
  }
}

/** **What a tap on the terminal asks of the keyboard** (item 49), given
    the mode and whether it landed on the input area; the caller raises
    only what is down and lowers only what is up. With no cursor on
    screen, `auto` asks to `toggle` (item 74): raise what is down, lower
    what is up. */
export function tapAsks(wake: Wake, onInput: boolean, noCursor = false): "raise" | "lower" | "toggle" | null {
  if (wake === "screen") return "raise";
  if (wake === "auto") return noCursor ? "toggle" : onInput ? "raise" : "lower";
  return null;
}

/** **Whether the program shows a cursor** (item 74): DECTCEM, `?25h` /
    `?25l`, read as xterm parses it (xterm keeps it to itself), back to
    shown on a reset (RIS, DECSTR). xterm still acts on every one. */
export function watchesCursor(term: Terminal): { shown: () => boolean; dispose: () => void } {
  let shown = true;
  const set = (on: boolean) => (params: (number | number[])[]) => {
    if (params.some((p) => (Array.isArray(p) ? p[0] : p) === 25)) shown = on;
    return false;
  };
  const reset = () => {
    shown = true;
    return false;
  };
  const watching = [
    term.parser.registerCsiHandler({ prefix: "?", final: "h" }, set(true)),
    term.parser.registerCsiHandler({ prefix: "?", final: "l" }, set(false)),
    term.parser.registerEscHandler({ final: "c" }, reset),
    term.parser.registerCsiHandler({ intermediates: "!", final: "p" }, reset),
  ];
  return {
    shown: () => shown,
    dispose: () => {
      for (const w of watching) w.dispose();
    },
  };
}

export function setTermWake(wake: Wake) {
  session = wake;
  try {
    window.localStorage.setItem(KEY, wake);
  } catch {
    // Kept for this session only.
  }
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: wake }));
}

// Set in another window of this app (the Mac's settings window,
// `lib/other-windows`): heard here as if set here.
heardFrom(KEY, () => {
  session = null;
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: termWake() }));
});

export function useTermWake(): Wake {
  const [w, setW] = useState(termWake);
  useEffect(() => {
    const changed = (e: Event) => setW((e as CustomEvent<Wake>).detail);
    window.addEventListener(CHANGED, changed);
    return () => window.removeEventListener(CHANGED, changed);
  }, []);
  return w;
}
