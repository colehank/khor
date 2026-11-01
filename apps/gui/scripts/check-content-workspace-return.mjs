import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {build,transform} from 'esbuild';
const legacy=process.argv.includes('--legacy');
const source=legacy?execFileSync('git',['show','e7c335c8:apps/gui/src/views/MobileSessionContent.tsx'],{encoding:'utf8'}):readFileSync('src/views/MobileSessionContent.tsx','utf8');
const callback=source.match(/ const returnWorkspace=\(\)=>\{[^\n]+?\};/)?.[0];
assert(callback,'picker needs an explicit return-to-current-workspace action');
const condition=source.match(/ const canReturnWorkspace=([^\n]+);/)?.[1];
assert(condition,'picker exposes return only for a different workspace');
const code=(await transform(callback,{loader:'ts'})).code;
const bundle=await build({stdin:{contents:"export {useContentNavigation} from './src/hooks/use-content-navigation';export {sessionCwdOnScreen} from './src/lib/session-cwd';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false,plugins:[{name:'navigation-hooks-fixture',setup(b){b.onResolve({filter:/^react$/},()=>({path:'react',namespace:'fixture'}));b.onLoad({filter:/.*/,namespace:'fixture'},()=>({contents:'export const useState=v=>[typeof v==="function"?v():v,()=>{}];export const useRef=current=>({current});export const useEffect=()=>{};'}));}}]});
const {useContentNavigation,sessionCwdOnScreen}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
globalThis.localStorage={getItem:()=>null,setItem(){}};
globalThis.document={querySelector:()=>null};
const results=[];
async function test(name,run){try{await run();results.push({name,pass:true});console.log('PASS',name);}catch(e){results.push({name,pass:false,error:e.message});console.error('FAIL',name,e.message);}}
const available=(state,home,row)=>Function('state','home','row','sessionCwdOnScreen','return '+condition)(state,home,row,sessionCwdOnScreen);
await test('same-machine directory override offers a return action',()=>assert.equal(available({source:'workspace',scope:{device:'home-id',root:'/elsewhere'}},{id:'home-id'},{id:'s',place:'/session'}),true));
await test('different-machine override offers the same return action',()=>assert.equal(available({source:'workspace',scope:{device:'other-id',root:'/session'}},{id:'home-id'},{id:'s',place:'/session'}),true));
await test('current workspace and unknown home do not offer an unrelated Back',()=>{for(const [state,home] of [[{source:'workspace',scope:{device:'home-id',root:'/session'}},{id:'home-id'}],[{source:'workspace',scope:{device:'other-id',root:'/elsewhere'}},null]])assert.equal(available(state,home,{id:'s',place:'/session'}),false);});
await test('browser picker has a distinct direct return to session workspace',()=>assert.equal(available({source:'browser',scope:null},{id:'home-id'},{id:'s',place:'/session'}),true));
function fixture(){
 const tabs=[{id:'tab',session:'s',device:'home-id',view:'workspace',url:'/session'}];
 const content={allTabs:tabs,handle:tab=>tab.id,active:()=>tabs[0],ready:true};
 const navigation=useContentNavigation(content),steps=[];
 navigation.update('s',{scene:'picker',key:'tab',source:'workspace',scope:{device:'other-id',root:'/elsewhere'},query:'needle',address:'https://example.invalid/kept',exit:'other-id'});
 navigation.scroll('s','document-position',{top:200,left:7});
 const run=Function('stopSearch','endEdit','navigation','session',code+';return returnWorkspace;')(()=>steps.push('stop-search'),()=>steps.push('end-edit'),navigation,'s');
 return {navigation,steps,run,tabs};
}
await test('actual navigation.update clears scope with null while retaining the tab and reading owner',()=>{const f=fixture();const before=f.navigation.get('s');f.run();const after=f.navigation.get('s');assert.equal(after.scope,null);assert.equal(after.source,'workspace');assert.equal(after.query,'');assert.equal(after.key,'tab');assert.equal(after.address,'https://example.invalid/kept');assert.equal(after.exit,'other-id');assert.deepEqual(after.scrolls,before.scrolls);assert.deepEqual(f.steps,['stop-search','end-edit']);assert.equal(f.tabs.length,1);});
await test('return follows the latest session cwd after browsing stayed pinned',()=>{const f=fixture(),row={id:'s',place:'/born',doing:{cwd:'/session-latest'}};const root=state=>state.scope?.root||sessionCwdOnScreen(row)||'';assert.equal(root(f.navigation.get('s')),'/elsewhere');f.run();assert.equal(root(f.navigation.get('s')),'/session-latest');});
console.log(JSON.stringify({legacy,passed:results.filter(v=>v.pass).length,total:results.length,results},null,2));assert(results.every(v=>v.pass),'return-to-workspace regressions');
