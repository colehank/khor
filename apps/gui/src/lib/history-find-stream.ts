import type { HistoryPage } from '@/gen/bindings/HistoryPage';
import type { HistoryRecordRef } from '@/gen/bindings/HistoryRecordRef';
import type { historySource } from './history-source';
import { appendHistoryEntry, historyReferenceBetween, type HistoryItem, type HistoryPaintStep } from './history-paint';
import { literalMatcher, findSnippet, type FindHit, type FindQuery } from './history-find-model';

/** These are source-validated cursors and exact opaque contribution keys,
 * scoped to this query's one Worker/historySource lifetime, never persisted. */
export type HistoryReplay = { before: string; forward: boolean; first: string; endBefore: string; last: string; completeStart: boolean };
type IO = Pick<ReturnType<typeof historySource>, 'page'>;
const utf8 = new TextEncoder();
export function checkedReplayPage(page: HistoryPage | null): HistoryPage {
  if (!page) throw new Error('history replay was cancelled');
  if (page.reset) throw new Error('history replay source changed');
  if (page.entries.length > 128 || page.references.length > 256 || page.encoded_bytes > 512 * 1024
    || utf8.encode(JSON.stringify(page)).length > 512 * 1024
    || page.entries.some(entry => utf8.encode(entry.key).length > 128)
    || utf8.encode(page.newer).length > 4096 || (page.older !== null && utf8.encode(page.older).length > 4096))
    throw new Error('history replay page exceeds its reader budget');
  return page;
}
export type HistoryContext = {facts: HistoryPaintStep[]; complete: boolean};
const updateOf = (entry: HistoryPage['entries'][number]) => entry.frame.kind === 'note'
  ? (entry.frame.update as {update?: {sessionUpdate?:string;toolCallId?:string;compactionId?:string}})?.update : null;
const identity = (kind: string, id: string): HistoryPaintStep => kind === 'tool'
  ? {who:'tool',id,title:'',kind:'other',status:'pending',raw:null,out:null,diff:null,diffs:[],patches:[],where:[]}
  : {who:'compaction',id,status:'in_progress',summary:'',error:null};

/** Dependency reads only confirm identities before this page. They neither
 * advance the search cursor nor count as searched coverage. Facts are private
 * to this one bounded fold; the GUI window's projection is unchanged. */
export async function historyPageContext(page: HistoryPage, io: IO,
  available: HistoryPaintStep[] = []): Promise<HistoryContext> {
  const known = new Set(available.flatMap(item => item.who==='tool' || item.who==='compaction' ? [`${item.who}:${item.id}`] : []));
  const needed = new Set<string>(), facts: HistoryPaintStep[]=[];
  for(const entry of page.entries){
    const update=updateOf(entry);if(!update)continue;
    const key=update.toolCallId ? `tool:${update.toolCallId}` : update.compactionId ? `compaction:${update.compactionId}` : null;
    if(!key)continue;
    if(update.sessionUpdate==='tool_call' || update.sessionUpdate==='compaction_update')known.add(key);
    else if((update.sessionUpdate==='tool_call_update' || update.sessionUpdate==='compaction_summary_chunk') && !known.has(key))needed.add(key);
  }
  if(known.size+needed.size>512 || utf8.encode(JSON.stringify([...known,...needed])).length>512*1024)return {facts:[],complete:false};
  let cursor=page.older;
  while(needed.size && cursor!==null){
    const older=checkedReplayPage(await io.page(cursor,false));
    for(let index=older.entries.length-1;index>=0;index--){
      const update=updateOf(older.entries[index]);if(!update)continue;
      const kind=update.sessionUpdate==='tool_call'?'tool':update.sessionUpdate==='compaction_update'?'compaction':null;
      const id=kind==='tool'?update.toolCallId:update.compactionId;
      if(kind && typeof id==='string' && needed.delete(`${kind}:${id}`))facts.push(identity(kind,id));
    }
    if(older.older===cursor)throw new Error('history context source cursor did not advance');
    cursor=older.older;
  }
  // Reaching the real source beginning also proves an update has no earlier
  // Call: only then may the shared painter's existing orphan rule decide it.
  return {facts,complete:true};
}

const plain = (item: HistoryPaintStep): item is HistoryItem & {text: string} =>
  ['agent','user','thought'].includes(item.who) && 'text' in item;

/** A streaming consumer retires text after each exact shared projection step.
 * At most 512 small tool/compaction identities plus one text delta survive;
 * neither the full joined text nor its contribution list is retained. */
