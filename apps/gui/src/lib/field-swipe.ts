// **A field outside the terminal lets the keyboard go** (UX-43 §3, the
// owner 2026-10-07 13:40: 「工作区打字、键盘无法收起」).
//
// khor takes WKWebView's ‹ › ✓ bar off every field (`src-tauri`'s
// `inputAccessoryView` swap), because over a terminal it means nothing —
// and that bar's ✓ was the only way a field elsewhere (the workspace's
// find box, the file editor, a rename box, the 附栏's find row, a form)
// had of lowering the keyboard. The bar stays off; iOS's own rule for a
// scrolling page with a field takes its place (Messages, Notes:
// `keyboardDismissMode`): **a drag down the page lowers the keyboard**,
// wherever it starts, the editor included — the field loses the focus.
// Measured on the finger as the terminal's swipe measures it (item 49,
// `lib/term-swipe`): one finger out of the tap's dead zone more down than
// across; a touch held in the dead zone first is a press (a selection's
// start) and lowers nothing. The single-line boxes' return key is the
// other way (`endsOnEnter`).
//
// The terminal's own field keeps its own rules (`lib/soft-keyboard`, item
// 49): with it focused this does nothing.
import type { KeyboardEvent } from "react";

import { PRESS_MS, SLOP } from "@/lib/term-swipe";

/** The terminal's field and everything of its box: not this rule's. */
const TERMINAL = ".xterm, [data-terminal]";

/** The `<input>` types a keyboard types into. */
const TYPED = new Set(["", "text", "search", "url", "email", "password", "number", "tel"]);

/** The focused field a drag would end, or null: one typed into, outside
    the terminal. */
function typing(): HTMLElement | null {
  const a = document.activeElement;
  if (!(a instanceof HTMLElement) || a.closest(TERMINAL)) return null;
  const typed =
    a instanceof HTMLTextAreaElement || (a instanceof HTMLInputElement && TYPED.has(a.type)) || a.isContentEditable;
  return typed ? a : null;
}

/** Installs the rule on the page, once (`main.tsx`). Passive, capturing:
    the drag still scrolls whatever it scrolls. */
export function followFieldSwipe(): () => void {
  let x0 = 0;
  let y0 = 0;
  let t0 = 0;
  /** This touch is still being judged. */
  let one = false;
  /** …and has left the dead zone. */
  let away = false;
  const start = (e: TouchEvent) => {
    one = e.touches.length === 1 && typing() !== null;
    away = false;
    x0 = e.touches[0]?.clientX ?? 0;
    y0 = e.touches[0]?.clientY ?? 0;
    t0 = performance.now();
  };
  const move = (e: TouchEvent) => {
    if (e.touches.length !== 1) one = false;
    if (!one) return;
    const t = e.touches[0];
    const dx = t.clientX - x0;
    const dy = t.clientY - y0;
    if (!away) {
      if (Math.hypot(dx, dy) <= SLOP) return;
      if (performance.now() - t0 >= PRESS_MS) {
        one = false;
        return;
      }
      away = true;
    }
    if (dy <= Math.abs(dx)) return;
    one = false;
    typing()?.blur();
  };
  const end = () => {
    one = false;
  };
  const opts = { capture: true, passive: true } as const;
  document.addEventListener("touchstart", start, opts);
  document.addEventListener("touchmove", move, opts);
  document.addEventListener("touchend", end, opts);
  document.addEventListener("touchcancel", end, opts);
  return () => {
    document.removeEventListener("touchstart", start, opts);
    document.removeEventListener("touchmove", move, opts);
    document.removeEventListener("touchend", end, opts);
    document.removeEventListener("touchcancel", end, opts);
  };
}

/**
 * **A single-line box's return key ends it** under a finger (UX-43 §3): a
 * search box (`enterKeyHint="search"`) or a name box (`"done"`) where
 * Enter has no other meaning there loses the focus, and the keyboard goes
 * with it. With a mouse Enter leaves the focus where it is, as a Mac's
 * search field does. For the box's `onKeyDown`, beside its own keys.
 */
export function endsOnEnter(e: KeyboardEvent<HTMLInputElement>) {
  if (e.key !== "Enter" || e.nativeEvent.isComposing) return;
  if (!window.matchMedia("(pointer: coarse)").matches) return;
  e.currentTarget.blur();
}
