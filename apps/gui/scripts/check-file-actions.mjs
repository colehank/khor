// Production FileTree, FilesPane, Hits, FilePage and FileVerbs. Only disk/native transport
// and the clipboard/share OS boundary are mocked; no product components.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdirSync,mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url));
const port=Number(process.env.FILE_ACTIONS_PORT||23876);
const entry=join(gui,`scripts/.file-actions-${process.pid}.tsx`);
const out=process.env.FILE_ACTIONS_OUT||mkdtempSync(join(tmpdir(),'khor-file-actions-'));
mkdirSync(out,{recursive:true});
const pause=ms=>new Promise(r=>setTimeout(r,ms));
let vite,browser,page;
const errors=[],checks=[];
try {
 writeFileSync(entry,`import React from 'react'; import {createRoot} from 'react-dom/client'; import {flushSync} from 'react-dom';
import {FileTree} from '../src/views/FileTree.tsx'; import {FilesPane} from '../src/views/FilesPane.tsx'; import {Hits} from '../src/views/WorkspacePane.tsx'; import {FilePage,FileVerbs} from '../src/views/FilePage.tsx';
import {requestFileLocation,pendingFileLocation} from '../src/lib/file-location.ts';
import {TooltipProvider} from '../src/components/ui/tooltip.tsx'; import {speakLang} from '../src/lang.ts';
import {gui} from '../src/gen/catalog.ts';
import {NARROW_QUERY} from '../src/hooks/use-narrow.ts'; import '../src/styles/tokens.css'; import '../src/styles/app.css';
const root=createRoot(document.getElementById('root')); const no=()=>{};
const posture=()=>document.documentElement.toggleAttribute('data-narrow',matchMedia(NARROW_QUERY).matches); posture();addEventListener('resize',posture);speakLang('zh');
window.copyFailed=gui.copy_failed;window.located=[]; window.toggled=0; window.locate=path=>requestFileLocation('fixture',path); window.pending=()=>pendingFileLocation('fixture');
window.showTree=(here=true,hidden=false)=>flushSync(()=>root.render(<TooltipProvider><input data-focus-anchor aria-label='Focus anchor'/><section className='flex min-h-0 flex-1 flex-col' aria-hidden={hidden||undefined} inert={hidden||undefined}><FileTree key={String(here)} machine='fixture' root='/project' here={here} marks={null} open={null} onOpen={no} onGone={no}/></section></TooltipProvider>));
window.showFiles=(here=true,hidden=false)=>flushSync(()=>root.render(<TooltipProvider><input data-focus-anchor aria-label='Focus anchor'/><section className='flex min-h-0 flex-1 flex-col' aria-hidden={hidden||undefined} inert={hidden||undefined}><FilesPane key={String(here)} machine='fixture' path='/project' here={here} open={null} onPath={no} onLanded={no} onFile={no}/></section></TooltipProvider>));
window.showHits=(here=true)=>flushSync(()=>root.render(<TooltipProvider><Hits machine='fixture' here={here} root='/project' dir='/project' found={{names:['/project/notes.md'],lines:[{path:'/project/notes.md',line:3,text:'notes line'}],more:false}} error={null} query='notes' onOpen={no}/></TooltipProvider>));
window.showFile=(here=true,path='/project/notes.md')=>flushSync(()=>root.render(<TooltipProvider><FilePage machine='fixture' path={path} here={here} onBack={no} onGone={no} onMoved={no} onLocate={()=>window.located.push(path)} /></TooltipProvider>));
window.showVerbs=(here=true)=>flushSync(()=>root.render(<TooltipProvider><FileVerbs machine='fixture' path='/project/notes.md' here={here} onGone={no} onMoved={no} onLocate={()=>window.located.push('/project/notes.md')} faceToggle={{face:'source',run:()=>window.toggled++}}/></TooltipProvider>));
window.showTree();`);
 vite=spawn(join(gui,'node_modules/.bin/vite'),['--port',String(port),'--strictPort'],{cwd:gui,env:{...process.env,KHOR_FACE_GATE:'1'},stdio:['ignore','pipe','pipe']});
 vite.stderr.on('data',d=>process.stderr.write(d));
 let ready=false;vite.stdout.on('data',d=>{ready ||= String(d).includes(`:${port}`);});
 for(let i=0;i<100&&!ready;i++){assert(vite.exitCode===null,'Vite exited');await pause(100);}assert(ready,'Vite did not start');
 browser=await chromium.launch({channel:'chrome',headless:process.env.FILE_ACTIONS_HEADED==='0',args:['--no-proxy-server']});
 for(const scenario of [
  {name:'mac-local',platform:'MacIntel',width:1200,native:true,here:true,label:'在访达中显示'},
  {name:'windows-local',platform:'Win32',width:1200,native:true,here:true,label:'在文件资源管理器中显示'},
  {name:'linux-local',platform:'Linux x86_64',width:1200,native:true,here:true,label:'在文件管理器中显示'},
  {name:'mac-remote',platform:'MacIntel',width:1200,native:true,here:false},
  {name:'web-local',platform:'MacIntel',width:1200,native:false,here:true},
  {name:'phone-local',platform:'iPhone',width:390,native:false,here:true},
 ].filter(s=>!process.env.FILE_ACTIONS_CASE||s.name===process.env.FILE_ACTIONS_CASE)) {
  const context=await browser.newContext({viewport:{width:scenario.width,height:844},hasTouch:scenario.width<700});
  await context.addInitScript(({platform,phone})=>{
   Object.defineProperty(navigator,'platform',{get:()=>platform});Object.defineProperty(navigator,'maxTouchPoints',{get:()=>phone?5:0});
   window.copied=[];window.shared=[];
   Object.defineProperty(navigator,'clipboard',{value:{writeText:async t=>{if(window.refuseCopy)throw Error('Clipboard unavailable');window.copied.push(t)}}});
   Object.defineProperty(navigator,'canShare',{value:()=>phone});
   Object.defineProperty(navigator,'share',{value:async v=>window.shared.push(v.files.map(f=>f.name))});
  },{platform:scenario.platform,phone:scenario.width<700});
  page=await context.newPage();page.setDefaultTimeout(5000);page.on('pageerror',e=>errors.push(e.message));
  const calls=[];
  await page.route(`http://127.0.0.1:${port}/**`,async r=>{
   const cmd=new URL(r.request().url()).pathname.slice(1),args=r.request().postDataJSON()||{};calls.push({cmd,args});
   const text='# Notes\n\nReadable content\n';let result=null;
   if(cmd==='ls')result={path:args.path,entries:args.path==='/project'?[{name:'child',dir:true,size:0,at_ms:0},{name:'notes.md',dir:false,size:text.length,at_ms:0}]:args.path==='/project/child'?[{name:'nested.md',dir:false,size:20,at_ms:0}]:[],truncated:false};
   else if(cmd==='dirs_wait') {await pause(100);result={seq:0,dirs:[],unavailable:true,unreported:[]};}
   else if(cmd==='reveal_here')result=scenario.native;
   else if(cmd==='trash_here')result=false;
   else if(cmd==='read_text')result={text,total:text.length,at_ms:0,cut:false,binary:false,digest:'0'.repeat(64)};
   else if(cmd==='file_stat')result={total:text.length,at_ms:0,digest:'0'.repeat(64)};
   else if(cmd==='about')result={hosts:true,faces:[]};
   else if(cmd==='pull_temp')result=args.machine?'/tmp/khor-test/notes.md':args.path;
   await r.fulfill({json:result}).catch(()=>{});
  });
  await page.route('**/__file_actions?**',r=>r.fulfill({contentType:'text/html',body:`<!doctype html><html data-theme="dark"><body><div id="root" class="flex h-screen flex-col"></div><script type="module">import R from '/@react-refresh';R.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>t=>t;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/.file-actions-${process.pid}.tsx"></script></body></html>`}));
  await page.goto(`http://localhost:${port}/__file_actions?bridge=${port}`);
  await page.evaluate(here=>window.showTree(here),scenario.here);
  await page.locator('[data-file="notes.md"]').waitFor();await pause(150);
  const rowMenu=async selector=>{await page.locator('[data-row-menu]').waitFor({state:'detached'});await page.locator(selector).click({button:'right'});await page.locator('[data-row-menu]').waitFor();};
  for(const selector of ['[data-file="notes.md"]','[data-dir="child"]']) {
   await rowMenu(selector);
   const reveal=page.locator('[data-menu-item="reveal"]');
   assert.equal(await reveal.count(),scenario.label?1:0,`${scenario.name}: wrong reveal availability for ${selector}`);
   if(scenario.name==='mac-local'&&selector.includes('notes.md'))await page.screenshot({path:join(out,'tree-menu.png')});
   if(scenario.label){assert.equal((await reveal.innerText()).trim(),scenario.label);await reveal.click();await page.locator('[data-row-menu]').waitFor({state:'detached'});const call=calls.filter(c=>c.cmd==='reveal').at(-1);assert(call,'reveal did not invoke native boundary');assert.equal(call.args.path,selector.includes('child')?'/project/child':'/project/notes.md');}
   else await page.keyboard.press('Escape');
  }
  await rowMenu('[data-file="notes.md"]');await page.locator('[data-menu-item="copy-path"]').click();assert.equal((await page.evaluate(()=>window.copied)).at(-1),'/project/notes.md');
  await page.locator('[data-row-menu]').waitFor({state:'detached'});
  await rowMenu('[data-file="notes.md"]');await page.locator('[data-menu-item="copy-rel"]').click();assert.equal((await page.evaluate(()=>window.copied)).at(-1),'notes.md');
  await page.locator('[data-row-menu]').waitFor({state:'detached'});
  await rowMenu('[data-file="notes.md"]');assert.equal(await page.locator('[data-menu-item="rename"]').count(),1);assert.equal(await page.locator('[data-row-menu] [data-destructive="true"]').count(),1);await page.keyboard.press('Escape');
  if(scenario.name==='mac-local') {
   // A retained hidden workspace must not eat the pending request. Once
   // visible, it opens ancestors, focuses the actual nested row, and
   // acknowledges once. Later renders must not steal focus back.
   await page.evaluate(()=>{window.showTree(true,true);window.locate('/project/child/nested.md');});
   await pause(150);assert.equal(await page.evaluate(()=>window.pending()?.path),'/project/child/nested.md','hidden tree consumed locate');
   await page.evaluate(()=>window.showTree(true,false));
   await page.waitForFunction(()=>!window.pending());
   assert.equal(await page.evaluate(()=>document.activeElement?.getAttribute('data-file-path')),'/project/child/nested.md');
   await page.locator('[data-focus-anchor]').focus();await page.evaluate(()=>window.showTree(true,false));await pause(100);
   assert.equal(await page.evaluate(()=>document.activeElement?.hasAttribute('data-focus-anchor')),true,'ordinary render stole focus');
   await page.evaluate(()=>{window.showVerbs(true);window.locate('/project/child/nested.md');});
   assert.equal(await page.evaluate(()=>window.pending()?.path),'/project/child/nested.md');
   await page.evaluate(()=>window.showTree(true,false));await page.waitForFunction(()=>!window.pending());
   assert.equal(await page.evaluate(()=>document.activeElement?.getAttribute('data-file-path')),'/project/child/nested.md','locate did not survive tree unmount');
  }
  if(scenario.name==='mac-local') {
   await page.evaluate(()=>{window.showVerbs(true);window.locate('/project/notes.md');window.showFiles(true,true);});
   await page.locator('[data-files-list]').waitFor({state:'attached'});await pause(150);
   assert.equal(await page.evaluate(()=>window.pending()?.path),'/project/notes.md','hidden directory list consumed locate');
   await page.evaluate(()=>window.showFiles(true,false));await page.waitForFunction(()=>!window.pending());
   assert.equal(await page.evaluate(()=>document.activeElement?.getAttribute('data-file-path')),'/project/notes.md','directory list did not focus pending file');
   await page.locator('[data-focus-anchor]').focus();await page.evaluate(()=>window.showFiles(true,false));await pause(100);
   assert.equal(await page.evaluate(()=>document.activeElement?.hasAttribute('data-focus-anchor')),true,'directory list ordinary render stole focus');
   await rowMenu('[data-file="notes.md"]');await page.evaluate(()=>window.refuseCopy=true);await page.locator('[data-menu-item="copy-path"]').click();
   await page.waitForFunction(()=>document.querySelector('[data-menu-item="copy-path"]')?.textContent===window.copyFailed);
   assert.equal(await page.locator('[data-row-menu][data-state="open"]').count(),1,'clipboard refusal closed the menu');
   await page.evaluate(()=>window.refuseCopy=false);await page.locator('[data-menu-item="copy-path"]').click();await page.locator('[data-row-menu]').waitFor({state:'detached'});
   assert.equal((await page.evaluate(()=>window.copied)).at(-1),'/project/notes.md','copy retry did not succeed');
  }
  for(const view of ['showFiles','showHits']) {
   await page.evaluate(({view,here})=>window[view](here),{view,here:scenario.here});
   const selectors=view==='showFiles'?['[data-file="notes.md"]','[data-dir="child"]']:['[data-hit-name="/project/notes.md"]','[data-hit-line="notes.md:3"]'];
   for(const selector of selectors){
    await page.locator(selector).waitFor();await pause(100);await rowMenu(selector);
    const action=page.locator('[data-menu-item="reveal"]');assert.equal(await action.count(),scenario.label?1:0,scenario.name+' '+view+' reveal policy');
    if(scenario.label)assert.equal((await action.innerText()).trim(),scenario.label);
    assert.equal(await page.locator('[data-menu-item="copy-path"]').count(),1,view+' copy path missing');
    await page.locator('[data-menu-item="copy-rel"]').click();await page.locator('[data-row-menu]').waitFor({state:'detached'});
    assert.equal((await page.evaluate(()=>window.copied)).at(-1),selector.includes('child')?'child':'notes.md');
   }
  }
  await page.evaluate(here=>window.showFile(here),scenario.here);await page.locator('.cm-content').waitFor();
  const preview=page.locator('[data-file-face-toggle]');
  await preview.waitFor();assert.equal(await preview.evaluate(e=>!!e.closest('[data-file-more-menu]')),false,'preview still inside menu');
  assert.equal(await preview.evaluate(e=>e.closest('[data-glass-capsule]')?.querySelectorAll('[data-file-more]').length),1,'preview/more do not share a capsule');
  const height=await page.locator('[data-file-deck]').evaluate(e=>e.getBoundingClientRect().height);
  await preview.click();await page.locator('[data-file-preview="md"]').waitFor();await preview.click();await page.locator('.cm-content').waitFor();
  assert.equal(await page.locator('[data-file-deck]').evaluate(e=>e.getBoundingClientRect().height),height,'preview/source folded the deck');
  await page.locator('[data-file-more]').click();await page.locator('[data-file-more-menu]').waitFor();
  for(const key of ['rename','delete','remove','prev','next','previous','content-previous','content-next','content-browse','preview'])assert.equal(await page.locator(`[data-file-more-item="${key}"]`).count(),0,`content menu exposes ${key}`);
  assert.equal(await page.locator('[data-file-share]').count(),0,'duplicate standalone share control');
  assert.equal(await page.locator('[data-file-more-item="reveal"]').count(),scenario.label?1:0,'content reveal is not local desktop only');
  assert.equal(await page.locator('[data-file-more-item="open-default"]').count(),scenario.native&&scenario.here?1:0,'default app availability');
  assert.equal(await page.locator('[data-file-more-item="open-copy"]').count(),scenario.native&&!scenario.here?1:0,'remote app must say copy');
  assert.equal(await page.locator('[data-file-more-item="share"]').count(),scenario.width<700?1:0,'share availability');
  if(scenario.native){
   const key=scenario.here?'open-default':'open-copy';await page.locator('[data-file-more-item="'+key+'"]').click();
   await page.locator('[data-file-more-menu]').waitFor({state:'detached'});
   await pause(100);
   const open=calls.filter(c=>c.cmd==='launch').at(-1);assert(open,'open action did not cross native boundary');
   assert.equal(open.args.target,scenario.here?'/project/notes.md':'/tmp/khor-test/notes.md');
   if(scenario.here)assert.equal(calls.filter(c=>c.cmd==='pull_temp').length,0,'local default app used temporary copy');
   else {assert.equal(calls.filter(c=>c.cmd==='pull_temp').at(-1).args.machine,'fixture');assert.equal(calls.filter(c=>c.cmd==='reveal').length,0,'remote opened system reveal');}
   await page.locator('[data-file-more]').click();await page.locator('[data-file-more-menu]').waitFor();
  } else if(scenario.width<700) {
   await page.locator('[data-file-more-item="share"]').click();await page.locator('[data-file-more-menu]').waitFor({state:'detached'});await pause(100);
   assert.deepEqual(await page.evaluate(()=>window.shared),[['notes.md']]);await page.locator('[data-file-more]').click();await page.locator('[data-file-more-menu]').waitFor();
  }
  const locate=page.locator('[data-file-more-item="locate"]');await locate.click();await page.locator('[data-file-more-menu]').waitFor({state:'detached'});assert.deepEqual(await page.evaluate(()=>window.located),['/project/notes.md']);
  await page.locator('[data-file-more-menu]').waitFor({state:'detached'});
  await page.screenshot({path:join(out,`${scenario.name}.png`)});
  await page.evaluate(here=>{window.toggled=0;window.showVerbs(here)},scenario.here);
  assert.equal(await page.locator('[data-file-share]').count(),0,'desktop verbs have a duplicate share control');
  assert.equal(await page.locator('[data-file-face-toggle]').count(),1,'workspace verbs omitted preview');
  await page.locator('[data-file-face-toggle]').click();assert.equal(await page.evaluate(()=>window.toggled),1,'workspace preview button did not run');
  if(scenario.name==='phone-local') {
   await page.evaluate(()=>window.showFile(true,'/project/一个很长的文件标题_very-long-filename-that-must-truncate-without-pushing-preview-and-more-out.md'));
   await page.locator('.cm-content').waitFor();
   const geometry=await page.locator('[data-file-deck]').evaluate(deck=>{const group=deck.querySelector('[data-file-actions]').getBoundingClientRect(),pill=deck.querySelector('[data-file-pill]').getBoundingClientRect(),name=deck.querySelector('[data-file-name]');return {group:{x:group.x,right:group.right,width:group.width},pill:{x:pill.x,right:pill.right},truncated:name.scrollWidth>name.clientWidth,viewport:innerWidth,scroll:document.documentElement.scrollWidth};});
   assert(geometry.truncated,'long filename did not truncate');assert(geometry.pill.right<=geometry.group.x&&geometry.group.right<=geometry.viewport,'long name overlaps/pushes actions');assert(geometry.scroll<=geometry.viewport,'long file caused horizontal page overflow');
   await page.screenshot({path:join(out,'phone-long-name.png')});
  }
  checks.push(scenario.name);console.log('PASS',scenario.name);await context.close();
 }
 assert.deepEqual(errors,[]);writeFileSync(join(out,'results.json'),JSON.stringify({checks,errors},null,2));console.log('file actions: clean',out);
} catch(e){await page?.screenshot({path:join(out,'failed.png')}).catch(()=>{});throw e;}
finally {await browser?.close().catch(()=>{});if(vite?.exitCode===null){vite.kill('SIGTERM');await Promise.race([new Promise(r=>vite.once('exit',r)),pause(2000)]);if(vite.exitCode===null)vite.kill('SIGKILL');}rmSync(entry,{force:true});}
