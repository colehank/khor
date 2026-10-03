// **The terminal's cursor: its shape and whether it blinks** (item 16: the
// owner found the blinking irregular and distracting, and asked for a
// style setting — Appearance → Terminal, cursor style and blinking).
//
// A block that does not blink by default, as the field ships it:
// Terminal.app's and iTerm2's profiles have blinking off, xterm.js's
// `cursorBlink` is false, WezTerm's default style is a steady block and VS
// Code's `terminal.integrated.cursorBlinking` is false. Off is also what
// the owner asked for.
// A program's own request (`CSI n q`, DECSCUSR) still wins while it runs;
// `views/TerminalPane` has how.
//
// This device's, in its `localStorage` like the terminal's font, size and
// tap setting (`lib/term-font`, `lib/type-size`, `lib/term-tap`). A face
// that refuses storage keeps the default.
import { useEffect, useState } from "react";

export type CursorShape = "block" | "bar" | "underline";
export type CursorChoice = { shape: CursorShape; blink: boolean };

const KEY = "khor.term-cursor";
const CHANGED = "khor:term-cursor";
const DEFAULT: CursorChoice = { shape: "block", blink: false };
const SHAPES: CursorShape[] = ["block", "bar", "underline"];

/** This session's choice once it has been set, for a face whose storage
    refuses it. */
let session: CursorChoice | null = null;

export function termCursor(): CursorChoice {
  if (session) return session;
  try {
    const raw = window.localStorage.getItem(KEY);
    if (!raw) return DEFAULT;
    const v = JSON.parse(raw) as Partial<CursorChoice>;
    return {
      shape: SHAPES.includes(v.shape as CursorShape) ? (v.shape as CursorShape) : DEFAULT.shape,
      blink: typeof v.blink === "boolean" ? v.blink : DEFAULT.blink,
    };
  } catch {
    return DEFAULT;
  }
}

export function setTermCursor(next: CursorChoice) {
  session = next;
  try {
    window.localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    // Kept for this session only.
  }
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: next }));
}

/** Calls `then` with every later choice; answers the way to stop. */
export function onTermCursor(then: (c: CursorChoice) => void): () => void {
  const changed = (e: Event) => then((e as CustomEvent<CursorChoice>).detail);
  window.addEventListener(CHANGED, changed);
  return () => window.removeEventListener(CHANGED, changed);
}

export function useTermCursor(): CursorChoice {
  const [c, setC] = useState(termCursor);
  useEffect(() => onTermCursor(setC), []);
  return c;
}
