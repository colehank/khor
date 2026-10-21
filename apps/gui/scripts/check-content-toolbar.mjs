// Real App + private node/PTYs. Transport is the existing dev bridge;
// native page overlay/borrow rendering requires desktop app acceptance.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdirSync,writeFileSync,readFileSync,copyFileSync,rmSync,realpathSync} from 'node:fs';
import {join,resolve} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright-core';
import {fakeHome,fakeUsage} from './fake-home.mjs';
const gui=process.env.CONTENT_UI_ROOT||fileURLToPath(new URL('..',import.meta.url));
const repo=resolve(gui,'../..');
const scratch=process.env.CONTENT_CHECK_DIR||join(tmpdir(),'khor-content-toolbar-check');
const target=process.env.CARGO_TARGET_DIR||join(repo,'target');
const port=Number(process.env.CONTENT_VITE_PORT||23840), bridgePort=Number(process.env.CONTENT_BRIDGE_PORT||23841);
const word=key=>readFileSync(join(repo,'crates/catalog/zh.toml'),'utf8').match(new RegExp('^'+key+' = "([^"]*)"','m'))?.[1];
const children=[],sessions=[];
let browser,usage,page,vite;
const until=async(fn,ms=20000)=>{const end=Date.now()+ms;let last;do{const failed=children.find(c=>c.exitCode!==null&&c.exitCode!==0);if(failed)throw Error('Private test service exited: '+failed.exitCode);try{last=await fn();if(last)return last;}catch(e){last=e;}await new Promise(r=>setTimeout(r,100));}while(Date.now()<end);throw Error('Timed out: '+last);};
const run=(cmd,args,env,cwd=gui)=>{const c=spawn(cmd,args,{env,cwd,detached:true,stdio:['ignore','pipe','pipe']});children.push(c);c.stderr.on('data',d=>process.stderr.write(d));return c;};
const api=async(cmd,args={})=>{const r=await fetch(`http://127.0.0.1:${bridgePort}/${cmd}`,{signal:AbortSignal.timeout(5000),method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(args)});const value=await r.json();if(!r.ok)throw Error(JSON.stringify(value));return value;};
try{
 rmSync(scratch,{recursive:true,force:true});mkdirSync(scratch,{recursive:true});usage=await fakeUsage();
 const env={...process.env,...fakeHome(join(scratch,'home'),usage.url),KHOR_HOME:join(scratch,'node'),KHOR_NAME:'toolbar-check',KHOR_NO_MDNS:'1',BRIDGE_PORT:String(bridgePort)};delete env.KHOR_SESSION;
 for(const n of ['bridge','khor'])copyFileSync(join(target,'debug',n),join(scratch,n));run(join(scratch,'bridge'),[],env);await until(()=>api('devices').then(r=>r.length));
 const device=(await api('devices')).find(d=>d.me),rootAt=join(scratch,'project');mkdirSync(rootAt,{recursive:true});const root=realpathSync(rootAt);writeFileSync(join(root,'notes.md'),'# Notes\n\nKept content.');mkdirSync(join(root,'child'));writeFileSync(join(root,'child','second.txt'),'Second file');mkdirSync(join(root,'child-renamed-other'));writeFileSync(join(root,'child-renamed-other','keep.txt'),'Kept sibling');
 const id=await api('open_on',{machine:'',title:'Toolbar checks',cwd:root,cols:100,rows:35});sessions.push(id);await api('recent_visit',{kind:'page',machine:device.name,place:usage.url});
 // A test owns its optimizer cache. Shared node_modules is linked across
 // worktrees; a different Lab's dependency scan must not reload this App.
 process.env.KHOR_FACE_GATE='1';if(process.env.CONTENT_TRACE_RELOAD)process.env.DEBUG='vite:deps,vite:hmr';
 const {createServer}=await import('vite');
 vite=await createServer({root:gui,cacheDir:join(scratch,'vite-cache'),optimizeDeps:{entries:[join(gui,'index.html')]},server:{port,strictPort:true},clearScreen:false});
 await vite.listen();await until(()=>fetch(`http://localhost:${port}/`).then(r=>r.ok));
 browser=await chromium.launch({channel:'chrome',headless:process.env.CONTENT_HEADED!=='1',args:['--no-proxy-server']});const context=await browser.newContext({viewport:{width:390,height:844},hasTouch:true,isMobile:true});
 await context.addInitScript(ids=>{localStorage.setItem('khor.sessions.tabs',JSON.stringify({tabs:ids,active:ids[0]}));localStorage.setItem('khor.place',JSON.stringify({landing:'sessions',screen:'detail'}));localStorage.setItem('khor.lang','zh');},sessions);
 page=await context.newPage();page.setDefaultTimeout(10000);const errors=[];page.on('pageerror',e=>errors.push(e.message));if(process.env.CONTENT_TRACE_RELOAD){page.on('console',m=>console.log('browser',m.type(),m.text()));page.on('framenavigated',f=>{if(f===page.mainFrame())console.log('navigation',f.url());});}
 const remote={...device,id:'a'.repeat(64),name:'toolbar-remote',shown:'远端 A',me:false,hop:'direct',paused:false};await page.route('**/devices',r=>r.fulfill({json:[device,remote]}));
 await page.goto(`http://localhost:${port}/?bridge=${bridgePort}`,{waitUntil:'domcontentloaded',timeout:15000});await page.locator('[data-session-title]').filter({hasText:/^Toolbar checks$/}).click();await page.locator('[data-session-deck] [data-deck-title]').click();
 const picker=()=>page.locator('[data-content-picker]:visible'),shown=()=>page.locator('[data-session-content][data-content-visible=true]');
 const settle=()=>page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>a.effect?.getComputedTiming().iterations!==Infinity).map(a=>a.finished.catch(()=>{}))));
 await picker().getByText('notes.md',{exact:true}).waitFor();await settle();assert.equal(await picker().locator('[data-content-scope-segmented]').count(),0,'mobile keeps compact source control');assert.equal(await picker().locator('[data-content-scope] svg').count(),2,'mobile source icon keeps its disclosure chevron');
 assert.equal(await picker().locator('input:visible').count(),0);assert.equal((await picker().locator('[data-band-seg=cur]').innerText()).trim(),word('content-new'));assert.equal(await picker().locator('[data-band-seg=cur] [data-side-mark=new-tab]').count(),1);
 const align=await picker().locator('[data-band-seg=cur] .content-new-identity').evaluate(el=>{const a=el.querySelector('.content-owner-face').getBoundingClientRect(),t=el.querySelector('.content-new-title').getBoundingClientRect(),g=el.querySelector('[data-side-mark] svg').getBoundingClientRect();return {delta:Math.abs(a.y+a.height/2-t.y-t.height/2),glyph:g.width,face:a.width};});assert(align.delta<.5&&align.glyph<align.face,'new tab identity alignment');
 const panel=page.locator('[data-content-panel]'),before=await panel.boundingBox();await picker().locator('[data-content-directory-search]').click();await picker().getByRole('searchbox').fill('notes');await settle();assert(Math.abs((await panel.boundingBox()).height-before.height)<1,'search expanded sheet');
 await picker().getByRole('button',{name:word('back'),exact:true}).click();await picker().locator('[data-content-directory-new]').click();await page.getByRole('menuitem',{name:word('dir-new'),exact:true}).waitFor();await page.keyboard.press('Escape');assert.equal(await picker().locator('[data-glass-capsule] [data-glass-item]').count(),2);
 await picker().locator('[data-band-seg=cur] button').tap();await picker().locator('[data-content-path-editor] input').waitFor();assert.equal(await picker().locator('[data-content-path-editor] input').inputValue(),root);
 await picker().locator('[data-content-path-machine]').click();await picker().getByText(remote.shown,{exact:true}).waitFor();await picker().locator('[data-content-edit-end]').click();await picker().locator('[data-deck-band]').waitFor();console.log('PASS directory search keeps height, search/new group, single tap location and machine choice');
 const blank=(await api('tabs_list',{kind:'content'})).tabs.find(t=>t.session===id),second=await api('tabs_open',{kind:'content',session:id,device:device.id,url:join(root,'notes.md'),view:'file'}),third=await api('tabs_open',{kind:'content',session:id,device:device.id,url:usage.url,view:'page'});await api('tabs_focus',{kind:'content',id:blank.id});
 await until(async()=>await picker().locator('[data-content-overview-button]').innerText()==='3');await settle();const touch=await context.newCDPSession(page),box=await picker().locator('[data-deck-band]').boundingBox(),q={x:box.x+box.width*.6,y:box.y+box.height/2};
 await touch.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[q]});await page.locator('[data-deck-selector]').waitFor();assert.equal(await picker().locator('input:visible').count(),0);assert.equal(await page.locator('.deck-selector-wheel').count(),1);await touch.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:q.x,y:q.y-140}]});assert.equal(await page.locator('[data-deck-selector]').count(),1);await touch.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await page.locator('[data-deck-selector]').waitFor({state:'detached'});assert.equal(await picker().locator('input:visible').count(),0);console.log('PASS hold uses formal arc, stays owned outside pill, release does not edit');
 await settle();const swipe=await picker().locator('[data-deck-band]').boundingBox(),start={x:swipe.x+swipe.width*.8,y:swipe.y+swipe.height/2};
 await touch.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[start]});for(const t of [.2,.5,.8,1])await touch.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:start.x-swipe.width*.65*t,y:start.y}]});assert.equal(await shown().count(),0,'preview opened an intermediate document');await touch.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await shown().locator('.cm-content').waitFor();assert.equal(await shown().getAttribute('data-session-content'),second);assert.equal(await shown().locator('[data-content-path-editor]').count(),0,'swipe opened editor');assert.equal(await shown().locator('[data-file-back]').count(),0);console.log('PASS touch swipe previews without opening and commits only the final content');
 const handle=await shown().getAttribute('data-content-handle');await shown().locator('.cm-content').evaluate(el=>el.dataset.retained='yes');await shown().locator('[data-band-seg=cur] button').tap();await shown().locator('[data-content-path-editor]').waitFor();assert.equal(await shown().getAttribute('data-content-handle'),handle);assert.equal(await shown().locator('.cm-content').getAttribute('data-retained'),'yes');await shown().locator('.cm-content').click();await shown().locator('[data-deck-band]').waitFor();
 await shown().locator('[data-content-overview-button]').click();await page.locator(`[data-tab-sheet-open="${third}"]`).click();await shown().locator('[data-deck-band]').waitFor();await shown().locator('[data-band-seg=cur] button').tap();await shown().locator('[data-page-address]').waitFor();await page.locator('[data-page-list] [data-omni-menu]').waitFor();assert(await page.locator('[data-page-list] [data-omni-item]').count()>0,'opened webpage address editor offers recent addresses');await shown().locator('[data-content-edit-end]').click();await shown().locator('[data-content-page-more]').click();await page.getByRole('menuitem',{name:word('page-back'),exact:true}).waitFor();await page.getByRole('menuitem',{name:word('close-tab'),exact:true}).waitFor();await page.keyboard.press('Escape');console.log('PASS file editor is retained, webpage edits directly, native history stays in menu');
 await shown().locator('[data-content-overview-button]').click();await page.locator('[data-tab-sheet-new]').click();await picker().locator('[data-content-scope]').click();await page.locator('[data-content-source=browser]').click();assert.equal(await picker().locator('input:visible').count(),0);assert.equal(await picker().locator('[data-content-directory-new]').count(),0);await picker().locator('[data-band-seg=cur] button').tap();await picker().locator('[data-page-address]').waitFor();assert.equal(await page.locator('[data-page-list] [data-omni-menu]').count(),0);await picker().locator('[data-content-edit-end]').click();await page.screenshot({path:join(scratch,'mobile.png'),timeout:5000});
 const wide=await browser.newContext({viewport:{width:1500,height:950}});await wide.addInitScript(session=>{localStorage.setItem('khor.sessions.tabs',JSON.stringify({tabs:[session],active:session}));localStorage.setItem('khor.place',JSON.stringify({landing:'sessions',screen:'detail'}));localStorage.setItem('khor.lang','zh');},id);page=await wide.newPage();page.setDefaultTimeout(10000);page.on('pageerror',e=>errors.push(e.message));if(process.env.CONTENT_TRACE_RELOAD){page.on('console',m=>console.log('browser',m.type(),m.text()));page.on('framenavigated',f=>{if(f===page.mainFrame())console.log('navigation',f.url());});}await page.goto(`http://localhost:${port}/?bridge=${bridgePort}`,{waitUntil:'domcontentloaded',timeout:15000});await page.locator('[data-session-tabs]').waitFor();const parent=page.locator(`[data-session-tabs] [data-tab-open="${id}"]`).first();if(!await picker().isVisible())await parent.click();await picker().waitFor();assert.equal(await page.locator('[data-content-maximize],[data-content-hide],[data-session-verbs]').count(),0);await parent.click({button:'right'});await page.getByRole('menuitem',{name:word('term-reload'),exact:true}).waitFor();await page.keyboard.press('Escape');const sourceControl=picker().locator('[data-content-scope-segmented]');await sourceControl.waitFor();await sourceControl.getByRole('radio').first().waitFor();assert.equal(await sourceControl.getByRole('radio').count(),2);assert.equal(await sourceControl.locator('svg').count(),2,'desktop source segments use the shared icons');await sourceControl.locator('[data-content-source=workspace]').click();await page.keyboard.press('ArrowRight');assert.equal(await sourceControl.locator('[data-content-source=browser]').getAttribute('aria-checked'),'true');await page.keyboard.press('ArrowLeft');assert.equal(await sourceControl.locator('[data-content-source=workspace]').getAttribute('aria-checked'),'true');await picker().locator('[data-content-location-edit]').click();await picker().locator('[data-content-path-editor] input').waitFor();await page.screenshot({path:join(scratch,'desktop.png'),timeout:5000});console.log('PASS desktop toolbar, reconnect menu, no parent action clutter');
 // A directory is a filesystem object, and every open descendant is a
 // session tab. Rename keeps its identity; deletion closes only actual
 // descendants, never another directory that merely shares the prefix.
 await picker().locator('[data-content-path-editor] input').press('Escape');
 if(await picker().locator('[data-content-path-editor] input').isVisible())await picker().locator('[data-content-path-editor] input').press('Escape');
 const nestedPath=join(root,'child','second.txt'),siblingPath=join(root,'child-renamed-other','keep.txt');
 const nested=await api('tabs_open',{kind:'content',session:id,device:device.id,url:nestedPath,view:'file'});
 const sibling=await api('tabs_open',{kind:'content',session:id,device:device.id,url:siblingPath,view:'file'});
 await api('tabs_focus',{kind:'content',id:nested});
 await until(async()=>await shown().getAttribute('data-session-content')===nested);
 await shown().locator('.cm-content').waitFor();
 await shown().locator('[data-file-more]').click();await page.locator('[data-file-more-menu]').waitFor();
 assert.equal(await page.locator('[data-file-more-item="content-previous"],[data-file-more-item="content-next"],[data-file-more-item="rename"],[data-file-more-item="remove"]').count(),0);
 await page.locator('[data-file-more-item="locate"]').click();
 await picker().locator('[data-file="second.txt"]').waitFor();
 await until(()=>page.evaluate(()=>document.activeElement?.getAttribute('data-file-path')) .then(path=>path===nestedPath));
 // Locate starts at the file's parent. The location editor deliberately
 // returns to the containing project to manage its child directory.
 await picker().locator('[data-content-location-edit]').click();
 await picker().locator('[data-content-path-editor] input').fill(root);
 await picker().locator('[data-content-path-editor] input').press('Enter');
 await picker().locator('[data-dir="child"]').waitFor();
 const treeMenu=async(name,key)=>{await page.locator('[data-row-menu]').waitFor({state:'detached'});await picker().locator('[data-dir="'+name+'"]').click({button:'right'});await page.locator('[data-menu-item="'+key+'"]').click();};
 await treeMenu('child','rename');
 await picker().locator('[data-title-input]').fill('child-renamed');
 await picker().locator('[data-title-input]').press('Enter');
 const movedPath=join(root,'child-renamed','second.txt');
 await until(async()=>{const tabs=(await api('tabs_list',{kind:'content'})).tabs;return tabs.some(t=>t.id===nested&&t.url===movedPath)&&tabs.some(t=>t.id===sibling&&t.url===siblingPath);});
 const afterRename=(await api('tabs_list',{kind:'content'})).tabs;
 await picker().locator('[data-dir="child-renamed"]').waitFor();
 await treeMenu('child-renamed','remove');
 await page.locator('[data-remove-dialog]').waitFor();await page.locator('[data-remove-go]').click();
 await until(async()=>{const tabs=(await api('tabs_list',{kind:'content'})).tabs;return !tabs.some(t=>t.id===nested)&&tabs.some(t=>t.id===sibling&&t.url===siblingPath);});
 const afterDelete=(await api('tabs_list',{kind:'content'})).tabs;
 assert(afterDelete.some(t=>t.id===sibling&&t.url===siblingPath),'similarly prefixed sibling tab was closed');
 writeFileSync(join(scratch,'file-lifecycle.json'),JSON.stringify({nested,sibling,nestedPath,movedPath,siblingPath,afterRename,afterDelete},null,2));
 await page.screenshot({path:join(scratch,'file-lifecycle.png'),timeout:5000});
 assert.deepEqual(errors,[]);console.log('PASS real session host: locate file, rename directory keeps tab id and updates path, delete closes descendants and retains prefix sibling');console.log('content toolbar: clean',scratch);
}catch(e){console.error(e);await page?.screenshot({path:join(scratch,'failed.png'),timeout:5000}).catch(()=>{});throw e;}
finally{for(const id of sessions)await api('close_session',{id}).catch(()=>{});await browser?.close().catch(()=>{});await vite?.close().catch(()=>{});for(const child of children)try{process.kill(-child.pid,'SIGKILL');}catch{};for(const n of ['bridge','khor'])rmSync(join(scratch,n),{force:true});await usage?.close();}
