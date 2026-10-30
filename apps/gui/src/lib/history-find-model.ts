import type { HistoryPage } from '@/gen/bindings/HistoryPage';
import type { HistoryItem } from './history-paint';
import { appendHistoryEntry, historyReferenceBetween, type HistoryPaintStep } from './history-paint';
import { toolInputText, type AcpUpdate } from './acp-view';
import type { HistoryReplay, HistoryContext } from './history-find-stream';

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
  replay?: HistoryReplay;
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
    feed(text: string, limit = FIND_HITS, collect = true) {
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
            if (collect) ranges.push({ from: origins[(serial - pattern.length) % pattern.length], to: position });
            matched = 0; // Match the existing Find's non-overlapping steps.
          }
        }
        if (collect && ranges.length >= limit) break;
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

export function findSnippet(text: string, from: number, to: number): string {
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

/** A clipped leading group cannot be matched at a guessed non-overlap phase.
 * Keep only its authenticated bounds until the earlier true start is known. */
function plainHistoryText(item: HistoryItem): boolean {
  return ['agent','user','thought'].includes(item.who) && 'text' in item;
}
function replayStart(item: HistoryItem, page: HistoryPage): Pick<HistoryReplay,'before'|'forward'|'first'> {
  const record=page.entries[0]?.record;
  return {before:record?.cursor ?? page.newer,forward:record!==null && record!==undefined,first:item.historyEntries[0]};
}
type PendingReplay = HistoryReplay & {head: HistoryPage['entries'][number] | null};
function prefixFact(item: HistoryItem, page: HistoryPage): PendingReplay['head'] {
  const entry=page.entries.find(entry=>entry.key===item.historyEntries[0]);
  if(!entry || entry.frame.kind!=='note')return null;
  const update=(entry.frame.update as {update?:AcpUpdate})?.update;
  if(!update || Array.isArray(update.content) || update.content?.type!=='text')return null;
  // This one-character canonical fact probes only the shared fold's identity
  // decision. It is not a body copy or an independently implemented rule.
  return {...entry,record:null,frame:{kind:'note',update:{update:{sessionUpdate:update.sessionUpdate,
    content:{type:'text',text:'x'},...(typeof update.messageId==='string'?{messageId:update.messageId}:{})}}}};
}
function joinsPrefix(left: HistoryItem, head: PendingReplay['head']): boolean {
  if(!head)return true; // Unknown fact is deferred, never assumed separate.
  const probe: HistoryPaintStep[]=[{...left,text:'',historyEntries:left.historyEntries.slice(-1),historyRecords:[]} as HistoryItem];
  appendHistoryEntry(probe,head);
  return probe.length===1;
}
function replayBounds(pending: PendingReplay, completeStart: boolean): HistoryReplay {
  const {head: _head,...range}=pending;return {...range,completeStart};
}
function knownHead(item: HistoryItem, index: number, page: HistoryPage, items: HistoryItem[]): boolean {
  const at = page.entries.findIndex(entry => entry.key === item.historyEntries[0]);
  const preceding=items.slice(0,index).some(prior=>prior.historyEntries.some(key=>{
    const position=page.entries.findIndex(entry=>entry.key===key);return position>=0 && position<at;
  }));
  return preceding || page.older === null || page.entries[at]?.starts_message === true
    || page.entries.slice(0,Math.max(0,at)).some(entry => entry.frame.kind === 'turn' || entry.frame.kind === 'capture_gap');
}

export function historyFindBatch(raw: FindQuery) {
  const query = findQuery(raw);
  let current: HistoryPage | null = null;
  let items: HistoryItem[] = [], painted: HistoryItem[] = [];
  let item = 0;
  let part = 0;
  let offset = 0;
  let matcher = literalMatcher(query);
  let projection: ReturnType<typeof historyText> | null = null;
  let unsearchedInputs = 0;
  let firstPage = true, atTail = false;
  let ceiling: HistoryReplay['ceiling']={entry:null,end:null,unknown:true};
  let pending: PendingReplay | null = null, replay: HistoryReplay | null = null;
  function next(): Extract<FindWorkerOutput, { kind: 'result' }> {
    if (!current) throw new Error('history search has no pending page');
    const page = current;
    const hits: FindHit[] = [];
    let hitBytes = 0;
    while (item < items.length && hits.length < FIND_HITS) {
      if (!projection) {
        projection = historyText(items[item]);
        if (!projection.complete || (!plainHistoryText(items[item]) && !historyPartComplete(painted, painted.indexOf(items[item]), page, atTail))) unsearchedInputs++;
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
        complete: projection!.complete && historyPartComplete(painted, painted.indexOf(items[item]), page, atTail), atTail,
        ...(plainHistoryText(items[item]) ? {replay:{...replayStart(items[item],page),endBefore:page.newer,
          last:items[item].historyEntries.at(-1)!,ceiling,completeStart:true}} : {}),
        snippet: findSnippet(text, range.from, range.to) }));
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
    const pageDone = item >= items.length;
    if (pageDone) { current = null; items = []; projection = null; }
    const answer: Extract<FindWorkerOutput, { kind: 'result' }> = { kind: 'result', hits, pageDone, older: page.older,
      unsearchedRanges: pageDone ? page.references.length + unsearchedInputs : 0 };
    if (utf8.encode(JSON.stringify(answer)).length > RESULT_BYTES) throw new Error('history search result exceeds its byte budget');
    return answer;
  }
  return {
    page(page: HistoryPage, context: HistoryContext = {facts:[],complete:true}): Extract<FindWorkerOutput, { kind: 'result' }> {
      if (current || replay) throw new Error('history search page has not been consumed');
      if (page.entries.length > 128 || page.encoded_bytes > PAGE_BYTES || utf8.encode(JSON.stringify(page)).length > PAGE_BYTES)
        throw new Error('history search page exceeds the reader budget');
      if (page.entries.some(entry => utf8.encode(entry.key).length > 128))
        throw new Error('history search entry key exceeds its metadata budget');
      if (utf8.encode(page.newer).length > 4096 || (page.older !== null && utf8.encode(page.older).length > 4096))
        throw new Error('history search source cursor exceeds its metadata budget');
      current = page;
      if(firstPage){
        const last=page.entries.at(-1), record=last?.record;
        ceiling={entry:last?.key ?? null,end:record?.end ?? (page.references.length ? Math.max(...page.references.map(ref=>ref.end)) : null),
          unknown:page.references.some(reference=>!record || historyReferenceBetween(reference,record,null))};
      }
      const out: HistoryPaintStep[]=[...context.facts];
      let previous: import('@/gen/bindings/HistoryRecordRef').HistoryRecordRef | null=null;
      for(const entry of page.entries)previous=appendHistoryEntry(out,entry,page.references,previous);
      painted=out.filter((item):item is HistoryItem=>item.who!=='boundary' && 'historyKey' in item);
      if(!context.complete){
        // Budget exhaustion is explicit partial coverage, not a guessed orphan
        // boundary or a provisional hit with the wrong non-overlap phase.
        pending=null;current=null;items=[];projection=null;
        return {kind:'result',hits:[],pageDone:true,older:page.older,unsearchedRanges:page.references.length+1};
      }
      // A confirmed older tool/compaction is updated in place. Its page-local
      // update is not a newly inserted chronological body boundary.
      const logical=painted.filter(item=>!context.facts.some(fact=>
        (fact.who==='tool' || fact.who==='compaction') && item.who===fact.who && 'id' in item && item.id===fact.id));
      const omitted = new Set<number>();
      if (pending) {
        const last = logical.at(-1), index = last ? painted.indexOf(last) : -1;
        if (last && plainHistoryText(last)) {
          const lastAt=page.entries.findIndex(entry=>entry.key===last.historyEntries.at(-1));
          const separated=page.entries.slice(lastAt+1).some(entry=>entry.frame.kind==='turn' || entry.frame.kind==='capture_gap');
          if(separated || !joinsPrefix(last,pending.head)) {replay=replayBounds(pending,true);pending=null;}
          else {
            pending = {...pending,...replayStart(last,page),head:prefixFact(last,page)};
            omitted.add(index);
            if (knownHead(last,logical.length-1,page,logical)) {replay=replayBounds(pending,true);pending=null;}
          }
        } else if (logical.length || page.older === null || page.references.length
          || page.entries.some(entry=>entry.frame.kind==='turn' || entry.frame.kind==='capture_gap')) {
          replay=replayBounds(pending,page.references.length===0);pending=null;
        }
      }
      const firstLogical=logical.findIndex(plainHistoryText), first=logical[firstLogical], firstIndex=first ? painted.indexOf(first) : -1;
      if (first && !omitted.has(firstIndex) && !knownHead(first,firstLogical,page,logical)) {
        pending={...replayStart(first,page),endBefore:page.newer,
          last:first.historyEntries.at(-1)!,ceiling,completeStart:false,head:prefixFact(first,page)};
        omitted.add(firstIndex);
      }
      items = painted.filter((_item,index)=>!omitted.has(index));
      atTail = firstPage; firstPage = false;
      item = 0; part = 0; offset = 0; matcher = literalMatcher(query); projection = null; unsearchedInputs = 0;
      return next();
    },
    next,
    hasReplay: () => replay !== null,
    takeReplay() {
      if (current) throw new Error('normal history page has not been consumed');
      const result=replay;replay=null;return result;
    },
  };
}
