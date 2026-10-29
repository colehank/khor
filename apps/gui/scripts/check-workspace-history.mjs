import assert from "node:assert/strict";
import { build } from "esbuild";
const output = await build({entryPoints:["src/views/workspace/workspace-history.ts"],bundle:true,write:false,platform:"node",format:"esm"});
const {WorkspaceHistoryMemory:History}=await import(`data:text/javascript;base64,${Buffer.from(output.outputFiles[0].text).toString("base64")}`);
const row=(id,head="A")=>({id:`${head}-${id}`,subject:`commit ${id}`,author:"A",when_ms:1700000000000+id,cut:false});
function server(total=600){
 let head="A";const calls=[];
 const fetch=async(cursor,limit)=>{
  calls.push({cursor,limit});const [snapshot,offset]=cursor===null?[head,0]:[cursor.split(":")[0],Number(cursor.split(":")[1])];
  const count=Math.min(limit,total-offset);
  return {head:snapshot,commits:Array.from({length:count},(_,i)=>row(offset+i,snapshot)),next:offset+count<total?`${snapshot}:${offset+count}`:null};
 };
 return {fetch,calls,move:value=>{head=value;}};
}
const deferred=()=>{let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return{promise,resolve,reject};};
let count=0;
async function check(name,run){await run();console.log(`ok ${++count} - ${name}`);}
await check("all 260 commits remain reachable; terminal page stops",async()=>{
 const model=new History(),backend=server(260),release=model.attach();await model.refresh(backend.fetch);const ids=[];
 for(;;){const page=model.snapshot().current;ids.push(...page.data.commits.map(c=>c.id));if(page.data.next===null)break;await model.go(page.index+1,backend.fetch);}
 assert.deepEqual(ids,Array.from({length:260},(_,i)=>`A-${i}`));const calls=backend.calls.length;await model.go(7,backend.fetch);assert.equal(backend.calls.length,calls);release();
});
await check("evicted newer pages replay bounded anchors on the original snapshot",async()=>{
 const model=new History(),backend=server(),release=model.attach();await model.refresh(backend.fetch);
 for(let i=1;i<=12;i++)await model.go(i,backend.fetch);
 const cachedBefore=backend.calls.length;await model.go(6,backend.fetch);
 assert.equal(backend.calls.length,cachedBefore,"the seven recent pages plus page zero fit the eight-page cache");
 backend.move("B");const before=backend.calls.length;await model.go(2,backend.fetch);
 assert.equal(backend.calls.length-before,2,"page 2 was evicted; restart from permanent page zero, not an unbounded cursor chain");
 assert.deepEqual(backend.calls.slice(before).map(c=>c.cursor),["A:40","A:80"]);assert.equal(model.snapshot().current.data.commits[0].id,"A-80");release();
});
await check("duplicate clicks issue one request and retain the visible page",async()=>{
 const model=new History(),backend=server(),release=model.attach();await model.refresh(backend.fetch);
 const pending=deferred(),current=model.snapshot().current;let calls=0;
 const fetch=(...args)=>{calls++;return pending.promise.then(()=>backend.fetch(...args));};
 const first=model.go(1,fetch);await model.go(1,fetch);assert.equal(calls,1);assert.equal(model.snapshot().current,current);
 pending.resolve();await first;assert.equal(model.snapshot().current.index,1);assert.equal(model.snapshot().busy,false);release();
});
await check("failure and retry preserve exact current page and reading owner",async()=>{
 const model=new History(),backend=server(),release=model.attach();await model.refresh(backend.fetch);const current=model.snapshot().current;
 current.scroll.position={key:"reading",top:320,height:900};await model.go(1,async()=>{throw new Error("offline");});
 assert.equal(model.snapshot().current,current);assert.equal(model.snapshot().error,"offline");await model.retry(backend.fetch);assert.equal(model.snapshot().current.index,1);
 await model.go(0,backend.fetch);assert.equal(model.snapshot().current,current);assert.equal(model.snapshot().current.scroll.position.top,320);release();
});
await check("detach rejects late pages and starts no hidden follow-up IO",async()=>{
 const model=new History(),backend=server(),release=model.attach();await model.refresh(backend.fetch);const current=model.snapshot().current,pending=deferred();let calls=0;
 const next=model.go(4,async(...args)=>{calls++;await pending.promise;return backend.fetch(...args);});release();pending.resolve();await next;
 assert.equal(calls,1);assert.equal(model.snapshot().current,current);const returned=model.attach();assert.equal(model.snapshot().refreshRequested,false);assert.equal(model.snapshot().current,current);returned();
});
await check("explicit refresh replaces snapshots only after success",async()=>{
 const model=new History(),backend=server(),release=model.attach();await model.refresh(backend.fetch);await model.go(2,backend.fetch);
 const current=model.snapshot().current,generation=model.snapshot().generation;backend.move("B");model.invalidate();await model.refresh(async()=>{throw new Error("offline");});
 assert.equal(model.snapshot().current,current);assert.equal(model.snapshot().generation,generation);await model.retry(backend.fetch);
 assert.equal(model.snapshot().current.index,0);assert.equal(model.snapshot().current.data.head,"B");assert.notEqual(model.snapshot().current.scroll,current.scroll);assert.equal(model.snapshot().generation,generation+1);release();
});
await check("refresh during navigation coalesces until the request settles",async()=>{
 const model=new History(),backend=server(),release=model.attach();await model.refresh(backend.fetch);const pending=deferred(),current=model.snapshot().current;let inflight=0,peak=0;
 const fetch=async(...args)=>{peak=Math.max(peak,++inflight);await pending.promise;const result=await backend.fetch(...args);inflight--;return result;};
 const next=model.go(1,fetch);model.invalidate();model.invalidate();await model.refresh(fetch);assert.equal(inflight,1);backend.move("B");pending.resolve();await next;
 assert.equal(model.snapshot().current,current);assert.equal(model.snapshot().refreshRequested,true);await model.refresh(fetch);assert.equal(peak,1);assert.equal(model.snapshot().current.data.head,"B");release();
});
await check("bad snapshot, cursor, duplicates or byte budget never replace visible data",async()=>{
 const model=new History(),backend=server(),release=model.attach();await model.refresh(backend.fetch);const current=model.snapshot().current;
 for(const bad of [
  {head:"B",commits:[row(40)],next:null},{head:"A",commits:[row(40)],next:"A:40"},
  {head:"A",commits:[row(40),row(40)],next:null},
  {head:"A",commits:[{...row(40),subject:"x".repeat(256*1024)}],next:null},
 ]){await model.go(1,async()=>bad);assert.equal(model.snapshot().current,current);assert.notEqual(model.snapshot().error,null);assert.equal(model.snapshot().busy,false);}
 await model.retry(backend.fetch);assert.equal(model.snapshot().current.index,1);release();
});
await check("StrictMode-style first-load detach cannot accept stale results or strand busy state",async()=>{
 const model=new History(),backend=server(),release=model.attach(),pending=deferred();
 const first=model.refresh(async()=>{await pending.promise;return backend.fetch(null,40);});release();const returned=model.attach();pending.resolve();await first;
 assert.equal(model.snapshot().current,null);assert.equal(model.snapshot().refreshRequested,true);assert.equal(model.snapshot().busy,false);await model.refresh(backend.fetch);assert.equal(model.snapshot().current.index,0);returned();
});
console.log(`${count} workspace history checks passed`);
