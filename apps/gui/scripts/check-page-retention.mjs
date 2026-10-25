// Real ledger/operation queue; only native IPC promises are controlled.
import assert from 'node:assert/strict';
import {build} from 'esbuild';
import {createHash} from 'node:crypto';
import {mkdirSync, readFileSync, writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {tmpdir} from 'node:os';
const gui=fileURLToPath(new URL('..',import.meta.url));
const out=process.env.PAGE_RETENTION_OUT||join(tmpdir(),`khor-page-retention-${process.pid}`);
const files=['src/views/held.ts','src/lib/native-page-open.ts','scripts/check-page-retention.mjs'];
const fingerprint=()=>Object.fromEntries(files.map(file=>[file,createHash('sha256').update(readFileSync(join(gui,file))).digest('hex')]));
const sources=fingerprint();
const bundled=await build({stdin:{contents:'export * from "./src/views/held.ts"; export * from "./src/lib/native-page-open.ts";',resolveDir:gui,loader:'ts'},bundle:true,write:false,format:'esm',platform:'node',alias:{'@':join(gui,'src')},plugins:[{name:'native-boundary',setup(b){b.onResolve({filter:/^@\/api$/},()=>({path:'native',namespace:'fixture'}));b.onLoad({filter:/.*/,namespace:'fixture'},()=>({loader:'js',contents:`const n=globalThis.__pageRetentionNative;export const openPage=(...a)=>n.open(...a);export const closePage=t=>n.close(t);export const dropWebBorrow=s=>n.drop(s);`}));}}]});
let serial=0;
const pending=()=>{let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return {promise,resolve,reject};};
const settle=async()=>{for(let i=0;i<12;i++)await Promise.resolve();};
const box={x:0,y:0,w:200,h:200,view:800};
async function fixture(){
 const calls=[],hooks={};
 globalThis.__pageRetentionNative={open:(tab,machine,url)=>{calls.push(['open',tab,url]);return hooks.open?.(tab,machine,url)??Promise.resolve({session:'s',addr:'addr'});},close:tab=>{calls.push(['close',tab]);return hooks.close?.(tab)??Promise.resolve();},drop:session=>{calls.push(['drop',session]);return hooks.drop?.(session)??Promise.resolve();}};
 const m=await import(`data:text/javascript;base64,${Buffer.from(bundled.outputFiles[0].text).toString('base64')}#${serial++}`);
 return {m,calls,hooks};
}
const cases={
 async 'late-page'({m,calls}){m.releasePages([],'page');m.holdPage('late','page');await settle();assert.deepEqual(calls,[['close','late']],'late acquisition reconciles latest closed tab set');},
 async 'late-borrow'({m,calls}){m.releaseBorrows([],'page');m.holdBorrow('exit','session-exit','page');await settle();assert.deepEqual(calls,[['drop','session-exit']],'late acquisition releases exit nobody wants');},
 async 'strict-late-borrow'({m,calls}){m.releaseBorrows([],'page');m.holdBorrow('exit','s','page');m.holdBorrow('exit','s','page');await settle();assert.deepEqual(calls,[['drop','s']],'shared pending open completion retires its borrow once');},
 async 'unknown-owner'({m,calls}){
  m.releasePages([],'page');m.releaseBorrows([],'page');m.holdPage('child','content');m.holdPage('second-child','content');m.holdBorrow('exit','s','content');m.releasePages([],'page');m.releaseBorrows([],'page');await settle();assert.deepEqual(calls,[],'unknown first collection must survive another owner sweep');
  m.releasePages(['second-child'],'content');m.releaseBorrows([],'content');await settle();assert.deepEqual(calls.filter(c=>c[0]==='close'),[['close','child']]);assert.deepEqual(calls.filter(c=>c[0]==='drop'),[['drop','s']]);
 },
 async 'owner-union'({m,calls}){
  m.releasePages(['shared','landing'],'page');m.releasePages(['shared','child'],'content');m.releaseBorrows(['exit'],'page');m.releaseBorrows(['exit'],'content');m.holdPage('shared','page');m.holdPage('landing','page');m.holdPage('child','content');m.holdBorrow('exit','s','page');m.holdBorrow('exit','s','content');m.releasePages([],'page');m.releaseBorrows([],'page');await settle();assert.deepEqual(calls,[['close','landing']]);
  m.releasePages([],'content');m.releaseBorrows([],'content');await settle();assert.equal(calls.filter(c=>c[0]==='drop').length,1);assert.deepEqual(calls.filter(c=>c[0]==='close').map(c=>c[1]),['landing','shared','child']);
 },
 async 'cap-lru'({m,calls}){
  const ids=Array.from({length:11},(_,i)=>`tab-${i}`);m.releasePages(ids,'page');for(const id of ids.slice(0,10))m.holdPage(id,'page');await settle();assert.deepEqual(calls,[['close','tab-0'],['close','tab-1']],'acquisition applies global eight-page cap');m.holdPage('tab-2','page');m.holdPage('tab-10','page');await settle();assert.deepEqual(calls.at(-1),['close','tab-3']);
 },
 async 'strict-pending'({m,calls,hooks}){
  const opened=pending();hooks.open=()=>opened.promise;const first=m.openNativePage('same','exit','https://one.test',box),duplicate=m.openNativePage('same','exit','https://one.test',box);assert.equal(first,duplicate);assert.equal(calls.length,1);opened.resolve({session:'s',addr:'addr'});await first;hooks.open=null;await m.openNativePage('same','exit','https://one.test',box);assert.equal(calls.length,2,'completed work does not block native resume');
 },
 async 'different-targets'({m,calls,hooks}){
  const opened=pending();hooks.open=()=>opened.promise;const first=m.openNativePage('same','','https://one.test',box),second=m.openNativePage('same','','https://two.test',box);await settle();assert.equal(calls.length,1);hooks.open=null;opened.resolve({session:'',addr:''});await first;await second;assert.deepEqual(calls.map(c=>c[2]),['https://one.test','https://two.test']);
 },
 async 'close-reopen'({m,calls,hooks}){
  m.releasePages(['same'],'page');await m.openNativePage('same','','https://old.test',box);m.holdPage('same','page');const closed=pending();hooks.close=()=>closed.promise;m.releasePages([],'page');await settle();m.releasePages(['same'],'page');const reopened=m.openNativePage('same','','https://new.test',box);await settle();assert.equal(calls.filter(c=>c[0]==='open').length,1,'new owner waits for pending close of same label');closed.resolve();await reopened;m.holdPage('same','page');await settle();assert.deepEqual(calls,[['open','same','https://old.test'],['close','same'],['open','same','https://new.test']]);
 },
 async 'release-failure'({m,calls,hooks}){
  m.releasePages(['same'],'page');m.releaseBorrows(['exit'],'page');m.holdPage('same','page');m.holdBorrow('exit','s','page');hooks.close=()=>Promise.reject(Error('window gone'));hooks.drop=()=>Promise.reject(Error('peer gone'));m.releasePages([],'page');m.releaseBorrows([],'page');await settle();m.releasePages([],'page');m.releaseBorrows([],'page');await settle();assert.equal(calls.filter(c=>c[0]==='close').length,1);assert.equal(calls.filter(c=>c[0]==='drop').length,1);m.releasePages(['same'],'page');await m.openNativePage('same','','https://new.test',box);assert.deepEqual(calls.at(-1),['open','same','https://new.test']);
 },
};
const selected=process.env.PAGE_RETENTION_CASES?.split(',').filter(Boolean)??Object.keys(cases);
assert(selected.length,'Select at least one case');assert.equal(selected.length,new Set(selected).size,'Cases must not repeat');for(const name of selected)assert(Object.hasOwn(cases,name),`Unknown case: ${name}`);
mkdirSync(out,{recursive:true});const results=[];
for(const name of selected){const f=await fixture();try{await cases[name](f);results.push({name,ok:true,calls:f.calls});console.log('PASS',name);}catch(error){results.push({name,ok:false,error:error.stack,calls:f.calls});console.error('FAIL',name,error.message);}}
assert.deepEqual(fingerprint(),sources,'Production/test source stayed fixed');writeFileSync(join(out,'report.json'),JSON.stringify({sources,results,boundary:'Real ledger/queue modules, controlled native promises; no actual native handles'},null,2));assert(results.every(r=>r.ok),`${results.filter(r=>!r.ok).length} retention cases failed`);console.log(`page retention: ${results.length}/${results.length} passed`,out);
