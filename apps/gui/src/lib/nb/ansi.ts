// ANSI colour in a notebook's text outputs (lane nb-view): a stream, a
// traceback, a `text/plain` that a library printed with colour. IPython
// colours every traceback, so a preview that showed the escapes as
// `[0;31m` would be a worse reading than the terminal's.
//
// **Written here rather than pulled in** (`anser` is MIT and small, but
// it answers in HTML strings or class names of its own, and either way
// the colours would be decided somewhere other than the terminal's
// palettes). Only SGR is read — colour, weight, italic, underline,
// inverse — because a notebook's output is a record, not a screen:
// cursor moves and erases have nothing to move. Every other escape is
// dropped rather than shown.
//
// **The colours are the terminal's, never new ones.** A run carries the
// index it was asked for (0–255) or the RGB it spelled; which hex an
// index becomes is the renderer's decision (`Notebook`'s `ansiColor`),
// from the palettes `term-palettes.json` already holds.

export type Color = number | string;

export type Run = {
  text: string;
  fg?: Color;
  bg?: Color;
  bold?: boolean;
  dim?: boolean;
  italic?: boolean;
  underline?: boolean;
};

type Style = Omit<Run, "text"> & { inverse?: boolean };

/** `\r` and `\b` the way Jupyter folds them before drawing: a progress
    bar that rewrote its own line is shown as its last state, and a
    backspace takes the character before it. */
export function fold(text: string): string {
  let out = text;
  if (out.includes("\r")) {
    out = out
      .replace(/\r+\n/g, "\n")
      .split("\n")
      .map((line) => {
        const at = line.lastIndexOf("\r");
        return at === -1 ? line : line.slice(at + 1);
      })
      .join("\n");
  }
  if (out.includes("\b")) {
    let prev: string;
    do {
      prev = out;
      out = out.replace(/[^\n\b]\b/g, "");
    } while (out !== prev);
    out = out.replace(/\b/g, "");
  }
  return out;
}

// CSI with any parameters and final byte; OSC up to BEL or ST; any other
// two-byte escape.
const ESCAPE = /\x1b(?:\[([0-9;:?]*)([@-~])|\][^\x07\x1b]*(?:\x07|\x1b\\)|[@-Z\\-_])/g;

function extended(codes: number[], i: number): { color?: Color; used: number } {
  if (codes[i + 1] === 5 && codes[i + 2] !== undefined) {
    const n = codes[i + 2];
    return { color: n >= 0 && n <= 255 ? n : undefined, used: 2 };
  }
  if (codes[i + 1] === 2 && codes[i + 4] !== undefined) {
    const [r, g, b] = codes.slice(i + 2, i + 5).map((v) => Math.max(0, Math.min(255, v)));
    const hex = (v: number) => v.toString(16).padStart(2, "0");
    return { color: `#${hex(r)}${hex(g)}${hex(b)}`, used: 4 };
  }
  return { used: 0 };
}

function apply(style: Style, params: string): Style {
  const codes = params === "" ? [0] : params.split(/[;:]/).map((p) => (p === "" ? 0 : Number(p)));
  let s = { ...style };
  for (let i = 0; i < codes.length; i++) {
    const c = codes[i];
    if (c === 0) s = {};
    else if (c === 1) s.bold = true;
    else if (c === 2) s.dim = true;
    else if (c === 3) s.italic = true;
    else if (c === 4) s.underline = true;
    else if (c === 7) s.inverse = true;
    else if (c === 22) s.bold = s.dim = undefined;
    else if (c === 23) s.italic = undefined;
    else if (c === 24) s.underline = undefined;
    else if (c === 27) s.inverse = undefined;
    else if (c >= 30 && c <= 37) s.fg = c - 30;
    else if (c === 39) s.fg = undefined;
    else if (c >= 40 && c <= 47) s.bg = c - 40;
    else if (c === 49) s.bg = undefined;
    else if (c >= 90 && c <= 97) s.fg = c - 90 + 8;
    else if (c >= 100 && c <= 107) s.bg = c - 100 + 8;
    else if (c === 38 || c === 48) {
      const { color, used } = extended(codes, i);
      if (c === 38) s.fg = color;
      else s.bg = color;
      i += used;
    }
  }
  return s;
}

function run(text: string, s: Style): Run {
  const r: Run = { text };
  const fg = s.inverse ? s.bg : s.fg;
  const bg = s.inverse ? (s.fg ?? "fg") : s.bg;
  if (fg !== undefined) r.fg = fg;
  if (bg !== undefined) r.bg = bg;
  if (s.bold) r.bold = true;
  if (s.dim) r.dim = true;
  if (s.italic) r.italic = true;
  if (s.underline) r.underline = true;
  return r;
}

/** The text as styled runs; escapes that are not SGR are dropped. A
    plain text with no escape is one run with no style. */
export function parseAnsi(raw: string): Run[] {
  const text = fold(raw);
  if (!text.includes("\x1b")) return [{ text }];
  const out: Run[] = [];
  let style: Style = {};
  let last = 0;
  for (const m of text.matchAll(ESCAPE)) {
    const at = m.index ?? 0;
    if (at > last) out.push(run(text.slice(last, at), style));
    if (m[2] === "m") style = apply(style, m[1] ?? "");
    last = at + m[0].length;
  }
  if (last < text.length) out.push(run(text.slice(last), style));
  // A stray ESC with nothing after it.
  return out.map((r) => (r.text.includes("\x1b") ? { ...r, text: r.text.replace(/\x1b/g, "") } : r)).filter((r) => r.text !== "");
}

/** xterm's standard table for 16–255: the 6×6×6 cube, then 24 greys.
    The first sixteen belong to the palette, not to this table. */
export function xterm256(n: number): string {
  const hex = (v: number) => v.toString(16).padStart(2, "0");
  if (n >= 232) {
    const v = 8 + (n - 232) * 10;
    return `#${hex(v)}${hex(v)}${hex(v)}`;
  }
  const i = n - 16;
  const step = (v: number) => (v === 0 ? 0 : 55 + v * 40);
  return `#${hex(step(Math.floor(i / 36)))}${hex(step(Math.floor(i / 6) % 6))}${hex(step(i % 6))}`;
}
