// Exercise the actual receipt policy and hook, with controlled API latency,
// errors and timer scheduling. No real home, terminal, or peer is accessed.
import assert from 'node:assert/strict';
import { build } from 'esbuild';
import { fileURLToPath } from 'node:url';
import { SessionReads } from '../src/lib/session-read.ts';
const gui = fileURLToPath(new URL('..', import.meta.url));
const row = (id='shell/a', at_ms=100) => ({id, at_ms, unread:1, word:'done'});
const receipts = new SessionReads();
const first = receipts.read(row());
assert(first);
assert.equal(receipts.project([row()])[0].unread, 0, 'read locally before any persistence');
assert.equal(receipts.project([row('shell/a',99)])[0].unread, 0, 'older in-flight list cannot revive badge');
assert.equal(receipts.project([row('shell/a',101)])[0].unread, 1, 'new completion must remain unread');
assert.equal(receipts.project([row('shell/b')])[0].unread, 1, 'unrelated session unchanged');
assert.equal(receipts.project([row()])[0].word, 'done', 'local UI receipt only changes unread, not process state');
assert.equal(receipts.read(row()), null, 'same row is not sent twice');
const next = receipts.read(row('shell/a',101));
receipts.confirmed(first);
assert.deepEqual(receipts.pending(), [next], 'old request completion preserves newer pending receipt');
receipts.confirmed(next);
assert.equal(receipts.pending().length, 0);
assert.equal(receipts.project([row()])[0].unread, 0, 'keep local coverage after backend acknowledgement');
assert.equal(receipts.read(row('wide/all')), null, 'broadcast uses rendered message identities, never broad watermarks');
assert.equal(receipts.read(row('shell/a',NaN)), null);

let rendering;
globalThis.__hooks = {
  useState(initial) { const h=rendering,i=h.index++; if(!(i in h.slots))h.slots[i]=typeof initial==='function'?initial():initial; return [h.slots[i],v=>{h.slots[i]=typeof v==='function'?v(h.slots[i]):v;h.schedule();}]; },
  useRef(initial) { const h=rendering,i=h.index++; return h.slots[i]??(h.slots[i]={current:initial}); },
  useCallback(fn,deps) { const h=rendering,i=h.index++,old=h.slots[i];if(!old||deps.some((d,j)=>d!==old.deps[j]))h.slots[i]={fn,deps};return h.slots[i].fn; },
  useMemo(fn,deps) { const h=rendering,i=h.index++,old=h.slots[i];if(!old||deps.some((d,j)=>d!==old.deps[j]))h.slots[i]={value:fn(),deps};return h.slots[i].value; },
  useEffect(fn,deps) { const h=rendering,i=h.index++,old=h.slots[i];if(!old||deps.some((d,j)=>d!==old.deps[j]))h.effects.push(()=>{old?.cleanup?.();h.slots[i]={deps,cleanup:fn()};}); },
};
function mount(run) {
  const h={slots:[],effects:[],index:0,value:null,queued:false,closed:false,
    schedule(){if(h.queued||h.closed)return;h.queued=true;queueMicrotask(()=>{h.queued=false;h.render();});},
    render(){if(h.closed)return;h.index=0;h.effects=[];rendering=h;h.value=run();rendering=undefined;for(const fn of h.effects)fn();},
    close(){h.closed=true;for(const s of h.slots)s?.cleanup?.();},
  };h.render();return h;
}
let timer;
globalThis.window={setInterval(fn){timer=fn;return 1;},clearInterval(){timer=undefined;}};
const bundle=await build({stdin:{contents:'export * from "./src/hooks/use-session-reads.ts"',resolveDir:gui,loader:'ts'},bundle:true,write:false,format:'esm',platform:'node',plugins:[{name:'boundaries',setup(b){
  b.onResolve({filter:/^react$/},()=>({path:'react',namespace:'fixture'}));
  b.onResolve({filter:/^@\/api$/},()=>({path:'api',namespace:'fixture'}));
  b.onLoad({filter:/.*/,namespace:'fixture'},args=>({contents:args.path==='react'?'export const {useState,useEffect,useRef,useCallback,useMemo}=globalThis.__hooks;':'export const markSeen=(...args)=>globalThis.__markSeen(...args);',loader:'js'}));
}}]});
const {useSessionReads}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
const tick=()=>new Promise(r=>setImmediate(r));
const calls=[];let settle;
globalThis.__markSeen=(id,at_ms)=>{calls.push({id,at_ms});return new Promise((resolve,reject)=>{settle={resolve,reject};});};
let raw=[row(),row('shell/b')];
const view=mount(()=>useSessionReads(raw));
view.value.read(raw[0]);await tick();
assert.equal(view.value.rows[0].unread,0,'visible badge clears while request is deliberately unresolved');
assert.equal(view.value.rows[1].unread,1);
assert.deepEqual(calls,[{id:'shell/a',at_ms:100}],'send the observed row stamp');
raw=[row(),row('shell/b')];view.render();assert.equal(view.value.rows[0].unread,0,'poll race is masked');
settle.reject(new Error('offline'));await tick();
assert.equal(view.value.rows[0].unread,0,'transient write failure does not flash the old badge');
timer();await tick();assert.equal(calls.length,2,'failed write retries after navigating away');
raw=[row('shell/a',200)];view.render();assert.equal(view.value.rows[0].unread,1,'later message still visible while old receipt retries');
view.value.read(raw[0]);await tick();assert.equal(view.value.rows[0].unread,0);
assert.equal(calls.length,2,'do not stack requests for one session');
settle.resolve();await tick();timer();await tick();
assert.deepEqual(calls.at(-1),{id:'shell/a',at_ms:200},'old acknowledgement did not erase queued new read');
settle.resolve();await tick();timer();await tick();assert.equal(calls.length,3,'confirmed intent stops retrying');
view.close();assert.equal(timer,undefined,'unmount releases retry timer');
console.log('session read: immediate feedback, delayed/stale replies, new versions, independent sessions, bounded API, failed-write retry and cleanup passed');
