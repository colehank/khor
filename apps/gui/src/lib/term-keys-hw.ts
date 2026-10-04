// **Where the key row starts with a hardware keyboard** (UX-12 ruling 3,
// 2026-10-04): on a phone the terminal's key row (Esc Ctrl Tab …) opens
// by itself when the soft keyboard comes up (`lib/key-row-open`). Typing on a
// hardware keyboard brings no soft keyboard, so the row stays as it was —
// collapsed — unless 外观 › 终端 › 「外接键盘时显示辅助键」 is on. The
// deck's circle toggles it either way. Off by default. This device's, in
// its `localStorage`.
import { useEffect, useState } from "react";

const KEY = "khor.term-keys-hw";
const CHANGED = "khor:term-keys-hw";
let session: boolean | null = null;

export function termKeysHw(): boolean {
  if (session !== null) return session;
  try {
    return window.localStorage.getItem(KEY) === "on";
  } catch {
    return false;
  }
}

export function setTermKeysHw(on: boolean) {
  session = on;
  try {
    window.localStorage.setItem(KEY, on ? "on" : "off");
  } catch {
    // Kept for this session only.
  }
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: on }));
}

export function useTermKeysHw(): boolean {
  const [on, setOn] = useState(termKeysHw);
  useEffect(() => {
    const changed = (e: Event) => setOn((e as CustomEvent<boolean>).detail);
    window.addEventListener(CHANGED, changed);
    return () => window.removeEventListener(CHANGED, changed);
  }, []);
  return on;
}
