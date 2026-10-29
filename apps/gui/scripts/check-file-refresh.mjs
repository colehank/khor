// Executes FileBody's real refresh callback with controlled transport replies.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {build,transform} from 'esbuild';
const legacy = process.argv.includes('--legacy');
const source = legacy ? execFileSync('git', ['show', 'e7c335c8:apps/gui/src/views/FileView.tsx'], {encoding:'utf8'}) : readFileSync('src/views/FileView.tsx','utf8');
const callback = source.match(/  const again = \(\) => \{[\s\S]*?\n  \};/)?.[0];
assert(callback, 'FileBody refresh callback exists');
const editCallback = source.match(/  const setEdit = useCallback\([\s\S]*?\n  \);/)?.[0];
const textCallback = source.match(/          onText=\{(\(t\) => \{[\s\S]*?\n          \})\}/)?.[1];
assert(editCallback && textCallback, 'FileBody edit and editor text callbacks exist');
const editCode = (await transform(editCallback, {loader:'tsx',format:'esm'})).code;
const makeRefresh = legacy ? () => ({invalidate(){}, setComposing(){}}) : (await import(`data:text/javascript;base64,${Buffer.from((await build({entryPoints:['src/lib/file-refresh.ts'],bundle:true,write:false,format:'esm'})).outputFiles[0].text).toString('base64')}`)).fileRefresh;
const deferred = () => { let resolve; const promise = new Promise(r=>resolve=r); return {promise,resolve}; };
const tick = () => new Promise(resolve=>setImmediate(resolve));
const file = text => ({text,digest:text,total:text.length,at_ms:1,cut:false,binary:false});
const tests = [];
async function test(name, run) {try {await run();tests.push({name,pass:true});console.log('PASS', name);} catch(e){tests.push({name,pass:false,error:e.message});console.error('FAIL',name,e.message);}}
function fixture() {
 const refresh={current:makeRefresh()}, pending=[], applied=[];
 let shown='A', remembered=[];
 const accepted={current:{file:file('A'),edit:{text:'A'},clashes:null}};
 const acceptBuffer=change=>{const before=accepted.current;accepted.current=change(before);shown=accepted.current.edit?.text??'';if(accepted.current.file!==before.file)applied.push(accepted.current.file?.text);};
 const setEdit = Function('useCallback','setOwn','draft','refresh','acceptBuffer',editCode+'; return setEdit;')(f=>f, update=>{shown=update({text:shown})?.text ?? '';},undefined,refresh,acceptBuffer);
 const onText=Function('leaving','setEdit','return '+textCallback)({current:{edit:null}},setEdit);
 const env={kindOf:()=> 'text',path:'/fixture.txt',machine:'fixture',file:file('A'),dirty:false,refresh,accepted,acceptBuffer,
 fetchFileStat:async()=>file('changed'), unchanged:()=>false,
 fetchFileText:()=>{const d=deferred();pending.push(d);return d.promise;},
 remember:(_,__,f)=>remembered.push(f.text),setFile:f=>applied.push(f.text),
 setEdit,newLine:()=> 'line',setDiffAt:()=>{}};
 const again = Function(...Object.keys(env), callback+'; return again;')(...Object.values(env));
 return {again,refresh,pending,applied,get shown(){return shown;},get remembered(){return remembered;},type:onText};
}
await test('clean refresh still accepts changed disk contents',async()=>{const f=fixture();f.again();await tick();f.pending[0].resolve(file('B'));await tick();assert.equal(f.shown,'B');});
await test('typing while refresh is pending preserves draft and merge base',async()=>{const f=fixture();f.again();await tick();f.type('nihao');f.pending[0].resolve(file('B'));await tick();assert.equal(f.shown,'nihao');assert.deepEqual(f.applied,[]);assert.deepEqual(f.remembered,[]);});
await test('edit followed by undo still invalidates the old read',async()=>{const f=fixture();f.again();await tick();f.type('ni');f.type('A');f.pending[0].resolve(file('B'));await tick();assert.equal(f.shown,'A');assert.deepEqual(f.applied,[]);});
await test('newer request wins when replies arrive in reverse order',async()=>{const f=fixture();f.again();await tick();f.again();await tick();f.pending[1].resolve(file('new'));await tick();f.pending[0].resolve(file('old'));await tick();assert.equal(f.shown,'new');});
await test('composition starts before the read reply without changing text',async()=>{const f=fixture();f.again();await tick();f.refresh.current.setComposing(true);f.pending[0].resolve(file('B'));await tick();assert.equal(f.shown,'A');});
await test('composition ending does not revive an old read',async()=>{const f=fixture();f.again();await tick();f.refresh.current.setComposing(true);f.refresh.current.setComposing(false);f.pending[0].resolve(file('B'));await tick();assert.equal(f.shown,'A');});
await test('refresh during composition does not request disk contents',async()=>{const f=fixture();f.refresh.current.setComposing(true);f.again();await tick();assert.equal(f.pending.length,0);});
await test('unmount or document invalidation rejects pending reply',async()=>{const f=fixture();f.again();await tick();f.refresh.current.invalidate();f.pending[0].resolve(file('B'));await tick();assert.deepEqual(f.applied,[]);});
console.log(JSON.stringify({legacy,passed:tests.filter(t=>t.pass).length,total:tests.length,tests},null,2));
assert(tests.every(t=>t.pass), 'file refresh behavior regressions');
