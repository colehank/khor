// Executes FileBody's real refresh callback with controlled transport replies.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {build,transform} from 'esbuild';
import ts from 'typescript';
const legacy = process.argv.includes('--legacy');
const source = legacy ? execFileSync('git', ['show', 'e7c335c8:apps/gui/src/views/FileView.tsx'], {encoding:'utf8'}) : readFileSync('src/views/FileView.tsx','utf8');
const parsed=ts.createSourceFile('FileView.tsx',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
const declarations={};let textCallback,compositionCallback;
function visit(node){
 if(ts.isVariableStatement(node))for(const declaration of node.declarationList.declarations)declarations[declaration.name.getText(parsed)]=node.getText(parsed);
 if(ts.isJsxAttribute(node)&&node.initializer&&ts.isJsxExpression(node.initializer)){
  if(node.name.getText(parsed)==='onText')textCallback=node.initializer.expression.getText(parsed);
  if(node.name.getText(parsed)==='onComposition')compositionCallback=node.initializer.expression.getText(parsed);
 }
 ts.forEachChild(node,visit);
}
visit(parsed);
const callback=declarations.again,editCallback=declarations.setEdit;
assert(callback&&editCallback&&textCallback,'FileBody refresh/edit/text callbacks located');
const editCode=(await transform(editCallback,{loader:'tsx'})).code;
const refreshCode=(await transform((declarations.readNeeded??'')+'\n'+callback,{loader:'tsx'})).code;
const makeRefresh = legacy ? () => ({invalidate(){}, setComposing(){}}) : (await import(`data:text/javascript;base64,${Buffer.from((await build({entryPoints:['src/lib/file-refresh.ts'],bundle:true,write:false,format:'esm'})).outputFiles[0].text).toString('base64')}`)).fileRefresh;
const deferred = () => { let resolve,reject; const promise = new Promise((yes,no)=>{resolve=yes;reject=no;}); return {promise,resolve,reject}; };
const tick = () => new Promise(resolve=>setImmediate(resolve));
const file = text => ({text,digest:text,total:text.length,at_ms:1,cut:false,binary:false});
const tests = [];
async function test(name, run) {try {await run();tests.push({name,pass:true});console.log('PASS', name);} catch(e){tests.push({name,pass:false,error:e.message});console.error('FAIL',name,e.message);}}
function fixture() {
 const refresh={current:makeRefresh()},refreshReading={current:false},bufferLive={current:true},pending=[],applied=[];
 let activeReads=0,maximumReads=0;
 let shown='A', remembered=[];
 const accepted={current:{file:file('A'),edit:{text:'A'},clashes:null}};
 const acceptBuffer=change=>{const before=accepted.current;accepted.current=change(before);shown=accepted.current.edit?.text??'';if(accepted.current.file!==before.file)applied.push(accepted.current.file?.text);};
 const setEdit = Function('useCallback','setOwn','draft','refresh','acceptBuffer',editCode+'; return setEdit;')(f=>f, update=>{shown=update({text:shown})?.text ?? '';},undefined,refresh,acceptBuffer);
 const onText=Function('leaving','setEdit','return '+textCallback)({current:{edit:null}},setEdit);
 const env={kindOf:()=> 'text',path:'/fixture.txt',machine:'fixture',file:file('A'),dirty:false,refresh,accepted,acceptBuffer,bufferLive,refreshReading,
 fetchFileStat:async()=>file('changed'), unchanged:()=>false,
 fetchFileText:()=>{const d=deferred();pending.push(d);activeReads+=1;maximumReads=Math.max(maximumReads,activeReads);return d.promise.then(value=>{activeReads-=1;return value;},error=>{activeReads-=1;throw error;});},
 remember:(_,__,f)=>remembered.push(f.text),setFile:f=>applied.push(f.text),
 setEdit,newLine:()=> 'line',setDiffAt:()=>{}};
 const bound=Function(...Object.keys(env),'useCallback',refreshCode+';return {again,readNeeded:typeof readNeeded==="undefined"?undefined:readNeeded};')(...Object.values(env),f=>f);
 const again=bound.again;
 const compose=compositionCallback?Function('refresh','readNeeded','return '+compositionCallback)(refresh,bound.readNeeded):()=>{};
 return {again,resume:bound.readNeeded,compose,refresh,pending,applied,unmount(){bufferLive.current=false;refresh.current.invalidate();},get maximumReads(){return maximumReads;},get shown(){return shown;},get remembered(){return remembered;},type:onText};
}
await test('clean refresh still accepts changed disk contents',async()=>{const f=fixture();f.again();await tick();f.pending[0].resolve(file('B'));await tick();assert.equal(f.shown,'B');});
await test('typing while refresh is pending preserves draft and merge base',async()=>{const f=fixture();f.again();await tick();f.type('nihao');f.pending[0].resolve(file('B'));await tick();assert.equal(f.shown,'nihao');assert.deepEqual(f.applied,[]);assert.deepEqual(f.remembered,[]);});
await test('edit followed by undo still invalidates the old read',async()=>{const f=fixture();f.again();await tick();f.type('ni');f.type('A');f.pending[0].resolve(file('B'));await tick();assert.equal(f.shown,'A');assert.deepEqual(f.applied,[]);});
await test('read controller rejects older replies after a newer read has completed',async()=>{const refresh=makeRefresh(),one=deferred(),two=deferred(),accepted=[];const first=refresh.read(()=>one.promise,value=>accepted.push(value.text));const second=refresh.read(()=>two.promise,value=>accepted.push(value.text));two.resolve(file('new'));await second;one.resolve(file('old'));await first;assert.deepEqual(accepted,['new']);});
await test('composition starts before the read reply without changing text',async()=>{const f=fixture();f.again();await tick();f.refresh.current.setComposing(true);f.pending[0].resolve(file('B'));await tick();assert.equal(f.shown,'A');});
await test('composition ending does not revive an old read',async()=>{const f=fixture();f.again();await tick();f.refresh.current.setComposing(true);f.refresh.current.setComposing(false);f.pending[0].resolve(file('B'));await tick();assert.equal(f.shown,'A');});
await test('refresh during composition does not request disk contents',async()=>{const f=fixture();f.refresh.current.setComposing(true);f.again();await tick();assert.equal(f.pending.length,0);});
await test('unmount or document invalidation rejects pending reply',async()=>{const f=fixture();f.again();await tick();f.refresh.current.invalidate();f.pending[0].resolve(file('B'));await tick();assert.deepEqual(f.applied,[]);});
if(!legacy){
 await test('a refresh requested during composition retries when composition ends',async()=>{const f=fixture();f.compose(true);f.again();await tick();assert.equal(f.pending.length,0);f.compose(false);await tick();assert.equal(f.pending.length,1);f.pending[0].resolve(file('B'));await tick();assert.equal(f.shown,'B');});
 await test('composition invalidating an in-flight refresh retains a fresh-read request',async()=>{const f=fixture();f.again();await tick();f.compose(true);f.pending[0].resolve(file('old'));await tick();assert.equal(f.shown,'A');f.compose(false);await tick();assert.equal(f.pending.length,2);f.pending[1].resolve(file('new'));await tick();assert.equal(f.shown,'new');});
 await test('a refresh blocked by a draft retries only after the buffer becomes clean',async()=>{const f=fixture();f.type('draft');f.again();await tick();assert.equal(f.pending.length,0);f.resume();await tick();assert.equal(f.pending.length,0);f.type('A');f.resume();await tick();assert.equal(f.pending.length,1);f.pending[0].resolve(file('B'));await tick();assert.equal(f.shown,'B');});
 await test('a completed unchanged read consumes the request without replaying it on composition end',async()=>{const f=fixture();f.again();await tick();f.pending[0].resolve(null);await tick();f.compose(true);f.compose(false);await tick();assert.equal(f.pending.length,1);assert.equal(f.refresh.current.needsRead(),false);});
 await test('failed text read and its existing fallback retain only the need for a later fresh request',async()=>{const f=fixture();f.again();await tick();f.pending[0].reject(new Error('fixture offline'));await tick();f.pending[1].reject(new Error('fixture fallback offline'));await tick();assert.equal(f.refresh.current.needsRead(),true);f.resume();await tick();assert.equal(f.pending.length,3);f.pending[2].resolve(file('B'));await tick();assert.equal(f.shown,'B');assert.equal(f.refresh.current.needsRead(),false);});
 await test('new requests during a valid read remain pending for a fresh followup',async()=>{const refresh=makeRefresh(),answer=deferred();refresh.requestRead();const read=refresh.read(()=>answer.promise,()=>{});refresh.requestRead();answer.resolve(null);await read;assert.equal(refresh.needsRead(),true);});
 await test('file refresh coalesces a burst into one physical read and one fresh followup',async()=>{const f=fixture();f.again();f.again();f.again();await tick();assert.equal(f.pending.length,1);f.pending[0].resolve(file('first'));await tick();assert.equal(f.pending.length,2);f.pending[1].resolve(file('latest'));await tick();assert.equal(f.pending.length,2);assert.equal(f.maximumReads,1);assert.equal(f.shown,'latest');assert.equal(f.refresh.current.needsRead(),false);});
 await test('failed burst gets one fresh followup without a failure retry loop',async()=>{const f=fixture();f.again();f.again();await tick();f.pending[0].reject(new Error('first'));await tick();f.pending[1].reject(new Error('first fallback'));await tick();assert.equal(f.pending.length,3);f.pending[2].reject(new Error('followup'));await tick();f.pending[3].reject(new Error('followup fallback'));await tick();await tick();assert.equal(f.pending.length,4);assert.equal(f.maximumReads,1);assert.equal(f.refresh.current.needsRead(),true);});
 await test('departure during a read prevents a queued fresh followup',async()=>{const f=fixture();f.again();f.again();await tick();f.unmount();f.pending[0].resolve(file('late'));await tick();assert.equal(f.pending.length,1);assert.deepEqual(f.applied,[]);});
 await test('late composition end after departure cannot start the deferred disk read',async()=>{const f=fixture();f.compose(true);f.again();f.unmount();f.compose(false);await tick();assert.equal(f.pending.length,0);});
}
console.log(JSON.stringify({legacy,passed:tests.filter(t=>t.pass).length,total:tests.length,tests},null,2));
assert(tests.every(t=>t.pass), 'file refresh behavior regressions');
