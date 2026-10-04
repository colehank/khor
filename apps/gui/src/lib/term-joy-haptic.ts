// **Whether the cursor joystick is felt** (item 23, Appearance → Terminal
// → "Joystick haptics"): on by default, and a finger's only — the
// joystick is the phone key row's. Separate from the bell (`lib/term-bell`): one is a program
// asking for attention, the other the person's own hand on a control.
// This device's, in its `localStorage`.
import { useEffect, useState } from "react";

const KEY = "khor.term-joy-haptic";
const CHANGED = "khor:term-joy-haptic";
let session: boolean | null = null;

export function termJoyHaptic(): boolean {
  if (session !== null) return session;
  try {
    return window.localStorage.getItem(KEY) !== "off";
  } catch {
    return true;
  }
}

export function setTermJoyHaptic(on: boolean) {
  session = on;
  try {
    window.localStorage.setItem(KEY, on ? "on" : "off");
  } catch {
    // Kept for this session only.
  }
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: on }));
}

export function useTermJoyHaptic(): boolean {
  const [on, setOn] = useState(termJoyHaptic);
  useEffect(() => {
    const changed = (e: Event) => setOn((e as CustomEvent<boolean>).detail);
    window.addEventListener(CHANGED, changed);
    return () => window.removeEventListener(CHANGED, changed);
  }, []);
  return on;
}
