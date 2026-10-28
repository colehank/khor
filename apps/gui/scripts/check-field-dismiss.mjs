import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {build,transform} from 'esbuild';
const legacy=process.argv.includes('--legacy');
const source=legacy?execFileSync('git',['show','e7c335c8:apps/gui/src/hooks/use-field-dismiss.ts'],{encoding:'utf8'}):readFileSync('src/hooks/use-field-dismiss.ts','utf8');
const bundle=await build({stdin:{contents:source,loader:'ts'},bundle:true,write:false,format:'esm',plugins:[{name:'react-effect-fixture',setup(b){b.onResolve({filter:/^react$/},()=>({path:'react',namespace:'fixture'}));b.onLoad({filter:/.*/,namespace:'fixture'},()=>({contents:'export const useRef=current=>({current});export const useEffect=effect=>globalThis.installEffect(effect);'}));}}]});
const {useFieldDismiss}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
// Node's EventTarget treats capture booleans differently on removal.
class DocumentFixture extends EventTarget {
 addEventListener(type,listener,capture){super.addEventListener(type,listener,{capture:!!capture});}
 removeEventListener(type,listener,capture){super.removeEventListener(type,listener,{capture:!!capture});}
}
class Region {constructor(parent=null){this.parent=parent;}contains(node){for(let n=node;n;n=n.parent)if(n===this)return true;return false;}}
const wait=()=>new Promise(r=>setTimeout(r,5));
let cleanup;
function fixture(){
 const doc=new DocumentFixture(), field=new Region(),portal=new Region(),outer=new Region(),inside=new Region(field),item=new Region(portal),actions=[];
 globalThis.document=doc;globalThis.Node=Region;globalThis.installEffect=effect=>{cleanup=effect();};
 useFieldDismiss(true,()=>[field,portal],()=>actions.push('leave'));
 const fire=(type,node,properties={})=>{const e=new Event(type);Object.defineProperties(e,Object.fromEntries(Object.entries({button:0,pointerId:1,isPrimary:true,detail:1,...properties}).map(([k,value])=>[k,{value}])));e.composedPath=()=>[node];doc.dispatchEvent(e);};
 return {field,portal,outer,inside,item,actions,fire};
}
const results=[];
async function test(name,run){const f=fixture();try{await run(f);results.push({name,pass:true});console.log('PASS',name);}catch(e){results.push({name,pass:false,error:e.message});console.error('FAIL',name,e.message);}finally{cleanup?.();}}
await test('selection drag starting inside stays editing when released outside',async f=>{f.fire('pointerdown',f.inside);f.fire('pointerup',f.outer);f.fire('click',f.outer);await wait();assert.deepEqual(f.actions,[]);});
await test('portaled candidate is part of the field region',async f=>{f.fire('pointerdown',f.item);f.fire('click',f.item);await wait();assert.deepEqual(f.actions,[]);});
await test('first outside click acts before dismissing',async f=>{f.fire('pointerdown',f.outer);f.fire('click',f.outer);f.actions.push('activate');assert.deepEqual(f.actions,['activate']);await wait();assert.deepEqual(f.actions,['activate','leave']);});
await test('cancelled internal pointer does not swallow following outside click',async f=>{f.fire('pointerdown',f.inside);f.fire('pointercancel',f.inside);f.fire('click',f.outer);await wait();assert.deepEqual(f.actions,['leave']);});
await test('keyboard click is independent of an unfinished pointer',async f=>{f.fire('pointerdown',f.inside);f.fire('click',f.outer,{detail:0});await wait();assert.deepEqual(f.actions,['leave']);});
await test('next outside pointer replaces old drag ownership even without a click',async f=>{f.fire('pointerdown',f.inside);f.fire('pointerup',f.outer);f.fire('pointerdown',f.outer);f.fire('click',f.outer);await wait();assert.deepEqual(f.actions,['leave']);});
await test('right click does not dismiss editing',async f=>{f.fire('pointerdown',f.outer,{button:2});f.fire('click',f.outer,{button:2});await wait();assert.deepEqual(f.actions,[]);});
await test('unmount cancels pending dismiss and removes listeners',async f=>{f.fire('click',f.outer);cleanup();cleanup=null;await wait();assert.deepEqual(f.actions,[]);f.fire('click',f.outer);await wait();assert.deepEqual(f.actions,[]);});
// Keep the picker -> search -> dismiss wiring in the behavioral check.
const mobile=legacy?execFileSync('git',['show','e7c335c8:apps/gui/src/views/MobileSessionContent.tsx'],{encoding:'utf8'}):readFileSync('src/views/MobileSessionContent.tsx','utf8');
const searchFunction=mobile.match(/export function ContentSearch\([\s\S]*?\n\}/)?.[0];
const searchMarkup=mobile.match(/ const search=<div[^\n]+;/)?.[0];
const openCallback=mobile.match(/ const open=\(d:DeviceRow,path:string[^\n]+;/)?.[0];
const stopSearch=mobile.match(/ const stopSearch=\(\)=>\{[^\n]+?\};/)?.[0];
assert(searchFunction&&searchMarkup&&stopSearch,'production search and dismissal callbacks exist');
const compile=async source=>(await transform(source,{loader:'tsx'})).code;
const searchCode=await compile(searchFunction.replace('export ','')),markupCode=await compile(searchMarkup),stopCode=await compile(stopSearch),openCode=await compile(openCallback);
function searchFixture(f){
 cleanup?.();cleanup=null;
 let searching=true,query='needle';
 const gui={tree_search:'Search',search_clear:'Clear'};
 const React={createElement:(type,props,...children)=>({type,props:props??{},children})};
 const stop=Function('document','setSearching','setQuery',stopCode+';return stopSearch;')(document,value=>searching=value,value=>query=value);
 const Search=Function('React','useRef','useFieldDismiss','IconSearch','Input','IconClose','gui','imeKey',searchCode+';return ContentSearch;')(React,current=>({current}),useFieldDismiss,'IconSearch','Input','IconClose',gui,e=>e.isComposing);
 const markup=Function('React','ContentSearch','gui','state','setQuery','stopSearch',markupCode+';return search;')(React,Search,gui,{query},value=>query=value,stop);
 const tree=Search(markup.children[0].props);
 if(tree.props.ref)tree.props.ref.current=f.field;
 const input=tree.children.find(child=>child?.type==='Input');
 const key=(key,isComposing=false)=>input.props.onKeyDown({key,isComposing,preventDefault(){},stopPropagation(){}});
 const activate=Function('stopSearch','endEdit','onOpen',openCode+';return open;')(stop,()=>{},()=>{f.actions.push('open');cleanup?.();cleanup=null;});
 const clear=()=>tree.children.find(child=>child?.type==='button').props.onClick();
 return {key,clear,activate,get searching(){return searching;},get query(){return query;}};
}
await test('picker search outside click exits and clears the filter after activation',async f=>{const search=searchFixture(f);f.fire('pointerdown',f.outer);f.fire('click',f.outer);assert.equal(search.searching,true);await wait();assert.equal(search.searching,false);assert.equal(search.query,'');});
await test('picker search Escape exits and clears the filter',async f=>{const search=searchFixture(f);search.key('Escape');assert.equal(search.searching,false);assert.equal(search.query,'');});
await test('IME Escape remains owned by composition',async f=>{const search=searchFixture(f);search.key('Escape',true);assert.equal(search.searching,true);assert.equal(search.query,'needle');});
await test('search clear button keeps editing with an empty query',async f=>{const search=searchFixture(f);f.fire('pointerdown',f.inside);search.clear();f.fire('click',f.inside);await wait();assert.equal(search.searching,true);assert.equal(search.query,'');});
await test('opening a result clears search before its component unmounts',async f=>{const search=searchFixture(f);f.fire('pointerdown',f.outer);f.fire('click',f.outer);search.activate({},'/result','file');await wait();assert.deepEqual(f.actions,['open']);assert.equal(search.searching,false);assert.equal(search.query,'');});
console.log(JSON.stringify({legacy,passed:results.filter(t=>t.pass).length,total:results.length,results},null,2));
assert(results.every(t=>t.pass),'field dismissal regressions');
