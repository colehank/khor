// Controlled replies through FileBody's actual merge, edit and text callbacks.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {build,transform} from 'esbuild';
const legacy=process.argv.includes('--legacy');
const source=legacy?execFileSync('git',['show','e7c335c8:apps/gui/src/views/FileView.tsx'],{encoding:'utf8'}):readFileSync('src/views/FileView.tsx','utf8');
const mergeSource=source.match(/  const merge = \(\) => \{[\s\S]*?\n  \};/)?.[0];
const editSource=source.match(/  const setEdit = useCallback\([\s\S]*?\n  \);/)?.[0];
const textSource=source.match(/          onText=\{(\(t\) => \{[\s\S]*?\n          \})\}/)?.[1];
const publishSource=source.match(/  const publishDraft = useCallback\([\s\S]*?\n  \}, \[[^\]]*\]\);/)?.[0];
assert(mergeSource&&editSource&&textSource&&(legacy||publishSource),'production merge, edit and draft callbacks exist');
const compile=async source=>(await transform(source,{loader:'tsx'})).code;
const mergeCode=await compile(mergeSource),editCode=await compile(editSource),publishCode=publishSource?await compile(publishSource):null;
const {fileRefresh}=await import('data:text/javascript;base64,'+Buffer.from((await build({entryPoints:['src/lib/file-refresh.ts'],bundle:true,format:'esm',write:false})).outputFiles[0].text).toString('base64'));
const deferred=()=>{let resolve,reject;const promise=new Promise((yes,no)=>{resolve=yes;reject=no;});return {promise,resolve,reject};};
const tick=()=>new Promise(resolve=>setImmediate(resolve));
function fixture(){
 const read=deferred(),merged=deferred(),refresh={current:fileRefresh()},overwriteEpoch={current:0};
 let edit={text:'mine',was:'base',by:'line'},file={text:'base',digest:'base'},busy=false,clashes=null,errors=[],mergeCalls=0;
 const leaving={current:{edit}};
 const setEdit=Function('useCallback','setOwn','draft','refresh',editCode+';return setEdit;')(f=>f,update=>{edit=update(edit);leaving.current.edit=edit;},undefined,refresh);
 const published=[];
 const publishDraft=publishCode?Function('useCallback','file','onDraft','reportDraftCount',publishCode+';return publishDraft;')(f=>f,file,value=>published.push(value),()=>{throw new Error('delegated editor must not count twice');}):undefined;
 const type=Function('leaving','setEdit','publishDraft','return '+textSource)(leaving,setEdit,publishDraft);
 const env={edit,file,working:false,refresh,overwriteEpoch,leaving,
  setWorking:value=>busy=value,setOops:value=>{if(value!==null)errors.push(value);},
  fetchFileText:()=>read.promise,machine:'fixture',path:'/note',
  mergeFileText:()=>{mergeCalls++;return merged.promise;},
  setFile:update=>file=typeof update==='function'?update(file):update,setEdit,setClashes:value=>clashes=value,
  forget:()=>{},said:e=>e.message};
 const merge=Function(...Object.keys(env),mergeCode+';return merge;')(...Object.values(env));
 const finish=async()=>{read.resolve({text:'theirs',digest:'theirs',at_ms:2});await tick();merged.resolve({text:'merged',clashes:1});await tick();};
 return {merge,type,refresh,overwriteEpoch,read,merged,finish,get text(){return edit.text;},get base(){return file.text;},get busy(){return busy;},get errors(){return errors;},get calls(){return mergeCalls;},get clashes(){return clashes;}};
}
const results=[];
async function test(name,run){try{await run();results.push({name,pass:true});console.log('PASS',name);}catch(e){results.push({name,pass:false,error:e.message});console.error('FAIL',name,e.message);}}
await test('unchanged buffer accepts the complete merge and its base together',async()=>{const f=fixture();f.merge();await f.finish();assert.equal(f.text,'merged');assert.equal(f.base,'theirs');assert.equal(f.clashes,1);assert.equal(f.busy,false);});
await test('typing while the disk read is pending preserves current buffer and base',async()=>{const f=fixture();f.merge();f.type('new typing');await f.finish();assert.equal(f.text,'new typing');assert.equal(f.base,'base');assert.equal(f.clashes,null);assert.equal(f.busy,false);});
await test('typing while merge calculation is pending preserves current buffer',async()=>{const f=fixture();f.merge();f.read.resolve({text:'theirs',digest:'theirs'});await tick();f.type('new typing');f.merged.resolve({text:'merged',clashes:1});await tick();assert.equal(f.text,'new typing');assert.equal(f.base,'base');});
await test('editing then undoing back to the initial text still invalidates merge',async()=>{const f=fixture();f.merge();f.type('new');f.type('mine');await f.finish();assert.equal(f.text,'mine');assert.equal(f.base,'base');});
await test('composition without a doc change invalidates the pending merge',async()=>{const f=fixture();f.merge();f.refresh.current.setComposing(true);f.refresh.current.setComposing(false);await f.finish();assert.equal(f.text,'mine');assert.equal(f.base,'base');});
await test('another conflict action invalidates the old merge and owns working state',async()=>{const f=fixture();f.merge();f.overwriteEpoch.current++;await f.finish();assert.equal(f.text,'mine');assert.equal(f.base,'base');assert.equal(f.busy,true);});
await test('unmount invalidation prevents a merge landing in the departed view',async()=>{const f=fixture();f.merge();f.refresh.current.invalidate();await f.finish();assert.equal(f.text,'mine');assert.equal(f.base,'base');});
await test('merge does not start against active marked text',async()=>{const f=fixture();f.refresh.current.setComposing(true);f.merge();assert.equal(f.busy,false);});
await test('a failure from an invalidated merge cannot replace current feedback',async()=>{const f=fixture();f.merge();f.type('new typing');f.read.reject(new Error('old failure'));await tick();assert.deepEqual(f.errors,[]);assert.equal(f.busy,false);});
console.log(JSON.stringify({legacy,passed:results.filter(t=>t.pass).length,total:results.length,results},null,2));assert(results.every(t=>t.pass),'merge revision regressions');
