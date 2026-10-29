import assert from "node:assert/strict";
import { build } from "esbuild";
const result=await build({entryPoints:["src/views/workspace/workspace-refresh.ts"],bundle:true,write:false,format:"esm"});
const {WorkspaceRefresh}=await import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].text).toString("base64")}`);
const deferred=()=>{let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return{promise,resolve,reject};};
const settle=async()=>{for(let i=0;i<12;i++)await Promise.resolve();};
function fixture(){const owner=new WorkspaceRefresh(),calls=[],landed=[],failed=[];const attach=()=>owner.attach(()=>{const p=deferred();calls.push(p);return p.promise;},value=>landed.push(value),error=>failed.push(error));return{owner,calls,landed,failed,attach};}
let count=0;async function check(name,run){await run();console.log(`ok ${++count} - ${name}`);}
await check("100 invalidations keep one physical request plus one fresh followup",async()=>{
 const f=fixture(),close=f.attach();await settle();assert.equal(f.calls.length,1);for(let i=0;i<100;i++)f.owner.invalidate();assert.equal(f.calls.length,1);
 f.calls[0].resolve("stale");await settle();assert.deepEqual(f.landed,[]);assert.equal(f.calls.length,2);f.calls[1].resolve("fresh");await settle();assert.deepEqual(f.landed,["fresh"]);close();
});
await check("detach rejects results and cannot launch a hidden followup",async()=>{
 const f=fixture(),close=f.attach();await settle();f.owner.invalidate();close();f.calls[0].resolve("departed");await settle();assert.equal(f.calls.length,1);assert.deepEqual(f.landed,[]);
});
await check("quick reattach waits for the old physical request before reading its own view",async()=>{
 const f=fixture(),close=f.attach();await settle();close();const again=f.attach();await settle();assert.equal(f.calls.length,1);f.calls[0].resolve("old");await settle();assert.equal(f.calls.length,2);f.calls[1].resolve("new");await settle();assert.deepEqual(f.landed,["new"]);again();
});
await check("failure stays idle until a new explicit invalidation",async()=>{
 const f=fixture(),close=f.attach();await settle();f.calls[0].reject(new Error("offline"));await settle();assert.equal(f.failed.length,1);assert.equal(f.calls.length,1);f.owner.invalidate();await settle();assert.equal(f.calls.length,2);f.calls[1].resolve("retry");await settle();assert.deepEqual(f.landed,["retry"]);close();
});
await check("same-task detach skips a not-yet-started read and StrictMode can attach again",async()=>{
 const f=fixture(),close=f.attach();close();await settle();assert.equal(f.calls.length,0);const again=f.attach();await settle();assert.equal(f.calls.length,1);f.calls[0].resolve("current");await settle();assert.deepEqual(f.landed,["current"]);again();
});
console.log(`${count} workspace metadata coalescing checks passed`);
