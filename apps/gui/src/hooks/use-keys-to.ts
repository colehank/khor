// Where a keystroke goes when a session is on screen.
//
// A session pane is the thing a person is looking at, and the thing
// they are about to type into — but a click on the row that opened it
// leaves focus on that row's button, and a click on the rail leaves it
// on the rail. Every keystroke after that went to a button, which
// ignores it, and the way in was one more click on the box (found live,
// 2026-09-01:「每次打字，都在选中 session 列表或侧边栏，而要点一下输入框
// 才能开始打字」). Two rules put the keys where the attention is:
//
// 1. **Arriving is focusing.** When a pane with a box opens, the box
//    takes focus — on a machine with a real pointer only. On a phone
//    focus is a soft keyboard sliding over half the screen, and a pane
//    that opened the keyboard before anything was read is a pane that
//    hid what it was opened for.
//
// 2. **A stray key lands in the box.** A printable key pressed while
//    nothing on the page takes text moves focus to the box *during the
//    keydown*, so the browser's own default action — inserting the
//    character — lands in the box rather than on a button. An input
//    method's opening key is included: Chrome fires the first `keydown`
//    of a composition with `keyCode 229` before `compositionstart`, so
//    it is the one chance to aim the composition anywhere.
//
// What it must never take: a key meant for something else. So it yields
// to any field that takes text (the rail's search, a rename in flight),
// to anything open on top (a dialog, a menu — Radix traps focus there
// and stealing it would fight the trap), to modified keys (⌘C is not a
// C), and to keys that are not text (Enter on a row opens the row).
import { useEffect, type RefObject } from "react";

/** Somewhere a keystroke already has a home. */
function takesText(el: Element | null): boolean {
  if (!el) return false;
  if (el instanceof HTMLTextAreaElement || el instanceof HTMLSelectElement) return true;
  if (el instanceof HTMLInputElement) {
    return !["button", "checkbox", "radio", "submit", "reset", "range", "file", "color"].includes(
      el.type,
    );
  }
  return el instanceof HTMLElement && el.isContentEditable;
}

/** Anything **open** on top of the page.
 *
 * ⚠️ **「Radix unmounts these when they close」 is not always true, and
 * that sentence stood here until 2026-09-17.** A content that is
 * animating out stays in the document wearing `data-state="closed"`,
 * and so does one whose exit animation never got a frame — which is
 * what a busy machine hands you. Measured in the smoke run (25f, twice
 * in a row): a row menu that had already been dismissed was still
 * there, `data-state="closed"`, and the key typed at the rail went
 * nowhere because this selector counted it as something on top. The
 * diagnostic that named it was added the round before for exactly
 * this.
 *
 * So the test is what the overlay itself says about whether it is up.
 * `closed` is radix's own word, and an element that is not radix's has
 * no `data-state` at all and goes on matching. */
const OPEN = ':not([data-state="closed"])';
const ON_TOP = [
  '[role="dialog"]',
  '[role="alertdialog"]',
  '[role="menu"]',
  '[role="listbox"]',
]
  .map((role) => `${role}${OPEN}`)
  .join(", ");

/** A pointer that hovers — a desktop, where focusing a box does not
    slide a keyboard over the screen. */
const FINE_POINTER = "(hover: hover) and (pointer: fine)";

export function useKeysTo(field: RefObject<HTMLElement | null>, on: boolean) {
  useEffect(() => {
    if (!on || !window.matchMedia(FINE_POINTER).matches) return;
    field.current?.focus({ preventScroll: true });
  }, [field, on]);

  useEffect(() => {
    if (!on) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.metaKey || e.ctrlKey || e.altKey) return;
      const opening = e.key === "Process" || e.keyCode === 229;
      if (!opening && e.key.length !== 1) return;
      const active = document.activeElement;
      if (takesText(active)) return;
      if (active?.closest(ON_TOP) || document.querySelector(ON_TOP)) return;
      const box = field.current;
      if (!box || box === active) return;
      box.focus({ preventScroll: true });
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [field, on]);
}
