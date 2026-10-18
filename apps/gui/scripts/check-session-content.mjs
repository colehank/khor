// Real App + private node/PTYs. Transport is the existing dev bridge;
// native page overlay/borrow rendering requires desktop app acceptance.
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
  const head=id=>page.locator(`[data-tab-open="${id}"]`);
  const hoverHead=async id=>{const b=await head(id).boundingBox();await page.mouse.move(b.x+b.width/2,b.y+b.height/2);};
  const side=id=>page.locator(`[data-session-side="${id}"]`);
  const shown=()=>page.locator('[data-session-content][data-content-visible=true]');
  const table=()=>api('tabs_list',{kind:'content'});
  const desktopPicker=()=>page.locator(`[data-content-picker="${sessions[0]}"]:visible`);
  const desktopAdd=()=>page.locator(`[data-content-tabs="${sessions[0]}"] .session-content-add>button`);
  const max=()=>page.locator('[data-content-maximize]');
  const cli=(...args)=>execFileSync(join(scratch,'khor'),args,{env,encoding:'utf8',timeout:15000}).trim();
  await head(sessions[0]).waitFor();
  assert.equal(await page.locator('[data-session-tabs] [data-tab-new]').count(),1,'outer + missing');
  const dragTab=async(from,to)=>{
    const a=await from.boundingBox(),b=await to.boundingBox();assert(a&&b,'drag slots missing');
    await page.mouse.move(a.x+a.width*.4,a.y+a.height/2);await page.mouse.down();
    await page.mouse.move(b.x+b.width*(b.x > a.x ? .8 : .2),b.y+b.height/2,{steps:18});await page.mouse.up();
  };
  const outerOrder=()=>page.locator('[data-session-tabs] [data-tab]').evaluateAll(els=>els.map(el=>el.dataset.tab));
  await dragTab(head(sessions[1]),head(sessions[0]));
  await until(async()=>JSON.stringify(await outerOrder())===JSON.stringify([...sessions].reverse()));
  assert.equal(await page.locator('[data-tab-group][data-on=true]').getAttribute('data-tab'),sessions[0],'session reorder changed focus');
  await page.reload();await head(sessions[0]).waitFor();assert.deepEqual(await outerOrder(),[...sessions].reverse(),'session order did not survive reload');
  await dragTab(head(sessions[1]),head(sessions[0]));await until(async()=>JSON.stringify(await outerOrder())===JSON.stringify(sessions));

  await head(sessions[0]).click();
  await until(async()=>await page.locator('[data-session-tabs] [data-tab]').count()===1);
  await side(sessions[0]).waitFor();
  assert.equal(await page.locator('[data-session-tabs] [data-tab-new]').count(),0,'focused group still shows the outer session +');
  const emptyBrowse=()=>page.locator(`[data-content-tabs="${sessions[0]}"] [data-content-browse]`);
  await emptyBrowse().locator('.session-content-close').click();
  await until(async()=>await emptyBrowse().count()===0&&await side(sessions[0]).count()===0);
  assert.deepEqual(await outerOrder(),sessions,'closing the last content tab did not restore plain session tabs');
  assert.equal((await table()).tabs.length,0,'closing synthetic browsing wrote a node tab');
  await head(sessions[0]).click();await side(sessions[0]).waitFor();
  await desktopPicker().locator('[data-side-find-field]').focus();await page.keyboard.press('ControlOrMeta+w');
  await until(async()=>await emptyBrowse().count()===0&&await side(sessions[0]).count()===0);
  assert.deepEqual(await outerOrder(),sessions,'content shortcut closed the session instead of browsing');
  await page.reload();await head(sessions[0]).waitFor();
  assert.equal(await emptyBrowse().count(),0,'reload recreated a closed browsing tab');
  await head(sessions[0]).click();await side(sessions[0]).waitFor();
  console.log('PASS: closing empty browsing removes tab and pane, restores session strip, survives reload and reopens; keyboard close shares the same action');
  await desktopAdd().click();await side(sessions[0]).waitFor();
  await desktopPicker().locator('[data-file-tree]').waitFor();
  assert.equal((await table()).tabs.length,0,'opening the desktop picker created a persistent empty tab');
  const virtual=page.locator(`[data-content-tabs="${sessions[0]}"] [data-content-browse]`);
  assert.equal(await virtual.locator('svg:not(.session-content-close svg)').count(),0,'unresolved browsing tab has an ambiguous icon');
  await desktopAdd().click();assert.equal((await table()).tabs.length,0,'content + created an empty tab');
  const scope=()=>desktopPicker().locator('[data-content-scope]:visible');
  const scopeBox=()=>scope().evaluate(el=>({box:el.getBoundingClientRect().toJSON(),glyph:el.querySelector('svg').getBoundingClientRect().toJSON()}));
  const measureIcons=()=>page.evaluate(()=>{
    const rect=el=>{const r=el.getBoundingClientRect();return {x:r.x,y:r.y,w:r.width,h:r.height,cx:r.x+r.width/2,cy:r.y+r.height/2};};
    const selectors={maximize:'[data-content-maximize]',hide:'[data-content-hide]',refresh:'[data-session-verbs] [data-glass-circle]',scope:'[data-content-picker]:not(.session-content-retained) [data-content-scope]',plus:'[data-content-picker]:not(.session-content-retained) [data-side-new]',search:'[data-content-picker]:not(.session-content-retained) [data-search-wrap]>svg',reference:'[data-search-open]'};
    return Object.fromEntries(Object.entries(selectors).map(([key,selector])=>{const el=[...document.querySelectorAll(selector)].find(e=>e.getClientRects().length);if(!el)return [key,null];return [key,{button:rect(el),svgs:[...el.querySelectorAll('svg')].map(svg=>({box:rect(svg),viewBox:svg.getAttribute('viewBox'),bbox:(()=>{const r=svg.getBBox();return {x:r.x,y:r.y,w:r.width,h:r.height}})()})),glass:el.hasAttribute('data-glass-circle')}];}));
  });
  await page.locator('[data-session-verbs] [data-glass-circle]').waitFor({state:'visible'});
  const iconResults=[];
  const checkIcons=async(name)=>{
    const icons=await measureIcons();iconResults.push({name,icons});
    for(const key of ['maximize','hide','refresh','scope','plus']){
      const control=icons[key];assert(control,'missing '+key);
      assert(control.glass,key+' uses a separate button family');
      assert.equal(control.button.w,key==='scope'?52:36,key+' left the glass control family');assert.equal(control.button.h,36);
      const glyph=control.svgs[0].box;assert.equal(glyph.w,22,key+' glyph size differs');assert.equal(glyph.h,22);
      if(key==='scope'){
        assert.equal(control.svgs.length,2,'desktop source switch lacks its down chevron');
        const arrow=control.svgs[1].box;assert.equal(arrow.w,12);assert.equal(arrow.h,12);
        assert(Math.abs((glyph.x+arrow.x+arrow.w)/2-control.button.cx)<.1,'source group is off-center');
        assert(Math.abs(glyph.cy-control.button.cy)<.1&&Math.abs(arrow.cy-control.button.cy)<.1,'source glyphs do not share a centerline');
      }else assert(Math.abs(glyph.cx-control.button.cx)<.1&&Math.abs(glyph.cy-control.button.cy)<.1,key+' glyph is off-center: '+JSON.stringify(control));
    }
    writeFileSync(join(scratch,'desktop-icons.json'),JSON.stringify(iconResults,null,2));
    await page.screenshot({path:join(scratch,'icons-'+name+'.png'),clip:{x:0,y:0,width:page.viewportSize().width,height:156}});
  };
  for(const theme of ['light','dark'])for(const width of [1000,1600,1920]){
    await page.setViewportSize({width,height:1000});await page.evaluate(theme=>document.documentElement.dataset.theme=theme,theme);
    await checkIcons(theme+'-'+width);
  }
  await page.setViewportSize({width:1600,height:1000});
  await max().click();await checkIcons('restore');await max().click();
  await page.evaluate(()=>document.documentElement.style.setProperty('--ui-scale',String(23/17)));
  await checkIcons('large-ui');await page.evaluate(()=>document.documentElement.style.removeProperty('--ui-scale'));
  console.log('PASS: desktop glass controls and centered glyphs at 1000/1600/1920, light/dark, restore and large UI');
  const workspaceScope=await scopeBox();
  await scope().click();
  const choices=await page.locator('[data-content-source]').evaluateAll(els=>els.map(el=>{const r=el.querySelector('.content-source-glyph').getBoundingClientRect();const svg=el.querySelector('svg').getBoundingClientRect();const text=[...el.childNodes].find(n=>n.nodeType===Node.TEXT_NODE&&n.textContent.trim());const range=document.createRange();range.selectNode(text);return {x:r.x,tx:range.getBoundingClientRect().x,w:r.width,h:r.height,sw:svg.width,sh:svg.height};}));
  assert.equal(choices.length,2);assert(Math.abs(choices[0].x-choices[1].x)<.1,'source menu leading seats differ');assert(Math.abs(choices[0].tx-choices[1].tx)<.1,'source menu labels have different leading insets');
  assert(choices.every(c=>Math.abs(c.w-20)<.1&&Math.abs(c.h-20)<.1&&Math.abs(c.sw-20)<.1&&Math.abs(c.sh-20)<.1),'source menu used rail-sized icons: '+JSON.stringify(choices));
  await page.locator('[data-content-source=browser]').click();
  const browserScope=await scopeBox();
  for(const key of ['x','y','width','height'])assert(Math.abs(workspaceScope.box[key]-browserScope.box[key])<.1,'source switch moved at '+key);
  assert(Math.abs(workspaceScope.glyph.x-browserScope.glyph.x)<.1,'source glyph changed its leading inset');
  const address=()=>desktopPicker().locator('[data-page-address]');
  await address().fill('http://localhost:'+port+'/kept-draft');
  await scope().click();await page.locator('[data-content-source=workspace]').click();
  const search=()=>desktopPicker().locator('[data-side-find-field]');await search().fill('notes');
  await scope().click();await page.locator('[data-content-source=browser]').click();await address().focus();assert.equal(await address().inputValue(),'http://localhost:'+port+'/kept-draft');
  await scope().click();await page.locator('[data-content-source=workspace]').click();assert.equal(await search().inputValue(),'notes');
  await desktopPicker().getByText('notes.md',{exact:true}).click();await shown().locator('.cm-content').waitFor();
  await until(async()=>(await table()).tabs.length===1);
  assert.equal((await table()).tabs[0].url,join(projects[0],'notes.md'),'opening the file did not persist it immediately');
  const sessionPaint=await page.locator('[data-tab-group][data-on=true]').evaluate(el=>({group:getComputedStyle(el).backgroundColor,head:getComputedStyle(el.querySelector('[data-tab-head]')).backgroundColor,chosen:getComputedStyle(el.querySelector('[data-tab-head]')).getPropertyValue('--glass-sel').trim()}));
  assert.equal(sessionPaint.group,'rgba(0, 0, 0, 0)','expanded group stacks another glass fill');
  assert.notEqual(sessionPaint.head,'rgba(0, 0, 0, 0)','session lost its original selected glass');
  assert.equal(await page.locator('[data-content-on=true]').count(),1,'side selection disappeared');

  assert.equal(await shown().locator('[data-file-embedded-tools]').count(),0,'desktop content added an alternate file toolbar');await shown().locator('[data-file-deck]').waitFor();
  const stable=await shown().getAttribute('data-content-handle');
  await shown().locator('.cm-content').evaluate(el=>window.__desktopContentEditor=el);
  await desktopAdd().click();await search().fill('notes');await desktopPicker().getByText('notes.md',{exact:true}).click();
  await until(async()=>(await table()).tabs.length===1);
  const first=(await table()).tabs[0];assert.equal(first.view,'file');assert.equal(first.url,join(projects[0],'notes.md'));
  assert.equal(await shown().getAttribute('data-content-handle'),stable);assert(await shown().locator('.cm-content').evaluate(el=>el===window.__desktopContentEditor),'reopening rebuilt the editor');
  await shown().locator('[data-file-face-toggle]').click();await shown().locator('[data-file-preview]').waitFor();
  const beforeToggle=await page.locator('[data-session-panes]').evaluate(el=>({box:el.getBoundingClientRect().toJSON(),side:el.querySelector('[data-session-side]').getBoundingClientRect().toJSON()}));
  await head(sessions[0]).click();await until(async()=>await page.locator('[data-session-tabs] [data-tab]').count()===2);
  await shown().locator('[data-file-preview]').waitFor();
  assert.deepEqual(await page.locator('[data-session-panes]').evaluate(el=>({box:el.getBoundingClientRect().toJSON(),side:el.querySelector('[data-session-side]').getBoundingClientRect().toJSON()})),beforeToggle,'tab presentation changed pane geometry');
  assert.deepEqual(await page.locator('[data-session-tabs] [data-tab]').evaluateAll(els=>els.map(el=>el.dataset.tab)),sessions,'restoring tabs changed their order');
  await head(sessions[0]).click();await until(async()=>await page.locator('[data-session-tabs] [data-tab]').count()===1);
  await page.locator(`[data-row="${sessions[1]}"] [data-row-open]`).click();await head(sessions[1]).waitFor();
  assert.equal(await page.locator('[data-session-tabs] [data-tab]').count(),1,'focus did not follow list selection');
  await page.locator(`[data-content-tabs="${sessions[1]}"] .session-content-add>button`).click();await page.locator(`[data-content-picker="${sessions[1]}"]:visible`).getByText('only-1.txt',{exact:true}).waitFor();
  await page.locator(`[data-row="${sessions[0]}"] [data-row-open]`).click();await shown().locator('[data-file-preview]').waitFor();
  await shown().locator('[data-file-back]').click();await search().waitFor();assert.equal(await search().inputValue(),'notes');
  await search().fill('only-0');await desktopPicker().getByText('only-0.txt',{exact:true}).click();await shown().locator('.cm-content').waitFor();
  await shown().locator('.cm-content').click();await page.keyboard.press('ControlOrMeta+End');await page.keyboard.type('\nSaved from desktop content');
  await until(()=>readFileSync(join(projects[0],'only-0.txt'),'utf8').includes('Saved from desktop content'));
  await until(async()=>(await table()).tabs.length===2);
  console.log('PASS: aligned source seats and menu icons; iconless browser draft; no persistent empty tabs; real file deck; immediate durable opens and duplicate activation; autosave, return query and session resume');
  const notesPreview=()=>page.locator(`[data-content-handle="${stable}"] [data-file-preview]`);
  assert.equal(await notesPreview().count(),1,'notes preview lost before drag');
  const childrenBefore=(await table()).tabs.filter(t=>t.session===sessions[0]).map(t=>t.id),frontBefore=await shown().getAttribute('data-content-handle');
  const childButton=id=>page.locator(`[data-content-tab="${id}"] [role=tab]`);
  await dragTab(childButton(childrenBefore[0]),childButton(childrenBefore[1]));
  await until(async()=>JSON.stringify((await table()).tabs.filter(t=>t.session===sessions[0]).map(t=>t.id))===JSON.stringify([...childrenBefore].reverse()));
  assert.equal(await shown().getAttribute('data-content-handle'),frontBefore,'drag focused another document');
  assert.equal(await notesPreview().count(),1,'reorder reset a retained file preview');
  assert.deepEqual(await outerOrder(),[sessions[0]],'child drag moved or expanded its parent session');
  await childButton(childrenBefore[0]).hover();
  const hover=await page.locator(`[data-content-tab="${childrenBefore[0]}"]`).evaluate(el=>({pill:getComputedStyle(el).backgroundColor,opener:getComputedStyle(el.querySelector('.session-content-open')).backgroundColor}));
  assert.notEqual(hover.pill,'rgba(0, 0, 0, 0)','child hover does not include its close slot');assert.equal(hover.opener,'rgba(0, 0, 0, 0)','child opener paints a separate partial pill');
  const cancelBefore=(await table()).tabs.map(t=>t.id),a=await childButton(childrenBefore[0]).boundingBox(),b=await childButton(childrenBefore[1]).boundingBox();
  await page.mouse.move(a.x+a.width*.4,a.y+a.height/2);await page.mouse.down();await page.mouse.move(b.x+b.width*.2,b.y+b.height/2,{steps:12});await page.locator('[data-tab-reordering=true]').waitFor();await page.keyboard.press('Escape');await page.mouse.up();
  assert.deepEqual((await table()).tabs.map(t=>t.id),cancelBefore,'Escape committed a cancelled drag');
  assert.equal(await notesPreview().count(),1,'Escape cancel reset a retained file preview: '+JSON.stringify(await page.locator('[data-session-content]').evaluateAll(els=>els.map(el=>({id:el.dataset.sessionContent,key:el.dataset.contentHandle,show:el.dataset.contentVisible,preview:!!el.querySelector('[data-file-preview]'),text:el.textContent.slice(0,60)})))));
  if(process.env.CONTENT_STOP_AFTER==='drag'){assert.deepEqual(errors,[]);break checks;}
  console.log('PASS: outer reload order; durable nested order; parent/session and side selection; whole-pill hover; drag click suppression and Escape cancel');

  // CLI writes reach the same content scene without changing session scope.
  const opened=cli('tabs','--session',sessions[0],'open','--view','file',join(projects[0],'extra-01.txt'));
  await shown().locator('.cm-content').filter({hasText:'Neighbor 1'}).waitFor();await page.locator(`[data-content-tab="${opened}"]`).waitFor();
  cli('tabs','--session',sessions[0],'close',opened);await page.locator(`[data-content-tab="${opened}"]`).waitFor({state:'detached'});
  assert(!cli('tabs','--session',sessions[1],'ls').includes(first.id),'session scope leaked');
  const keyTab=cli('tabs','--session',sessions[0],'open','--view','file',join(projects[0],'extra-02.txt'));
  const keyButton=page.locator(`[data-content-tab="${keyTab}"] [role=tab]`);await keyButton.waitFor();await keyButton.focus();await page.keyboard.press('ControlOrMeta+w');await page.locator(`[data-content-tab="${keyTab}"]`).waitFor({state:'detached'});
  assert.equal(await page.locator('[data-session-tabs] [data-tab]').count(),1,'content close changed focused session');
  await desktopAdd().click();await search().waitFor();await scope().click();await page.locator('[data-content-source=browser]').click();
  await address().focus();await address().fill('http://localhost:'+port);assert.equal(await address().inputValue(),'http://localhost:'+port,'draft replacement changed before submission');await address().press('Enter');await shown().locator('[data-content-page-bar]').waitFor();await shown().locator('[data-page-address]').focus();assert.equal(await shown().locator('[data-page-address]').inputValue(),'http://localhost:'+port,'submitted address changed');await shown().locator('[data-page-address]').blur();await until(async()=>(await table()).tabs.length===3);
  writeFileSync(join(scratch,'page-before-reopen.json'),JSON.stringify(await table(),null,2));
  const desktopPageHandle=await shown().getAttribute('data-content-handle');await desktopAdd().click();await scope().click();await page.locator('[data-content-source=browser]').click();await address().focus();await address().fill('http://localhost:'+port);assert.equal(await address().inputValue(),'http://localhost:'+port,'reopened draft replacement changed before submission');await address().press('Enter');await shown().locator('[data-content-page-bar]').waitFor();writeFileSync(join(scratch,'page-after-reopen.json'),JSON.stringify(await table(),null,2));assert.equal((await table()).tabs.length,3,'reopening the webpage created a duplicate');assert.equal(await shown().getAttribute('data-content-handle'),desktopPageHandle);
  if(process.env.CONTENT_STOP_AFTER==='page-dedup'){assert.deepEqual(errors,[]);console.log('PASS: exact webpage draft, durable duplicate reuse and stable handle');break checks;}
  await page.locator(`[data-content-tab="${first.id}"] [role=tab]`).click();await shown().locator('[data-file-preview]').waitFor();
  await max().click();assert.equal(await page.locator('[data-session-panes]').evaluate(el=>getComputedStyle(el.firstElementChild).display),'none');await max().click();
  await page.screenshot({path:join(scratch,'desktop.png')});
  await page.reload();await head(sessions[0]).waitFor();await head(sessions[0]).click();await page.locator(`[data-content-tabs="${sessions[0]}"] [data-content-tab="${first.id}"] [role=tab]`).click();await shown().locator('.cm-content').waitFor();assert.equal((await table()).tabs.length,3);assert.equal(await shown().getAttribute('data-session-content'),first.id);
  console.log('PASS: CLI synchronization and scoped shortcuts; immediate page tabs and duplicate activation; persisted ids after reload; maximize/restore');
  // Both add controls and the session identity remain under content pressure.
  for(let i=3;i<9;i++)cli('tabs','--session',sessions[0],'open','--view','file',join(projects[0],'extra-0'+i+'.txt'));
  await until(async()=>await page.locator(`[data-content-tabs="${sessions[0]}"] [data-content-tab]`).count()>=9);
  await page.setViewportSize({width:1000,height:850});await head(sessions[0]).waitFor();
  const geometry=await page.locator(`[data-tab="${sessions[0]}"]`).evaluate(el=>{const r=x=>x.getBoundingClientRect().toJSON();return {box:r(el),head:r(el.querySelector('[data-tab-head]')),add:r(el.querySelector('.session-content-add')),outer:document.querySelector('[data-session-tabs] [data-tab-new]')};});
  assert(geometry.head.x>=geometry.box.x-1&&geometry.head.right<=geometry.box.right+1,'session identity scrolled away');
  assert(geometry.add.width>0&&geometry.add.right<=geometry.box.right+1,'content + clipped');assert.equal(geometry.outer,null,'focused group still shows the outer session +');
  // Close successive inactive content tabs with the pointer at one spot.
  const contentStrip=page.locator(`[data-content-tabs="${sessions[0]}"]`);
  const pair=await contentStrip.evaluate(el=>{
    const viewport=el.querySelector('.session-content-scroll').getBoundingClientRect();
    const tabs=[...el.querySelectorAll('[data-content-tab]')];
    for(let i=0;i<tabs.length-1;i++){
      const a=tabs[i],b=tabs[i+1],ra=a.getBoundingClientRect(),rb=b.getBoundingClientRect();
      if(a.dataset.contentOn!=='true'&&b.dataset.contentOn!=='true'&&ra.left>=viewport.left+1&&rb.right<=viewport.right-1&&Math.abs(ra.width-rb.width)<.2)return [a.dataset.contentTab,b.dataset.contentTab];
    }
    return null;
  });
  assert(pair,'no adjacent compressed content tabs available');
  const closeAt=id=>page.locator(`[data-content-tab="${id}"] .session-content-close`);
  await page.locator(`[data-content-tab="${pair[0]}"]`).hover();
  const closeBox=await closeAt(pair[0]).boundingBox(),closeX=closeBox.x+closeBox.width/2,closeY=closeBox.y+closeBox.height/2;
  await page.mouse.click(closeX,closeY);await until(async()=>await closeAt(pair[0]).count()===0);
  const nextBox=await closeAt(pair[1]).boundingBox();
  assert(Math.abs(nextBox.x+nextBox.width/2-closeX)<1,'next content close moved away from pointer: '+JSON.stringify({closeBox,nextBox,held:await contentStrip.getAttribute('data-content-held'),tabs:await contentStrip.locator('[data-content-tab]').evaluateAll(els=>els.map(el=>({id:el.dataset.contentTab,w:el.getBoundingClientRect().width,style:el.getAttribute('style')})))}));
  assert.equal(await contentStrip.getAttribute('data-content-held'),'true');
  await page.mouse.click(closeX,closeY);await until(async()=>await closeAt(pair[1]).count()===0);
  await page.setViewportSize({width:1080,height:850});await until(async()=>await contentStrip.getAttribute('data-content-held')===null);
  await page.setViewportSize({width:1000,height:850});
  const paneBoxes=await page.locator('[data-session-panes]').evaluate(el=>({row:el.getBoundingClientRect().toJSON(),side:el.querySelector('[data-session-side]').getBoundingClientRect().toJSON()}));
  assert(paneBoxes.side.right<=paneBoxes.row.right+1&&paneBoxes.side.width>0,'content pane overflow');await page.screenshot({path:join(scratch,'crowded.png')});
  await page.setViewportSize({width:1600,height:1000});await max().click();
  for(const t of (await table()).tabs.filter(t=>t.session===sessions[0]))cli('tabs','--session',sessions[0],'close',t.id);
  await side(sessions[0]).waitFor({state:'detached'});assert.notEqual(await page.locator('[data-session-panes]').evaluate(el=>getComputedStyle(el.firstElementChild).display),'none','last child kept the terminal hidden');
  assert.equal(await emptyBrowse().count(),0,'last document left a synthetic browsing tab');
  assert.deepEqual(await outerOrder(),sessions,'last document did not restore plain session tabs');
  console.log('PASS: crowded strip preserves session identity and add controls; closing the last maximized document restores the terminal');
  if(process.env.CONTENT_DESKTOP_ICONS_ONLY==='1'){
  await head(sessions[0]).click();await until(async()=>await page.locator('[data-session-tabs] [data-tab]').count()===1);
  // Several long session titles at narrow desktop width. Presentation follows
  // list picks, and restoring the whole strip preserves its stored order.
  for(let i=0;i<4;i++){
    const id=await api('open_on',{machine:'',title:'Long desktop session title '+i+' · 原始顺序与内容布局保持',cwd:projects[0],cols:100,rows:35});sessions.push(id);
    await page.locator(`[data-row="${id}"] [data-row-open]`).click();await head(id).waitFor();
  }
  await page.setViewportSize({width:1000,height:850});
  const front=sessions.at(-1),wholeOrder=[...sessions];
  const paneRect=()=>page.locator('[data-session-panes]').evaluate(el=>el.getBoundingClientRect().toJSON());
  const terminalRect=await paneRect();
  await head(front).focus();await page.keyboard.press('Enter');
  await until(async()=>await page.locator('[data-session-tabs] [data-tab]').count()===sessions.length);
  assert.deepEqual(await page.locator('[data-session-tabs] [data-tab]').evaluateAll(els=>els.map(el=>el.dataset.tab)),wholeOrder);
  assert.deepEqual(await paneRect(),terminalRect,'keyboard restore resized terminal');
  await head(sessions[2]).click();assert.equal(await page.locator('[data-session-tabs] [data-tab]').count(),sessions.length,'switching another session toggled focus');
  await until(async()=>await head(sessions[2]).getAttribute('aria-expanded')==='false');
  await head(sessions[2]).focus();await page.keyboard.press('Space');
  await until(async()=>await page.locator('[data-session-tabs] [data-tab]').count()===1);
  await page.locator(`[data-content-tabs="${sessions[2]}"] .session-content-add>button`).click();
  const dividers=await page.locator('[data-session-tabs] [data-tab-divider]:visible').evaluateAll(els=>els.map(el=>{const r=el.getBoundingClientRect(),p=el.parentElement.getBoundingClientRect();return {w:r.width,h:r.height,cy:r.y+r.height/2,py:p.y+p.height/2,top:r.y-p.y,bottom:p.bottom-r.bottom};}));
  assert.equal(dividers.length,2,'focused group must have only its two inner dividers');
  assert.equal(await page.locator('[data-session-tabs] [data-tab-new]').count(),0,'focused group still has a session +');
  for(const d of dividers){assert(Math.abs(d.w-dividers[0].w)<.1&&Math.abs(d.h-dividers[0].h)<.1,'nested divider differs from outer');assert(Math.abs(d.cy-d.py)<.1&&d.top>0&&d.bottom>0,'divider is not centered with end space');}
  await page.screenshot({path:join(scratch,'focus-long-titles.png')});
  await head(sessions[2]).click();await until(async()=>await page.locator('[data-session-tabs] [data-tab]').count()===sessions.length);
  await page.locator('[data-session-tabs] [data-tab-group]').evaluateAll(els=>els.forEach(el=>{el.style.flex='0 0 auto';el.style.width='90px';}));
  const names=await page.locator('[data-session-tabs] [data-tab-head]').evaluateAll(els=>els.map(el=>({w:el.getBoundingClientRect().width,label:el.querySelector('[data-tab-label]').getBoundingClientRect().width,visible:el.querySelector('[data-tab-label]').getClientRects().length>0})));
  assert(names.every(n=>n.w>=89&&n.w<=91&&n.visible&&n.label>0),'90px session titles disappeared instead of truncating: '+JSON.stringify(names));
  await page.screenshot({path:join(scratch,'titles-90.png')});
  await page.locator('[data-session-tabs] [data-tab-group]').evaluateAll(els=>els.forEach(el=>{el.style.width=el.dataset.on==='true'?'64px':'var(--head-ctl)';}));
  await page.mouse.move(8,800);await page.evaluate(()=>document.activeElement?.blur());
  const compact=()=>page.locator('[data-session-tabs] [data-tab-head]').evaluateAll(els=>els.map(el=>{const head=el.getBoundingClientRect(),open=el.querySelector('[data-tab-open]'),face=open.firstElementChild.getBoundingClientRect(),close=el.querySelector('[data-tab-close]');return {on:el.parentElement.dataset.on==='true',w:head.width,cx:head.x+head.width/2,fx:face.x+face.width/2,name:el.querySelector('[data-tab-label]').getClientRects().length>0,position:getComputedStyle(close).position,opacity:getComputedStyle(close).opacity,hover:el.parentElement.matches(':hover'),focus:el.parentElement.matches(':focus-within'),active:document.activeElement?.tagName};}));
  await until(async()=>(await compact()).every(c=>!c.name&&(c.on?Math.abs(c.w-64)<.1&&c.position!=='absolute'&&c.opacity==='1':c.w<64&&c.position==='absolute'&&c.opacity==='0'&&Math.abs(c.cx-c.fx)<.1)));
  const activeClose=page.locator(`[data-tab="${sessions[2]}"] [data-tab-close]`);
  await hoverHead(sessions[2]);assert.equal(await activeClose.evaluate(el=>getComputedStyle(el).opacity),'1');
  await head(sessions[2]).focus();await page.mouse.move(8,800);assert.equal(await activeClose.evaluate(el=>getComputedStyle(el).opacity),'1','keyboard focus did not reveal close');
  writeFileSync(join(scratch,'tab-density.json'),JSON.stringify({names,narrow:await compact()},null,2));
  await page.screenshot({path:join(scratch,'avatar-floor-focus.png')});
  await page.evaluate(()=>document.activeElement?.blur());
  await hoverHead(sessions[3]);
  const outerClose=page.locator(`[data-tab="${sessions[3]}"] [data-tab-close]`),outerBox=await outerClose.boundingBox();
  const outerX=outerBox.x+outerBox.width/2,outerY=outerBox.y+outerBox.height/2;
  await page.mouse.click(outerX,outerY);await until(async()=>await head(sessions[3]).count()===0);
  const following=await page.locator(`[data-tab="${sessions[4]}"] [data-tab-close]`).boundingBox();
  assert(Math.abs(following.x+following.width/2-outerX)<1,'next session close moved away from pointer');
  await page.mouse.click(outerX,outerY);await until(async()=>await head(sessions[4]).count()===0);
  assert.equal(await page.locator('[data-session-tabs] [data-tab][data-on=true]').getAttribute('data-tab'),sessions[2],'closing another avatar changed selection');
  assert.equal(await head(sessions[2]).getAttribute('aria-expanded'),'false','close click toggled focus');
  await page.locator('[data-session-tabs] [data-tab-group]').evaluateAll(els=>els.forEach(el=>{el.style.removeProperty('width');el.style.removeProperty('flex');}));
  await until(async()=>await page.locator('[data-session-tabs] [data-tab]').count()===sessions.length-2);
  assert.deepEqual(await page.locator('[data-session-tabs] [data-tab]').evaluateAll(els=>els.map(el=>el.dataset.tab)),wholeOrder.filter(id=>id!==sessions[3]&&id!==sessions[4]));
  await page.locator('[data-session-tabs] [data-tab-new]').click();
  await until(async()=>await page.locator('[data-session-tabs] [data-tab]').count()===sessions.length-1);
  assert.equal(await page.locator('[data-session-tabs] [data-tab-capsule]').getAttribute('data-tabs-held'),null,'new session left close widths frozen');
  assert((await page.locator('[data-session-tabs] [data-tab]').last().getAttribute('data-tab')).startsWith('new-tab:'));
  await page.screenshot({path:join(scratch,'all-long-titles.png')});
  const draftClose=page.locator('[data-session-tabs] [data-tab][data-on=true] [data-tab-close]');
  await draftClose.focus();await page.keyboard.press('Space');
  await until(async()=>await page.locator('[data-session-tabs] [data-tab]').count()===sessions.length-2);
  console.log('PASS: six sessions, long titles, 1000px, Enter/Space focus, list following, unchanged pane geometry, shared short dividers, original order, close and outer +');
  }
  if(process.env.CONTENT_DESKTOP_ICONS_ONLY==='1'){assert.deepEqual(errors,[]);console.log('desktop content gate: clean');break checks;}
  // Narrow stays on the existing terminal/deck layout, with no desktop group.
  await page.setViewportSize({width:390,height:844});await page.locator('[data-session-tabs]').waitFor({state:'detached'});await page.locator('[data-terminal]').first().waitFor();
  assert.equal(await page.locator('[data-session-tabs]').count(),0);
  assert.equal(await page.locator('[data-session-side]').count(),0);
  assert(!await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth));
  const picker=()=>page.locator(`[data-content-picker="${sessions[0]}"]`);
  const mobileTable=async()=> (await table()).tabs.filter(t=>t.session===sessions[0]);
  const total=n=>page.getByRole('button',{name:word('content-tabs-count').replace('{0}',String(n)),exact:true});
  await page.waitForFunction(()=>document.querySelector('[data-terminal]')?.__xterm);
  await page.evaluate(()=>{const t=document.querySelector('[data-terminal]').__xterm;window.__contentTerm=t;window.__contentSize=[t.cols,t.rows];});
  await page.locator('[data-deck-title]:visible').click();await picker().waitFor({state:'visible'});if(!await picker().locator('[data-file-tree]').isVisible())await source(page,picker(),'workspace');const resumedFind=await field(page,picker(),'[data-content-deck] input[type=search]:visible');assert.equal(await resumedFind.inputValue(),'only-0','narrow picker lost its latest workspace query');await resumedFind.fill('');await resumedFind.blur();await picker().locator('[data-file-tree]').waitFor();await total(0).waitFor();
  assert.equal((await mobileTable()).length,0,'picker created a persistent tab');await browse(page);assert.equal((await mobileTable()).length,0,'+ created an empty tab');
  assert.equal(await page.locator('[data-content-panel] .session-content-tools').count(),0);
  assert(await page.evaluate(()=>document.querySelector('[data-terminal]').__xterm===window.__contentTerm));
  assert.deepEqual(await page.evaluate(()=>{const t=document.querySelector('[data-terminal]').__xterm;return [t.cols,t.rows];}),await page.evaluate(()=>window.__contentSize),'content refit terminal');
  assert.equal(await page.locator('[data-content-panel]').getAttribute('data-detent'),'full','picker editing did not expand sheet');await page.locator('[data-place-grab]').click();await page.locator('[data-content-panel][data-detent=half]').waitFor();
  const mouseGrip=await page.locator('[data-place-grab]').boundingBox();
  const mousePanel=()=>page.locator('[data-content-panel]');
  const mouseHeight=await mousePanel().evaluate(el=>el.getBoundingClientRect().height);
  await page.mouse.move(mouseGrip.x+mouseGrip.width/2,mouseGrip.y+mouseGrip.height/2);await page.mouse.down();
  for(const distance of [24,50,85]) {
    await page.mouse.move(mouseGrip.x+mouseGrip.width/2,mouseGrip.y+mouseGrip.height/2-distance);
    assert(Math.abs(await mousePanel().evaluate(el=>el.getBoundingClientRect().height)-mouseHeight-distance)<2,'sheet did not follow the mouse before release');
  }
  await page.mouse.up();await mousePanel().locator('[data-place-grab]').waitFor();
  await page.screenshot({path:join(scratch,'mobile-picker.png')});
  const find=()=>picker().locator('[data-content-deck] input[type=search]:visible');
  async function field(host,where,selector){
    const input=where.locator(selector);
    if(!await input.isVisible())await where.locator('[data-content-deck] [data-deck-band]:visible').click();
    await input.waitFor({state:'visible'});return input;
  }
  const fillFind=async(host,where,value)=>{const input=await field(host,where,'[data-content-deck] input[type=search]:visible');await input.fill(value);};
  const findValue=async(host,where)=>(await field(host,where,'[data-content-deck] input[type=search]:visible')).inputValue();
  async function source(host,where,value){
    const end=where.locator('[data-content-edit-end]:visible');if(await end.isVisible())await end.click();
    await where.locator('[data-content-scope]:visible').click();await host.locator('[data-content-source='+value+']').click();
    assert(!await host.evaluate(()=>document.activeElement?.matches('input,textarea')),'source switch focused an input');
  }
  const fileAction=async(host,document,name)=>{if(await document.locator('[data-deck-strip]').isVisible())await document.locator('[data-deck-strip]').click();await document.locator('[data-file-more]').click();await host.getByRole('menuitem',{name,exact:true}).click();};
  async function browse(host){const document=host.locator('[data-session-content][data-content-visible=true]');if(await document.locator('[data-deck-strip]').isVisible())await document.locator('[data-deck-strip]').click();const overview=host.locator('[data-content-overview]:not(.session-content-retained)');if(!await overview.isVisible())await host.locator('[data-content-overview-button]:visible').click();await overview.getByRole('button',{name:word('content-find'),exact:true}).click();};
  await fillFind(page,picker(),'only-0');await picker().getByText('only-0.txt',{exact:true}).click();await shown().locator('.cm-content').waitFor();await total(1).waitFor();
  await shown().locator('[data-file-deck]').waitFor();
  assert.equal(await page.locator('[data-session-bar]:visible').count(),0,'terminal deck remained visible behind the file');
  assert.equal(await page.locator('[data-content-overview-button]:visible').count(),1,'content has duplicate tab buttons');
  assert.equal(await shown().locator('[data-band-seg=cur] .content-owner-face [data-side-face]').count(),1,'file deck lost the session identity');
  assert.equal(await shown().locator('[data-file-embedded-tools]').count(),0,'phone used desktop embedded tools instead of the file deck');
  assert.equal(await shown().locator('.mobile-content-document-head').count(),0,'file still has a second top toolbar');
  const fileBox=await shown().locator('[data-file-page]').boundingBox(),fileDeck=await shown().locator('[data-file-deck]').boundingBox();
  assert(Math.abs(fileBox.y+fileBox.height-fileDeck.y-fileDeck.height)<2,'file deck is not on its page bottom');
  await shown().locator('.mobile-file-identity').click({trial:true});await shown().locator('[data-file-more]').click({trial:true});
  await page.screenshot({path:join(scratch,'mobile-file-deck.png')});
  await shown().locator('[data-file-back]').click();assert.equal(await findValue(page,picker()),'only-0');
  await fillFind(page,picker(),'notes.md');await picker().getByText('notes.md',{exact:true}).click();await shown().locator('.cm-content').waitFor();
  const handle=await shown().getAttribute('data-content-handle');await page.evaluate(()=>window.__keptEditor=document.querySelector('[data-content-visible=true] .cm-content'));await until(async()=>(await mobileTable()).length===2);
  await total(2).waitFor();
  assert.equal(await shown().getAttribute('data-content-handle'),handle);assert(await page.evaluate(()=>document.querySelector('[data-content-visible=true] .cm-content')===window.__keptEditor),'tab acknowledgement remounted editor');
  assert((await mobileTable()).every(t=>t.view==='file'));assert.notEqual((await mobileTable()).find(t=>t.url.endsWith('notes.md')).id,handle);
  await browse(page);await fillFind(page,picker(),'only-0');await picker().getByText('only-0.txt',{exact:true}).click();await shown().locator('.cm-content').waitFor();
  await page.evaluate(()=>window.__editingPreview=document.querySelector('[data-content-visible=true] .cm-content'));
  await shown().locator('.cm-content').click();await page.keyboard.press('ControlOrMeta+End');await page.keyboard.type('\nMobile document edit');if(await shown().locator('[data-deck-strip]').isVisible())await shown().locator('[data-deck-strip]').click();await total(2).waitFor();
  await until(()=>readFileSync(join(projects[0],'only-0.txt'),'utf8').includes('Mobile document edit'));
  assert(await page.evaluate(()=>document.querySelector('[data-content-visible=true] .cm-content')===window.__editingPreview),'editing remounted editor');
  await fileAction(page,shown(),word('content-previous'));await until(async()=> (await shown().innerText()).includes('# Alpha'));
  await fileAction(page,shown(),word('content-previous'));await until(async()=> (await shown().innerText()).includes('Neighbor'));
  await until(async()=>(await mobileTable()).length===3);
  assert.equal((await mobileTable()).filter(t=>t.url.endsWith('notes.md')).length,1,'neighbor browsing duplicated a tab');
  await total(3).click();const overview=()=>page.locator('[data-content-overview]:not(.session-content-retained)');
  assert.equal(await overview().locator('[data-temporary=true]').count(),0);
  assert.equal(await overview().locator('[data-tab-sheet-new]').innerText(),word('content-find'));
  assert(await overview().locator('[data-tab-sheet-new]').evaluate(el=>el===el.parentElement.lastElementChild),'open action is not the last row');
  assert.equal(await overview().locator('[data-tab-sheet-close] svg').first().evaluate(el=>el.getBoundingClientRect().width),16,'content close icon differs from TabSheet');
  assert.equal(await overview().locator('[data-tab-sheet-row][data-on=true] button:first-child>svg').count(),0,'current tab has a redundant check');assert.equal(await overview().locator('[data-content-overview-tab]').count(),3);
  await overview().locator('[data-content-overview-search]').click();await overview().getByRole('searchbox',{name:word('content-tabs-search'),exact:true}).fill('notes.md');assert.equal(await overview().locator('[data-content-overview-tab]').count(),1);
  await overview().locator('[data-tab-sheet-close]').click();await until(async()=>(await mobileTable()).length===2);
  await browse(page);await fillFind(page,picker(),'');await picker().locator('[data-file-tree]').waitFor();
  if(await page.locator('[data-content-panel]').getAttribute('data-detent')==='full')await page.locator('[data-place-grab]').click();
  const tree=picker().locator('[data-side-workspace] > div:first-child');await picker().locator('[data-file="extra-15.txt"]').scrollIntoViewIfNeeded();
  const scroll=await tree.evaluate(el=>{window.__originalTree=el;return el.scrollTop;});assert(scroll>0);
  await picker().locator('[data-file="extra-15.txt"]').click();await shown().locator('.cm-content').waitFor();
  await shown().locator('[data-file-scroll]').evaluate(el=>{el.scrollTop=el.scrollHeight;el.dispatchEvent(new Event('scroll',{bubbles:true}));});
  await shown().locator('[data-deck-strip]').waitFor();
  const foldedFace=await shown().locator('[data-deck-strip]').evaluate(el=>{const f=el.querySelector('[data-side-face]'),halo=f.querySelector('[data-halo]'),r=f.getBoundingClientRect(),box=el.getBoundingClientRect(),scale=new DOMMatrix(getComputedStyle(f).transform).a,out=-parseFloat(getComputedStyle(halo,'::before').top)*scale;return {top:r.top-out-box.top,bottom:box.bottom-r.bottom-out};});
  assert(foldedFace.top>=3.9&&foldedFace.bottom>=3.9,'folded avatar touches capsule edge: '+JSON.stringify(foldedFace));
  // Scrolling a folded deck into view changes the scroll-owned fold.
  // Hit its already-painted center, as the person's tap does.
  const fold=await shown().locator('[data-deck-strip]').evaluate(el=>{const r=el.getBoundingClientRect(),x=r.x+r.width/2,y=r.y+r.height/2;return {x,y,hit:el.contains(document.elementFromPoint(x,y))};});
  assert(fold.hit,'folded deck is not hittable');await page.mouse.click(fold.x,fold.y);await shown().locator('[data-file-back]').click();
  assert.equal(await tree.evaluate(el=>el.scrollTop),scroll);assert(await tree.evaluate(el=>el===window.__originalTree));
  await picker().getByRole('button',{name:word('content-workspace-actions'),exact:true}).click();await page.getByRole('menuitem',{name:word('file-new'),exact:true}).click();
  await picker().locator('[data-new-name=file] input').fill('mobile-created.txt');await picker().locator('[data-new-name=file] input').press('Enter');await picker().locator('[data-file="mobile-created.txt"]').waitFor();
  assert.equal(readFileSync(join(projects[0],'mobile-created.txt'),'utf8'),'');
  await source(page,picker(),'browser');const addr=()=>picker().locator('[data-page-address]');
  await (await field(page,picker(),'[data-page-address]')).fill('https://example.com');await source(page,picker(),'workspace');await source(page,picker(),'browser');await field(page,picker(),'[data-page-address]');
  await addr().focus();assert.equal(await addr().inputValue(),'https://example.com');await addr().press('Enter');await shown().locator('[data-content-page-more]').waitFor();await until(async()=>(await mobileTable()).length===4);
  const pageHandle=await shown().getAttribute('data-content-handle');await total(4).waitFor();assert.equal(await shown().getAttribute('data-content-handle'),pageHandle);
  await page.setViewportSize({width:1200,height:900});await side(sessions[0]).waitFor();assert.equal(await shown().getAttribute('data-content-handle'),pageHandle);
  await page.setViewportSize({width:320,height:740});await total(4).waitFor();assert(!await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth));const deck=await total(4).boundingBox();assert(deck.x>=0&&deck.x+deck.width<=320);
  await total(4).click();await page.screenshot({path:join(scratch,'mobile-overview.png')});
  await page.setViewportSize({width:1200,height:900});await overview().waitFor();assert.equal(await shown().count(),0,'resizing replaced the latest overview with a document');
  await page.setViewportSize({width:320,height:740});await overview().waitFor();
  await page.locator('.mobile-content-scrim').click({position:{x:12,y:12}});await page.locator('[data-deck-title]:visible').click();await overview().waitFor();
  await page.reload();await page.locator('[data-session-title]').filter({hasText:/^Content Alpha$/}).click();await page.locator('[data-deck-title]:visible').click();await overview().waitFor();assert.equal((await mobileTable()).length,4);assert.equal(await page.locator('[data-content-preview]').count(),0);
  const phoneContext=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true,reducedMotion:'reduce'});
  await phoneContext.addInitScript(ids=>{if(!localStorage.getItem('khor.sessions.tabs'))localStorage.setItem('khor.sessions.tabs',JSON.stringify({tabs:ids,active:ids[0]}));localStorage.setItem('khor.place',JSON.stringify({landing:'sessions',screen:'detail'}));localStorage.setItem('khor.lang','zh');},sessions);
  const phone=await phoneContext.newPage();phone.setDefaultTimeout(12000);phone.on('pageerror',e=>errors.push(e.message));await phone.goto(url);
  await phone.locator('[data-session-title]').filter({hasText:/^Content Alpha$/}).click();await phone.locator('[data-deck-title]:visible').click();await phone.locator('[data-content-panel]').waitFor();
  assert.equal(await phone.getByRole('button',{name:word('content-expand'),exact:true}).count(),0);
  // Raw CDP touch input needs the painted hit region, not just a DOM box.
  await phone.locator('[data-place-grab]').click({trial:true});
  const touch=await phoneContext.newCDPSession(phone),grip=await phone.locator('[data-place-grab]').boundingBox(),x=grip.x+grip.width/2,y=grip.y+grip.height/2;
    const touchHeight=await phone.locator('[data-content-panel]').evaluate(el=>el.getBoundingClientRect().height);
  await touch.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x,y}]});
  for(const distance of [30,90,160,280]) {
    await touch.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x,y:y-distance}]});
    const actual=await phone.locator('[data-content-panel]').evaluate(el=>el.getBoundingClientRect().height);
    assert(Math.abs(actual-touchHeight-distance)<2,'sheet did not follow the finger before release');
  }
  await touch.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});
  await phone.locator('[data-content-panel][data-detent=full]').waitFor();assert.equal(await phone.locator('[data-content-panel]').evaluate(el=>el.style.height),'');
  const fullGrip=await phone.locator('[data-place-grab]').boundingBox(),fy=fullGrip.y+fullGrip.height/2;
  await touch.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x,y:fy}]});await touch.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x,y:fy+110}]});await touch.send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]});
  assert.equal(await phone.locator('[data-content-panel]').getAttribute('data-sheet-pulling'),null);assert.equal(await phone.locator('[data-content-panel]').evaluate(el=>el.style.height),'');
  const pillBox=await phone.locator('[data-content-deck] .mobile-content-pill:visible').boundingBox(),px=pillBox.x+pillBox.width/2,py=pillBox.y+pillBox.height/2;
  const beforePill=await phone.locator('[data-content-panel]').evaluate(el=>el.getBoundingClientRect().height);
  await touch.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:px,y:py}]});
  await touch.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:px,y:py+95}]});
  const afterPill=await phone.locator('[data-content-panel]').evaluate(el=>el.getBoundingClientRect().height);
  assert(Math.abs(beforePill-afterPill-95)<2,'sheet did not track the expanded bottom pill');
  await touch.send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]});
  await browse(phone);await source(phone,phone.locator(`[data-content-picker="${sessions[0]}"]`),'workspace');await phone.locator('[data-place-grab]').click();
  // A pan in the workspace body, away from the grabber and deck, owns
  // the sheet until its full detent. Exercise the painted touch surface.
  const bodyTree=phone.locator(`[data-content-picker="${sessions[0]}"] [data-side-workspace] > div:first-child`);
  await bodyTree.evaluate(el=>{el.scrollTop=0;window.getSelection()?.removeAllRanges();});
  const bodyBox=await bodyTree.boundingBox(),bx=bodyBox.x+bodyBox.width/2,by=bodyBox.y+bodyBox.height-40;
  const bodyHeight=await phone.locator('[data-content-panel]').evaluate(el=>el.getBoundingClientRect().height);
  await touch.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:bx,y:by}]});
  await touch.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:bx,y:by-95}]});
  const bodyAfter=await phone.locator('[data-content-panel]').evaluate(el=>el.getBoundingClientRect().height);assert(Math.abs(bodyAfter-bodyHeight-95)<2,'content body did not resize its sheet: '+JSON.stringify({bodyBox,bx,by,bodyHeight,bodyAfter}));
  await touch.send('Input.dispatchTouchEvent',{type:'touchCancel',touchPoints:[]});
  await phone.screenshot({path:join(scratch,'phone-content.png')});
  await phone.locator('.mobile-content-scrim').click({position:{x:12,y:12}});await phone.locator('[data-deck-tabs]').click();await phone.locator('[data-content-sessions]').getByText('Content Beta',{exact:true}).click();await phone.locator('[data-content-overview]:not(.session-content-retained)').getByRole('button',{name:word('content-find'),exact:true}).click();
  await phone.locator(`[data-content-picker="${sessions[1]}"] [data-file-tree]`).waitFor();assert.equal(await findValue(phone,phone.locator(`[data-content-picker="${sessions[1]}"]`)),'');
  // A delayed first open must not take focus from a newer document.
  const beta=()=>phone.locator(`[data-content-picker="${sessions[1]}"]`),betaDoc=()=>phone.locator('[data-content-visible=true]');
  let releaseOpen;const delayedOpen=new Promise(resolve=>releaseOpen=resolve);
  await phone.route('**/tabs_open',async route=>{await delayedOpen;await route.continue();},{times:1});
  await fillFind(phone,beta(),'notes.md');await beta().getByText('notes.md',{exact:true}).click();await betaDoc().locator('.cm-content').waitFor();
  await betaDoc().locator('[data-file-back]').click();await fillFind(phone,beta(),'only-1');await beta().getByText('only-1.txt',{exact:true}).click();await betaDoc().locator('.cm-content').waitFor();
  releaseOpen();await until(async()=>(await table()).tabs.filter(t=>t.session===sessions[1]).length===2);
  assert((await betaDoc().locator('.cm-content').innerText()).includes('This file belongs to 1'),'slow open stole focus from the newer document');
  // Closing during the initial open waits for the node id, then closes it.
  await betaDoc().locator('[data-file-back]').click();await fillFind(phone,beta(),'extra-03');
  let releaseClose;const delayedClose=new Promise(resolve=>releaseClose=resolve);
  await phone.route('**/tabs_open',async route=>{await delayedClose;await route.continue();},{times:1});
  await beta().getByText('extra-03.txt',{exact:true}).click();await betaDoc().locator('.cm-content').waitFor();
  await phone.locator('[data-content-overview-button]:visible').click();
  const pendingOverview=phone.locator('[data-content-overview]:not(.session-content-retained)');
  await pendingOverview.locator('[data-content-overview-search]').click();await pendingOverview.getByRole('searchbox').fill('extra-03.txt');
  const closed=phone.waitForResponse(r=>r.url().endsWith('/tabs_close')&&r.request().method()==='POST');
  await pendingOverview.locator('[data-content-overview-tab] [data-tab-sheet-close]').click();releaseClose();await closed;
  await until(async()=>!(await table()).tabs.some(t=>t.session===sessions[1]&&t.url.endsWith('extra-03.txt')));
  await phoneContext.close();
  assert.deepEqual(errors,[]);
  console.log('PASS: delayed open focus and pending close; mobile picker and neighbor browsing; stable editors and PTY size; immediate tabs and duplicate reuse; autosave; overview search/close; directory scroll; new-file menu; URL drafts; width changes and reload');
  console.log('session content gate: clean');
}}catch(e){await page?.screenshot({path:join(scratch,'failed.png')}).catch(()=>{});console.error(e);throw e;}
finally{
  for(const id of sessions)await api('close_session',{id}).catch(()=>{});
  await browser?.close().catch(()=>{});
  for(const child of children)try{process.kill(-child.pid,'SIGKILL');}catch{}
  // Keep screenshots and fixture state, not a full binary pair per run.
  for(const name of ['bridge','khor'])rmSync(join(scratch,name),{force:true});
  await usage?.close();
}
