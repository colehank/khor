// **iOS's delete key repeats in the terminal** (item 35; the owner:
// 「长按删除是不是也得支持持续删的操作」).
//
// Recorded in the simulator (iOS 26.5, the 简体拼音 and English keyboards,
// a plain textarea): a press of the soft keyboard's delete is
//
//     keydown Backspace (8) · beforeinput/input deleteContentBackward · keyup
//
// and on an **empty** field only `keydown · keyup`: nothing to delete, so
// no edit. Holding it, UIKit repeats the delete as more edits for as long
// as the field has text to lose (vercel-labs/wterm#32, finedesignz/remo-
// code#459: the repeat comes as `deleteContentBackward` input events, and
// it ends when the field is empty or the first keydown was cancelled).
// xterm keeps its hidden field empty and answers the keydown itself, with
// `preventDefault` — so the first DEL went out and the repeat never began.
//
// So, the way Zellij's web client and remo-code keep it going (and hterm
// before them, which keeps its own text in the field): under the soft
// keyboard, the delete key's keydown is left to the browser, the field
// always holds a run of placeholder text for it to delete, and each edit
// the field reports becomes the terminal's key — DEL for a character,
// ⌃W for a word, ⌃U for a line (iOS sends the word form when a held
// delete speeds up), as the Mac's ⌥⌫ and ⌘⌫ do. The placeholder is no
// space a person typed: it never reaches the pty (the field's text is
// never read here, and xterm reads only what an edit or a composition
// adds), the clipboard (copies come from xterm's selection) or a
// selection. A hardware keyboard, and the keyboard mid-composition
// (pinyin, item 18B), keep xterm's own road.

/** What the field holds for the soft keyboard to delete. No-break spaces:
    no word for a spelling or prediction bar to read, and wrapped by none. */
export const PLACEHOLDER = " ";
/** The run the field is filled to… */
export const PAD = 64;
/** …when it has fewer than this left (or holds more than a composition left
    behind). */
const LOW = 16;
const HIGH = 4 * PAD;

/** The edits that delete backwards, and what each sends. */
export type SoftDelete = "char" | "word" | "line" | "forward";
const KIND: Record<string, SoftDelete> = {
  deleteContentBackward: "char",
  deleteWordBackward: "word",
  deleteSoftLineBackward: "line",
  deleteHardLineBackward: "line",
  deleteContentForward: "forward",
};

/**
 * Keeps the soft keyboard's delete repeating. `soft` says whether a soft
 * keyboard is the one typing now (a phone's, up on screen); `send` puts a
 * delete's bytes in. Returns `key`, for the pane's custom key handler to
 * ask first: `false` when this keydown is the soft delete's (xterm must
 * leave it alone — no bytes, no `preventDefault`), `null` otherwise.
 */
export function softDeletes(
  field: HTMLTextAreaElement,
  o: { soft: () => boolean; send: (kind: SoftDelete) => void },
): { key: (ev: KeyboardEvent) => false | null; stop: () => void } {
  /** A delete keydown left to the browser, whose edit has not come yet. */
  let pending = false;
  const fill = () => {
    const n = field.value.length;
    if (n >= LOW && n <= HIGH) return;
    field.value = PLACEHOLDER.repeat(PAD);
    field.setSelectionRange(PAD, PAD);
  };
  const ours = (ev: KeyboardEvent) =>
    ev.key === "Backspace" && ev.keyCode !== 229 && !ev.isComposing && !ev.ctrlKey && !ev.altKey && !ev.metaKey && o.soft();
  const key = (ev: KeyboardEvent): false | null => {
    if (ev.type !== "keydown" || !ours(ev)) return null;
    // The edit happens after this keydown, on whatever the field holds then.
    fill();
    pending = true;
    return false;
  };
  const before = (e: Event) => {
    const ev = e as InputEvent;
    const kind = KIND[ev.inputType];
    if (!kind || ev.isComposing || (!pending && !o.soft())) return;
    pending = false;
    o.send(kind);
  };
  const after = (e: Event) => {
    const ev = e as InputEvent;
    if (KIND[ev.inputType] && !ev.isComposing) fill();
  };
  // A delete on a field that had nothing to lose makes no edit: the key
  // still goes, once, at its keyup.
  const up = (ev: KeyboardEvent) => {
    if (ev.key !== "Backspace" || !pending) return;
    pending = false;
    o.send("char");
  };
  field.addEventListener("beforeinput", before, true);
  field.addEventListener("input", after, true);
  field.addEventListener("keyup", up, true);
  return {
    key,
    stop: () => {
      field.removeEventListener("beforeinput", before, true);
      field.removeEventListener("input", after, true);
      field.removeEventListener("keyup", up, true);
    },
  };
}
