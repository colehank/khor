// Actual hooks with deterministic React scheduling/API boundaries; no node/PTy/browser.
import assert from 'node:assert/strict';
import {build} from 'esbuild';
import {fileURLToPath} from 'node:url';
const gui=fileURLToPath(new URL('..',import.meta.url));
const storage=new Map();globalThis.localStorage={getItem:k=>storage.get(k)??null,setItem:(k,v)=>storage.set(k,v)};globalThis.window={setInterval,clearInterval};
let rendering;
globalThis.__hooks={
 useState(initial){const h=rendering,i=h.index++;if(!(i in h.slots))h.slots[i]=typeof initial==='function'?initial():initial;return [h.slots[i],v=>{h.slots[i]=typeof v==='function'?v(h.slots[i]):v;h.schedule();}];},
 useRef(initial){const h=rendering,i=h.index++;return h.slots[i]??(h.slots[i]={current:initial});},
 useCallback(fn,deps){const h=rendering,i=h.index++,old=h.slots[i];if(!old||deps.some((d,j)=>d!==old.deps[j]))h.slots[i]={fn,deps};return h.slots[i].fn;},
 useEffect(fn,deps){const h=rendering,i=h.index++,old=h.slots[i];if(!old||!deps||deps.some((d,j)=>d!==old.deps?.[j]))h.effects.push(()=>{old?.cleanup?.();h.slots[i]={deps,cleanup:fn()};});},
};
function mount(run){const h={slots:[],effects:[],index:0,value:null,queued:false,closed:false,schedule(){if(h.queued||h.closed)return;h.queued=true;queueMicrotask(()=>{h.queued=false;h.render();});},render(){if(h.closed)return;h.index=0;h.effects=[];rendering=h;h.value=run();rendering=undefined;for(const fn of h.effects)fn();},close(){h.closed=true;for(const s of h.slots)s?.cleanup?.();}};h.render();return h;}
const bundle=await build({stdin:{contents:'export * from "./src/hooks/use-session-content.ts";export * from "./src/hooks/use-content-navigation.ts";',resolveDir:gui,loader:'ts'},bundle:true,write:false,format:'esm',platform:'node',plugins:[{name:'boundaries',setup(b){b.onResolve({filter:/^react$/},()=>({path:'react',namespace:'fixture'}));b.onResolve({filter:/^@\/api$/},()=>({path:'api',namespace:'fixture'}));b.onLoad({filter:/.*/,namespace:'fixture'},args=>({contents:args.path==='react'?'export const {useState,useEffect,useRef,useCallback}=globalThis.__hooks;':['fetchTabs','openTab','focusTab','closeTab','goTab','pinTab','reorderTabs'].map(n=>`export const ${n}=(...a)=>globalThis.__api.${n}(...a);`).join(''),loader:'js'}));}}]});
const {useSessionContent,useContentNavigation}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
const tick=()=>new Promise(r=>setImmediate(r));const until=async fn=>{for(let i=0;i<150;i++){if(await fn())return;await tick();}throw Error('Unsettled model');};
const deferred=()=>{let resolve;const promise=new Promise(r=>resolve=r);return {promise,resolve};};
let node={tabs:[],active:''},clock=0,next=0,gate,focusFailure=false;const calls=[],snapshot=()=>structuredClone(node);
globalThis.__api={
 async fetchTabs(){return snapshot();},
 async openTab(kind,device,url,session,after,view){calls.push(['open',session,url]);if(gate){const g=gate;gate=undefined;await g.promise;}const id='tab-'+(++next),t={id,device,url,session,view,title:'',pinned:false,touched:++clock};const at=after?node.tabs.findIndex(t=>t.id===after):-1;if(at>=0)node.tabs.splice(at+1,0,t);else node.tabs.push(t);node.active=id;return id;},
 async focusTab(kind,id){if(focusFailure){focusFailure=false;throw Error('Focus transport failed');}const t=node.tabs.find(t=>t.id===id);assert(t,'missing node id '+id);t.touched=++clock;node.active=id;return snapshot();},
 async closeTab(kind,id){calls.push(['close',id]);node.tabs=node.tabs.filter(t=>t.id!==id);if(node.active===id)node.active=node.tabs.at(-1)?.id||'';return snapshot();},
 async goTab(kind,id,device,url,title,view){const t=node.tabs.find(t=>t.id===id);assert(t);Object.assign(t,{device,url,title,...(view?{view}:{})});return snapshot();},
 async reorderTabs(kind,session,ids){calls.push(['reorder',session,ids]);const rank=new Map(ids.map((id,i)=>[id,i]));const rows=node.tabs.filter(t=>t.session===session&&rank.has(t.id)).sort((a,b)=>rank.get(a.id)-rank.get(b.id));let i=0;node.tabs=node.tabs.map(t=>t.session===session&&rank.has(t.id)?rows[i++]:t);return snapshot();},
 async pinTab(kind,id,pinned){node.tabs.find(t=>t.id===id).pinned=pinned;return snapshot();},
};
let model=mount(()=>useSessionContent(false));await until(()=>model.value.ready);
assert.equal(await model.value.ensure('empty','machine'),undefined);assert.throws(()=>model.value.open('A','machine','/repo','workspace'));assert.equal(node.tabs.length,0);
gate=deferred();const slow=gate,a=model.value.open('A','machine','/a.txt','file',27);assert.equal(model.value.open('A','machine','/a.txt','file').id,a.id);const b=model.value.open('A','machine','https://example.test','page');await tick();assert.equal(model.value.tabs.length,2);assert.equal(model.value.active('A').id,b.id);
slow.resolve();await until(()=>node.tabs.length===2&&model.value.tabs.every(t=>!t.id.startsWith('opening:')));assert.equal(model.value.handle(model.value.tabs[0]),a.id);assert.equal(model.value.handle(model.value.active('A')),b.id);assert.equal(calls.filter(c=>c[0]==='open'&&c[2]==='/a.txt').length,1);assert.equal(model.value.lines[a.id].line,27);assert(!('keep' in model.value)&&!('previews' in model.value)&&!('peek' in model.value));assert(model.value.tabs.every(t=>!('temporary' in t)&&!('preview' in t)));
const savedA=model.value.tabs.find(t=>t.url==='/a.txt');model.value.open('A','machine','/a.txt','file');await until(()=>model.value.handle(model.value.active('A'))===a.id);await tick();assert.equal(node.tabs.length,2);
const c=model.value.open('B','machine','/a.txt','file');await until(()=>node.tabs.length===3&&model.value.tabs.some(t=>t.session==='B'&&!t.id.startsWith('opening:')));assert.notEqual(c.id,a.id);
const beforeOrder=model.value.tabs.map(t=>t.id),front=model.value.handle(model.value.active('A'));
await model.value.reorder('A',[b.id,a.id]);assert.equal(model.value.tabs.filter(t=>t.session==='A')[0].url,'https://example.test');assert.equal(model.value.handle(model.value.active('A')),front);assert.equal(model.value.tabs.findIndex(t=>t.session==='B'),beforeOrder.indexOf(model.value.tabs.find(t=>t.session==='B').id));
await model.value.point(a.id,'machine' ,'/renamed.txt','file','Renamed');assert.equal(node.tabs.find(t=>t.id===savedA.id).url,'/renamed.txt');assert.equal(model.value.handle(model.value.tabs.find(t=>t.id===savedA.id)),a.id);
gate=deferred();const cg=gate,closing=model.value.open('A','machine','/close-during-write.txt','file'),closed=model.value.close(closing.id);cg.resolve();await closed;assert(!node.tabs.some(t=>t.url==='/close-during-write.txt'));assert(!model.value.tabs.some(t=>t.id===closing.id));
focusFailure=true;const retry=model.value.open('A','machine','/retry.txt','file');await until(()=>model.value.error?.includes('Focus transport failed'));model.value.open('A','machine','/retry.txt','file');await until(()=>!model.value.error&&model.value.tabs.some(t=>t.url==='/retry.txt'&&!t.id.startsWith('opening:')));assert.equal(calls.filter(c=>c[0]==='open'&&c[2]==='/retry.txt').length,1);assert.equal(model.value.handle(model.value.tabs.find(t=>t.url==='/retry.txt')),retry.id);
focusFailure=true;const failedClose=model.value.open('A','machine','/failed-focus-close.txt','file');await until(()=>model.value.error?.includes('Focus transport failed'));await model.value.close(failedClose.id);assert(!node.tabs.some(t=>t.url==='/failed-focus-close.txt'),'failed acknowledgement left an orphan tab');
// Browser observations canonicalize URLs; object identity must not depend on draft spelling.
const pageTab=model.value.tabs.find(t=>t.view==='page');await model.value.follow(pageTab.id,'https://example.test/');const beforeCanonical=node.tabs.length,canonical=model.value.open('A','machine','https://EXAMPLE.test:443','page');assert.equal(canonical.id,pageTab.id);await tick();assert.equal(node.tabs.length,beforeCanonical,'canonical page URL created duplicate');
for(const url of ['https://example.test/?q=one','https://example.test/#section','https://example.test/other']){model.value.open('A','machine',url,'page');await until(()=>node.tabs.some(t=>t.url===url));}assert.equal(node.tabs.length,beforeCanonical+3,'page query/hash/path lost identity');
model.value.open('A','machine','https://example.test','file');await until(()=>node.tabs.some(t=>t.view==='file'&&t.url==='https://example.test'));model.value.open('A','machine','https://example.test/','file');await until(()=>node.tabs.some(t=>t.view==='file'&&t.url==='https://example.test/'));assert.equal(node.tabs.length,beforeCanonical+5,'file path was URL normalized');
model.close();model=mount(()=>useSessionContent(false));await until(()=>model.value.ready);assert.equal(model.value.handle(model.value.tabs.find(t=>t.id===savedA.id)),a.id);
let nav=mount(()=>useContentNavigation(model.value));nav.value.resume('A');await tick();assert.equal(nav.value.get('A').scene,'document');nav.value.browse('A',{device:'machine',root:'/repo/inside'});nav.value.update('A',{query:'needle',address:'https://draft.test/path',source:'browser',exit:'exit-machine'});nav.value.scroll('A','picker browser',{top:418,left:12});nav.value.document(model.value.tabs.find(t=>t.id===savedA.id));assert(nav.value.back('A'));nav.value.resume('A');assert.equal(nav.value.get('A').scene,'picker');assert.equal(nav.value.get('A').query,'needle');nav.value.edit(a.id,{address:'https://unsent.test',editing:true,exitPrompt:9});nav.value.resume('B');const bPlace=nav.value.get('B');nav.value.overview('B');nav.value.update('B',{filter:'B only'});nav.value.resume('B');assert.equal(nav.value.get('B').scene,'document','pill must restore content, not the nested tab overview');assert.equal(nav.value.get('B').key,bPlace.key);
nav.close();nav=mount(()=>useContentNavigation(model.value));nav.value.resume('A');assert.equal(nav.value.get('A').scene,'picker');assert.equal(nav.value.get('A').source,'browser');assert.deepEqual(nav.value.get('A').scope,{device:'machine',root:'/repo/inside'});assert.equal(nav.value.get('A').address,'https://draft.test/path');assert.equal(nav.value.get('A').exit,'exit-machine');assert.deepEqual(nav.value.scrollOf('A','picker browser'),{top:418,left:12});assert.deepEqual(nav.value.editor(a.id),{address:'https://unsent.test'});assert.equal(nav.value.get('B').filter,'');
// Tab/session selectors never replace the pill's last actual content, even
// across reloads. The explicit back stack inside a selector still works.
for(const scene of ['sessions','overview']) {
  nav.value[scene]('A');nav.value.update('A',{filter:'transient selector'});
  nav.close();nav=mount(()=>useContentNavigation(model.value));nav.value.resume('A');
  assert.equal(nav.value.get('A').scene,'picker');assert.equal(nav.value.get('A').source,'browser');
  assert.equal(nav.value.get('A').query,'needle');assert.equal(nav.value.get('A').address,'https://draft.test/path');
  assert.deepEqual(nav.value.get('A').scope,{device:'machine',root:'/repo/inside'});
  assert.deepEqual(nav.value.scrollOf('A','picker browser'),{top:418,left:12});
}
nav.value.document(model.value.tabs.find(t=>t.id===savedA.id));
nav.value.sessions('A');nav.value.inspect('A');assert.equal(nav.value.get('A').scene,'overview');
assert(nav.value.back('A'));assert.equal(nav.value.get('A').scene,'sessions','nested overview Back still returns to session list');
nav.close();nav=mount(()=>useContentNavigation(model.value));nav.value.resume('A');
assert.equal(nav.value.get('A').scene,'document');assert.equal(nav.value.get('A').key,a.id,'file resumes after dismissing selectors and reloading');
assert(nav.value.back('A'));assert.equal(nav.value.get('A').scene,'picker','resumed file keeps its real directory/browser Back route');
nav.value.inspect('B');nav.value.resume('B');assert.equal(nav.value.get('B').key,bPlace.key,'other session keeps its own content checkpoint');
for(let i=0;i<70;i++)nav.value.sessions('A');nav.value.resume('A');assert.equal(nav.value.get('A').scene,'picker');
// Existing installations may contain an old selector with its trail discarded.
nav.close();
const legacy=JSON.parse(storage.get('khor.content.navigation.v1'));
delete legacy.A.lastContent;legacy.A.scene='sessions';legacy.A.trail=[];legacy.A.filter='old session search';
storage.set('khor.content.navigation.v1',JSON.stringify(legacy));
nav=mount(()=>useContentNavigation(model.value));nav.value.resume('A');
assert.equal(nav.value.get('A').scene,'picker','old saved session list migrates to source browsing');
assert.equal(nav.value.get('A').source,'browser');assert.equal(nav.value.get('A').query,'needle');assert.equal(nav.value.get('A').address,'https://draft.test/path');
nav.value.sessions('never-opened');nav.value.resume('never-opened');assert.equal(nav.value.get('never-opened').scene,'picker','first pill opens source browsing');
assert.equal(nav.value.closePicker('never-opened'),false,'closing an empty picker must return to the session');
assert.equal(nav.value.get('never-opened').scene,'closed','closing removes the synthetic browsing tab');
nav.close();nav=mount(()=>useContentNavigation(model.value));
assert.equal(nav.value.get('never-opened').scene,'closed','reload must not recreate a closed browsing tab');
nav.value.resume('never-opened');assert.equal(nav.value.get('never-opened').scene,'picker','pill can reopen browsing after closing');
nav.value.closePicker('never-opened');nav.value.browse('never-opened');
assert.equal(nav.value.back('never-opened'),false,'closed content is not a blank Back destination');
nav.value.document(model.value.tabs.find(t=>t.id===savedA.id));nav.value.browse('A');nav.value.overview('A');nav.value.browse('A');
assert.equal(nav.value.closePicker('A'),true,'closing browsing with an existing file keeps content open');
assert.equal(nav.value.get('A').scene,'document');assert.equal(nav.value.get('A').key,a.id,'closing browsing selects the last valid file, not a list');
nav.value.browse('A');

nav.value.sessions('A');nav.value.update('A',{filter:'session search'});nav.value.resume('A');assert.equal(nav.value.get('A').scene,'picker','pill must restore workspace/browser, not the session list');assert.equal(nav.value.get('A').query,'needle');assert.equal(nav.value.get('A').address,'https://draft.test/path');assert.equal(nav.value.get('A').filter,'');nav.value.browse('A');nav.value.document(model.value.tabs.find(t=>t.id===savedA.id));nav.value.sessions('A');await model.value.close(savedA.id);nav.render();nav.value.resume('A');assert.equal(nav.value.get('A').scene,'picker');nav.close();model.close();
console.log('PASS: pill resumes real content after session/nested lists, reload, legacy migration and deleted files; immediate durable tabs; deduplicated/session-scoped opens; slow write focus and stable handles; edit/close/retry races; reload handles; last scene/source/scope/query/address draft/scroll restoration');
