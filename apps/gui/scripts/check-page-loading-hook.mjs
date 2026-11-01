// Actual useLoading + projection, with a controlled hook scheduler and
// native-event/query/timer boundary. This is not ReactDOM or WK validation.
import assert from 'node:assert/strict';
import {build} from 'esbuild';
import {fileURLToPath} from 'node:url';
const root=fileURLToPath(new URL('..',import.meta.url));
const bundle=await build({absWorkingDir:root,entryPoints:['src/browse/use-loading.ts'],bundle:true,write:false,format:'esm',platform:'node',alias:{'@':root+'/src'},plugins:[{name:'controlled-native-boundary',setup(b){
 b.onResolve({filter:/^(react|@\/api|@\/lib\/native-events)$/},args=>({path:args.path,namespace:'fixture'}));
 b.onLoad({filter:/.*/,namespace:'fixture'},args=>({contents:args.path==='react'?`export const useState=(...a)=>globalThis.__pageLoading.state(...a);export const useRef=(...a)=>globalThis.__pageLoading.ref(...a);export const useCallback=(...a)=>globalThis.__pageLoading.callback(...a);export const useEffect=(...a)=>globalThis.__pageLoading.effect(...a,'passive');export const useLayoutEffect=(...a)=>globalThis.__pageLoading.effect(...a,'layout');`:args.path==='@/api'?`export const inApp=true;export const pageState=(...a)=>globalThis.__pageLoading.query(...a);`:`export const listenNativeEvent=(...a)=>globalThis.__pageLoading.listen(...a);`}));
}}]});
const {useLoading}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
const defer=()=>{let resolve;const promise=new Promise(yes=>{resolve=yes;});return{promise,resolve};};
const settle=async()=>{for(let n=0;n<8;n++)await Promise.resolve();};
const saved={setTimeout,clearTimeout,setInterval,clearInterval};
const sample=(patch={})=>({tab:'A',view:'1',navigation:'2',revision:'3',phase:'started',estimate:.2,requested_url:'https://requested.test',url:'',exit:'exit',proxied:true,error:null,...patch});
const same=(a,b)=>!!a&&a.length===b.length&&a.every((v,i)=>Object.is(v,b[i]));
function fixture(){
 let cursor=0,slots=[],jobs=[],dirty=false,ids=['A'],current='A',out,now=0,sequence=0,unmounted=false,sets=0;
 const listeners=[],queries=[],timers=new Map();
 const schedule=(run,ms,repeat=false)=>{const id=++sequence;timers.set(id,{run,at:now+ms,ms,repeat});return id;};
 globalThis.setTimeout=(run,ms)=>schedule(run,ms);globalThis.clearTimeout=id=>timers.delete(id);
 globalThis.setInterval=(run,ms)=>schedule(run,ms,true);globalThis.clearInterval=id=>timers.delete(id);
 globalThis.__pageLoading={
  state(initial){const i=cursor++;if(!slots[i])slots[i]={value:typeof initial==='function'?initial():initial};return[slots[i].value,value=>{assert(!unmounted,'setState after unmount');sets++;const next=typeof value==='function'?value(slots[i].value):value;if(!Object.is(next,slots[i].value)){slots[i].value=next;dirty=true;}}];},
  ref(initial){const i=cursor++;return(slots[i]??={current:initial});},
  callback(run,deps){const i=cursor++;if(!slots[i]||!same(slots[i].deps,deps))slots[i]={deps,run};return slots[i].run;},
  effect(run,deps,kind){const i=cursor++;if(!slots[i]||!same(slots[i].deps,deps))jobs.push({i,run,deps,kind});},
  query(tab){const pending=defer();queries.push({tab,...pending});return pending.promise;},
  listen(event,run,options){assert.equal(event,'page:navigation');const entry={run,options,closed:false};listeners.push(entry);return()=>{entry.closed=true;};},
 };
 const render=()=>{let rounds=0;do{assert(rounds++<20,'hook did not settle');dirty=false;cursor=0;out=useLoading(ids,current);const pending=jobs;jobs=[];pending.sort((a,b)=>a.kind==='layout'&&b.kind!=='layout'?-1:0);for(const job of pending){slots[job.i]?.cleanup?.();slots[job.i]={deps:job.deps,cleanup:job.run()};}}while(dirty);return out;};
 const ready=()=>{listeners.at(-1).options.onReady?.();return render();};
 const event=state=>{listeners.at(-1).run(state);return render();};
 const tick=ms=>{const end=now+ms;for(;;){const next=[...timers].sort((a,b)=>a[1].at-b[1].at).find(([,t])=>t.at<=end);if(!next)break;const[id,t]=next;now=t.at;if(t.repeat)t.at+=t.ms;else timers.delete(id);t.run();render();}now=end;return render();};
 const unmount=()=>{for(const slot of slots)slot?.cleanup?.();unmounted=true;};
 render();return{read:()=>out,render,ready,event,tick,unmount,listeners,queries,timers,sets:()=>sets,select:(next,tab)=>{ids=next;current=tab;return render();}};
}
let passed=0;async function check(name,run){await run();console.log('PASS',name);passed++;}
try{
 await check('snapshot waits for actual listener readiness and cannot rewind a delivered event',async()=>{
  const f=fixture();assert.equal(f.queries.length,0);f.ready();assert.equal(f.queries.length,1);
  f.event(sample({revision:'5',estimate:.8}));f.queries[0].resolve(sample());await settle();f.render();assert.equal(f.read().share('A'),.8);f.unmount();
 });
 await check('resuming an already-finished native snapshot does not invent a new completion line',async()=>{
  const f=fixture();f.ready();f.queries[0].resolve(sample({revision:'4',phase:'finished',estimate:1}));await settle();f.render();
  assert.equal(f.read().state('A').phase,'finished');assert.equal(f.read().share('A'),undefined);assert.equal(f.timers.size,0);f.unmount();
 });
 await check('successful completion reaches one, then removes only that completed line',()=>{
  const f=fixture();f.ready();f.event(sample({revision:'4',phase:'finished',estimate:1}));assert.equal(f.read().share('A'),1);assert.equal(f.read().busy('A'),false);
  f.tick(599);assert.equal(f.read().share('A'),1);f.tick(1);assert.equal(f.read().share('A'),undefined);assert.equal(f.timers.size,0);f.unmount();
 });
 await check('non-WK settled ends static pending without a successful share or finish timer',()=>{
  const f=fixture();f.ready();f.event(sample({estimate:null}));assert.equal(f.read().busy('A'),true);
  f.event(sample({revision:'4',phase:'settled',estimate:null}));assert.equal(f.read().busy('A'),false);assert.equal(f.read().share('A'),undefined);assert.equal(f.timers.size,0);f.unmount();
 });
 await check('failure and cancellation never manufacture success or a repeating progress timer',()=>{
  const f=fixture();f.ready();f.event(sample());f.event(sample({revision:'4',phase:'failed',estimate:.2,error:{domain:'NSURLErrorDomain',code:'-1005',description:'raw lost',url:'https://failed.test',underlying:[],truncated:false}}));
  assert.equal(f.read().share('A'),undefined);assert.equal(f.read().busy('A'),false);assert.equal(f.read().state('A').error.code,'-1005');assert.equal(f.timers.size,0);
  f.event(sample({navigation:'5',revision:'6',phase:'cancelled',estimate:.1}));assert.equal(f.read().share('A'),undefined);assert.equal(f.read().busy('A'),false);f.unmount();
 });
 await check('an old finish timer cannot hide the next navigation estimate',()=>{
  const f=fixture();f.ready();f.event(sample({revision:'4',phase:'finished',estimate:1}));f.event(sample({navigation:'5',revision:'6',estimate:.3}));f.tick(600);
  assert.equal(f.read().share('A'),.3);assert.equal(f.read().busy('A'),true);f.unmount();
 });
 await check('same-view late progress/start snapshots keep the new navigation intact',()=>{
  const f=fixture();f.ready();f.event(sample({navigation:'5',revision:'6',estimate:.3}));f.event(sample({revision:'4',estimate:.9}));f.event(sample());assert.equal(f.read().share('A'),.3);f.unmount();
 });
 await check('closing/removing a tab rejects its late query and native event',async()=>{
  const f=fixture();f.ready();f.event(sample({revision:'4',phase:'finished',estimate:1}));f.select(['B'],'B');assert.equal(f.timers.size,0);
  f.queries[0].resolve(sample({revision:'9',estimate:.9}));await settle();f.event(sample({revision:'10'}));assert.equal(f.read().state('A'),null);assert.equal(f.read().share('A'),undefined);f.unmount();
 });
 await check('an errand finishing cannot complete an active native navigation',()=>{
  const f=fixture();f.ready();f.event(sample());f.read().errand('A',true);f.render();f.read().errand('A',false);f.render();assert.equal(f.read().busy('A'),true);assert.equal(f.read().share('A'),.2);f.unmount();
 });
 await check('unmount cancels owned timers and late query/event callbacks',async()=>{
  const f=fixture();f.ready();f.event(sample({revision:'4',phase:'finished',estimate:1}));const old=f.listeners[0];f.unmount();const sets=f.sets();assert.equal(f.timers.size,0);assert(old.closed);
  old.run(sample({revision:'10'}));f.queries[0].resolve(sample({revision:'11'}));await settle();assert.equal(f.sets(),sets);
 });
 console.log(`${passed} controlled native loading hook checks passed; no DOM/WK`);
}finally{Object.assign(globalThis,saved);delete globalThis.__pageLoading;}
