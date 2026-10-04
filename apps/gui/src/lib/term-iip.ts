// **An iTerm2 inline image without `size=` still draws** (the ux lane's
// walk, 2026-10-04: `printf '\e]1337;File=inline=1;width=12:…\a'` drew
// nothing). iTerm2's protocol makes `size` optional (iterm2.com/
// documentation-images: "size: file size in bytes. Optional"), but
// `@xterm/addon-image` sizes its base64 decoder by it and drops a header
// without one (IIPHandler.ts: `!this._header.size` → aborted). `imgcat`
// always sends it; a hand-written `printf` and some libraries do not.
//
// So the pane's output passes through here first: a `File=` sequence whose
// header has no `size` is held until it is complete — it may span many
// reads — and `size=N` is written into its header, N the payload's decoded
// length. Everything else passes untouched, and a sequence held past the
// addon's own limit is let go as it came.

const ESC = 0x1b;
const BEL = 0x07;
const OPEN = "\x1b]1337;File=";
/** The addon's `iipSizeLimit` default: past it, it drops the picture anyway. */
const HOLD_MAX = 20_000_000;

/** Bytes as a string, one character per byte. (`TextDecoder("latin1")` is
    windows-1252 in browsers and would remap 0x80–0x9F, so not that.) */
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

/** The decoded length of a base64 payload (padding and whitespace aside). */
function decodedLength(b64: string): number {
  const clean = b64.replace(/[^A-Za-z0-9+/=]/g, "");
  const pad = clean.endsWith("==") ? 2 : clean.endsWith("=") ? 1 : 0;
  return Math.floor((clean.length * 3) / 4) - pad;
}

/** A complete `ESC ] 1337 ; File= header : payload END` with a size. */
function sized(seq: string): string {
  const colon = seq.indexOf(":", OPEN.length);
  if (colon < 0) return seq;
  const header = seq.slice(OPEN.length, colon);
  if (/(^|;)size=/.test(header)) return seq;
  const endLen = seq.endsWith("\x07") ? 1 : 2;
  const payload = seq.slice(colon + 1, seq.length - endLen);
  const size = decodedLength(payload);
  return `${OPEN}size=${size}${header ? ";" : ""}${header}${seq.slice(colon)}`;
}

/** Where the sequence starting at `from` ends (one past BEL or ST), or -1. */
function endOf(s: string, from: number): number {
  for (let i = from; i < s.length; i++) {
    const c = s.charCodeAt(i);
    if (c === BEL) return i + 1;
    if (c === ESC && s[i + 1] === "\\") return i + 2;
  }
  return -1;
}

/** Whether `bytes` holds the marker, or ends in what could start one. */
function mayHold(bytes: Uint8Array): boolean {
  for (let i = bytes.indexOf(ESC); i >= 0; i = bytes.indexOf(ESC, i + 1)) {
    let k = 0;
    while (k < OPEN.length && i + k < bytes.length && bytes[i + k] === OPEN.charCodeAt(k)) k++;
    if (k === OPEN.length || i + k === bytes.length) return true;
  }
  return false;
}

/** A stream rewriter, one per terminal: bytes in, bytes out. */
export function iipSizes(): (bytes: Uint8Array) => Uint8Array {
  let held = "";
  return (bytes) => {
    if (!held && !mayHold(bytes)) return bytes;
    let s = held + latinOf(bytes);
    held = "";
    let out = "";
    let i = 0;
    for (;;) {
      const at = s.indexOf(OPEN, i);
      if (at < 0) {
        // A tail that may be the start of the marker waits for the next read.
        const tail = s.lastIndexOf("\x1b", s.length - 1);
        if (tail >= i && OPEN.startsWith(s.slice(tail))) {
          out += s.slice(i, tail);
          held = s.slice(tail);
        } else out += s.slice(i);
        break;
      }
      out += s.slice(i, at);
      const end = endOf(s, at + OPEN.length);
      if (end < 0) {
        const rest = s.slice(at);
        // Held until it ends — unless it already says its size, or it is
        // past what the addon would take, when it goes as it is.
        const colon = rest.indexOf(":");
        const header = colon < 0 ? rest.slice(OPEN.length) : rest.slice(OPEN.length, colon);
        if ((colon >= 0 && /(^|;)size=/.test(header)) || rest.length > HOLD_MAX) out += rest;
        else held = rest;
        break;
      }
      out += sized(s.slice(at, end));
      i = end;
    }
    s = "";
    return bytesOf(out);
  };
}
