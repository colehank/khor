// **The key row's 「…」 is a keyboard, not a sheet** (UX-31, the owner
// 2026-10-06, iPhone: tapping 「…」 lowered the keyboard — the sheet took
// the focus — and the keyboard's lowering closed the sheet; with the
// keyboard down the sheet did not open at all).
//
// khor's panel stands where the system keyboard stands, as tall as it,
// and the terminal above neither moves nor resizes for the swap:
//
// - keyboard up, 「…」: the foot is held at the keyboard's measured
//   height (`lib/viewport`'s `holdFoot`), the keyboard is put away, and
//   the panel is drawn in the band it leaves — the shell above is
//   stamped the same before and after;
// - panel up, 「…」 again (over a panel that replaced the keyboard) or
//   its 「键盘」 key: the keyboard is asked for from inside the press
//   (iOS raises one only for a gesture), and the panel stays drawn under
//   it until the keyboard has settled up, then the hold is released at
//   the same height — nothing above moves. A keyboard that never comes
//   (a hardware one typing) lets the panel go after a beat instead;
// - keyboard down, 「…」: the foot is held at the last keyboard's height
//   (`keyboardHeight`), the shell shrinks once as it would for a
//   keyboard, and 「…」 again releases it;
// - the system keyboard rising on its own while the panel is up (a tap
//   on the screen in 自动) takes the panel's place; the panel's view
//   unmounting (the key row collapsed, the page left) lets the hold go.
//
// The panel moves no focus: the terminal's field keeps it with
// `inputmode="none"` (`lib/soft-keyboard`), as it does under a lowered
// keyboard, which is the whole of why this is a panel and not a sheet.
import { useSyncExternalStore } from "react";

import { keyboardUp, lowerKeyboard, onKeyboardChange, raiseKeyboard } from "@/lib/soft-keyboard";
import { footHeld, holdFoot, keyboardCover, keyboardHeight, keyboardShown } from "@/lib/viewport";

const CHANGED = "khor:key-panel";
/** How long a keyboard asked for may take to show before the panel
    stops waiting for it (a hardware keyboard shows none). */
const KEYBOARD_WAIT_MS = 1_500;

let up = false;
/** The panel took a keyboard's place: 「…」 gives it back. */
let replaced = false;
/** The keyboard was asked for and the panel waits for it to show. */
let swapping = 0;
let watching: (() => void) | null = null;

function changed() {
  window.dispatchEvent(new Event(CHANGED));
}

function release() {
  window.clearTimeout(swapping);
  swapping = 0;
  watching?.();
  watching = null;
  up = false;
  replaced = false;
  holdFoot(null);
  changed();
}

/** Whether khor's panel is up. */
export function keyPanelUp(): boolean {
  return up;
}

export function useKeyPanel(): boolean {
  return useSyncExternalStore(
    (run) => {
      window.addEventListener(CHANGED, run);
      return () => window.removeEventListener(CHANGED, run);
    },
    () => up,
    () => false,
  );
}

/** 「…」: the panel up in the keyboard's place or from the foot; up
    already, back to the keyboard it replaced, else down. Only from
    inside a press. */
export function toggleKeyPanel(field: HTMLElement) {
  if (!up) raise(field);
  else if (replaced) swapToKeyboard(field);
  else release();
}

function raise(field: HTMLElement) {
  const keys = keyboardUp(field);
  // The measured cover while the keyboard is up (its true height, the
  // home indicator included); the remembered one otherwise.
  holdFoot(keys ? Math.max(keyboardCover(), 1) : keyboardHeight());
  up = true;
  replaced = keys;
  if (keys) lowerKeyboard(field);
  // A keyboard that comes up on its own while the panel stands takes
  // its place: the same height, nothing above moves. The one being put
  // away now still reads as up until the screen settles; it is waited
  // out first.
  let goingDown = keys;
  watching = onKeyboardChange(() => {
    if (!up || swapping) return;
    if (goingDown) {
      if (!keyboardShown()) goingDown = false;
      return;
    }
    if (keyboardShown()) release();
  });
  changed();
}

/** The panel's 「键盘」 key: the keyboard is asked for, the panel stays
    until it shows. */
export function swapToKeyboard(field: HTMLElement) {
  if (!up || swapping) return;
  raiseKeyboard(field);
  watching?.();
  watching = onKeyboardChange(() => {
    if (keyboardShown()) release();
  });
  swapping = window.setTimeout(release, KEYBOARD_WAIT_MS);
}

/** The panel's view is gone (the row collapsed, the page left): the
    foot is let go. */
export function keyPanelGone() {
  if (up || footHeld() !== null) release();
}
