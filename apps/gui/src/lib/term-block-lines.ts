// **Whether command blocks are drawn** (item 27; the owner on his Mac: the
// hairline and the exit stripe go into the terminal settings, off by
// default). Off removes only the drawing: the marks are still read, so
// the block menu, sending a block to an agent, ⌘↑ ⌘↓ and the key sheet's
// jumps all work. This device's, in its `localStorage`, like the other
// terminal looks.
import { useEffect, useState } from "react";

import { heardFrom } from "@/lib/other-windows";

const KEY = "khor.term-block-lines";
const CHANGED = "khor:term-block-lines";
let session: boolean | null = null;

export function termBlockLines(): boolean {
  if (session !== null) return session;
  try {
    return window.localStorage.getItem(KEY) === "on";
  } catch {
    return false;
  }
}

export function setTermBlockLines(on: boolean) {
  session = on;
  try {
    window.localStorage.setItem(KEY, on ? "on" : "off");
  } catch {
    // Kept for this session only.
  }
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: on }));
}

// Set in another window of this app (the Mac's settings window,
// `lib/other-windows`): heard here as if set here.
heardFrom(KEY, () => {
  session = null;
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: termBlockLines() }));
});

export function useTermBlockLines(): boolean {
  const [on, setOn] = useState(termBlockLines);
  useEffect(() => {
    const changed = (e: Event) => setOn((e as CustomEvent<boolean>).detail);
    window.addEventListener(CHANGED, changed);
    return () => window.removeEventListener(CHANGED, changed);
  }, []);
  return on;
}

/** An open terminal's hook on the setting. */
export function onTermBlockLines(run: (on: boolean) => void): () => void {
  const on = (e: Event) => run((e as CustomEvent<boolean>).detail);
  window.addEventListener(CHANGED, on);
  return () => window.removeEventListener(CHANGED, on);
}
