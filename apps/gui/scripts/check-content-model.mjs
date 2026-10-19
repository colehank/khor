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
const bundle=await build({stdin:{contents:'export * from "./src/hooks/use-session-content.ts";export * from "./src/hooks/use-content-navigation.ts";',resolveDir:gui,loader:'ts'},bundle:true,write:false,format:'esm',platform:'node',plugins:[{name:'boundaries',setup(b){b.onResolve({filter:/^react$/},()=>({path:'react',namespace:'fixture'}));b.onResolve({filter:/^@\/api$/},()=>({path:'api',namespace:'fixture'}));b.onLoad({filter:/.*/,namespace:'fixture'},args=>({contents:args.path==='react'?'export const {useState,useEffect,useRef,useCallback}=globalThis.__hooks;':['fetchTabs','openTab','focusTab','closeTab','goTab','pinTab','reorderTabs','fetchPageIcons'].map(n=>`export const ${n}=(...a)=>globalThis.__api.${n}(...a);`).join(''),loader:'js'}));}}]});
const {useSessionContent,useContentNavigation}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
const tick=()=>new Promise(r=>setImmediate(r));const until=async fn=>{for(let i=0;i<150;i++){if(await fn())return;await tick();}throw Error('Unsettled model');};
const deferred=()=>{let resolve;const promise=new Promise(r=>resolve=r);return {promise,resolve};};
let node={tabs:[],active:''},clock=0,next=0,gate,focusFailure=false,openAckFailure=false;const calls=[],snapshot=()=>structuredClone(node);
globalThis.__api={
 async fetchTabs(){return snapshot();},
 async fetchPageIcons(){return {'https://kept.test':'data:image/png;base64,kept','https://disk.test':'data:image/png;base64,disk'};},
 async openTab(kind,device,url,session,after,view,request){const existing=request&&node.tabs.find(t=>t.request===request&&t.session===session);if(existing){node.active=existing.id;return existing.id;}calls.push(['open',session,url]);if(gate){const g=gate;gate=undefined;await g.promise;}const id='tab-'+(++next),t={id,device,url,session,view,request,title:'',pinned:false,touched:++clock};const at=after?node.tabs.findIndex(t=>t.id===after):-1;if(at>=0)node.tabs.splice(at+1,0,t);else node.tabs.push(t);node.active=id;if(openAckFailure){openAckFailure=false;throw Error('Open acknowledgement lost');}return id;},
 async focusTab(kind,id){if(focusFailure){focusFailure=false;throw Error('Focus transport failed');}const t=node.tabs.find(t=>t.id===id);assert(t,'missing node id '+id);t.touched=++clock;node.active=id;return snapshot();},
 async closeTab(kind,id){calls.push(['close',id]);node.tabs=node.tabs.filter(t=>t.id!==id);if(node.active===id)node.active=node.tabs.at(-1)?.id||'';return snapshot();},
 async goTab(kind,id,device,url,title,view){const t=node.tabs.find(t=>t.id===id);assert(t);Object.assign(t,{device,url,title,...(view?{view}:{})});return snapshot();},
 async reorderTabs(kind,session,ids){calls.push(['reorder',session,ids]);const rank=new Map(ids.map((id,i)=>[id,i]));const rows=node.tabs.filter(t=>t.session===session&&rank.has(t.id)).sort((a,b)=>rank.get(a.id)-rank.get(b.id));let i=0;node.tabs=node.tabs.map(t=>t.session===session&&rank.has(t.id)?rows[i++]:t);return snapshot();},
 async pinTab(kind,id,pinned){node.tabs.find(t=>t.id===id).pinned=pinned;return snapshot();},
};
let model=mount(()=>useSessionContent(false));
model.value.recordPageIcon?.('data:image/png;base64,newer','https://kept.test/live');
await until(()=>model.value.ready);
assert.equal(typeof model.value.pageIcon,'function','session content must read the same persisted favicon cache as the main browser');
assert.equal(model.value.pageIcon('https://kept.test/path?q=1'),'data:image/png;base64,newer','late disk cache cannot overwrite a live icon');
model.value.recordPageIcon('data:image/png;base64,kept','https://kept.test/path');await tick();
assert.equal(model.value.pageIcon('https://disk.test/path'),'data:image/png;base64,disk','cached icons survive reload without asking the site');
model.value.recordPageIcon('data:image/png;base64,live','https://redirect.test/final');await tick();
assert.equal(model.value.pageIcon('https://redirect.test/other'),'data:image/png;base64,live');
assert.equal(model.value.pageIcon('https://other.test'),undefined);
assert.equal(model.value.pageIcon('/not-a-site'),undefined);
assert.equal(typeof model.value.newTab,'function','each + must create an independent persisted navigation tab');
assert.equal(await model.value.ensure('empty','machine'),undefined);assert.equal(node.tabs.length,0);
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
// Opening from a source picker consumes only a blank tab. Existing documents
// survive going back to browse, including clicks before a React render/ack.
assert.equal(typeof model.value.openFromPicker,'function','source selection needs a durable open policy');
const choose=(key,path,view='file',device='machine')=>model.value.openFromPicker('BrowseFiles',key,device,path,view);
const sourceBlank=model.value.newTab('BrowseFiles','machine');
const docA=choose(sourceBlank.id,'/viewed-a.md');
assert.equal(model.value.handle(docA),sourceBlank.id,'first document fills the blank in place');
const docB=choose(sourceBlank.id,'/viewed-b.md');
assert.notEqual(model.value.handle(docB),sourceBlank.id,'rapid second choice must not overwrite the first document');
await until(()=>model.value.tabs.filter(t=>t.session==='BrowseFiles').length===2&&model.value.tabs.filter(t=>t.session==='BrowseFiles').every(t=>!t.id.startsWith('opening:')));
assert.deepEqual(node.tabs.filter(t=>t.session==='BrowseFiles').map(t=>t.url),['/viewed-a.md','/viewed-b.md']);
const revisit=choose(docB.id,'/viewed-a.md');assert.equal(model.value.handle(revisit),sourceBlank.id,'reopening an existing document focuses its tab');
const docPage=choose(docB.id,'https://selected.test','page');
await until(()=>node.tabs.some(t=>t.session==='BrowseFiles'&&t.url==='https://selected.test'));
const anotherPage=choose(docPage.id,'https://other-selected.test','page');
await until(()=>node.tabs.some(t=>t.session==='BrowseFiles'&&t.url==='https://other-selected.test'));
assert.equal(node.tabs.filter(t=>t.session==='BrowseFiles').length,4,'choosing another page preserves previously viewed pages and files');
await model.value.follow(anotherPage.id,'https://other-selected.test/inside');
assert.equal(node.tabs.filter(t=>t.session==='BrowseFiles').length,4,'navigation inside a page does not create a new tab');
model.close();model=mount(()=>useSessionContent(false));await until(()=>model.value.ready);
assert.equal(model.value.tabs.filter(t=>t.session==='BrowseFiles').length,4,'viewed documents survive reload');
let browseNav=mount(()=>useContentNavigation(model.value));
const firstDoc=model.value.tabs.find(t=>t.session==='BrowseFiles'&&t.url==='/viewed-a.md');
browseNav.value.document(firstDoc);browseNav.value.browse('BrowseFiles',{device:'machine',root:'/repo'});
browseNav.value.update('BrowseFiles',{query:'kept search'});
browseNav.value.select(firstDoc);
assert.equal(browseNav.value.get('BrowseFiles').scene,'document','selecting a file tab opens the document, not its saved source picker');
assert(browseNav.value.back('BrowseFiles'));assert.equal(browseNav.value.get('BrowseFiles').scene,'picker');
assert.equal(browseNav.value.get('BrowseFiles').query,'kept search','Back still restores the source search');
const pickerBefore=browseNav.value.get('BrowseFiles');
const pickerScrollKey=key=>JSON.stringify(['picker',key,pickerBefore.source,pickerBefore.scope,pickerBefore.query])+' [data-side-workspace] > div 0';
browseNav.value.scroll('BrowseFiles',pickerScrollKey(pickerBefore.key),{top:327,left:0});
const pickedAfterBack=model.value.openFromPicker('BrowseFiles',browseNav.value.get('BrowseFiles').key,'machine','/after-back.md','file');
browseNav.value.document(pickedAfterBack);
assert(browseNav.value.back('BrowseFiles'),'new document keeps the source picker as its Back destination');
assert.equal(browseNav.value.get('BrowseFiles').scene,'picker');
assert.equal(browseNav.value.get('BrowseFiles').query,'kept search');
assert.deepEqual(browseNav.value.get('BrowseFiles').scope,{device:'machine',root:'/repo'});
assert.deepEqual(browseNav.value.scrollOf('BrowseFiles',pickerScrollKey(model.value.handle(pickedAfterBack))),{top:327,left:0},'Back from a new document retains the source list scroll position');
await until(()=>node.tabs.some(t=>t.url==='/after-back.md'));
assert(node.tabs.some(t=>t.url==='/viewed-a.md'),'opening after Back preserves the first viewed file');
browseNav.close();
// Independent starting tabs are durable and turn into content in place.
const blanks=Array.from({length:3},()=>model.value.newTab('Blank','machine'));
assert.equal(new Set(blanks.map(t=>t.id)).size,3);
await until(()=>model.value.tabs.filter(t=>t.session==='Blank').length===3&&model.value.tabs.filter(t=>t.session==='Blank').every(t=>!t.id.startsWith('opening:')));
const firstBlank=model.value.tabs.find(t=>model.value.handle(t)===blanks[0].id),savedBlank=firstBlank.id;
const pointing=model.value.point(blanks[0].id,'machine','/blank-notes.md','file');await tick();
assert.equal(model.value.tabs.find(t=>model.value.handle(t)===blanks[0].id).url,'/blank-notes.md','new tab must show its target immediately');
await pointing;assert.equal(node.tabs.find(t=>t.id===savedBlank).view,'file');
assert.equal(model.value.tabs.filter(t=>t.session==='Blank').length,3,'pointing a starting tab allocated another tab');
focusFailure=true;const blankRetry=model.value.newTab('RetryBlank');await until(()=>model.value.error?.includes('Focus transport failed'));
await model.value.focus(blankRetry.id);assert.equal(node.tabs.filter(t=>t.session==='RetryBlank').length,1,'retry duplicated an empty address');
await model.value.close(blankRetry.id);
openAckFailure=true;const lostBlank=model.value.newTab('LostBlank');await until(()=>model.value.error?.includes('Open acknowledgement lost'));await model.value.focus(lostBlank.id);assert.equal(node.tabs.filter(t=>t.session==='LostBlank').length,1,'a lost open acknowledgement created a second blank tab');await model.value.close(lostBlank.id);
openAckFailure=true;const lostThenNext=model.value.newTab('LostThenNext');await until(()=>model.value.error?.includes('Open acknowledgement lost'));const nextAfterLost=model.value.newTab('LostThenNext');await until(()=>model.value.tabs.filter(t=>t.session==='LostThenNext').length===2&&model.value.tabs.filter(t=>t.session==='LostThenNext').every(t=>!t.id.startsWith('opening:')));assert.equal(model.value.handle(model.value.active('LostThenNext')),nextAfterLost.id,'reconciling an older opening stole focus');await model.value.close(lostThenNext.id);await model.value.close(nextAfterLost.id);
model.close();model=mount(()=>useSessionContent(false));await until(()=>model.value.ready);assert.equal(model.value.handle(model.value.tabs.find(t=>t.id===savedA.id)),a.id);
let nav=mount(()=>useContentNavigation(model.value));nav.value.resume('A');await tick();assert.equal(nav.value.get('A').scene,'document');nav.value.document(model.value.tabs.find(t=>t.id===savedA.id));nav.value.browse('A',{device:'machine',root:'/repo/inside'});nav.value.update('A',{query:'needle',address:'https://draft.test/path',source:'browser',exit:'exit-machine'});nav.value.scroll('A','picker browser',{top:418,left:12});nav.value.document(model.value.tabs.find(t=>t.id===savedA.id));assert(nav.value.back('A'));nav.value.resume('A');assert.equal(nav.value.get('A').scene,'picker');assert.equal(nav.value.get('A').query,'needle');nav.value.edit(a.id,{address:'https://unsent.test',editing:true,exitPrompt:9});nav.value.resume('B');const bPlace=nav.value.get('B');nav.value.overview('B');nav.value.update('B',{filter:'B only'});nav.value.resume('B');assert.equal(nav.value.get('B').scene,'document','pill must restore content, not the nested tab overview');assert.equal(nav.value.get('B').key,bPlace.key);
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
// Closing a picker now closes that real tab, then selects the neighbour.
nav.value.add('CloseOnly');await until(()=>model.value.tabs.some(t=>t.session==='CloseOnly'&&!t.id.startsWith('opening:')));
assert.equal(nav.value.closePicker('CloseOnly'),false);assert.equal(nav.value.get('CloseOnly').scene,'closed');
await until(()=>!node.tabs.some(t=>t.session==='CloseOnly'));
// Every content tab owns its own picker, draft and back route.
const aBlank=model.value.tabs.find(t=>model.value.handle(t)===blanks[0].id),bBlank=model.value.tabs.find(t=>model.value.handle(t)===blanks[1].id);
nav.value.select(aBlank);nav.value.browse('Blank',{device:'machine',root:'/first'});nav.value.update('Blank',{query:'first query',address:'https://first.test',source:'workspace'});
nav.value.select(bBlank);nav.value.update('Blank',{scope:{device:'machine',root:'/second'},query:'second query',address:'https://second.test',source:'browser'});
const scopedA=nav.value.forTab('Blank',blanks[0].id);scopedA.update('Blank',{query:'late first query'});
assert.equal(nav.value.get('Blank').query,'second query','inactive picker stole active input');
nav.value.select(aBlank);assert.equal(nav.value.get('Blank').query,'late first query');assert.equal(nav.value.get('Blank').scope.root,'/first');
nav.value.document(aBlank);assert(nav.value.back('Blank'));assert.equal(nav.value.get('Blank').scene,'picker');
nav.close();nav=mount(()=>useContentNavigation(model.value));nav.value.resume('Blank');assert.equal(nav.value.get('Blank').key,blanks[0].id);assert.equal(nav.value.get('Blank').query,'late first query');
nav.value.select(bBlank);assert.equal(nav.value.get('Blank').source,'browser');assert.equal(nav.value.get('Blank').address,'https://second.test');assert.equal(nav.value.get('Blank').scope.root,'/second');
nav.close();model.close();
console.log('PASS: durable picker opens preserve viewed files/pages and source Back/scroll; independent starting tabs; original identity on point; retry without duplicate blank rows; per-tab picker/draft/back/reload isolation; retained favicon, content identity, ordering, transport race and legacy migration coverage');
