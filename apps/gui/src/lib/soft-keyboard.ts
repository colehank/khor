// **The terminal is always the focus; the soft keyboard is a separate
// switch** (2026-10-03, the owner: 「每次进入 term 都不是对焦，只有通过点击来
// 得到焦点。是不是可以进入后任意输入都可以控制到焦点？」).
//
// A person never deals with focus, only with whether the keyboard is up.
// Under a finger the terminal's field holds the focus with
// `inputmode="none"` — iOS's way to keep a field focused without its
// virtual keyboard, while a hardware keyboard types into it — and the
// keyboard is raised by switching the field to `inputmode="text"` and
// focusing it again **inside the press** that asked (iOS shows a keyboard
// only for a focus a gesture made). Lowering is the reverse, and keeps
// the focus. Under a mouse there is no soft keyboard and none of this
// applies: the field is a plain focused field.
import { termWake, type Wake } from "@/lib/term-tap";
import { keyboardShown } from "@/lib/viewport";

const CHANGED = "khor:soft-keyboard";

/** Whether the person has the keyboard up on the session page: kept
    across a tab switch and a reload (a new terminal takes it over), and
    forgotten once no terminal is open — coming back from another page,
    the keyboard is down. */
let wanted = false;
let open = 0;
let forget = 0;
/** The terminal field the keyboard is about: the one a terminal last
    held the focus with. */
let current: HTMLElement | null = null;

/** A terminal is on screen until the answer is called. */
export function terminalShown(): () => void {
  open += 1;
  window.clearTimeout(forget);
  return () => {
    open -= 1;
    // A tab switch unmounts one terminal and mounts the next in the same
    // turn; only a page with none left forgets.
    forget = window.setTimeout(() => {
      if (open === 0) wanted = false;
    }, 0);
  };
}

function refocus(field: HTMLElement) {
  if (document.activeElement === field) field.blur();
  field.focus({ preventScroll: true });
  window.dispatchEvent(new Event(CHANGED));
}

/** Focus without raising anything: what opening the page, choosing a tab
    and a reload do. The keyboard stays as it was — up if the person had
    it up on this page (a tab chosen by a press keeps it), down otherwise. */
export function holdFocus(field: HTMLElement, finger: boolean) {
  current = field;
  if (finger) field.setAttribute("inputmode", wanted ? "text" : "none");
  if (document.activeElement !== field) field.focus({ preventScroll: true });
  window.dispatchEvent(new Event(CHANGED));
}

/** Raise the soft keyboard. Only from inside a press. */
export function raiseKeyboard(field: HTMLElement) {
  current = field;
  wanted = true;
  field.setAttribute("inputmode", "text");
  refocus(field);
}

/** Put the soft keyboard away, keeping the focus — a hardware keyboard
    keeps typing and the next raise needs no tap first. */
export function lowerKeyboard(field: HTMLElement) {
  wanted = false;
  field.setAttribute("inputmode", "none");
  refocus(field);
}

/** **Whether the soft keyboard is up for this field: as the screen says**
    (item 28). The field's `inputmode` is only the request; iOS lowers the
    keyboard on its own (its key for it, a rotation, another app), and a
    state read off the request then said "up" and the next press did
    nothing. So: the field has the focus and the visual viewport, settled,
    has a keyboard in it (`lib/viewport`'s `keyboardShown`). */
export function keyboardUp(field: HTMLElement | null): boolean {
  return field !== null && document.activeElement === field && keyboardShown();
}

/** Called when the field's keyboard state may have changed. */
export function onKeyboardChange(run: () => void): () => void {
  window.addEventListener(CHANGED, run);
  document.addEventListener("focusin", run);
  // `focusout` runs before the focus has gone anywhere; read after it.
  const later = () => queueMicrotask(run);
  document.addEventListener("focusout", later);
  return () => {
    window.removeEventListener(CHANGED, run);
    document.removeEventListener("focusin", run);
    document.removeEventListener("focusout", later);
  };
}

/**
 * **The keyboard, for the session page's deck** (roadmap 2.2 with the ux
 * lane's UX-3): the deck draws a keyboard circle only in the `button` mode
 * and calls these; the modes, a tap's mapping and the raise and lower path
 * stay here and in `lib/term-tap`. One store key (`khor.term-wake`).
 *
 * - `raise()` / `lower()`: the terminal on screen's keyboard, from inside
 *   the press that asked (iOS raises a keyboard only for a gesture).
 * - `up()`: whether it is up now.
 * - `mode()`: `auto`, `screen` or `button` (`lib/term-tap`'s `termWake`).
 */
export const terminalKeyboard = {
  raise() {
    if (current?.isConnected) raiseKeyboard(current);
  },
  lower() {
    if (current?.isConnected) lowerKeyboard(current);
  },
  up(): boolean {
    return keyboardUp(current);
  },
  mode(): Wake {
    return termWake();
  },
};
