// **When a tap in the terminal raises the keyboard** (roadmap 2.2, Appearance
// → Terminal → keyboard wake; it replaces item 5's on/off switch).
//
// - `auto` (the default): a tap on the line being typed — from where the
//   shell's input starts (`OSC 133 ; B`) to the cursor, or the cursor's row
//   while a program runs or when the shell marks nothing — raises it; a
//   tap anywhere else never does, and puts it away when it is up. The
//   input region is the one the field's marks define (FinalTerm's B, the
//   prompt line Kitty lets a click move the cursor in with
//   `click_events=1`, which fish 4 announces); a phone also has to decide
//   the keyboard there.
// - `screen`: any tap raises it (item 5's "on").
// - `button`: only the bar's keyboard switch moves it (item 5's "off").
//
// Only under a finger: a Mac's click focuses the terminal either way. This
// device's, in its `localStorage` like the terminal's other settings; the
// old switch's stored answer is carried over once.
import { useEffect, useState } from "react";

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

export function setTermWake(wake: Wake) {
  session = wake;
  try {
    window.localStorage.setItem(KEY, wake);
  } catch {
    // Kept for this session only.
  }
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: wake }));
}

export function useTermWake(): Wake {
  const [w, setW] = useState(termWake);
  useEffect(() => {
    const changed = (e: Event) => setW((e as CustomEvent<Wake>).detail);
    window.addEventListener(CHANGED, changed);
    return () => window.removeEventListener(CHANGED, changed);
  }, []);
  return w;
}
