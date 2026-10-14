/** IME owns preedit, including keys whose per-event marker is missing.
 * Keep the textarea edit native, while bypassing terminal key encoders and
 * insertText workarounds until composition has actually committed. A normal
 * keyCode during preedit must not make xterm finalize the raw pinyin.
 */
import type { Terminal } from "@xterm/xterm";

type Helper = { isComposing: boolean; _isSendingComposition: boolean };

/** Install on an ancestor, before the pane's own input listeners. xterm's
 * input capture listeners already live on its textarea: another listener
 * there would run after bytes escaped. Call key() first in the pane's custom
 * key handler; xterm still clears its keyup bookkeeping. No edit is cancelled.
 */
export function keepTerminalIme(term: Terminal, root: HTMLElement): { key: (event: KeyboardEvent) => boolean; cancel: () => void; stop: () => void } {
  const field = term.textarea;
  if (!field) return { key: () => false, cancel: () => {}, stop: () => {} };
  const helper = (term as unknown as { _core?: { _compositionHelper?: Helper } })._core?._compositionHelper;
  let composing = false;
  // An IME-owned keyup can arrive after compositionend, with isComposing
  // already false. Do not encode a release for a press never sent to PTY.
  const owned = new Set<string>();
  const cancel = () => { composing = false; owned.clear(); };
  const start = (e: Event) => { if (e.target === field) composing = true; };
  const end = (e: Event) => { if (e.target === field) composing = false; };
  const active = () => composing || !!helper?.isComposing;
  const key = (e: KeyboardEvent): boolean => {
    const id = e.code || e.key;
    if (active() || e.isComposing || e.type === "keypress" && helper?._isSendingComposition) {
      if (e.type === "keydown") owned.add(id);
      else if (e.type === "keyup") owned.delete(id);
      return true;
    }
    if (e.type === "keyup" && owned.delete(id)) return true;
    // A new ordinary press supersedes a missing keyup of the same key.
    if (e.type === "keydown") owned.delete(id);
    return false;
  };
  const input = (event: Event) => {
    if (event.target !== field) return;
    const e = event as InputEvent;
    // After compositionend, xterm defers its read until the textarea's
    // native edit finishes. Let it send the commit once; insertText and
    // Option/late-commit fallbacks must not send it a second time.
    if (active() || e.isComposing || helper?._isSendingComposition) e.stopImmediatePropagation();
  };
  root.addEventListener("compositionstart", start, true);
  root.addEventListener("compositionend", end, true);
  for (const kind of ["beforeinput", "input"]) root.addEventListener(kind, input, true);
  field.addEventListener("blur", cancel);
  return {
    key,
    cancel,
    stop: () => {
      root.removeEventListener("compositionstart", start, true);
      root.removeEventListener("compositionend", end, true);
      for (const kind of ["beforeinput", "input"]) root.removeEventListener(kind, input, true);
      field.removeEventListener("blur", cancel);
      cancel();
    },
  };
}
