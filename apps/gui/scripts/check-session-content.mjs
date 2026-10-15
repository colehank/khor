// Real App + private node/PTYs. Transport is the existing dev bridge;
// native page overlay/borrow rendering requires desktop app acceptance.
import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {mkdirSync,writeFileSync,readFileSync,copyFileSync,rmSync} from 'node:fs';
import {join,resolve} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright-core';
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
try{
  rmSync(scratch,{recursive:true,force:true});mkdirSync(scratch,{recursive:true});
  usage=await fakeUsage();
  const env={...process.env,...fakeHome(join(scratch,'home'),usage.url),KHOR_HOME:join(scratch,'node'),KHOR_NAME:'content-check',KHOR_NO_MDNS:'1',BRIDGE_PORT:String(bridgePort)};delete env.KHOR_SESSION;
  for(const name of ['bridge','khor'])copyFileSync(join(target,'debug',name),join(scratch,name));
  run(join(scratch,'bridge'),[],env);
  await until(()=>api('devices').then(rows=>rows.length));
  const device=(await api('devices')).find(d=>d.me);
  const projects=['alpha','beta'].map(name=>join(scratch,'project-'+name));
  for(const [i,dir] of projects.entries()){
    mkdirSync(dir,{recursive:true});writeFileSync(join(dir,'notes.md'),`# ${i===0?'Alpha':'Beta'}\n\nSession contents.\n`);writeFileSync(join(dir,'only-'+i+'.txt'),'This file belongs to '+i+'\n');
    const id=await api('open_on',{machine:'',title:'Content '+(i===0?'Alpha':'Beta'),cwd:dir,cols:100,rows:35});sessions.push(id);
  }
  run(join(gui,'node_modules/.bin/vite'),['--port',String(port),'--strictPort'],{...process.env,KHOR_FACE_GATE:'1'});
  await until(()=>fetch(`http://localhost:${port}/`).then(r=>r.ok));
  browser=await chromium.launch({channel:'chrome',headless:true,args:['--no-proxy-server']});
  const context=await browser.newContext({viewport:{width:1600,height:1000},reducedMotion:'reduce'});
  await context.addInitScript(ids=>{
    localStorage.setItem('khor.sessions.tabs',JSON.stringify({tabs:ids,active:ids[0]}));localStorage.setItem('khor.place',JSON.stringify({landing:'sessions',screen:'detail'}));localStorage.setItem('khor.lang','zh');
  },sessions);
  page=await context.newPage();page.setDefaultTimeout(12000);const errors=[];page.on('pageerror',e=>{errors.push(e.message);console.error('PAGEERROR',e.message);});
  const url=`http://localhost:${port}/?bridge=${bridgePort}`;
  await page.goto(url);
  const head=id=>page.locator(`[data-tab-open="${id}"]`);
  const side=id=>page.locator(`[data-session-side="${id}"]`);
  const shown=()=>page.locator('[data-session-content][data-content-visible=true]');
  const table=()=>api('tabs_list',{kind:'content'});
  await head(sessions[0]).waitFor();
  assert.equal(await page.locator('[data-session-tabs] [data-tab-new]').count(),1,'outer + missing');
  await head(sessions[0]).click();
  await side(sessions[0]).waitFor();
  await shown().locator('[data-file-tree]').waitFor();
  assert(await shown().getByText('only-0.txt',{exact:true}).isVisible());
  let first=(await table()).tabs.find(t=>t.session===sessions[0]);assert(first);
  await shown().getByText('notes.md',{exact:true}).click();
  await shown().locator('.cm-content').waitFor();
  assert.equal((await table()).tabs.find(t=>t.id===first.id).view,'file');
  assert.equal(await shown().getAttribute('data-session-content'),first.id);
  await shown().locator('.cm-content').click();await page.keyboard.press('ControlOrMeta+End');await page.keyboard.type('\nSaved from session');
  await until(()=>readFileSync(join(projects[0],'notes.md'),'utf8').includes('Saved from session'));
  await shown().locator('[data-file-face-toggle]').click();await shown().locator('[data-file-preview]').waitFor();
  await head(sessions[0]).click();await side(sessions[0]).waitFor({state:'detached'});
  await head(sessions[0]).click();await shown().locator('[data-file-preview]').waitFor();
  await head(sessions[1]).click();await head(sessions[1]).click();
  await shown().getByText('only-1.txt',{exact:true}).waitFor();
  await head(sessions[0]).click();await shown().locator('[data-file-preview]').waitFor();
  console.log('PASS: source by default, stable handle, real autosave, preview retained through collapse and session changes');
  // CLI writes must reach the running UI, using the same node handles.
  const cli=(...args)=>execFileSync(join(scratch,'khor'),args,{env,encoding:'utf8',timeout:15000}).trim();
  const opened=cli('tabs','--session',sessions[0],'open','--view','file',join(projects[0],'only-0.txt'));
  await page.locator(`[data-content-tab="${opened}"]`).waitFor();await shown().locator('.cm-content').waitFor();
  assert((await shown().innerText()).includes('This file belongs to 0'));
  cli('tabs','--session',sessions[0],'close',opened);await page.locator(`[data-content-tab="${opened}"]`).waitFor({state:'detached'});
  assert(!cli('tabs','--session',sessions[1],'ls').includes(first.id),'session scope leaked');
  const keyTab=cli('tabs','--session',sessions[0],'open','--view','file',join(projects[0],'only-0.txt'));
  const keyButton=page.locator(`[data-content-tab="${keyTab}"] [role=tab]`);await keyButton.waitFor();await keyButton.focus();await page.keyboard.press('ControlOrMeta+w');
  await page.locator(`[data-content-tab="${keyTab}"]`).waitFor({state:'detached'});assert.equal(await page.locator('[data-session-tabs] [data-tab]').count(),2,'content close closed a session');

  // + becomes a page in place, rather than minting a second child id.
  await page.locator(`[data-content-tabs="${sessions[0]}"]`).getByRole('button',{name:word('content-new'),exact:true}).click();
  await shown().locator('[data-file-tree]').waitFor();const fresh=await shown().getAttribute('data-session-content');
  const search=shown().locator('[data-side-find-field]');await search.fill('notes');
  await shown().getByRole('button',{name:word('content-scope'),exact:true}).click();await page.getByRole('menuitem',{name:word('browser-tab'),exact:true}).click();
  const address=shown().locator('[data-content-browser-field] input').first();await address.fill('http://localhost:'+port);
  await shown().getByRole('button',{name:word('content-scope'),exact:true}).click();await page.getByRole('menuitem',{name:word('workspaces-tab'),exact:true}).click();assert.equal(await search.inputValue(),'notes');
  await shown().getByRole('button',{name:word('content-scope'),exact:true}).click();await page.getByRole('menuitem',{name:word('browser-tab'),exact:true}).click();await address.press('Enter');
  await until(async()=>{const t=(await table()).tabs.find(t=>t.id===fresh);return t?.view==='page'&&t.url.startsWith('http://localhost:');});
  await page.locator(`[data-content-tab="${first.id}"] [role=tab]`).click();await shown().locator('[data-file-preview]').waitFor();
  await page.locator('.session-content-tools button').first().click();assert.equal(await page.locator('[data-session-panes]').evaluate(el=>getComputedStyle(el.firstElementChild).display),'none');
  await page.locator('.session-content-tools button').first().click();
  await page.screenshot({path:join(scratch,'desktop.png')});
  const ids=(await table()).tabs.map(t=>t.id);
  await page.reload();await head(sessions[0]).waitFor();await head(sessions[0]).click();await side(sessions[0]).waitFor();assert.deepEqual((await table()).tabs.map(t=>t.id),ids);
  await shown().locator('.cm-content').waitFor();
  console.log('PASS: CLI open/close and session scope, scope input retained, page conversion in place, restart restores ids');
  // Content pressure must never scroll the session identity or either + away.
  for(let i=0;i<6;i++)cli('tabs','--session',sessions[0],'open','--view','workspace',projects[0]);
  await until(async()=>await page.locator(`[data-content-tabs="${sessions[0]}"] [data-content-tab]`).count()===(await table()).tabs.filter(t=>t.session===sessions[0]).length);
  await page.setViewportSize({width:1100,height:850});
  await page.waitForTimeout(250);
  const geometry=await page.locator('[data-session-tabs]').evaluate(el=>{
    const face=el.querySelector('[data-on="true"] [data-side-face]').getBoundingClientRect(), strip=el.querySelector('[data-tab-strip]').getBoundingClientRect(), box=el.getBoundingClientRect(), add=el.querySelector('[data-content-tabs][data-open="true"] .session-content-add').getBoundingClientRect(), outer=el.querySelector('[data-tab-new]').getBoundingClientRect();
    return {face:face.toJSON(),strip:strip.toJSON(),box:box.toJSON(),add:add.toJSON(),outer:outer.toJSON()};
  });
  assert(geometry.face.left>=geometry.strip.left-1&&geometry.face.right<=geometry.strip.right+1,'session identity scrolled away');
  assert(geometry.add.width>0&&geometry.add.right<=geometry.box.right+1,'content + clipped');
  assert(geometry.outer.width>0&&geometry.outer.right<=geometry.box.right+1,'session + clipped');
  const paneBoxes=await page.locator('[data-session-panes]').evaluate(el=>({row:el.getBoundingClientRect().toJSON(),side:el.querySelector('[data-session-side]').getBoundingClientRect().toJSON(),main:{rect:el.firstElementChild.getBoundingClientRect().toJSON(),style:el.firstElementChild.getAttribute('style'),min:getComputedStyle(el.firstElementChild).minWidth,flex:getComputedStyle(el.firstElementChild).flex},sideStyle:el.querySelector('[data-session-side]').getAttribute('style'),ancestors:[el,el.parentElement,el.parentElement.parentElement].map(x=>({tag:x.tagName,cls:x.className,style:x.getAttribute('style'),width:x.getBoundingClientRect().width}))}));
  assert(paneBoxes.side.right<=paneBoxes.row.right+1&&paneBoxes.side.width>0,'content pane fell outside the viewport: '+JSON.stringify(paneBoxes));
  await page.screenshot({path:join(scratch,'crowded.png')});
  await page.setViewportSize({width:1600,height:1000});
  await page.locator('.session-content-tools button').first().click();
  for(const t of (await table()).tabs.filter(t=>t.session===sessions[0]))cli('tabs','--session',sessions[0],'close',t.id);
  await side(sessions[0]).waitFor({state:'detached'});
  assert.notEqual(await page.locator('[data-session-panes]').evaluate(el=>getComputedStyle(el.firstElementChild).display),'none','last child closed with terminal still hidden');
  console.log('PASS: crowded strip keeps avatar and both add buttons; closing last maximized child restores terminal');
  // Narrow stays on the existing terminal/deck layout, with no desktop group.
  await page.setViewportSize({width:390,height:844});await page.locator('[data-session-tabs]').waitFor({state:'detached'});await page.locator('[data-terminal]').first().waitFor();
  assert.equal(await page.locator('[data-session-tabs]').count(),0);
  assert.equal(await page.locator('[data-session-side]').count(),0);
  assert(!await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth));
  assert.deepEqual(errors,[]);
  console.log('PASS: narrow terminal/deck preserved; no browser exceptions');
  console.log('session content gate: clean');
}catch(e){await page?.screenshot({path:join(scratch,'failed.png')}).catch(()=>{});console.error(e);throw e;}
finally{
  for(const id of sessions)await api('close_session',{id}).catch(()=>{});
  await browser?.close();
  for(const child of children)try{process.kill(-child.pid,'SIGKILL');}catch{}
  await usage?.close();
}
