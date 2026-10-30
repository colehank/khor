import { decodeRecordString, recordScaffold, type RecordScaffold, type RecordStringRange } from './history-record-scaffold';

type Read = (from: number, end: number) => Promise<Uint8Array>;
type Account = (bytes: number) => Promise<void> | null;
const utf8 = new TextEncoder();

/** The existing normalizer may decode one JSON-encoded input string. Its
 * ranges use decoded UTF-8 offsets inside that one original container. */
export async function* recordString(range: Pick<RecordStringRange, 'start' | 'end' | 'container'>, read: Read, account: Account): AsyncGenerator<string> {
  let char = '';
  const decoder = decodeRecordString(range.start, value => { char = value; return false; });
  const emit = async function* () {
    while (decoder.pending()) {
      const pause = account(utf8.encode(char).length);
      if (pause) await pause;
      yield char;
      char = '';
      decoder.resume();
    }
  };
  if (!range.container) {
    for (let from = range.start; from < range.end;) {
      const bytes = await read(from, range.end);
      decoder.push(bytes, from);
      yield* emit();
      from += bytes.length;
    }
  } else {
    let offset = 0;
    for await (const value of recordString(range.container, read, account)) {
      const bytes = utf8.encode(value);
      if (offset >= range.start && offset < range.end) {
        decoder.push(bytes, offset);
        yield* emit();
      }
      offset += bytes.length;
      if (offset >= range.end) break;
    }
  }
  decoder.finish(range.end);
}

/** Apply only the original parser's selected encoded-input field. JSON.parse
 * here receives the bounded scaffold, never the original large record. */
export async function prepareEncodedInput(scaffold: RecordScaffold, pointer: string, read: Read, account: Account): Promise<{ scaffold: RecordScaffold; complete: boolean }> {
  if (!pointer.startsWith('/') || utf8.encode(pointer).length > 4096) throw new Error('invalid history input projection pointer');
  const path = pointer.slice(1).split('/').map(key => key.replace(/~1/g, '/').replace(/~0/g, '~'));
  if (path.length > 128) throw new Error('history input projection pointer exceeds its depth budget');
  const value = JSON.parse(scaffold.json);
  let parent = value;
  for (const key of path.slice(0, -1)) parent = parent?.[key];
  if (!parent || typeof parent !== 'object') throw new Error('history input projection field is missing');
  const key = path.at(-1)!;
  const input = parent[key];
  const range = scaffold.strings.find(range => input === range.prefix + range.marker);
  // A short literal was already normalized from its exact original value.
  if (!range) return { scaffold, complete: true };
  if (range.container) throw new Error('history input projection exceeds its one encoded layer');
  const parser = recordScaffold(0, crypto.randomUUID());
  let length = 0;
  let piece = '';
  const flush = () => {
    const bytes = utf8.encode(piece); piece = '';
    parser.push(bytes, length); length += bytes.length;
  };
  let failure: unknown;
  // Keep transport failures outside the grammar catch. A replaced source
  // must invalidate results rather than masquerade as invalid embedded JSON.
  for await (const char of recordString(range, read, account)) {
    piece += char;
    if (piece.length >= 4096) {
      try { flush(); } catch (error) { failure = error; break; }
    }
  }
  let inner: RecordScaffold;
  try {
    if (failure) throw failure;
    if (piece) flush();
    inner = parser.finish(length);
  } catch (error) {
    // Invalid syntax uses the original parser's exact string fallback.
    if (/budget|too many|exceeds/.test(String(error))) return { scaffold, complete: false };
    return { scaffold, complete: true };
  }
  parent[key] = inner.json;
  const json = JSON.stringify(value);
  const strings = [...scaffold.strings.filter(other => other !== range),
    ...inner.strings.map(string => ({ ...string, container: { start: range.start, end: range.end } }))];
  if (utf8.encode(json).length > 64 * 1024 || strings.length > 128) return { scaffold, complete: false };
  return { scaffold: { json, strings }, complete: true };
}

/** Rust str::trim uses Unicode White_Space (not JavaScript trim's BOM
 * rule). Only one visible output prefix and pending whitespace are held. */
export async function limitedToolOutput(chars: AsyncIterable<string>, maxBytes: number, suffix: string): Promise<string> {
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 0 || maxBytes > 64 * 1024 || utf8.encode(suffix).length > 1024)
    throw new Error('invalid history tool output policy');
  let text = '', waiting = '';
  let bytes = 0, waitingBytes = 0;
  let started = false;
  let overflow = false;
  const append = (char: string) => {
    const size = utf8.encode(char).length;
    if (bytes + size > maxBytes) return false;
    bytes += size; text += char; return true;
  };
  for await (const char of chars) {
    if (/^\p{White_Space}$/u.test(char)) {
      if (started) {
        const size = utf8.encode(char).length;
        if (!overflow && bytes + waitingBytes + size <= maxBytes) { waiting += char; waitingBytes += size; }
        else overflow = true;
      }
      continue;
    }
    started = true;
    for (const white of waiting) append(white);
    waiting = ''; waitingBytes = 0;
    if (overflow || !append(char)) return text + suffix;
  }
  return text;
}
