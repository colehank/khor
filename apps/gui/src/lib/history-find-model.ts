import type { HistoryPage } from '@/gen/bindings/HistoryPage';
import type { HistoryItem } from './history-paint';
import { paintHistory, historyReferenceBetween } from './history-paint';
import { toolInputText, type AcpUpdate } from './acp-view';

export const FIND_HITS = 32;
export const FIND_QUERY_BYTES = 256;
export const FIND_SNIPPET_BYTES = 512;
const PAGE_BYTES = 512 * 1024;
const RESULT_BYTES = 64 * 1024;
const utf8 = new TextEncoder();

export type FindQuery = { text: string; caseSensitive: boolean };
export type FindHit = {
  before: string;
  entry: string;
  part: number;
  from: number;
  to: number;
  snippet: string;
  /** Complete available message context, not execution completion. */
  complete?: boolean;
  atTail?: boolean;
  /** Match continues into the immediately newer canonical page. */
  continued?: boolean;
  record?: import('@/gen/bindings/HistoryRecordRef').HistoryRecordRef;
};
export type FindWorkerInput =
  | { kind: 'query'; query: FindQuery; language: 'zh' | 'en' }
  | { kind: 'page'; page: HistoryPage }
  | { kind: 'next' }
  | { kind: 'io'; id: number; result?: unknown; error?: string }
  | { kind: 'action'; id: number; action: import('./history-find-action').HistoryFindAction; hit: FindHit };
export type FindWorkerOutput =
  | { kind: 'read_page'; id: number; before: string; newer?: boolean }
  | { kind: 'action_result'; id: number; answer: import('./history-find-action').HistoryActionAnswer }
  | { kind: 'action_error'; id: number; code: 'too_large' | 'incomplete' | 'source_changed' | 'read_failed'; error?: string }
  | { kind: 'read'; id: number; cursor: string; from: number }
  | { kind: 'project'; id: number; cursor: string; start: number; end: number; scaffold: string | null }
  | { kind: 'result'; hits: FindHit[]; pageDone: boolean; older: string | null; unsearchedRanges: number }
  | { kind: 'error'; error: string };

export function findQuery(query: FindQuery): FindQuery {
  if (typeof query.text !== 'string' || !query.text || utf8.encode(query.text).length > FIND_QUERY_BYTES)
    throw new Error('history search query exceeds its text budget');
  if ([...query.text].some(char => char.length === 1 && char.charCodeAt(0) >= 0xd800 && char.charCodeAt(0) <= 0xdfff))
    throw new Error('history search query is not valid Unicode');
  if (typeof query.caseSensitive !== 'boolean') throw new Error('invalid history search options');
  return { text: query.text, caseSensitive: query.caseSensitive };
}

/** Search and message copying consume projected items, never JSON keys,
 * encoded attachments, or a mounted DOM subtree. Projection remains paintHistory's. */
export function historyText(item: HistoryItem): { texts: string[]; complete: boolean; outputPart?: number } {
  if ('text' in item) return { texts: [item.text], complete: true };
  if (item.who === 'compaction') return { texts: [item.summary, ...(item.error ? [item.error] : [])], complete: true };
  if (item.who === 'resource') return { texts: [item.name, ...(item.uri ? [item.uri] : [])], complete: true };
  if (item.who === 'tool') {
    const diffs = item.diffs ?? (item.diff ? [item.diff] : []);
    const patches = item.patches ?? [];
    const input = !diffs.length && !patches.length && item.raw !== null ? toolInputText(item.raw, 64 * 1024) : null;
    return { texts: [
    item.title,
    ...(input?.text ? [input.text] : []),
    ...(item.out ? [item.out] : []),
    ...diffs.flatMap(diff => [diff.path, ...(diff.old ? [diff.old] : []), diff.next]),
    ...patches.flatMap(patch => [patch.path, patch.text]),
    ], complete: input?.complete !== false, ...(item.out ? { outputPart: 1 + (input?.text ? 1 : 0) } : {}) };
  }
  return { texts: [], complete: true };
}

