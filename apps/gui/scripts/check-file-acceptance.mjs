import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import ts from 'typescript';
import {build,transform} from 'esbuild';

const source=ts.createSourceFile('FileView.tsx',readFileSync('src/views/FileView.tsx','utf8'),ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
const callbacks={};
function visit(node) {
  if(ts.isVariableStatement(node))for(const declaration of node.declarationList.declarations) {
    const name=declaration.name.getText(source);
    if(['acceptBuffer','setEdit','setClashes','publishDraft','flush','save','merge','mine','theirs'].includes(name))callbacks[name]=node.getText(source);
    if(name==='take'&&declaration.initializer?.parameters?.[0]?.type?.getText(source)==='FileText')callbacks.loaded=node.getText(source);
  }
  if(ts.isCallExpression(node)&&node.expression.getText(source)==='onLanded')callbacks.landed=node.arguments[2].getText(source);
  if(ts.isCallExpression(node)&&node.expression.getText(source)==='useEffect') {
    const effect=node.arguments[0];
    if(effect&&ts.isArrowFunction(effect)&&ts.isCallExpression(effect.body)&&effect.body.expression.getText(source)==='publishDraft')callbacks.passive=effect.getText(source);
  }
  if(ts.isJsxAttribute(node)&&['onText','onChunks'].includes(node.name.getText(source)))callbacks[node.name.getText(source)]=node.initializer.expression.getText(source);
  ts.forEachChild(node,visit);
}
visit(source);
for(const name of ['acceptBuffer','setEdit','setClashes','publishDraft','flush','save','merge','mine','theirs','loaded','landed','passive','onText','onChunks']) {
  assert(callbacks[name],name+' production callback found');
  callbacks[name]=(await transform(callbacks[name],{loader:'tsx'})).code;
}
const {fileRefresh}=await import('data:text/javascript;base64,'+Buffer.from((await build({entryPoints:['src/lib/file-refresh.ts'],bundle:true,format:'esm',write:false})).outputFiles[0].text).toString('base64'));
function fixture({text='A',digest='A',clashes=null,mergeClashes=1,by='line',retained=false}={}) {
  const accepted={current:{file:{text:'A',digest:'A',total:1,at_ms:1},edit:{text,was:digest,by},clashes}};
  const live={current:true}, publication={current:()=>{}}, projections=[], published=[], publishedStates=[], writes=[], road={trouble:null,busy:false};
  const refresh={current:fileRefresh()}, overwriteEpoch={current:0};
  const acceptBuffer=Function('useCallback','bufferLive','accepted','publishAccepted','projectBuffer',callbacks.acceptBuffer+';return acceptBuffer;')(fn=>fn,live,accepted,publication,next=>projections.push(next));
  const setEdit=Function('useCallback','refresh','acceptBuffer',callbacks.setEdit+';return setEdit;')(fn=>fn,refresh,acceptBuffer);
  const setClashes=Function('useCallback','acceptBuffer',callbacks.setClashes+';return setClashes;')(fn=>fn,acceptBuffer);
  const onDraft=draft=>published.push(draft);
  const publishDraft=Function('useCallback','accepted','onDraft','reportDraftCount',callbacks.publishDraft+';return publishDraft;')(fn=>fn,accepted,onDraft,()=>{throw new Error('delegated buffer must not count twice');});
  publication.current=()=>{publishedStates.push(accepted.current);publishDraft(accepted.current.edit);};
  let resolveOverwrite;
  const overwrite=new Promise(resolve=>resolveOverwrite=resolve);
  const env={accepted,bufferLive:live,edit:accepted.current.edit,publishDraft,refresh,acceptBuffer,setEdit,setClashes,setOops:()=>{},setDiffAt:()=>{},savedRef:{current:null},bytesOf:text=>Buffer.byteLength(text),machine:'fixture',path:'/note',roadOf:()=>road,write:async(_machine,_path,draft,options)=>{writes.push({draft,options});},leavingCallbacks:{current:{onDraft,onSaved:()=>{}}},confirmSaved:()=>{},overwriteEpoch,setWorking:()=>{},forget:()=>{},onDraft,setReread:()=>{},working:false,fetchFileText:async()=>({text:'B',digest:'B',at_ms:2}),mergeFileText:async()=>({text:'merged',clashes:mergeClashes}),said:error=>error.message,readForOverwrite:()=>overwrite,remember:()=>{},live:true,draft:retained?accepted.current.edit:undefined,newLine:()=> 'loaded-line'};
  const bind=(name,expression=false)=>Function(...Object.keys(env),expression?'return '+callbacks[name]:callbacks[name]+';return '+(name==='loaded'?'take':name)+';')(...Object.values(env));
  const type=Function('setEdit','return '+callbacks.onText)(setEdit);
  const chunks=Function('setClashes','return '+callbacks.onChunks)(setClashes);
  return {accepted,live,projections,published,publishedStates,writes,road,type,chunks,landed:bind('landed',true),passive:bind('passive',true),flush:bind('flush'),save:bind('save'),merge:bind('merge'),mine:bind('mine'),loaded:bind('loaded'),resolveOverwrite,theirs:bind('theirs')};
}
const tick=()=>new Promise(resolve=>setImmediate(resolve));
const results=[];
async function test(name,run) {try{await run();results.push({name,pass:true});}catch(error){results.push({name,pass:false,error:error.message});}}
await test('first accepted edit flushes before React commits',async()=>{const f=fixture();f.type('B');await f.flush();assert.equal(f.writes.length,1);assert.equal(f.writes[0].draft.text,'B');});
await test('save followed by same-task undo remains dirty against accepted disk base',()=>{const f=fixture({text:'B'});f.landed({text:'B',digest:'B',at_ms:2,by:'line'});f.type('A');assert.equal(f.published.at(-1)?.text,'A');});
await test('same-task typing after save keeps the accepted digest and full text',()=>{const f=fixture({text:'B'});f.landed({text:'B',digest:'B',at_ms:2,by:'line'});f.type('C');assert.equal(f.accepted.current.edit.was,'B');assert.equal(f.projections.at(-1).edit.text,'C');assert.equal(f.projections.at(-1).edit.was,'B');});
for(const count of [0,1])await test('merge '+count+' publishes base, edit and permission atomically before departure',async()=>{const f=fixture({text:'mine',mergeClashes:count});f.merge();await tick();assert.equal(f.accepted.current.file.digest,'B');assert.equal(f.accepted.current.edit.text,'merged');assert.equal(f.accepted.current.clashes,count);assert.equal(f.publishedStates.at(-1).clashes,count);await f.flush();assert.equal(f.writes.length,0);});
await test('latest road clash prevents departure even with a stale React road projection',async()=>{const f=fixture();f.type('B');f.road.trouble={kind:'clash'};await f.flush();assert.equal(f.writes.length,0);});
await test('failed road remains retryable on departure',async()=>{const f=fixture();f.type('B');f.road.trouble={kind:'failed'};await f.flush();assert.equal(f.writes.length,1);});
await test('an incomplete file without a digest never flushes',async()=>{const f=fixture({digest:''});f.type('B');await f.flush();assert.equal(f.writes.length,0);});
await test('resolving the last merge chunk re-enables departure save synchronously',async()=>{const f=fixture({text:'merged',clashes:1});f.chunks(0);await f.flush();assert.equal(f.writes.length,1);assert.equal(f.writes[0].draft.text,'merged');});
await test('a different editing line landing preserves current pin and merge permission',async()=>{const f=fixture({text:'merged',clashes:0});f.landed({text:'B',digest:'B',at_ms:2,by:'foreign'});assert.equal(f.accepted.current.file.digest,'B');assert.equal(f.accepted.current.edit.was,'A');assert.equal(f.accepted.current.clashes,0);await f.flush();assert.equal(f.writes.length,0);});
await test('save landing re-pins the current buffer without replacing intervening typing',()=>{const f=fixture({text:'C'});f.landed({text:'B',digest:'B',at_ms:2,by:'line'});assert.equal(f.accepted.current.edit.text,'C');assert.equal(f.accepted.current.edit.was,'B');});
await test('explicit Save can accept a merge while automatic Save cannot',async()=>{const f=fixture({text:'merged',clashes:0});f.save(false);assert.equal(f.writes.length,0);f.save(true);assert.equal(f.writes.length,1);assert.equal(f.writes[0].options.asked,true);});
await test('discard clears the accepted edit rather than reviving a retained draft',()=>{const f=fixture({text:'draft',clashes:1});f.theirs();assert.equal(f.accepted.current.edit,null);assert.equal(f.accepted.current.clashes,null);assert.equal(f.published.at(-1),null);f.type('late old text');assert.equal(f.accepted.current.edit,null);});
await test('unmounted editor callbacks cannot publish or project another draft',()=>{const f=fixture();f.live.current=false;f.type('late');assert.equal(f.accepted.current.edit.text,'A');assert.equal(f.published.length,0);assert.equal(f.projections.length,0);});
await test('initial clean load publishes the matching file and edit together',()=>{const f=fixture();f.loaded({text:'B',digest:'B',at_ms:2});assert.equal(f.accepted.current.file.digest,'B');assert.equal(f.accepted.current.edit.text,'B');assert.equal(f.accepted.current.edit.was,'B');assert.equal(f.published.at(-1),null);});
await test('initial load preserves edits made to a retained draft while IO was pending',()=>{const f=fixture({text:'retained',retained:true});f.type('typed while loading');f.loaded({text:'B',digest:'B',at_ms:2});assert.equal(f.accepted.current.edit.text,'typed while loading');assert.equal(f.accepted.current.edit.was,'A');assert.equal(f.published.at(-1).text,'typed while loading');});
const overwriteRead={file:{text:'B',digest:'B',at_ms:2},pin:text=>({text,was:'B',by:'fresh'})};
await test('overwrite read pins the latest text and the next same-task edit to one new line',async()=>{const f=fixture({text:'mine'});f.mine();f.type('typed during read');f.resolveOverwrite(overwriteRead);await tick();assert.equal(f.accepted.current.file.digest,'B');assert.deepEqual(f.accepted.current.edit,{text:'typed during read',was:'B',by:'fresh'});f.type('typed after pin');f.save(true);assert.deepEqual(f.writes.at(-1).draft,{text:'typed after pin',was:'B',by:'fresh'});});
await test('discard invalidates an overwrite read before it can accept or write',async()=>{const f=fixture({text:'mine'});f.mine();f.theirs();f.resolveOverwrite(overwriteRead);await tick();assert.equal(f.accepted.current.edit,null);assert.equal(f.writes.length,0);});
await test('a deferred passive publication cannot replace a newer accepted edit with its old projection',()=>{const f=fixture();f.type('B');f.passive();assert.equal(f.published.at(-1)?.text,'B');});
await test('layout departure rejects an overwrite before passive epoch cleanup',async()=>{const f=fixture({text:'mine'});f.mine();f.live.current=false;f.resolveOverwrite(overwriteRead);await tick();assert.equal(f.writes.length,0);assert.equal(f.accepted.current.edit.text,'mine');});
console.log(JSON.stringify({passed:results.filter(r=>r.pass).length,total:results.length,results},null,2));
assert(results.every(r=>r.pass),'accepted file draft/base/flush contract');
