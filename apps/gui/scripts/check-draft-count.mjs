// Executes the actual reporting hook and each owner's production count expression.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {build} from 'esbuild';
const legacy=process.argv.includes('--legacy');
const bundle=await build({stdin:{contents:(legacy?'export const useDraftCount=()=>{};':"export {useDraftCount} from './src/hooks/use-draft-count';")+"export {draftCount,watchDrafts,reportDrafts} from './src/lib/drafts';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false,plugins:[{name:'draft-hook-alias',setup(b){b.onResolve({filter:/^@\/lib\/drafts$/},()=>({path:process.cwd()+'/src/lib/drafts.ts'}));}},{name:'layout-fixture',setup(b){b.onResolve({filter:/^react$/},()=>({path:'react',namespace:'fixture'}));b.onLoad({filter:/.*/,namespace:'fixture'},()=>({contents:'export const useRef=current=>globalThis.ownerRef(current);export const useLayoutEffect=(effect,deps)=>globalThis.layoutEffect(effect,deps);'}));}}]});
const {useDraftCount,draftCount,watchDrafts,reportDrafts}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
const sources={session:'src/views/SessionContent.tsx',machine:'src/views/MachineFiles.tsx',side:'src/views/SideWorkspace.tsx',local:'src/views/FileView.tsx'};
const code=Object.fromEntries(Object.entries(sources).map(([name,path])=>[name,legacy?execFileSync('git',['show','e7c335c8:apps/gui/'+path],{encoding:'utf8'}):readFileSync(path,'utf8')]));
function mount(kind,initial){
 let ref,cleanup,values=initial;
 const expression=code[kind].match(/  ?useDraftCount\(([^;]+)\);/)?.[1];
 const render=()=>{
  globalThis.ownerRef=current=>ref??=( {current} );
  globalThis.layoutEffect=(effect,deps)=>{if(deps.length===0){if(!cleanup)cleanup=effect();}else effect();};
  if(expression)Function(...Object.keys(values),'useDraftCount','useDraftCount('+expression+');')(...Object.values(values),useDraftCount);
 };
 render();return {replay(){cleanup?.();cleanup=null;render();},update(next){values=next;render();},unmount(){cleanup?.();cleanup=null;}};
}
const results=[];
async function test(name,run){const owners=[];const make=(...args)=>{const owner=mount(...args);owners.push(owner);return owner;};try{await run(make);results.push({name,pass:true});console.log('PASS',name);}catch(e){results.push({name,pass:false,error:e.message});console.error('FAIL',name,e.message);}finally{for(const owner of owners)owner.unmount();reportDrafts('fixture-workspace',0);assert.equal(draftCount(),0,'all test owners are released');}}
await test('session drafts count dirty entries, not undefined slots',make=>{const owner=make('session',{drafts:{first:{text:'draft'},saved:undefined}});assert.equal(draftCount(),1);owner.update({drafts:{first:undefined,saved:undefined}});assert.equal(draftCount(),0);});
for(const kind of ['machine','side'])await test(kind+' file owner reports dirty, saved/discarded, and unmount',make=>{const drafts=new Map();const owner=make(kind,{drafts});assert.equal(draftCount(),0);owner.update({drafts:new Map([['note',{text:'draft'}]])});assert.equal(draftCount(),1);owner.update({drafts:new Map()});assert.equal(draftCount(),0);owner.update({drafts:new Map([['other',{text:'draft'}]])});owner.unmount();assert.equal(draftCount(),0);});
await test('hidden retained owner continues protecting its drafts',make=>{const values={drafts:{note:{text:'draft'}}};const owner=make('session',values);owner.update(values);assert.equal(draftCount(),1);});
await test('simultaneous instances never clear another reporter',make=>{const values={drafts:new Map([['same-path',{text:'draft'}]])};const one=make('machine',values);make('machine',values);assert.equal(draftCount(),2);one.unmount();assert.equal(draftCount(),1);});
await test('standalone FileBody reports local dirty but delegated drafts are counted upstream',make=>{const owner=make('local',{onDraft:undefined,dirty:true});assert.equal(draftCount(),1);owner.update({onDraft:()=>{},dirty:true});assert.equal(draftCount(),0);owner.update({onDraft:undefined,dirty:false});assert.equal(draftCount(),0);});
await test('existing workspace reports remain independent',make=>{reportDrafts('fixture-workspace',2);const owner=make('side',{drafts:new Map([['note',{text:'draft'}]])});assert.equal(draftCount(),3);owner.unmount();assert.equal(draftCount(),2);reportDrafts('fixture-workspace',0);});
await test('changing a retained dirty count never publishes an intermediate zero',make=>{const owner=make('machine',{drafts:new Map([['one',{}],['two',{}]])});const seen=[];const stop=watchDrafts(n=>seen.push(n));owner.update({drafts:new Map([['two',{}]])});stop();assert.deepEqual(seen,[1]);});
await test('StrictMode lifetime replay restores registration',make=>{const owner=make('session',{drafts:{note:{text:'draft'}}});owner.replay();assert.equal(draftCount(),1);owner.unmount();assert.equal(draftCount(),0);});
console.log(JSON.stringify({legacy,passed:results.filter(t=>t.pass).length,total:results.length,results},null,2));assert(results.every(t=>t.pass),'draft reporting regressions');