/** The streaming form also works across decoded raw-string chunks. Only
 * query-length origin offsets are retained, including lowercase expansions. */
export function literalMatcher(raw: FindQuery) {
  const query = findQuery(raw);
  // Default (non-locale) lowercasing, with the two sigma forms treated as
  // one letter across chunk boundaries. This is not Unicode normalization.
  const fold = (char: string) => query.caseSensitive ? char : char.toLowerCase().replace(/ς/g, 'σ');
  const pattern = [...query.text].flatMap(char => [...fold(char)]);
  const prefix = new Array<number>(pattern.length).fill(0);
  for (let i = 1, matched = 0; i < pattern.length; i++) {
    while (matched && pattern[i] !== pattern[matched]) matched = prefix[matched - 1];
    if (pattern[i] === pattern[matched]) matched++;
    prefix[i] = matched;
  }
  let matched = 0;
  let position = 0;
  let serial = 0;
  const origins = new Array<number>(pattern.length);
  return {
    contextUnits: pattern.length * 2,
    checkpoint: () => ({ matched, position, serial, origins: origins.slice() }),
    restore(saved: { matched: number; position: number; serial: number; origins: number[] }) {
      matched = saved.matched; position = saved.position; serial = saved.serial;
      for (let index = 0; index < origins.length; index++) origins[index] = saved.origins[index];
    },
    feed(text: string, limit = FIND_HITS) {
      const ranges: { from: number; to: number }[] = [];
      let consumed = 0;
      for (const char of text) {
        const start = position;
        position += char.length;
        consumed += char.length;
        for (const unit of fold(char)) {
          origins[serial % pattern.length] = start;
          serial++;
          while (matched && unit !== pattern[matched]) matched = prefix[matched - 1];
          if (unit === pattern[matched]) matched++;
          if (matched === pattern.length) {
            ranges.push({ from: origins[(serial - pattern.length) % pattern.length], to: position });
            matched = 0; // Match the existing Find's non-overlapping steps.
          }
        }
        if (ranges.length >= limit) break;
      }
      return { ranges, consumed };
    },
  };
}

/** Coverage uses the same painted groups and trusted record positions. A
 * clipped group remains searchable, but cannot claim whole-message copying. */
export function historyPartComplete(items: HistoryItem[], index: number, page: HistoryPage, atTail: boolean): boolean {
  const item = items[index];
  const first = page.entries.find(entry => entry.key === item.historyEntries[0]);
  const firstRecord = item.historyRecords[0]?.record, lastRecord = item.historyRecords.at(-1)?.record;
  const from = page.entries.findIndex(entry => entry.key === item.historyEntries[0]);
  const to = page.entries.findIndex(entry => entry.key === item.historyEntries.at(-1));
  const boundary = (entry: HistoryPage['entries'][number]) => entry.frame.kind === 'turn' || entry.frame.kind === 'capture_gap';
  const boundaryBefore = from > 0 && page.entries.slice(0, from).some(boundary);
  const boundaryAfter = to >= 0 && page.entries.slice(to + 1).some(boundary);
  const previous = items[index - 1]?.historyRecords.at(-1)?.record;
  const next = items[index + 1]?.historyRecords[0]?.record;
  const unknownBefore = page.references.some(reference => !firstRecord
    || historyReferenceBetween(reference, previous, firstRecord));
  const unknownAfter = page.references.some(reference => !lastRecord
    || historyReferenceBetween(reference, lastRecord, next));
  const starts = first?.starts_message || (!unknownBefore && (index > 0 || boundaryBefore || page.older === null));
  const singleComplete = first?.starts_message && item.historyEntries.length === 1;
  const ends = !unknownAfter && (singleComplete || index < items.length - 1 || boundaryAfter || atTail);
  return starts && ends;
}

