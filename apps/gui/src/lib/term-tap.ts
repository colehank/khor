// **Whether a tap in the terminal raises the keyboard** (2026-10-03, the
// owner, 外观 → 终端 「点击终端时弹出键盘」).
//
// xterm focuses its hidden textarea on any press inside the terminal, and
// on a phone a focused field is the soft keyboard sliding over half the
// screen — so every tap meant to scroll or read raised it. On (the
// default, the owner: 「默认开」) keeps that; off, a tap leaves the
// keyboard where it is and only the bar's switch moves it
// (`views/TermKeys`' `KeyboardSwitch`). Only under a finger: a Mac's click
// focuses the terminal either way, and the row is not drawn there.
//
// This device's, in its `localStorage` like the terminal's font and size
// (`lib/term-font`, `lib/type-size`). A face that refuses storage keeps
// the default.
import { useEffect, useState } from "react";

const KEY = "khor.term-tap-keyboard";
const CHANGED = "khor:term-tap-keyboard";

/** This session's answer once it has been set, for a face whose storage
    refuses it. */
let session: boolean | null = null;

export function tapRaisesKeyboard(): boolean {
  if (session !== null) return session;
  try {
    return window.localStorage.getItem(KEY) !== "off";
  } catch {
    return true;
  }
}

export function setTapRaisesKeyboard(on: boolean) {
  session = on;
  try {
    window.localStorage.setItem(KEY, on ? "on" : "off");
  } catch {
    // Kept for this session only.
  }
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: on }));
}

export function useTapRaisesKeyboard(): boolean {
  const [on, setOn] = useState(tapRaisesKeyboard);
  useEffect(() => {
    const changed = (e: Event) => setOn((e as CustomEvent<boolean>).detail);
    window.addEventListener(CHANGED, changed);
    return () => window.removeEventListener(CHANGED, changed);
  }, []);
  return on;
}
