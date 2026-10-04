// **The key row opens from the deck** (UX-12, the owner 2026-10-04: the
// deck's circle is khor's key row — Esc, Ctrl, Tab, the joystick, 「…」 —
// not the system keyboard).
//
// On a phone the terminal's key row is collapsed while the soft keyboard
// is down, so the session page is one row, the deck; and open while it is
// up, so the deck and the row stand over the keyboard. The deck's circle
// toggles it either way: open without a keyboard for Esc and the
// joystick, or collapsed while typing. A choice lasts until the keyboard
// next comes up or goes down — collapsed while typing stays collapsed
// until the keyboard goes, and the next raise opens it again; a row opened
// before typing stays open through it. Typing on a
// hardware keyboard brings no soft keyboard; there the row starts as
// 「外接键盘时显示辅助键」 says (`lib/term-keys-hw`).
//
// The soft keyboard is the root's `data-keyboard` (`lib/viewport`), the
// same signal the safe-area inset follows, so the row and the keyboard
// change on one frame. "Typing" is the terminal's field asking for a
// keyboard (`lib/soft-keyboard`), whether or not one came.
import { useEffect, useState, useSyncExternalStore, type RefObject } from "react";

import { keyboardUp, onKeyboardChange } from "@/lib/soft-keyboard";
import { useTermKeysHw } from "@/lib/term-keys-hw";

const CHANGED = "khor:key-row-open";

/** The person's toggle since the keyboard last came or went; `null` is
    the default for the keyboard's state. Kept across a tab switch (the
    next terminal takes the row over) and forgotten once no terminal is
    on screen, like the keyboard (`lib/soft-keyboard`'s `terminalShown`). */
let chosen: boolean | null = null;
/** Whether the keyboard was up (or asked for) when `chosen` was made. */
let typing = false;
let shown = 0;
let forget = 0;

function choose(to: boolean | null) {
  chosen = to;
  window.dispatchEvent(new Event(CHANGED));
}

function subscribe(run: () => void) {
  window.addEventListener(CHANGED, run);
  return () => window.removeEventListener(CHANGED, run);
}

const softUp = () => document.documentElement.hasAttribute("data-keyboard");

/** The soft keyboard is up: the root's `data-keyboard`, as state. */
function useSoftUp(): boolean {
  const [up, setUp] = useState(softUp);
  useEffect(() => {
    const watch = new MutationObserver(() => setUp(softUp()));
    watch.observe(document.documentElement, { attributes: true, attributeFilter: ["data-keyboard"] });
    setUp(softUp());
    return () => watch.disconnect();
  }, []);
  return up;
}

/**
 * The key row for the terminal whose field is `field`: whether it is open
 * now, and the deck circle's toggle. One terminal at a time holds it —
 * the one on screen.
 */
export function useKeyRowOpen(field: RefObject<HTMLElement | null>): { open: boolean; toggle: () => void } {
  const soft = useSoftUp();
  const [asked, setAsked] = useState(false);
  const hardware = useTermKeysHw();
  const mine = useSyncExternalStore(subscribe, () => chosen);

  useEffect(() => {
    const read = () => setAsked(keyboardUp(field.current));
    read();
    return onKeyboardChange(read);
  }, [field]);

  useEffect(() => {
    shown += 1;
    window.clearTimeout(forget);
    return () => {
      shown -= 1;
      forget = window.setTimeout(() => {
        if (shown === 0) {
          chosen = null;
          typing = false;
        }
      }, 0);
    };
  }, []);

  // The keyboard came or went: a toggle made before is spent — except a
  // row opened by hand before typing, which typing does not close (a
  // hardware keyboard's default may be collapsed; a soft one's is open).
  const now = soft || asked;
  useEffect(() => {
    if (now === typing) return;
    typing = now;
    if (chosen !== null && !(now && chosen)) choose(null);
  }, [now]);

  // Until that effect has run, this render reads what it will leave.
  const kept = now === typing ? mine : now && mine === true ? true : null;
  const open = kept ?? (soft || (asked && hardware));
  return { open, toggle: () => choose(!open) };
}