function snippet(text: string, from: number, to: number): string {
  let start = Math.max(0, from - 80);
  let end = Math.min(text.length, to + 80);
  const low = (at: number) => text.charCodeAt(at) >= 0xdc00 && text.charCodeAt(at) <= 0xdfff;
  if (low(start)) start++;
  if (end < text.length && low(end)) end--;
  const context = text.slice(start, end);
  let answer = '';
  let bytes = 0;
  for (const char of context) {
    const size = utf8.encode(char).length;
    if (bytes + size > FIND_SNIPPET_BYTES) break;
    answer += char;
    bytes += size;
  }
  return answer;
}

type TextPrefix = { entries: HistoryPage['entries']; references: HistoryPage['references'] };
/** Retain only the first projected text group's query-length prefix. The
 * shared painter decides grouping; this copies canonical text DTO facts only.
 * Source metadata remains bounded by one page, not a history-wide key map. */
function textPrefix(page: HistoryPage, newer: TextPrefix | null, units: number): TextPrefix | null {
  const entries = [...page.entries, ...(newer?.entries ?? [])];
  const references = [...page.references, ...(newer?.references ?? [])];
  const first = paintHistory(entries, references)[0];
  if (!first || !['agent', 'user', 'thought'].includes(first.who) || !('text' in first)) return null;
  const keys = new Set(first.historyEntries);
  const kept: HistoryPage['entries'] = [];
  let left = units;
  for (const entry of entries) {
    if (!keys.has(entry.key)) {
      if (!kept.length && entry.frame.kind === 'turn') kept.push(entry);
      continue;
    }
    if (entry.frame.kind !== 'note') return null;
    const carried = entry.frame.update as { update?: AcpUpdate } | null;
    const update = carried?.update;
    if (!update || Array.isArray(update.content) || update.content?.type !== 'text' || typeof update.content.text !== 'string') return null;
    let text = '';
    for (const char of update.content.text) {
      if (left <= 0) break;
      text += char; left -= char.length;
    }
    kept.push({...entry, frame: {kind: 'note', update: {update: {
      sessionUpdate: update.sessionUpdate, content: {type: 'text', text},
      ...(typeof update.messageId === 'string' ? {messageId: update.messageId} : {}),
    }}}});
    if (left <= 0) break;
  }
  const lastRecord = kept.at(-1)?.record;
  if (references.length && !lastRecord) return null;
  const answer = {entries: kept, references: lastRecord
    ? references.filter(reference => historyReferenceBetween(reference, undefined, lastRecord)) : []};
  return kept.length <= 512 && utf8.encode(JSON.stringify(answer)).length <= PAGE_BYTES ? answer : null;
}

/** A worker holds one server-bounded page plus a query-length text prefix
 * with page-bounded metadata. Thirty-two hits resume within the same page. */
