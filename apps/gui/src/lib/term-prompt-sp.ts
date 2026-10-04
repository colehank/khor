// **zsh's prompt mark at the face's own width** (the ux lane's walk,
// 2026-10-04: after a Mac window was narrowed, every earlier prompt wore a
// reverse-video `%` on the line above it).
//
// Before each prompt zsh writes its PROMPT_SP mark: the mark (`%`, or `#`
// for root, in standout), then spaces to the terminal's width, then
// `\r \r` (zsh `utils.c`, `preprompt`). At the width it was written for,
// the spaces fill the line exactly when the cursor began at column 0 and
// the prompt erases the mark; only output that ended without a newline
// leaves it. Written into a narrower terminal — a replay after the narrow
// layout took the pane, a face smaller than the one that opened the shell
// — the spaces wrap and every mark stays.
//
// So on the way into xterm the run of spaces is made to fill *this*
// terminal's width: zsh's test, asked again at the face's size. Live output
// at the same width is unchanged.

const MARK = /((?:\x1b\[[0-9;]*m)*[%#](?:\x1b\[[0-9;]*m)*)( {8,})\r \r/g;

function latinOf(bytes: Uint8Array): string {
  let s = "";
  for (let i = 0; i < bytes.length; i += 8192) s += String.fromCharCode(...bytes.subarray(i, i + 8192));
  return s;
}
function bytesOf(s: string): Uint8Array {
  const out = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i) & 0xff;
  return out;
}

/** Whether `bytes` holds `\r \r`, the mark's end. */
function mayHold(bytes: Uint8Array): boolean {
  for (let i = bytes.indexOf(0x0d); i >= 0 && i + 2 < bytes.length; i = bytes.indexOf(0x0d, i + 1)) {
    if (bytes[i + 1] === 0x20 && bytes[i + 2] === 0x0d) return true;
  }
  return false;
}

/** Rewrites zsh's prompt marks for a terminal `cols()` wide. */
export function promptMarks(cols: () => number): (bytes: Uint8Array) => Uint8Array {
  return (bytes) => {
    if (!mayHold(bytes)) return bytes;
    const s = latinOf(bytes);
    const width = cols();
    let changed = false;
    const out = s.replace(MARK, (all, mark: string, spaces: string) => {
      const fill = Math.max(1, width - 1);
      if (spaces.length === fill) return all;
      changed = true;
      return `${mark}${" ".repeat(fill)}\r \r`;
    });
    return changed ? bytesOf(out) : bytes;
  };
}
