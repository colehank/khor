import type { HistoryEntry } from '@/gen/bindings/HistoryEntry';
import type { HistoryRecordRef } from '@/gen/bindings/HistoryRecordRef';
import type { historySource } from './history-source';
import { historyText, literalMatcher, type FindHit, type FindQuery } from './history-find-model';
import { paintHistory } from './history-paint';
import { recordString, prepareEncodedInput, limitedToolOutput } from './history-record-text';
import type { HistoryProjection } from '@/gen/bindings/HistoryProjection';
import type { HistoryTextTransform } from '@/gen/bindings/HistoryTextTransform';
import { recordScaffold, type RecordStringRange } from './history-record-scaffold';

type IO = Pick<ReturnType<typeof historySource>, 'record' | 'project'>;
type Segment = { text: string } | { range: RecordStringRange; escaped: boolean };
const utf8 = new TextEncoder();
const CHUNK_BYTES = 64 * 1024;

/** These are templates selected by the ordinary renderer projection. Static
 * separators remain in the stream, so a match may cross content blocks. */
function segments(text: string, ranges: RecordStringRange[]): Segment[] | null {
  const parts: Segment[] = [];
  let from = 0;
  while (from < text.length) {
    let next: { at: number; length: number; prefix: string; range: RecordStringRange; escaped: boolean } | null = null;
    for (const range of ranges) {
      for (const escaped of [false, true]) {
        const marker = escaped ? JSON.stringify(range.marker).slice(1, -1) : range.marker;
        const at = text.indexOf(marker, from);
        if (at < 0 || (next && at >= next.at)) continue;
        next = { at, length: marker.length, prefix: escaped ? JSON.stringify(range.prefix).slice(1, -1) : range.prefix, range, escaped };
      }
    }
    if (!next) { parts.push({ text: text.slice(from) }); break; }
    const start = next.at - next.prefix.length;
    // A normalizer changed the prefix (e.g. trim). Its explicit transform
    // policy must handle that case; this direct path cannot claim coverage.
    if (start < from || text.slice(start, next.at) !== next.prefix) return null;
    if (start > from) parts.push({ text: text.slice(from, start) });
    parts.push({ range: next.range, escaped: next.escaped });
    from = next.at + next.length;
  }
  return parts;
}

/** At most one original chunk is cached. All scans, including a second
 * string pass, count toward the same 256KiB cooperative scheduling budget. */
function recordReader(io: IO, ref: HistoryRecordRef) {
  let cached: { offset: number; bytes: Uint8Array } | null = null;
  let scheduledBytes = 0;
  const account = (bytes: number): Promise<void> | null => {
    scheduledBytes += bytes;
    if (scheduledBytes < 256 * 1024) return null;
    scheduledBytes = 0;
    return new Promise(resolve => setTimeout(resolve, 0));
  };
  return {
    account,
    async read(from: number, end: number) {
      if (!(cached && from >= cached.offset && from < cached.offset + cached.bytes.length)) {
        const chunk = await io.record(ref.cursor, from);
        if (!chunk) throw new Error('history search read was cancelled');
        if (chunk.offset !== from || typeof chunk.data !== 'string' || chunk.data.length > 4 * Math.ceil(CHUNK_BYTES / 3))
          throw new Error('history search record reply exceeds its range or byte budget');
        const encoded = atob(chunk.data);
        if (!encoded.length || encoded.length > CHUNK_BYTES || from + encoded.length > ref.end)
          throw new Error('history search record reply did not advance within its range');
        if (chunk.next !== (from + encoded.length < ref.end ? from + encoded.length : null))
          throw new Error('history search record continuation does not match its bytes');
        cached = { offset: from, bytes: Uint8Array.from(encoded, char => char.charCodeAt(0)) };
        const pause = account(cached.bytes.length);
        if (pause) await pause;
      }
      const bytes = cached.bytes.subarray(from - cached.offset, Math.min(end - cached.offset, cached.bytes.length));
      const pause = account(bytes.length);
      if (pause) await pause;
      return bytes;
    },
  };
}

/** A single record search is resumable at each hit. No complete raw JSON,
 * decoded body, or unbounded hit list is assembled in the worker. */
