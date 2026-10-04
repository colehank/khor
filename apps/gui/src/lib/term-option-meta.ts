// **Option as Meta** (item 22, Appearance → Terminal): off by default, as in
// Terminal.app (Profiles › Keyboard › "Use Option as Meta key", off) and
// iTerm2 (Profiles › Keys › "Left/Right Option key: Normal"). Off, ⌥ types
// the Mac's characters (⌥2 ™, ⌥E then E é); on, ⌥ is Meta and ⌥b, ⌥f, ⌥d reach
// the shell as ESC b, ESC f, ESC d. ⌥← ⌥→ ⌥⌫ move and delete by word
// either way (`TerminalPane`'s `macEditing`). Open terminals take it at
// once. This device's, in its `localStorage`.
import { useEffect, useState } from "react";

const KEY = "khor.term-option-meta";
const CHANGED = "khor:term-option-meta";
let session: boolean | null = null;

export function termOptionMeta(): boolean {
  if (session !== null) return session;
  try {
    return window.localStorage.getItem(KEY) === "on";
  } catch {
    return false;
  }
}

export function setTermOptionMeta(on: boolean) {
  session = on;
  try {
    window.localStorage.setItem(KEY, on ? "on" : "off");
  } catch {
    // Kept for this session only.
  }
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: on }));
}

export function useTermOptionMeta(): boolean {
  const [on, setOn] = useState(termOptionMeta);
  useEffect(() => {
    const changed = (e: Event) => setOn((e as CustomEvent<boolean>).detail);
    window.addEventListener(CHANGED, changed);
    return () => window.removeEventListener(CHANGED, changed);
  }, []);
  return on;
}

/** A terminal's hook on the setting. */
export function onTermOptionMeta(run: (on: boolean) => void): () => void {
  const on = (e: Event) => run((e as CustomEvent<boolean>).detail);
  window.addEventListener(CHANGED, on);
  return () => window.removeEventListener(CHANGED, on);
}