function retire(out: HistoryPaintStep[]): HistoryPaintStep[] | null {
  const last = out.at(-1);
  const facts: HistoryPaintStep[] = [];
  for (const item of out) {
    if (item.who === 'tool') facts.push({who:'tool', id:item.id, title:'', kind:item.kind, status:item.status,
      raw:null,out:null,diff:null,diffs:[],patches:[],where:[]});
    else if (item.who === 'compaction') facts.push({who:'compaction',id:item.id,status:item.status,summary:'',error:null});
  }
  if (facts.length > 512 || utf8.encode(JSON.stringify(facts)).length > 512 * 1024)
    return null;
  if (last && plain(last)) facts.push({...last,text:'',historyEntries:last.historyEntries.slice(-1),historyRecords:last.historyRecords.slice(-1)});
  else if (last && last.who !== 'tool' && last.who !== 'compaction') facts.push({who:'boundary'});
  return facts;
}

export type ReplayCoverage = {complete: boolean; unsearched: number};
const stopped: ReplayCoverage={complete:false,unsearched:0};
type Fragment = {group: string; first: string; entry: string; pageBefore: string; text: string};
export async function* historyReplayText(range: HistoryReplay, io: IO): AsyncGenerator<Fragment, ReplayCoverage> {
  let page = checkedReplayPage(await io.page(range.before, range.forward));
  let out: HistoryPaintStep[] = [], previousRecord: HistoryRecordRef | null = null;
  let started = false, complete = range.completeStart;
  let group = '', first = '';
  while (true) {
    const cutoff=page.entries.findIndex(entry=>entry.key===range.last);
    const entries=cutoff<0?page.entries:page.entries.slice(0,cutoff+1);
    const context=await historyPageContext({...page,entries},io,out);
    if(!context.complete)return {complete:false,unsearched:1};
    out=[...context.facts,...out];
    for (const entry of entries) {
      if (entry.key === range.first) started = true;
      const record = entry.record ?? null;
      if (started && page.references.some(reference => !record || historyReferenceBetween(reference,previousRecord,record))) complete = false;
      previousRecord = appendHistoryEntry(out,entry,page.references,previousRecord);
      const last = out.at(-1);
      if (started && last && plain(last)) {
        if (last.historyKey !== group) {group=last.historyKey;first=entry.key;}
        if (last.text) yield {group,first,entry:entry.key,pageBefore:page.newer,text:last.text};
      } else if (started) group = '';
      const retained=retire(out);
      if(!retained)return {complete:false,unsearched:1};
      out=retained;
      if (started && entry.key === range.last) return {complete,unsearched:0};
    }
    if (page.newer === range.endBefore || (page.has_more === false && page.entries.length === 0))
      throw new Error('history replay cutoff contribution is no longer present');
    const cursor = page.newer;
    page = checkedReplayPage(await io.page(cursor,true));
    if (page.newer === cursor) throw new Error('history replay source cursor did not advance');
  }
}

export async function* replayHistoryQuery(range: HistoryReplay, query: FindQuery, io: IO): AsyncGenerator<FindHit, ReplayCoverage> {
  const fragments = historyReplayText(range,io);
  let group = '', tail = '', matcher = literalMatcher(query);
  let locations: {from:number;to:number;entry:string;before:string}[]=[];
  try {
    while (true) {
      const next = await fragments.next();
      if (next.done) return next.value;
      const part = next.value;
      if (group !== part.group) {group=part.group;tail='';locations=[];matcher=literalMatcher(query);}
      const position=matcher.checkpoint().position;
      const base = position - tail.length;
      locations.push({from:position,to:position+part.text.length,entry:part.entry,before:part.pageBefore});
      if(locations.length>512 || utf8.encode(JSON.stringify(locations)).length>512*1024)
        throw new Error('history replay contribution locator budget exceeded');
      let offset = 0;
      while (offset < part.text.length) {
        const found = matcher.feed(part.text.slice(offset),1);offset+=found.consumed;
        for (const hit of found.ranges) {
          const origin=locations.find(location=>location.from<=hit.from && location.to>hit.from);
          if(!origin)throw new Error('history replay match lost its source contribution');
          yield {before:origin.before,entry:origin.entry,part:0,...hit,
            snippet:findSnippet(tail+part.text,hit.from-base,hit.to-base),complete:false,
            replay:{...range,first:part.first}};
        }
      }
      let at = Math.max(0,tail.length+part.text.length-matcher.contextUnits-160);
      const joined = tail+part.text;
      if (joined.charCodeAt(at)>=0xdc00 && joined.charCodeAt(at)<=0xdfff) at++;
      tail=joined.slice(at);
      const state=matcher.checkpoint();
      const earliest=state.matched ? state.origins[(state.serial-state.matched)%state.origins.length] : state.position;
      locations=locations.filter(location=>location.to>earliest);
    }
  } finally {await fragments.return(stopped);}
}
