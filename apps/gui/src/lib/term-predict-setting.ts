// **Whether typing is drawn before a slow echo** (roadmap 5.2, Appearance →
// Terminal → Predict Typing): on by default, and only ever drawing while a
// session's echoes are slow (`lib/term-predict`). Off, every character is
// the program's own echo, as before. This device's, in its
// `localStorage`.
import { useEffect, useState } from "react";

import { heardFrom } from "@/lib/other-windows";

const KEY = "khor.term-predict";
const CHANGED = "khor:term-predict";
let session: boolean | null = null;

export function termPredict(): boolean {
  if (session !== null) return session;
  try {
    return window.localStorage.getItem(KEY) !== "off";
  } catch {
    return true;
  }
}

export function setTermPredict(on: boolean) {
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
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: termPredict() }));
});

export function useTermPredict(): boolean {
  const [on, setOn] = useState(termPredict);
  useEffect(() => {
    const changed = (e: Event) => setOn((e as CustomEvent<boolean>).detail);
    window.addEventListener(CHANGED, changed);
    return () => window.removeEventListener(CHANGED, changed);
  }, []);
  return on;
}
