/** Private giant-history-record preparation. This preserves one JSON record's
 * structure and short string prefixes for the existing source parser. It
 * never decides which fields are message bodies: paintHistory does that. */
const SCAFFOLD_BYTES = 64 * 1024;
const SHORT_STRING_BYTES = 1024;
const PREFIX_BYTES = 256;
const STRING_RANGES = 128;
const encoder = new TextEncoder();

export type RecordStringRange = { marker: string; start: number; end: number };
export type RecordScaffold = { json: string; strings: RecordStringRange[] };
type Frame = { kind: 'object'; state: 'keyOrEnd' | 'key' | 'colon' | 'value' | 'commaOrEnd' }
  | { kind: 'array'; state: 'valueOrEnd' | 'value' | 'commaOrEnd' };
type Quoted = {
  key: boolean; start: number; bytes: number; short: string; prefix: string;
  prefixBytes: number; prefixEnded: boolean; large: boolean; high: number | null;
  escapeStart: number; highStart: number;
  quotedBytes: number;
};

function bytesOf(char: string) {
  const point = char.codePointAt(0)!;
  return point < 0x80 ? 1 : point < 0x800 ? 2 : point < 0x10000 ? 3 : 4;
}

type ConsumeText = (char: string, from: number, to: number, rawStart: number, rawEnd: number) => boolean;

