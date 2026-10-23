// Real App + private node/PTYs. Transport is the existing dev bridge;
// native page overlay/borrow rendering requires desktop app acceptance.
import {openTabs} from "./deck-gestures.mjs";
import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {mkdirSync,writeFileSync,readFileSync,copyFileSync,rmSync} from 'node:fs';
import {join,resolve} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {chromium,webkit} from 'playwright-core';
import {build} from 'esbuild';
import {directoryOf} from '../src/lib/paths.ts';
import {fakeHome,fakeUsage} from './fake-home.mjs';
const gui=fileURLToPath(new URL('..',import.meta.url));
const repo=resolve(gui,'../..');
const scratch=process.env.CONTENT_CHECK_DIR||join(tmpdir(),'khor-session-content-check');
const target=process.env.CARGO_TARGET_DIR||join(repo,'target');
const port=Number(process.env.CONTENT_VITE_PORT||1476), bridgePort=Number(process.env.CONTENT_BRIDGE_PORT||1477);
const word=key=>readFileSync(join(repo,'crates/catalog/zh.toml'),'utf8').match(new RegExp('^'+key+' = "([^"]*)"','m'))?.[1];
// Exercise the real shared resource ledger with only its native calls replaced.
const calls=[];globalThis.__contentHeldCalls=calls;
const bundled=await build({stdin:{contents:'export * from "./src/views/held.ts";',resolveDir:gui,loader:'ts'},bundle:true,write:false,format:'esm',platform:'node',plugins:[{name:'native-boundary',setup(b){b.onResolve({filter:/^@\/api$/},()=>({path:'native-boundary',namespace:'fixture'}));b.onLoad({filter:/.*/,namespace:'fixture'},()=>({contents:'export const closePage=id=>{globalThis.__contentHeldCalls.push(["page",id]);return Promise.resolve()};export const dropWebBorrow=id=>{globalThis.__contentHeldCalls.push(["borrow",id]);return Promise.resolve()};',loader:'js'}));}}]});
const held=await import('data:text/javascript;base64,'+Buffer.from(bundled.outputFiles[0].text).toString('base64'));
held.holdBorrow('shared','borrow-shared');held.holdPage('landing');held.holdPage('child');
held.releaseBorrows(['shared'],'page');held.releaseBorrows(['shared'],'content');held.releasePages(['landing'],'page');held.releasePages(['child'],'content');
held.releasePages([],'content');held.releaseBorrows([],'content');
assert.deepEqual(calls,[['page','child']],'closing content released a page or borrow owned by the browser landing');
held.releasePages([],'page');held.releaseBorrows([],'page');assert.deepEqual(calls,[['page','child'],['page','landing'],['borrow','borrow-shared']]);
calls.length=0;const capped=Array.from({length:10},(_,i)=>'cap-'+i);for(const id of capped)held.holdPage(id);held.releasePages(capped,'page');assert.deepEqual(calls,[['page','cap-0'],['page','cap-1']],'global native page cap changed');held.releasePages([],'page');
assert.equal(directoryOf('/notes.md'),'/');assert.equal(directoryOf("C:\\notes.md"),"C:\\");assert.equal(directoryOf('C:/notes.md'),'C:/');assert.equal(directoryOf('/repo/a.txt'),'/repo');
console.log('PASS: shared page/borrow ownership, native page cap, POSIX and Windows parent directories');
if(process.env.CONTENT_MODEL_ONLY==='1')process.exit(0);
const children=[],sessions=[];
let browser,usage,page;
const until=async(fn,ms=20000)=>{const end=Date.now()+ms;let last;do{try{last=await fn();if(last)return last;}catch(e){last=e;}await new Promise(r=>setTimeout(r,100));}while(Date.now()<end);throw Error('Timed out: '+last);};
const run=(cmd,args,env,cwd=gui)=>{const c=spawn(cmd,args,{env,cwd,detached:true,stdio:['ignore','pipe','pipe']});children.push(c);c.stderr.on('data',d=>process.stderr.write(d));return c;};
const api=async(cmd,args={})=>{const r=await fetch(`http://127.0.0.1:${bridgePort}/${cmd}`,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(args)});const value=await r.json();if(!r.ok)throw Error(JSON.stringify(value));return value;};
try{ checks:{
  rmSync(scratch,{recursive:true,force:true});mkdirSync(scratch,{recursive:true});
  usage=await fakeUsage();
  const env={...process.env,...fakeHome(join(scratch,'home'),usage.url),KHOR_HOME:join(scratch,'node'),KHOR_NAME:'content-check',KHOR_NO_MDNS:'1',BRIDGE_PORT:String(bridgePort)};delete env.KHOR_SESSION;
  for(const name of ['bridge','khor'])copyFileSync(join(target,'debug',name),join(scratch,name));
  run(join(scratch,'bridge'),[],env);
  await until(()=>api('devices').then(rows=>rows.length));
  const device=(await api('devices')).find(d=>d.me);
  const projects=['alpha','beta'].map(name=>join(scratch,'project-'+name));
  for(const [i,dir] of projects.entries()){
    mkdirSync(dir,{recursive:true});for(let j=0;j<24;j++)writeFileSync(join(dir,'extra-'+String(j).padStart(2,'0')+'.txt'),'Neighbor '+j+'\n'+(j===15?Array.from({length:80},(_,k)=>'Line '+k).join('\n'):''));writeFileSync(join(dir,'notes.md'),`# ${i===0?'Alpha':'Beta'}\n\nSession contents.\n`);writeFileSync(join(dir,'only-'+i+'.txt'),'This file belongs to '+i+'\n');
    const id=await api('open_on',{machine:'',title:'Content '+(i===0?'Alpha':'Beta'),cwd:dir,cols:100,rows:35});sessions.push(id);
  }
  run(join(gui,'node_modules/.bin/vite'),['--port',String(port),'--strictPort'],{...process.env,KHOR_FACE_GATE:'1'});
  await until(()=>fetch(`http://localhost:${port}/`).then(r=>r.ok));
  browser=process.env.CONTENT_ENGINE==='webkit'?await webkit.launch({headless:process.env.CONTENT_HEADED!=='1'}):await chromium.launch({channel:'chrome',headless:process.env.CONTENT_HEADED!=='1',args:['--no-proxy-server']});
  const context=await browser.newContext({viewport:{width:1600,height:1000},deviceScaleFactor:Number(process.env.CONTENT_DPR||1),reducedMotion:'reduce'});
  // The LAN HTTP face has no secure-context randomUUID API.
  await context.addInitScript(()=>Object.defineProperty(globalThis.crypto,'randomUUID',{configurable:true,value:undefined}));
  await context.addInitScript(ids=>{
    if(!localStorage.getItem('khor.sessions.tabs'))localStorage.setItem('khor.sessions.tabs',JSON.stringify({tabs:ids,active:ids[0]}));localStorage.setItem('khor.place',JSON.stringify({landing:'sessions',screen:'detail'}));localStorage.setItem('khor.lang','zh');
  },sessions);
  page=await context.newPage();page.setDefaultTimeout(12000);const errors=[];page.on('pageerror',e=>{errors.push(e.message);console.error('PAGEERROR',e.message);});
  const url=`http://localhost:${port}/?bridge=${bridgePort}`;
  await page.goto(url);
  assert(await page.evaluate(()=>typeof globalThis.crypto.randomUUID==='undefined'),'LAN compatibility probe did not run');
  const head=id=>page.locator(`[data-session-tabs] [data-tab-open="${id}"]`);
  const childrenOf=session=>page.locator(`[data-content-tabs="${session}"]`);
  const child=id=>page.locator(`[data-content-tab="${id}"]`);
  const picker=()=>page.locator('[data-content-picker]:visible');
  const shown=()=>page.locator('[data-session-content][data-content-visible=true]');
  const searchPicker=async()=>{if(!await picker().getByRole('searchbox').count())await picker().locator('[data-content-directory-search]').click();return picker().getByRole('searchbox');};
  const table=()=>api('tabs_list',{kind:'content'});
  const peers=async session=>(await table()).tabs.filter(t=>t.session===session);
  const add=()=>childrenOf(sessions[0]).locator('[data-tab-new]');
  const outerOrder=()=>page.locator('[data-session-tabs] [data-tab]').evaluateAll(els=>els.filter(e=>!e.closest('[data-content-tabs]')).map(e=>e.dataset.tab));
  const dragTab=async(from,to)=>{const a=await from.boundingBox(),b=await to.boundingBox();assert(a&&b);await page.mouse.move(a.x+a.width*.4,a.y+a.height/2);await page.mouse.down();await page.mouse.move(b.x+b.width*(b.x>a.x?.8:.2),b.y+b.height/2,{steps:18});await page.mouse.up();};
  await head(sessions[0]).waitFor();
  await dragTab(head(sessions[1]),head(sessions[0]));
  await until(async()=>JSON.stringify(await outerOrder())===JSON.stringify([...sessions].reverse()));
  await page.reload();await head(sessions[0]).waitFor();assert.deepEqual(await outerOrder(),[...sessions].reverse());
  // First expansion creates one real starting tab; repeated + creates distinct rows.
  await head(sessions[0]).click();await picker().waitFor();await until(async()=>(await peers(sessions[0])).length===1);
  assert.equal((await peers(sessions[0]))[0].view,'workspace');
  assert.equal(await page.locator('[data-session-tabs] > [data-tab-capsule] > [data-tab-new-seat]').count(),0);
  const single=await peers(sessions[0]);await head(sessions[0]).click();await picker().waitFor({state:'hidden'});
  assert.deepEqual((await peers(sessions[0])).map(t=>t.id),single.map(t=>t.id),'collapse removed the starting tab');
  await head(sessions[0]).click();await picker().waitFor();
  await add().click();await add().click();await until(async()=>(await peers(sessions[0])).length===3);
  const blanks=await peers(sessions[0]);assert.equal(new Set(blanks.map(t=>t.id)).size,3);
  const activeBlank=blanks[2].id;
  assert.equal(await childrenOf(sessions[0]).locator('[data-side-mark=new-tab]').count(),3,'starting tabs need plus avatars');
  await picker().getByText('notes.md',{exact:true}).click();await shown().locator('.cm-content').waitFor();
  await until(async()=>(await peers(sessions[0])).find(t=>t.id===activeBlank)?.view==='file');
  assert.equal((await peers(sessions[0])).length,3,'opening a file appended another tab');
  const firstHandle=await shown().getAttribute('data-content-handle');
  // A different content tab must have an independent starting page and search.
  await child(blanks[0].id).getByRole('tab').click();await (await searchPicker()).fill('notes');
  await child(blanks[1].id).getByRole('tab').click();assert.equal(await (await searchPicker()).inputValue(),'');
  await (await searchPicker()).fill('guide');
  await child(blanks[0].id).getByRole('tab').click();assert.equal(await (await searchPicker()).inputValue(),'notes');
  await page.reload();await head(sessions[0]).waitFor();await head(sessions[0]).click();await picker().waitFor();
  assert.equal(await (await searchPicker()).inputValue(),'notes','reload lost per-tab search');
  await child(blanks[1].id).getByRole('tab').click();assert.equal(await (await searchPicker()).inputValue(),'guide');
  await (await searchPicker()).fill('');await picker().getByText('extra-03.txt',{exact:true}).click();await shown().locator('.cm-content').waitFor();
  const secondId=blanks[1].id;
  // Shared tab geometry: whole-pill hover, centred close, and shared mouse reorder.
  const centre=await childrenOf(sessions[0]).locator('[data-tab-close]').evaluateAll(els=>els.map(e=>{const r=e.getBoundingClientRect(),p=e.parentElement.getBoundingClientRect();return Math.abs(r.top+r.height/2-p.top-p.height/2);}));
  assert(centre.every(n=>n<.1),'nested close buttons are off-centre: '+centre);
  const chosenBefore=await child(secondId).getByRole('tab').getAttribute('aria-selected');assert.equal(chosenBefore,'true');
  await dragTab(child(activeBlank).getByRole('tab'),child(blanks[0].id).getByRole('tab'));
  await until(async()=>(await peers(sessions[0]))[0].id===activeBlank);
  assert.equal(await child(secondId).getByRole('tab').getAttribute('aria-selected'),'true','reorder changed the active document');
  await child(activeBlank).getByRole('tab').click();assert.equal(await shown().getAttribute('data-content-handle'),firstHandle);
  // Browsing from an opened file preserves that file and opens a separate website tab.
  await shown().locator('[data-file-back]').click();await picker().waitFor();
  await picker().locator('[data-content-scope]:visible').click();await page.locator('[data-content-source=browser]').click();
  const address=()=>picker().locator('[data-page-address]');
  await address().focus();await address().fill('http://localhost:'+port);await address().press('Enter');
  await shown().locator('[data-content-page-bar]').waitFor();
  await until(async()=>(await peers(sessions[0])).some(t=>t.view==='page'));
  assert.equal((await peers(sessions[0])).length,4);assert.equal((await peers(sessions[0])).find(t=>t.id===activeBlank)?.view,'file');
  const pageHandle=await shown().getAttribute('data-content-handle');assert.notEqual(pageHandle,firstHandle);
  await page.screenshot({path:join(scratch,'desktop.png')});
  // Native view and borrow lifetime is checked above; the HTTP renderer is not
  // evidence for a UIKit WebView. Here test model/navigation/host boundaries.
  await head(sessions[0]).click();await until(async()=>await shown().count()===0);
  await head(sessions[0]).click();await shown().locator('[data-content-page-bar]').waitFor();
  assert.equal(await shown().getAttribute('data-content-handle'),pageHandle,'collapse replaced the page handle');
  // Closing a pinned starting tab is refused, and unpin restores the shared ×.
  await add().click();await picker().waitFor();await until(async()=>(await peers(sessions[0])).length===5);
  const pin=(await peers(sessions[0])).at(-1);await api('tabs_pin',{kind:'content',id:pin.id,on:true});
  await until(async()=>await child(pin.id).locator('[data-tab-close]').count()===0);
  await api('tabs_pin',{kind:'content',id:pin.id,on:false});await child(pin.id).locator('[data-tab-close]').waitFor();
  // Wide→narrow uses the same persisted tabs and per-tab locations.
  await page.setViewportSize({width:390,height:800});await picker().waitFor();
  await openTabs(page);
  const overview=()=>page.locator('[data-content-overview]:not(.session-content-retained)');
  await overview().waitFor();assert.equal(await overview().locator('[data-content-overview-tab]').count(),5);
  await overview().locator('[data-tab-sheet-new]').click();await picker().waitFor();
  await until(async()=>(await peers(sessions[0])).length===6);
  await openTabs(page);await overview().waitFor();
  await overview().locator('[data-content-overview-search]').click();await overview().getByRole('searchbox').fill('extra-03');
  assert.equal(await overview().locator('[data-content-overview-tab]').count(),1);
  await overview().getByRole('searchbox').fill('');await overview().locator('[data-content-edit-end]').click();
  await page.screenshot({path:join(scratch,'mobile.png')});
  while((await peers(sessions[0])).length){
    const before=(await peers(sessions[0])).length;
    await overview().locator('[data-tab-sheet-close]').first().click();await until(async()=>(await peers(sessions[0])).length===before-1);
  }
  await until(async()=>await page.locator('[data-content-panel]').count()===0);
  assert.equal(await page.locator('[data-session-screen]').count(),1,'last content close removed the root session');
  assert.deepEqual(errors,[]);
  console.log('session content gate: independent durable starting tabs; original handles on file/page point; per-tab query/reload; shared close geometry, pin and mouse reorder; wide/narrow overview and close-last passed');
}}catch(e){await page?.screenshot({path:join(scratch,'failed.png')}).catch(()=>{});console.error(e);throw e;}
finally{
  for(const id of sessions)await api('close_session',{id}).catch(()=>{});
  await browser?.close().catch(()=>{});
  for(const child of children)try{process.kill(-child.pid,'SIGKILL');}catch{}
  // Keep screenshots and fixture state, not a full binary pair per run.
  for(const name of ['bridge','khor'])rmSync(join(scratch,name),{force:true});
  await usage?.close();
}
