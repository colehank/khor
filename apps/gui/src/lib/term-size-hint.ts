// **The size a new shell is started at** (walk fix, 2026-10-04: a fresh
// zsh showed its reverse-video `%` above the first prompt on the phone).
// A shell writes its first prompt before any face has attached, for the
// size the host was given; zsh's prompt mark (`PROMPT_SP`: the `%`, a line
// of spaces, then `\r \r`) is erased by the prompt only in a terminal at
// least that wide, and wraps onto a line of its own in a narrower one. So
// a shell this device opens starts at the size this device's terminals
// were last fitted to — or, before any has been, at what the window holds
// at the terminal's type size. This device's, in its `localStorage`.
import { typeSize } from "@/lib/type-size";

const KEY = "khor.term-size-hint";

/** Called on each fit: the size this device's terminals have. */
export function rememberTermSize(cols: number, rows: number) {
  if (cols < 2 || rows < 2) return;
  try {
    window.localStorage.setItem(KEY, `${cols}x${rows}`);
  } catch {
    // A face without storage estimates each time.
  }
}

/** The size to start a new shell at. */
export function termSizeHint(): { cols: number; rows: number } {
  try {
    const m = /^(\d+)x(\d+)$/.exec(window.localStorage.getItem(KEY) ?? "");
    if (m) return { cols: Number(m[1]), rows: Number(m[2]) };
  } catch {
    // Estimated below.
  }
  // A monospaced cell is about 0.6 of the type size wide and 1.2 tall;
  // the pane's side margins are 8 each.
  const px = typeSize().term;
  return {
    cols: Math.max(20, Math.floor((window.innerWidth - 16) / (px * 0.6))),
    rows: Math.max(5, Math.floor((window.innerHeight * 0.8) / (px * 1.2))),
  };
}