export type RecordTextPart = { entry: string; part: number; before: string; record: HistoryRecordRef; complete: boolean; text: AsyncIterable<string> };
async function projectRecord(ref: HistoryRecordRef, before: string, io: IO) {
  if (!Number.isSafeInteger(ref.start) || !Number.isSafeInteger(ref.end) || ref.start < 0 || ref.end <= ref.start
    || utf8.encode(ref.cursor).length > 4096 || utf8.encode(before).length > 4096)
    throw new Error('invalid history search record range');
  const joined = await io.project(ref.cursor, ref.start, ref.end, null);
  if (!joined) throw new Error('history search projection was cancelled');
  if (joined.record.start !== ref.start || joined.record.end !== ref.end || utf8.encode(joined.record.cursor).length > 4096)
    throw new Error('history search projection changed its requested range');
  const record = { ...joined.record, item: ref.item };
  const { read, account } = recordReader(io, record);
  const lexer = recordScaffold(ref.start, crypto.randomUUID());
  for (let from = ref.start; from < ref.end;) {
    const bytes = await read(from, ref.end);
    try { lexer.push(bytes, from); } catch { return null; }
    from += bytes.length;
  }
  let scaffold: ReturnType<typeof lexer.finish>;
  try { scaffold = lexer.finish(ref.end); } catch { return null; }
  // Only retain known physical boundaries supplied by the source (or its
  // contiguous-fragment join). Valid JSON alone never proves a JSONL boundary.
  if (!ref.missing_start && !ref.missing_end) {
    record.missing_start = false;
    record.missing_end = false;
  }
  const project = async (json: string) => {
    const answer = await io.project(record.cursor, ref.start, ref.end, json);
    if (!answer) throw new Error('history search projection was cancelled');
    if (answer.record.start !== ref.start || answer.record.end !== ref.end || answer.record.cursor !== record.cursor
      || answer.entries.length > 128 || utf8.encode(JSON.stringify(answer)).length > 512 * 1024
      || answer.entries.some((entry: HistoryEntry) => entry.frame.kind !== 'note' || !/^projection:\d+$/.test(entry.key))
      || answer.transforms.length > 128 || utf8.encode(JSON.stringify(answer.transforms)).length > 16 * 1024)
      throw new Error('invalid history search body projection');
    return answer as HistoryProjection;
  };
  let projected = await project(scaffold.json);
  const encoded = projected.transforms.filter((fact: HistoryTextTransform) => fact.kind === 'json_string');
  if (encoded.length > 1) throw new Error('history projection exceeds its encoded input budget');
  if (encoded[0]?.kind === 'json_string' && !encoded[0].parsed) {
    const prepared = await prepareEncodedInput(scaffold, encoded[0].pointer, read, account);
    if (!prepared.complete) return null;
    if (prepared.scaffold !== scaffold) {
      const normalized = await project(prepared.scaffold.json);
      // The original parser decides whether an encoded value is accepted
      // (including numeric range rules). Its fallback keeps the original text.
      if (normalized.transforms.some((fact: HistoryTextTransform) => fact.kind === 'json_string' && fact.pointer === encoded[0].pointer && fact.parsed)) {
        scaffold = prepared.scaffold;
        projected = normalized;
      }
    }
  }
  return { record, read, account, scaffold, projected };
}

/** A complete physical record can prove the next body starts here without
 * decoding its text. Unknown/partial records cannot end a selected message. */
export async function recordStartsMessage(ref: HistoryRecordRef, before: string, io: IO): Promise<boolean> {
  if (ref.missing_start || ref.missing_end) return false;
  const prepared = await projectRecord(ref, before, io);
  if (!prepared || !prepared.projected.complete) return false;
  const entries = ref.item === null ? prepared.projected.entries
    : prepared.projected.entries.filter(entry => entry.key === `projection:${ref.item}`);
  return entries[0]?.starts_message === true;
}

export async function* recordText(
  ref: HistoryRecordRef, before: string, io: IO,
): AsyncGenerator<RecordTextPart, boolean> {
  const prepared = await projectRecord(ref, before, io);
  if (!prepared) return false;
  const { record, read, account, scaffold, projected } = prepared;
  let complete = projected.complete;
  const entries = ref.item === null ? projected.entries
    : projected.entries.filter((entry: HistoryEntry) => entry.key === `projection:${ref.item}`);
  if (ref.item !== null && !entries.length) complete = false;
  for (const item of paintHistory(entries)) {
    const body = historyText(item);
    complete &&= body.complete;
    for (const [part, text] of body.texts.entries()) {
      const template = segments(text, scaffold.strings);
      if (!template) { complete = false; continue; }
      const characters = async function* () {
        for (const segment of template) {
          if ('text' in segment) { yield* segment.text; continue; }
          for await (const char of recordString(segment.range, read, account))
            yield* segment.escaped ? JSON.stringify(char).slice(1, -1) : char;
        }
      };
      const output = body.outputPart === part ? projected.transforms.find((fact: HistoryTextTransform) => fact.kind === 'tool_output'
        && fact.item === Number(item.historyKey.split(':')[1])) : undefined;
      const visible = output?.kind === 'tool_output'
        ? await limitedToolOutput(characters(), output.max_bytes, output.suffix) : null;
      const consume = async function* () { if (visible !== null) yield* visible; else yield* characters(); };
      const textStream = async function* () {
        for await (const char of consume()) {
          const pause = account(utf8.encode(char).length);
          if (pause) await pause;
          yield char;
        }
      };
      yield { before, entry: `${ref.start}:${item.historyKey.split(':')[1]}`, part, record,
        complete: projected.complete && body.complete, text: textStream() };
    }
  }
  return complete;
}

/** Matching and copying consume exactly the same selected, normalized body. */
export async function* findRecord(query: FindQuery, ref: HistoryRecordRef, before: string, io: IO): AsyncGenerator<FindHit, boolean> {
  const bodies = recordText(ref, before, io);
  try {
    while (true) {
      const next = await bodies.next();
      if (next.done) return next.value;
      const body = next.value;
      const matcher = literalMatcher(query);
      let context = '';
      for await (const char of body.text) {
        context = (context + char).slice(-160);
        if (/^[\uDC00-\uDFFF]/.test(context)) context = context.slice(1);
        const result = matcher.feed(char, 1).ranges[0];
        if (result) yield { before: body.before, entry: body.entry, part: body.part, ...result,
          snippet: context, record: body.record };
      }
    }
  } finally { await bodies.return(false); }
}
