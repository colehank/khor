import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import ts from "typescript";
const before=process.argv.find(argument=>argument.startsWith("--before="))?.slice("--before=".length);
const text=readFileSync(before??"src/views/FileView.tsx","utf8");
const source=ts.createSourceFile("FileView.tsx",text,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
function one(predicate){const found=[];const visit=node=>{if(predicate(node))found.push(node);ts.forEachChild(node,visit);};visit(source);assert.equal(found.length,1);return found[0];}
function execute(text,values={}){const compiled=ts.transpileModule(`return (${text});`,{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText;return Function(...Object.keys(values),compiled)(...Object.values(values));}
const deferred=()=>{let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return{promise,resolve,reject};};
const settle=async()=>{for(let i=0;i<12;i++)await Promise.resolve();};
if(before){
 const effect=one(node=>ts.isCallExpression(node)&&node.expression.getText(source)==="useEffect"&&node.arguments[0]?.getText(source).includes("fetchPreview(machine, path)"));
 const original={kind:"image",data:"old",total:3};let look=original;const pending=deferred();
 execute(effect.arguments[0].getText(source),{kindOf:()=>"image",path:"/a.png",machine:"fixture",nativeOffice:false,handoff:{previewSupported:false},fetchPreview:()=>pending.promise,setLook:value=>{look=value;},setLookError:()=>{}})();
 assert.equal(look,original,"refreshing the same image must keep its accepted content until replacement succeeds");
} else {
 const factory=one(node=>ts.isFunctionDeclaration(node)&&node.name?.text==="previewRequests");
 const make=execute(factory.getText(source));let count=0;
 async function check(name,run){await run();console.log(`ok ${++count} - ${name}`);}
 function fixture(){const owner=make(),calls=[],landed=[],failed=[];owner.activate();const ask=(identity="same")=>owner.ask(()=>{const pending=deferred();calls.push({identity,...pending});return pending.promise;},value=>landed.push(value),error=>failed.push(error));return{owner,calls,landed,failed,ask};}
 await check("preview refresh keeps one physical IO and accepts only the latest requested result",async()=>{
  const f=fixture();f.ask();await settle();for(let i=0;i<100;i++)f.ask();assert.equal(f.calls.length,1);f.calls[0].resolve("stale");await settle();assert.deepEqual(f.landed,[]);assert.equal(f.calls.length,2);f.calls[1].resolve("fresh");await settle();assert.deepEqual(f.landed,["fresh"]);f.owner.leave();
 });
 await check("departing rejects old image data and starts no queued IO",async()=>{
  const f=fixture();f.ask();await settle();f.ask();f.owner.leave();f.calls[0].resolve("old");await settle();assert.deepEqual(f.landed,[]);assert.equal(f.calls.length,1);
 });
 await check("a new identity waits for prior IO but never accepts its payload",async()=>{
  const f=fixture();f.ask("A");await settle();f.owner.leave();f.owner.activate();f.ask("B");assert.equal(f.calls.length,1);f.calls[0].resolve("A");await settle();assert.equal(f.calls[1].identity,"B");f.calls[1].resolve("B");await settle();assert.deepEqual(f.landed,["B"]);f.owner.leave();
 });
 await check("failure reports once without a hot retry, and explicit retry can succeed",async()=>{
  const f=fixture();f.ask();await settle();f.calls[0].reject(new Error("offline"));await settle();assert.equal(f.failed.length,1);assert.equal(f.calls.length,1);f.ask();await settle();f.calls[1].resolve("retry");await settle();assert.deepEqual(f.landed,["retry"]);f.owner.leave();
 });
 await check("layout detach before a queued read prevents starting it",async()=>{
  const f=fixture();f.ask();f.owner.leave();await settle();assert.equal(f.calls.length,0);f.owner.activate();f.ask();await settle();assert.equal(f.calls.length,1);f.calls[0].resolve("current");await settle();assert.deepEqual(f.landed,["current"]);f.owner.leave();
 });
 await check("actual preview effect retains accepted bytes until a current replacement succeeds",async()=>{
  const effect=one(node=>ts.isCallExpression(node)&&node.expression.getText(source)==="useEffect"&&node.arguments[0]?.getText(source).includes("previewReader.current.ask"));
  assert.ok(effect.arguments[1].elements.some(node=>node.getText(source)==="readAt"),"external invalidation reaches this actual effect");
  const owner=make();owner.activate();const pending=deferred(),retry=deferred();const original={key:"same",preview:{kind:"image",data:"old",total:3},total:3};let accepted=original,failure=null,calls=0;
  const refresh=execute(effect.arguments[0].getText(source),{previewMode:"preview",previewReader:{current:owner},machine:"fixture",path:"/image.png",previewKey:"same",setLookFailure:value=>{failure=value;},setLookValue:value=>{accepted=value;},fetchPreview:()=>[pending,retry][calls++].promise,fetchFileStat:()=>{throw Error("image never asks independent stat");},said:error=>error.message});
  refresh();assert.equal(accepted,original);await settle();assert.equal(calls,1);pending.reject(new Error("offline"));await settle();assert.equal(accepted,original,"same-file failure retains the identical accepted image record");assert.deepEqual(failure,{key:"same",error:"offline"});assert.equal(calls,1,"failure never schedules its own retry");
  refresh();assert.equal(accepted,original);await settle();assert.equal(calls,2);retry.resolve({kind:"image",data:"new",total:3});await settle();assert.equal(accepted.preview.data,"new");assert.equal(failure,null);owner.leave();
 });
 await check("the same actual effect handles native office metadata without preview, handoff or a second stat",async()=>{
  const effect=one(node=>ts.isCallExpression(node)&&node.expression.getText(source)==="useEffect"&&node.arguments[0]?.getText(source).includes("previewReader.current.ask"));
  const owner=make();owner.activate();let calls=0,accepted;
  execute(effect.arguments[0].getText(source),{previewMode:"stat",previewReader:{current:owner},machine:"fixture",path:"/office.docx",previewKey:"office",setLookFailure:()=>{},setLookValue:value=>{accepted=value;},fetchPreview:()=>{throw Error("not the native office path");},fetchFileStat:async()=>{calls++;return{total:123};},said:error=>error.message})();
  await settle();assert.equal(calls,1);assert.deepEqual(accepted,{key:"office",preview:null,total:123});owner.leave();
  const away=one(node=>ts.isFunctionDeclaration(node)&&node.name?.text==="Away");const callsInside=[];const walk=node=>{if(ts.isCallExpression(node))callsInside.push(node.expression.getText(source));ts.forEachChild(node,walk);};walk(away);assert.equal(callsInside.includes("fetchFileStat"),false,"Away no longer owns a parallel metadata road");
 });
 await check("render expressions reject previous-identity pixels, metadata and errors before effects",()=>{
  const values={lookValue:{key:"A",preview:{data:"A"},total:12},lookFailure:{key:"A",error:"old"},previewKey:"B"};
  for(const name of ["look","lookTotal","lookError"]){const node=one(node=>ts.isVariableDeclaration(node)&&node.name.getText(source)===name);assert.equal(execute(`()=>${node.initializer.getText(source)}`,values)(),null);}
 });
 console.log(`${count} actual private preview-request/effect checks passed (no DOM)`);
}
