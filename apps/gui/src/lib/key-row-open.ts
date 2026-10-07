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
// change on one frame — or the screen's settled reading of it
// (`keyboardShown`, item 28), which is also the gates' seam. "Typing" is
// the terminal's field asking for a keyboard (its `inputmode`,
// `lib/soft-keyboard`), whether or not one came: `keyboardUp` reads the
// screen since item 28, and a hardware keyboard shows nothing there.
import { useEffect, useState, useSyncExternalStore, type RefObject } from "react";

import { onKeyboardChange } from "@/lib/soft-keyboard";
import { useTermKeysHw } from "@/lib/term-keys-hw";
import { footHeld, keyboardShown } from "@/lib/viewport";

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

/** **The row's choice forgotten, as a keyboard going forgets it**: khor's
    panel lowered by a swipe or a tap off the input area (item 49, a
    person wanting the screen) takes the row with it. Its own key pressed
    again does not come here: the row stays as it was. */
export function forgetKeyRowChoice() {
  if (chosen !== null) choose(null);
}

const softUp = () => document.documentElement.hasAttribute("data-keyboard") || keyboardShown();

/** The soft keyboard is up, as state. */
function useSoftUp(): boolean {
  const [up, setUp] = useState(softUp);
  useEffect(() => {
    const read = () => setUp(softUp());
    const watch = new MutationObserver(read);
    watch.observe(document.documentElement, { attributes: true, attributeFilter: ["data-keyboard"] });
    read();
    const off = onKeyboardChange(read);
    return () => {
      watch.disconnect();
      off();
    };
  }, []);
  return up;
}

/** The field asks for a keyboard: its mode, with the focus. */
const requested = (field: HTMLElement | null) =>
  field !== null && field.getAttribute("inputmode") === "text" && document.activeElement === field;

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
    const read = () => setAsked(requested(field.current));
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
  // **khor's panel is neither a keyboard coming nor one going** (the
  // owner 2026-10-07 12:30: 📎 pressed again put the panel away and the
  // row with it). The panel's hold stamps the root as a keyboard would,
  // so the shell does not move; read as typing, its release lowered a
  // keyboard that was never up and dropped the row opened from the deck.
  // While the foot is held, the row stays as it was.
  const now = footHeld() === null ? soft || asked : typing;
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
