import type { HistoryPage } from '@/gen/bindings/HistoryPage';
import type { HistoryEntry } from '@/gen/bindings/HistoryEntry';
import type { HistoryRecordRef } from '@/gen/bindings/HistoryRecordRef';
export const HISTORY_CACHE_LIMITS = { pages: 8, entries: 512, bytes: 2 * 1024 * 1024 } as const;
const PAGE_BYTES = 512 * 1024;
export type HistoryDirection = 'tail' | 'older' | 'newer';
export type HistoryWindow = {
  pages: HistoryPage[];
  older: string | null;
  newer: string | null;
  latest: boolean;
};
export const EMPTY_HISTORY_WINDOW: HistoryWindow = { pages: [], older: null, newer: null, latest: true };
/** Filling a short viewport must not evict the live tail. Reserve one maximum
 * page before speculative reads; an explicit reader gesture can move the window. */
export function historyWindowHasRoom(window:HistoryWindow){
  return window.pages.length<HISTORY_CACHE_LIMITS.pages
    &&window.pages.reduce((count,page)=>count+page.entries.length,0)<=HISTORY_CACHE_LIMITS.entries-128
    &&window.pages.reduce((bytes,page)=>bytes+page.encoded_bytes,0)<=HISTORY_CACHE_LIMITS.bytes-PAGE_BYTES;
}
export function canPrefetchOlder(window:HistoryWindow,readingLatest:boolean,readerUp=false){
  return !window.latest||!readingLatest||readerUp||historyWindowHasRoom(window);
}
function referenceKey(reference: HistoryRecordRef) { return `${reference.start}:${reference.item ?? 'record'}`; }
function pageHasContent(page: HistoryPage) { return page.entries.length > 0 || page.references.length > 0; }
function exceedsBudget(pages: HistoryPage[]) {
  return pages.length > HISTORY_CACHE_LIMITS.pages
    || pages.reduce((count, page) => count + page.entries.length, 0) > HISTORY_CACHE_LIMITS.entries
    || pages.reduce((bytes, page) => bytes + page.encoded_bytes, 0) > HISTORY_CACHE_LIMITS.bytes;
}
/** A server may send tiny pages while a reply streams. Pack only proven
 * contiguous body pages; keep wire-size and cache-size bounds unchanged.
 * Reference-only ranges retain their replacement semantics below. */
function packHistoryPages(source:HistoryPage[]):HistoryPage[]{
 const packed:HistoryPage[]=[];
 for(const right of source){
  const left=packed.at(-1);
  if(left&&left.entries.length&&right.entries.length&&left.newer!==null&&left.newer===right.older
   &&left.entries.length+right.entries.length<=128
   &&left.references.length+right.references.length<=256
   &&left.encoded_bytes+right.encoded_bytes<=PAGE_BYTES){
   packed[packed.length-1]={...right,older:left.older,reset:false,
    entries:[...left.entries,...right.entries],references:[...left.references,...right.references],
    encoded_bytes:left.encoded_bytes+right.encoded_bytes};
  }else packed.push(right);
 }
 return packed;
}
/** Admit a bounded wire page before folding or rendering any of its entries. */
export function receiveHistoryPage(window: HistoryWindow, page: HistoryPage, direction: HistoryDirection): HistoryWindow {
  if (!Number.isSafeInteger(page.encoded_bytes) || page.encoded_bytes < 0 || page.encoded_bytes > PAGE_BYTES
    || page.entries.length > 128 || page.references.length > 256)
    throw new Error('History page exceeds its declared budget');
  if (direction === 'tail' || page.reset) {
    return { pages: [page], older: page.older, newer: page.newer, latest: !page.has_more };
  }
  if (!pageHasContent(page)) {
    return { ...window, older: direction === 'older' ? page.older : window.older,
      newer: direction === 'newer' ? page.newer : window.newer,
      latest: direction === 'newer' ? !page.has_more : window.latest };
  }
  let pages = direction === 'older' ? [page, ...window.pages] : [...window.pages, page];
  // A growing, unfinished record replaces its previous reference-only page.
  // Preserve the original backward cursor while updating the pending range.
  if (direction === 'newer' && page.entries.length === 0 && window.pages.length) {
    const last = window.pages.at(-1)!;
    const keys = new Set(page.references.map(referenceKey));
    if (last.entries.length === 0 && last.references.length > 0 && last.references.every(reference => keys.has(referenceKey(reference)))) {
      pages = [...window.pages.slice(0, -1), { ...page, older: last.older }];
    }
  }
  pages=packHistoryPages(pages);
  let evictedNewer = false;
  let evictedOlder = false;
  while (pages.length > 1 && exceedsBudget(pages)) {
    if (direction === 'older') {
      pages.pop();
      evictedNewer = true;
    }
    else {
      pages.shift();
      evictedOlder = true;
    }
  }
  return { pages, older: direction === 'older' ? page.older : evictedOlder ? pages[0]?.older ?? null : window.older,
    newer: direction === 'newer' ? page.newer : evictedNewer ? pages.at(-1)?.newer ?? null : window.newer,
    latest: direction === 'older' ? window.latest && !evictedNewer : !page.has_more };
}
export function historyEntries(window: HistoryWindow): HistoryEntry[] {
  const entries = new Map<string, HistoryEntry>();
  for (const page of window.pages)
    for (const entry of page.entries)
      entries.set(entry.key, entry);
  return [...entries.values()];
}
export function historyReferences(window: HistoryWindow): HistoryRecordRef[] {
  const references = new Map<string, HistoryRecordRef>();
  for (const page of window.pages)
    for (const reference of page.references)
      references.set(referenceKey(reference), reference);
  return [...references.values()].sort((a, b) => a.start - b.start);
}