export function historyFindBatch(raw: FindQuery) {
  const query = findQuery(raw);
  const matcherContext = literalMatcher(query).contextUnits;
  let current: HistoryPage | null = null;
  let items: HistoryItem[] = [];
  let item = 0;
  let part = 0;
  let offset = 0;
  let matcher = literalMatcher(query);
  let projection: ReturnType<typeof historyText> | null = null;
  let unsearchedInputs = 0;
  let firstPage = true, atTail = false;
  let prefix: TextPrefix | null = null;
  let seam: { text: string; entry: string; before: string; split: number; offset: number; matcher: ReturnType<typeof literalMatcher> } | null = null;
  function next(): Extract<FindWorkerOutput, { kind: 'result' }> {
    if (!current) throw new Error('history search has no pending page');
    const page = current;
    const hits: FindHit[] = [];
    let hitBytes = 0;
    while (item < items.length && hits.length < FIND_HITS) {
      if (!projection) {
        projection = historyText(items[item]);
        if (!projection.complete || !historyPartComplete(items, item, page, atTail)) unsearchedInputs++;
      }
      const texts = projection.texts;
      if (part >= texts.length) {
        item++; part = 0; offset = 0; matcher = literalMatcher(query); projection = null;
        continue;
      }
      const text = texts[part];
      const saved = matcher.checkpoint();
      const result = matcher.feed(text.slice(offset), 1);
      const entry = items[item].historyEntries[0];
      const proposed = result.ranges.map(range => ({ before: page.newer,
        entry, part, ...range,
        complete: projection!.complete && historyPartComplete(items, item, page, atTail), atTail,
        snippet: snippet(text, range.from, range.to) }));
      const bytes = proposed.reduce((sum, hit) => sum + utf8.encode(JSON.stringify(hit)).length + 1, 0);
      if (hitBytes + bytes > RESULT_BYTES - 16 * 1024) {
        matcher.restore(saved);
        break;
      }
      hitBytes += bytes;
      hits.push(...proposed);
      offset += result.consumed;
      if (offset === text.length) {
        part++; offset = 0; matcher = literalMatcher(query);
      }
    }
    while (item >= items.length && seam && hits.length < FIND_HITS) {
      const saved = seam.matcher.checkpoint();
      const result = seam.matcher.feed(seam.text.slice(seam.offset), 1);
      const proposed = result.ranges.filter(range => range.from < seam!.split && range.to > seam!.split)
        .map(range => ({before: seam!.before, entry: seam!.entry, part: 0, ...range,
          continued: true, complete: false, atTail: false, snippet: snippet(seam!.text, range.from, range.to)}));
      const bytes = proposed.reduce((sum, hit) => sum + utf8.encode(JSON.stringify(hit)).length + 1, 0);
      if (hitBytes + bytes > RESULT_BYTES - 16 * 1024) { seam.matcher.restore(saved); break; }
      hitBytes += bytes; hits.push(...proposed); seam.offset += result.consumed;
      if (seam.offset >= seam.text.length) seam = null;
    }
    const pageDone = item >= items.length && !seam;
    if (pageDone) { current = null; items = []; projection = null; }
    const answer: Extract<FindWorkerOutput, { kind: 'result' }> = { kind: 'result', hits, pageDone, older: page.older,
      unsearchedRanges: pageDone ? page.references.length + unsearchedInputs : 0 };
    if (utf8.encode(JSON.stringify(answer)).length > RESULT_BYTES) throw new Error('history search result exceeds its byte budget');
    return answer;
  }
  return {
    page(page: HistoryPage): Extract<FindWorkerOutput, { kind: 'result' }> {
      if (current) throw new Error('history search page has not been consumed');
      if (page.entries.length > 128 || page.encoded_bytes > PAGE_BYTES || utf8.encode(JSON.stringify(page)).length > PAGE_BYTES)
        throw new Error('history search page exceeds the reader budget');
      if (page.entries.some(entry => utf8.encode(entry.key).length > 128))
        throw new Error('history search entry key exceeds its metadata budget');
      if (utf8.encode(page.newer).length > 4096 || (page.older !== null && utf8.encode(page.older).length > 4096))
        throw new Error('history search source cursor exceeds its metadata budget');
      current = page;
      items = paintHistory(page.entries, page.references);
      seam = null;
      if (prefix && items.length) {
        const left = items.at(-1)!;
        const right = paintHistory(prefix.entries, prefix.references)[0];
        if (right && 'text' in left && 'text' in right && ['agent','user','thought'].includes(left.who)) {
          const joined = paintHistory([...page.entries, ...prefix.entries], [...page.references, ...prefix.references])
            .find(item => item.historyEntries.includes(left.historyEntries.at(-1)!) && item.historyEntries.includes(right.historyEntries[0]));
          if (joined && 'text' in joined && joined.text.length > left.text.length) {
            const start = Math.max(0, left.text.length - matcherContext);
            const matcher = literalMatcher(query);
            // Prime offsets without retaining old body in the matcher. Its
            // query-sized origin ring is sufficient for the page seam.
            matcher.restore({...matcher.checkpoint(), position: start});
            seam = {text: joined.text, entry: left.historyEntries[0], before: page.newer,
              split: left.text.length, offset: start, matcher};
          }
        }
      }
      prefix = textPrefix(page, prefix, matcherContext);
      atTail = firstPage; firstPage = false;
      item = 0; part = 0; offset = 0; matcher = literalMatcher(query); projection = null; unsearchedInputs = 0;
      return next();
    },
    next,
  };
}
