// **An Enter that belongs to an input method is not a send** (2026-10-03).
//
// `isComposing` alone is not enough on WebKit, which is what every
// khor face runs in (the Mac app, the phone, Safari): traced in the
// simulator with 简体拼音, the keydown that starts a composition arrives
// with keyCode 229 and `isComposing` still false, and WebKit is known to
// deliver the key that ends one the same way. keyCode 229 is the
// platform's own word for 「this key went to the input method」, so
// either one means the field must leave the key alone.
import type { KeyboardEvent as ReactKeyboardEvent } from "react";

export function imeKey(e: ReactKeyboardEvent | KeyboardEvent): boolean {
  const n = "nativeEvent" in e ? e.nativeEvent : e;
  return n.isComposing || n.keyCode === 229;
}
