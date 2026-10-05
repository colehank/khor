// **A composition ends when the keyboard goes down** (item 37; the owner:
// 「如果敲到一半点了别的地方收起键盘了，这个预览的拼写框会持续存在」).
//
// Lowering the keyboard keeps the field focused and flips its `inputmode`
// to none (`lib/soft-keyboard`), and iOS then drops the input method
// without a `compositionend`. Nothing told the pane, so the strip drawn at
// `compositionstart` stayed, and so did xterm's own composition state. Now
// the keyboard going down — the field's `inputmode` turning to none, or
// the screen no longer showing a keyboard (`keyboardUp`, item 28's one
// source), whichever comes first — ends any composition the way iOS's own
// text fields do: the half-typed pinyin is discarded, nothing reaches the
// pty, and the keyboard coming back starts clean.
import type { Terminal } from "@xterm/xterm";

/** xterm's composition helper, as far as discarding needs it. */
type Helper = {
  _isComposing?: boolean;
  _isSendingComposition?: boolean;
  _compositionView?: HTMLElement;
};

export function dropsComposition(
  term: Terminal,
  o: {
    /** Whether the soft keyboard is up for the field, as the screen says. */
    keyboardUp: () => boolean;
    /** Subscribes to the keyboard's state changing; returns the unsubscribe. */
    onKeyboardChange: (run: () => void) => () => void;
    /** A composition was dropped: the pane takes its strip down. */
    dropped: () => void;
  },
): () => void {
  const field = term.textarea;
  if (!field) return () => {};
  let composing = false;
  const start = () => {
    composing = true;
  };
  const end = () => {
    composing = false;
  };
  const drop = () => {
    if (!composing) return;
    composing = false;
    // xterm's own state, set back without its finalize, which would send
    // what was composed: the view hidden, no composition, nothing queued.
    const helper = (term as unknown as { _core?: { _compositionHelper?: Helper } })._core?._compositionHelper;
    if (helper) {
      helper._isComposing = false;
      helper._isSendingComposition = false;
      helper._compositionView?.classList.remove("active");
      if (helper._compositionView) helper._compositionView.textContent = "";
    }
    // The letters iOS had put in the field go with it, so no later diff
    // of the field reads them as typed.
    field.value = "";
    o.dropped();
  };
  const changed = () => {
    if (composing && !o.keyboardUp()) drop();
  };
  const asked = new MutationObserver(() => {
    if (field.getAttribute("inputmode") === "none") drop();
  });
  asked.observe(field, { attributes: true, attributeFilter: ["inputmode"] });
  field.addEventListener("compositionstart", start, true);
  field.addEventListener("compositionend", end, true);
  const unwatch = o.onKeyboardChange(changed);
  return () => {
    asked.disconnect();
    field.removeEventListener("compositionstart", start, true);
    field.removeEventListener("compositionend", end, true);
    unwatch();
  };
}