function prepareRecord(start: number, seed: string, onText?: ConsumeText, stringOnly = false) {
  if (!Number.isSafeInteger(start) || start < 0 || !/^[\w-]{1,64}$/.test(seed))
    throw new Error('invalid history record scaffold identity');
  const decoder = new TextDecoder('utf-8', { fatal: true });
  const stack: Frame[] = [];
  const strings: RecordStringRange[] = [];
  const parts: string[] = [];
  let piece = '';
  let encodedBytes = 0;
  let offset = start;
  let nextInput = start;
  let rootDone = false;
  let retired = false;
  let paused = false;
  let pendingText = '';
  let decodedOffset = 0;
  let mode: 'normal' | 'string' | 'escape' | 'unicode' | 'number' | 'literal' = 'normal';
  let quoted: Quoted | null = null;
  let token = '';
  let literal = '';

  const append = (text: string) => {
    encodedBytes += encoder.encode(text).length;
    if (encodedBytes > SCAFFOLD_BYTES) throw new Error('history record structure exceeds its scaffold budget');
    if (piece.length + text.length > 4096) { parts.push(piece); piece = ''; }
    piece += text;
  };
  const needsValue = () => {
    const parent = stack.at(-1);
    return parent ? parent.state === 'value' || parent.state === 'valueOrEnd' : !rootDone;
  };
  const valueDone = () => {
    const parent = stack.at(-1);
    if (parent) {
      if (!needsValue()) throw new Error('unexpected value in history record');
      parent.state = 'commaOrEnd';
    } else {
      if (rootDone) throw new Error('multiple values in history record');
      rootDone = true;
    }
  };
  const decoded = (char: string, rawStart: number, rawEnd: number) => {
    const value = quoted!;
    const size = bytesOf(char);
    value.bytes += size;
    if (value.key && value.bytes > SCAFFOLD_BYTES) throw new Error('history record key exceeds its scaffold budget');
    const escaped = char === '"' || char === '\\' || '\b\t\n\f\r'.includes(char);
    value.quotedBytes += escaped ? 2 : char.charCodeAt(0) < 32 ? 6 : size;
    if (value.key && encodedBytes + value.quotedBytes > SCAFFOLD_BYTES)
      throw new Error('history record key exceeds its scaffold budget');
    if (value.key || (!value.large && value.bytes <= SHORT_STRING_BYTES)) value.short += char;
    else if (!value.large) { value.large = true; value.short = ''; }
    if (!value.prefixEnded) {
      if (value.prefixBytes + size <= PREFIX_BYTES) { value.prefix += char; value.prefixBytes += size; }
      else value.prefixEnded = true;
    }
    if (onText && !value.key) {
      const next = decodedOffset + char.length;
      if (!onText(char, decodedOffset, next, rawStart, rawEnd)) paused = true;
      decodedOffset = next;
    }
  };
  const closeString = () => {
    const value = quoted!;
    if (value.high !== null) throw new Error('unpaired Unicode surrogate in history record');
    if (value.large) {
      if (strings.length === STRING_RANGES) throw new Error('history record has too many long string ranges');
      const marker = `\ue000${seed}:${strings.length}\ue001`;
      strings.push({ marker, start: value.start, end: offset + 1 });
      append(JSON.stringify(value.prefix + marker));
    } else append(JSON.stringify(value.short));
    if (value.key) {
      const parent = stack.at(-1);
      if (!parent || parent.kind !== 'object' || (parent.state !== 'key' && parent.state !== 'keyOrEnd'))
        throw new Error('unexpected key in history record');
      parent.state = 'colon';
    } else valueDone();
    quoted = null;
    mode = 'normal';
  };
  const closeNumber = () => {
    if (!/^-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?$/.test(token)) throw new Error('invalid number in history record');
    append(token); token = ''; mode = 'normal'; valueDone();
  };

  function consume(char: string): void {
    if (mode === 'string') {
      if (quoted!.high !== null && char !== '\\') throw new Error('unpaired Unicode surrogate in history record');
      if (char === '"') closeString();
      else if (char === '\\') { quoted!.escapeStart = offset; mode = 'escape'; }
      else {
        if (char.charCodeAt(0) < 32) throw new Error('unescaped control in history record');
        decoded(char, offset, offset + bytesOf(char));
      }
      return;
    }
    if (mode === 'escape') {
      if (quoted!.high !== null && char !== 'u') throw new Error('unpaired Unicode surrogate in history record');
      if (char === 'u') { token = ''; mode = 'unicode'; return; }
      const escapes: Record<string, string> = { '"': '"', '\\': '\\', '/': '/', b: '\b', f: '\f', n: '\n', r: '\r', t: '\t' };
      if (!(char in escapes)) throw new Error('invalid escape in history record');
      decoded(escapes[char], quoted!.escapeStart, offset + 1); mode = 'string'; return;
    }
    if (mode === 'unicode') {
      if (!/^[0-9a-fA-F]$/.test(char)) throw new Error('invalid Unicode escape in history record');
      token += char;
      if (token.length === 4) {
        const point = Number.parseInt(token, 16);
        if (quoted!.high !== null) {
          if (point < 0xdc00 || point > 0xdfff) throw new Error('unpaired Unicode surrogate in history record');
          decoded(String.fromCodePoint(0x10000 + (quoted!.high - 0xd800) * 1024 + point - 0xdc00), quoted!.highStart, offset + 1);
          quoted!.high = null;
        } else if (point >= 0xd800 && point <= 0xdbff) { quoted!.high = point; quoted!.highStart = quoted!.escapeStart; }
        else {
          if (point >= 0xdc00 && point <= 0xdfff) throw new Error('unpaired Unicode surrogate in history record');
          decoded(String.fromCodePoint(point), quoted!.escapeStart, offset + 1);
        }
        token = ''; mode = 'string';
      }
      return;
    }
    if (mode === 'number') {
      if (/^[0-9eE+.\-]$/.test(char)) {
        token += char;
        if (token.length > SCAFFOLD_BYTES) throw new Error('history record number exceeds its scaffold budget');
        return;
      }
      closeNumber();
      consume(char);
      return;
    }
    if (mode === 'literal') {
      token += char;
      if (!literal.startsWith(token)) throw new Error('invalid literal in history record');
      if (token === literal) { append(token); token = ''; mode = 'normal'; valueDone(); }
      return;
    }
    if (/^[ \t\r\n]$/.test(char)) return;
    const parent = stack.at(-1);
    if (stringOnly && !parent && !rootDone && char !== '"') throw new Error('history text range is not a JSON string');
    if (char === '"') {
      const key = parent?.kind === 'object' && (parent.state === 'key' || parent.state === 'keyOrEnd');
      if (!key && !needsValue()) throw new Error('unexpected string in history record');
      quoted = { key, start: offset, bytes: 0, short: '', prefix: '', prefixBytes: 0, prefixEnded: false, large: false, high: null, escapeStart: 0, highStart: 0, quotedBytes: 2 };
      mode = 'string'; return;
    }
    if (char === '{' || char === '[') {
      if (!needsValue()) throw new Error('unexpected container in history record');
      if (stack.length === 128) throw new Error('history record nesting exceeds its scaffold budget');
      append(char);
      stack.push(char === '{' ? { kind: 'object', state: 'keyOrEnd' } : { kind: 'array', state: 'valueOrEnd' });
      return;
    }
    if (char === '}' || char === ']') {
      const object = char === '}';
      if (!parent || (parent.kind === 'object') !== object
        || !(parent.state === 'commaOrEnd' || parent.state === (object ? 'keyOrEnd' : 'valueOrEnd')))
        throw new Error('unexpected closing container in history record');
      stack.pop(); append(char); valueDone(); return;
    }
    if (char === ':') {
      if (parent?.kind !== 'object' || parent.state !== 'colon') throw new Error('unexpected colon in history record');
      parent.state = 'value'; append(char); return;
    }
    if (char === ',') {
      if (parent?.state !== 'commaOrEnd') throw new Error('unexpected comma in history record');
      parent.state = parent.kind === 'object' ? 'key' : 'value'; append(char); return;
    }
    if (!needsValue()) throw new Error('unexpected token in history record');
    if (/^[0-9\-]$/.test(char)) { token = char; mode = 'number'; return; }
    if (char === 't' || char === 'f' || char === 'n') {
      literal = char === 't' ? 'true' : char === 'f' ? 'false' : 'null';
      token = char; mode = 'literal'; return;
    }
    throw new Error('invalid token in history record');
  }

  const consumeText = (text: string) => {
    let consumed = 0;
    for (const char of text) {
      consume(char); offset += bytesOf(char); consumed += char.length;
      if (paused) { pendingText = text.slice(consumed); break; }
    }
  };
  return {
    push(bytes: Uint8Array, from: number) {
      if (retired) throw new Error('history scaffold parser has retired');
      try {
        if (from !== nextInput) throw new Error('history record chunks are not contiguous');
        if (paused || pendingText) throw new Error('previous history text chunk has not been consumed');
        if (bytes.byteLength > 64 * 1024) throw new Error('history record chunk exceeds its byte budget');
        nextInput += bytes.byteLength;
        if (!Number.isSafeInteger(nextInput)) throw new Error('history record offset exceeds the face integer range');
        consumeText(decoder.decode(bytes, { stream: true }));
      } catch (error) { retired = true; throw error; }
    },
    pending: () => paused || pendingText.length > 0,
    resume() {
      if (retired) throw new Error('history scaffold parser has retired');
      paused = false;
      const text = pendingText; pendingText = '';
      try { consumeText(text); } catch (error) { retired = true; throw error; }
    },
    finish(end: number): RecordScaffold {
      if (retired) throw new Error('history scaffold parser has retired');
      retired = true;
      if (paused || pendingText) throw new Error('history text chunk has not been consumed');
      consumeText(decoder.decode());
      if (mode === 'number') closeNumber();
      if (mode !== 'normal' || stack.length || !rootDone || nextInput !== end || offset !== end)
        throw new Error('history record is incomplete');
      parts.push(piece);
      return { json: parts.join(''), strings };
    },
  };
}

export function recordScaffold(start: number, seed: string) {
  return prepareRecord(start, seed);
}

/** The second pass uses this same lexer on one original quoted string.
 * Returning false pauses after one character, retaining at most one IO chunk. */
export function decodeRecordString(start: number, consume: ConsumeText) {
  const reader = prepareRecord(start, 'text', consume, true);
  return { push: reader.push, pending: reader.pending, resume: reader.resume,
    finish(end: number) { reader.finish(end); } };
}
