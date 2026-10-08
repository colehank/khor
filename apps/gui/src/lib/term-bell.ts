// **Whether a terminal's bell is shown or felt** (roadmap 4.2, Appearance →
// Terminal → Bell): on by default. On a Mac the pane's edge flashes in the
// terminal's yellow (iTerm2's and Terminal.app's visual bell); on a phone
// a light haptic tap (`bell_haptic`), the bell Blink Shell rings. Off, a
// bell is silent everywhere. This device's, in its `localStorage`.
import { useEffect, useState } from "react";

import { heardFrom } from "@/lib/other-windows";

const KEY = "khor.term-bell";
const CHANGED = "khor:term-bell";
let session: boolean | null = null;

export function termBell(): boolean {
  if (session !== null) return session;
  try {
    return window.localStorage.getItem(KEY) !== "off";
  } catch {
    return true;
  }
}

export function setTermBell(on: boolean) {
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
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: termBell() }));
});

export function useTermBell(): boolean {
  const [on, setOn] = useState(termBell);
  useEffect(() => {
    const changed = (e: Event) => setOn((e as CustomEvent<boolean>).detail);
    window.addEventListener(CHANGED, changed);
    return () => window.removeEventListener(CHANGED, changed);
  }, []);
  return on;
}
