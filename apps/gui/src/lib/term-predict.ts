// **Typing that shows before the far machine answers** (roadmap 5.2):
// predictive local echo, mosh's model (Winstein & Balakrishnan, "Mosh: An
// Interactive Remote Shell for Mobile Clients", USENIX ATC 2012, §3) as
// VS Code's terminal carries it onto xterm.js (`TypeAheadAddon`,
// `terminal.integrated.localEchoLatencyThreshold`).
//
// A printable key is drawn at once, underlined, where the shell would echo
// it. Before the program's output is written the guesses are taken back;
// once xterm has parsed it, **the screen is compared, not the bytes**
// (mosh's way): a guess whose cell now holds its character is confirmed —
// the shell's own echo, in the shell's colours (zsh's syntax highlighting
// redraws each character, so the bytes never equal the keys) — and the
// guesses not yet echoed are drawn again after it, if the cursor stands
// where they belong. Nothing is ever sent differently; this is only what
// is drawn while waiting.
//
// When to guess:
//   - only while echoes are slow: the time from a key to its echo, kept as
//     a smoothed average, above `ON_MS`, until it falls under `OFF_MS` —
//     on a near machine nothing is drawn twice;
//   - only a printable ASCII key typed at the end of the line, with room
//     left on it, on the normal screen, with the mouse not taken: a
//     full-screen program draws its own screen, a suggestion to the right
//     (fish) is the shell's, and an IME's text is the composition's;
//   - **only once this run of typing has been echoed** (mosh's epochs):
//     after Enter, a control key, a paste or anything that is not a
//     printable key, the next key is not drawn until its own echo comes
//     back. A password prompt never echoes, so its keys are never shown.
//
// Undoing is cheap because a guess is only ever drawn with nothing to its
// right: back over the drawn characters and erase to the line's end.

import type { Terminal } from "@xterm/xterm";

/** Echoes slower than this turn guessing on (ms), and faster than this
    turn it off again. Mosh starts showing guesses past a round trip of
    about 60 ms; the roadmap's line is 100. */
const ON_MS = 100;
const OFF_MS = 70;
/** Smoothing of the echo time, TCP's (RFC 6298: α = 1/8). */
const ALPHA = 1 / 8;
/** A key whose echo took longer than this is not a sample. */
const STALE_MS = 3000;

const UNDERLINE = "\x1b[4m";
const NO_UNDERLINE = "\x1b[24m";

export type Predict = {
  /** A key on its way to the program. */
  typed: (data: string) => void;
  /** Output on its way into xterm: what to write instead, and what to
      run once xterm has parsed it. */
  output: (bytes: Uint8Array) => { bytes: Uint8Array; parsed?: () => void };
  /** The screen started over (a reset, a re-join, a resize): forget. */
  clear: () => void;
  /** For the gates. */
  state: () => { srtt: number | null; on: boolean; pending: string; shown: number; echoed: boolean };
};

export function predictEcho(term: Terminal, allowed: () => boolean): Predict {
  /** Keys sent and not yet echoed, each with when it was sent. They sit
      in one line, from `x` on. */
  let pending: { ch: string; at: number }[] = [];
  let x = 0;
  let y = 0;
  /** How many of `pending`, from the first, are drawn on the screen. */
  let shown = 0;
  /** This run of typing has had an echo (mosh's confirmed epoch). */
  let echoed = false;
  /** This run began with something to its right (a right prompt): its
      echoes are timed, and nothing is drawn. */
  let measureOnly = false;
  let srtt: number | null = null;
  let on = false;
  /** Counts outputs: a redraw is only for the latest. */
  let outputs = 0;
  const encode = new TextEncoder();

  const sample = (ms: number) => {
    if (ms > STALE_MS) return;
    srtt = srtt === null ? ms : srtt + ALPHA * (ms - srtt);
    if (!on && srtt > ON_MS) on = true;
    else if (on && srtt < OFF_MS) on = false;
  };
  const undo = () => (shown > 0 ? `\x1b[${shown}D\x1b[K` : "");
  /** A run of typing ends: what is drawn is taken back now. */
  const breakRun = () => {
    const back = undo();
    pending = [];
    shown = 0;
    echoed = false;
    if (back) term.write(back);
  };
  /** Nothing from column `from` to the end of line `row`. */
  const blankFrom = (row: number, from: number) => {
    const line = term.buffer.active.getLine(row);
    if (!line) return false;
    for (let i = from; i < term.cols; i++) if ((line.getCell(i)?.getChars() ?? "") !== "") return false;
    return true;
  };
  const screenAllows = () => term.buffer.active.type === "normal" && term.modes.mouseTrackingMode === "none" && allowed();
  const draw = (chars: string) => UNDERLINE + chars + NO_UNDERLINE;

  const typed = (data: string) => {
    if (data.length !== 1 || data < " " || data > "~" || !screenAllows()) {
      breakRun();
      return;
    }
    const buf = term.buffer.active;
    if (pending.length === 0) {
      x = buf.cursorX;
      y = buf.baseY + buf.cursorY;
      measureOnly = !blankFrom(y, x);
    }
    if (x + pending.length >= term.cols - 1) {
      breakRun();
      return;
    }
    pending.push({ ch: data, at: performance.now() });
    // Drawn only once echoes are slow and this run has been echoed, and
    // only right after what is already drawn.
    if (on && echoed && !measureOnly && shown === pending.length - 1) {
      term.write(draw(data));
      shown += 1;
    }
  };

  const output = (bytes: Uint8Array) => {
    if (pending.length === 0) return { bytes };
    const mine = ++outputs;
    const back = undo();
    shown = 0;
    const parsed = () => {
      if (pending.length === 0) return;
      const line = term.buffer.active.getLine(y);
      let k = 0;
      while (k < pending.length && line?.getCell(x + k)?.getChars() === pending[k].ch) k++;
      const now = performance.now();
      for (let i = 0; i < k; i++) sample(now - pending[i].at);
      if (k > 0) echoed = true;
      pending = pending.slice(k);
      x += k;
      if (pending.length === 0) return;
      const buf = term.buffer.active;
      const here = buf.cursorX === x && buf.baseY + buf.cursorY === y && (measureOnly || blankFrom(y, x));
      if (!here) {
        // The program went elsewhere (Enter's new line, a redraw, an
        // error): the guesses left are not where they would go.
        pending = [];
        echoed = false;
        return;
      }
      // Not echoed yet: drawn again — unless newer output is already on
      // its way, whose own parse will do it.
      if (on && echoed && !measureOnly && mine === outputs && shown === 0) {
        term.write(draw(pending.map((p) => p.ch).join("")));
        shown = pending.length;
      }
    };
    if (!back) return { bytes, parsed };
    const head = encode.encode(back);
    const out = new Uint8Array(head.length + bytes.length);
    out.set(head, 0);
    out.set(bytes, head.length);
    return { bytes: out, parsed };
  };

  const clear = () => {
    pending = [];
    shown = 0;
    echoed = false;
  };

  return {
    typed,
    output,
    clear,
    state: () => ({ srtt, on, pending: pending.map((p) => p.ch).join(""), shown, echoed }),
  };
}
