import assert from "node:assert/strict";
import { build } from "esbuild";
const output=await build({entryPoints:["src/lib/directory-watch.ts"],bundle:true,write:false,platform:"node",format:"esm"});
const {DirectoryWatch,directoryWatchScopes,directoryScopeIsNative,directoryAliases}=await import(`data:text/javascript;base64,${Buffer.from(output.outputFiles[0].text).toString("base64")}`);
const deferred=()=>{let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return{promise,resolve,reject};};
const settle=async()=>{for(let i=0;i<10;i++)await Promise.resolve();};
const scope=path=>({kind:"entries",path});
const batch=(cursor="epoch:1",retry_ms=0)=>({cursor,retry_ms,reset:false,dirs:[],workspace:false,coverage:[{scope:0,mode:"native",reason:null}]});
const opened=(id="lease",machine_id="stable-machine")=>({id,machine_id,batch:batch()});
function fixture(waitMs=20_000){
 const calls=[],timers=[];
 const transport={
  begin:(machine,scopes)=>{const pending=deferred();calls.push({kind:"begin",machine,scopes,pending});return pending.promise;},
  wait:(machine,id,cursor,wait)=>{const pending=deferred();calls.push({kind:"wait",machine,id,cursor,wait,pending});return pending.promise;},
  leave:(machine,id)=>{const pending=deferred();calls.push({kind:"leave",machine,id,pending});return pending.promise;},
 };
 const model=new DirectoryWatch(transport,waitMs,(run,delay)=>{const timer={run,delay,canceled:false};timers.push(timer);return()=>{timer.canceled=true;};});
 const tick=()=>{const timer=timers.find(t=>!t.canceled);assert.ok(timer,"one pending timer");timer.canceled=true;timer.run();return timer;};
 const request=path=>model.set({machine:"mutable-label",scopes:[scope(path)]});
 return{model,calls,timers,tick,request};
}
let count=0;
async function check(name,run){await run();console.log(`ok ${++count} - ${name}`);}
await check("late begin after close releases stable machine lease and never starts a wait",async()=>{
 const f=fixture();f.request("/a");f.model.set(null);f.calls[0].pending.resolve(opened());await settle();
 assert.deepEqual(f.calls.map(c=>c.kind),["begin","leave"]);assert.equal(f.calls[1].machine,"stable-machine");assert.equal(f.model.snapshot().status,"idle");f.calls[1].pending.resolve();await settle();assert.equal(f.calls.length,2);
});
await check("scope changes coalesce until the physical begin and leave settle",async()=>{
 const f=fixture();f.request("/a");f.request("/b");f.request("/c");assert.equal(f.calls.length,1);
 f.calls[0].pending.resolve(opened("old"));await settle();assert.equal(f.calls.length,2);f.calls[1].pending.resolve();await settle();
 assert.equal(f.calls[2].kind,"begin");assert.deepEqual(f.calls[2].scopes,[scope("/c")]);
 f.calls[2].pending.resolve(opened("current"));await settle();assert.equal(f.model.snapshot().status,"watching");assert.equal(f.calls.length,3);f.model.set(null);f.calls[3].pending.resolve();await settle();
});
await check("held wait does not delay leave and its late result cannot overwrite a new owner",async()=>{
 const f=fixture();f.request("/a");f.calls[0].pending.resolve(opened("old"));await settle();f.tick();assert.equal(f.calls[1].kind,"wait");
 f.request("/b");assert.equal(f.calls[2].kind,"leave");f.calls[2].pending.resolve();await settle();f.calls[3].pending.resolve(opened("new"));await settle();
 f.calls[1].pending.resolve({...batch("old:999"),dirs:["/a"]});await settle();assert.equal(f.model.snapshot().batch.cursor,"epoch:1");assert.equal(f.calls.length,4);f.model.set(null);f.calls[4].pending.resolve();await settle();
});
await check("StrictMode-style detach and reattach never accepts the first begin",async()=>{
 const f=fixture();f.request("/a");f.model.set(null);f.request("/a");f.calls[0].pending.resolve(opened("first"));await settle();assert.equal(f.calls[1].kind,"leave");assert.equal(f.model.snapshot().batch,null);
 f.calls[1].pending.resolve();await settle();assert.equal(f.calls[2].kind,"begin");f.calls[2].pending.resolve(opened("second"));await settle();assert.equal(f.model.snapshot().status,"watching");f.model.set(null);f.calls[3].pending.resolve();await settle();
});
await check("HTTP zero-wait answers always wait at least one second before another poll",async()=>{
 const f=fixture(0);f.request("/a");f.calls[0].pending.resolve(opened());await settle();assert.equal(f.timers[0].delay,1000);f.tick();assert.equal(f.calls[1].wait,0);
 f.calls[1].pending.resolve(batch("epoch:2",1200));await settle();assert.equal(f.timers[1].delay,1200);assert.equal(f.calls.length,2);f.model.set(null);f.calls[2].pending.resolve();await settle();
});
await check("native waits use the returned identity and only one wait is in flight",async()=>{
 const f=fixture();f.request("/a");f.calls[0].pending.resolve(opened());await settle();f.tick();const wait=f.calls[1];assert.equal(wait.machine,"stable-machine");assert.equal(wait.wait,20000);assert.equal(f.timers.filter(t=>!t.canceled).length,0);
 wait.pending.resolve(batch("epoch:2"));await settle();assert.equal(f.timers.filter(t=>!t.canceled).length,1);f.model.set(null);assert.equal(f.timers.filter(t=>!t.canceled).length,0);f.calls[2].pending.resolve();await settle();
});
await check("begin failures back off visibly and closing cancels every retry",async()=>{
 const f=fixture();f.request("/a");f.calls[0].pending.reject(new Error("offline"));await settle();assert.equal(f.model.snapshot().status,"failed");assert.equal(f.timers[0].delay,5000);
 f.tick();f.calls[1].pending.reject(new Error("offline"));await settle();assert.equal(f.timers[1].delay,10000);f.model.set(null);assert.equal(f.timers.filter(t=>!t.canceled).length,0);assert.equal(f.calls.length,2);
});
await check("failed wait releases its lease once and cannot reopen while hidden",async()=>{
 const f=fixture();f.request("/a");f.calls[0].pending.resolve(opened());await settle();f.tick();f.calls[1].pending.reject(new Error("closed"));await settle();assert.equal(f.calls[2].kind,"leave");assert.equal(f.model.snapshot().status,"failed");
 f.model.set(null);f.calls[2].pending.reject(new Error("unreachable"));await settle();assert.equal(f.calls.length,3);assert.equal(f.timers.filter(t=>!t.canceled).length,0);
});
await check("scope budget preserves priority and reports every omitted level",()=>{
 const entries=Array.from({length:70},(_,i)=>`/entries/${i}`);const plan=directoryWatchScopes("/workspace",entries);
 assert.equal(plan.scopes.length,64);assert.deepEqual(plan.scopes[0],{kind:"workspace",root:"/workspace"});assert.deepEqual(plan.scopes[1],scope(entries[0]));assert.deepEqual(plan.omitted,entries.slice(63));
 const bytes=directoryWatchScopes(null,["/"+"中".repeat(22_000),"/small"]);assert.deepEqual(bytes.scopes,[scope("/small")]);assert.equal(bytes.omitted.length,1);
});
await check("coverage is indexed to the exact current scope, never to the first array position",()=>{
 const state={key:"current",status:"watching",scopes:[{kind:"workspace",root:"/workspace"},scope("/alias")],batch:{...batch(),coverage:[{scope:1,mode:"native",reason:null},{scope:0,mode:"partial",reason:"budget"}]},error:null};
 assert.equal(directoryScopeIsNative(state,scope("/alias")),true);assert.equal(directoryScopeIsNative(state,{kind:"workspace",root:"/workspace"}),false);assert.equal(directoryScopeIsNative(state,scope("/other")),false);assert.equal(directoryScopeIsNative({...state,status:"failed"},scope("/alias")),false);
});
await check("canonical listing facts identify inside and outside aliases without local resolution",()=>{
 const listings=new Map([["/asked",{path:"/physical"}],["/asked/normal",{path:"/physical/normal"}],["/asked/out",{path:"/other/place"}],["/asked/alias",{path:"/physical/normal"}]]);
 assert.deepEqual(directoryAliases("/asked",listings,[...listings.keys()]),["/asked/out","/asked/alias"]);
 assert.deepEqual(directoryAliases("/",new Map([["/",{path:"/"}],["/child",{path:"/child"}]]),["/","/child"]),[]);
});
console.log(`${count} directory watch controller/scope checks passed`);
