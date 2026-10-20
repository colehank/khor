// Real App + private node: external file changes with no local watch.
// Uses a fake home and its own services; never touches user sessions.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdirSync,writeFileSync,copyFileSync,rmSync,realpathSync} from 'node:fs';
import {join,resolve} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright-core';
import {fakeHome,fakeUsage} from './fake-home.mjs';
const gui=process.env.CONTENT_UI_ROOT||fileURLToPath(new URL('..',import.meta.url));
const repo=resolve(gui,'../..');
let scratch=process.env.CONTENT_CHECK_DIR||join(tmpdir(),'khor-tree-freshness-check');
const target=process.env.CARGO_TARGET_DIR||join(repo,'target');
const port=Number(process.env.CONTENT_VITE_PORT||23864), bridgePort=Number(process.env.CONTENT_BRIDGE_PORT||23865);
const children=[],sessions=[];
let browser,usage,page;
const until=async(fn,ms=20000)=>{const end=Date.now()+ms;let last;do{const failed=children.find(c=>c.exitCode!==null&&c.exitCode!==0);if(failed)throw Error('Private test service exited: '+failed.exitCode);try{last=await fn();if(last)return last;}catch(e){last=e;}await new Promise(r=>setTimeout(r,100));}while(Date.now()<end);throw Error('Timed out: '+last);};
const run=(cmd,args,env,cwd=gui)=>{const c=spawn(cmd,args,{env,cwd,detached:true,stdio:['ignore','pipe','pipe']});children.push(c);c.stderr.on('data',d=>process.stderr.write(d));return c;};
const api=async(cmd,args={})=>{const r=await fetch(`http://127.0.0.1:${bridgePort}/${cmd}`,{signal:AbortSignal.timeout(5000),method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(args)});const value=await r.json();if(!r.ok)throw Error(JSON.stringify(value));return value;};
try{
 rmSync(scratch,{recursive:true,force:true});mkdirSync(scratch,{recursive:true});scratch=realpathSync(scratch);usage=await fakeUsage();
 const env={...process.env,...fakeHome(join(scratch,'home'),usage.url),KHOR_HOME:join(scratch,'node'),KHOR_NAME:'tree-check',KHOR_NO_MDNS:'1',BRIDGE_PORT:String(bridgePort)};delete env.KHOR_SESSION;
 for(const n of ['bridge','khor'])copyFileSync(join(target,'debug',n),join(scratch,n));run(join(scratch,'bridge'),[],env);await until(()=>api('devices').then(r=>r.length));
 const device=(await api('devices')).find(d=>d.me),root=join(scratch,'project');mkdirSync(root,{recursive:true});
 const id=await api('open_on',{machine:'',title:'Tree freshness checks',cwd:root,cols:100,rows:35});sessions.push(id);
 run(join(gui,'node_modules/.bin/vite'),['--port',String(port),'--strictPort'],{...process.env,KHOR_FACE_GATE:'1'});await until(()=>fetch(`http://localhost:${port}/`).then(r=>r.ok));
 browser=await chromium.launch({channel:'chrome',headless:false,args:['--no-proxy-server']});const context=await browser.newContext({viewport:{width:1350,height:900},hasTouch:false,isMobile:false});
 await context.addInitScript(ids=>{localStorage.setItem('khor.sessions.tabs',JSON.stringify({tabs:ids,active:ids[0]}));localStorage.setItem('khor.place',JSON.stringify({landing:'sessions',screen:'detail'}));localStorage.setItem('khor.lang','zh');},sessions);
 page=await context.newPage();page.setDefaultTimeout(10000);const errors=[];page.on('pageerror',e=>errors.push(e.message));
 await page.route('**/devices',r=>r.fulfill({json:[device]}));



 const manuscripts=join(root,'docs','manuscripts');mkdirSync(manuscripts,{recursive:true});
 const reads=[],completed=[];
 let holdPath,releaseOld,oldHeld=false,watchMode=false;
 await page.route('**/ls',async r=>{
   const args=r.request().postDataJSON();reads.push(args.path);
   const listing=await api('ls',args);
   if(holdPath===args.path){holdPath=null;oldHeld=true;await new Promise(resolve=>{releaseOld=resolve;});}
   await r.fulfill({json:listing});completed.push(args.path);
 });
 const watches=[];
 await page.route('**/dirs_wait',async r=>{
   const {dirs,since,wait_ms:hold}=r.request().postDataJSON();watches.push(dirs);
   if(watchMode&&hold)await new Promise(resolve=>setTimeout(resolve,1000));
   await r.fulfill({json:{seq:since,dirs:watchMode?[]:dirs,unavailable:!watchMode,unreported:[]}}).catch(()=>{});
 });
 await page.goto(`http://localhost:${port}/?bridge=${bridgePort}`,{waitUntil:'domcontentloaded',timeout:45000});await page.bringToFront();
 await page.locator(`[data-session-tabs] [data-tab-open="${id}"]`).first().click();const tree=page.locator('[data-content-picker]:visible [data-file-tree]');await tree.waitFor();
 await tree.locator('[data-dir=docs]').click();await tree.locator('[data-dir=manuscripts]').click();await until(()=>completed.includes(manuscripts));await page.waitForTimeout(300);
 mkdirSync(join(manuscripts,'raw'));writeFileSync(join(manuscripts,'raw','01_introduction.md'),'# Introduction');
 await page.evaluate(()=>window.dispatchEvent(new Event('focus')));
 await until(()=>tree.locator('[data-dir=raw]').count(),5000);console.log('PASS local folder created outside khor appears on focus');
 await tree.locator('[data-dir=raw]').click();await tree.locator('[data-file="01_introduction.md"]').waitFor();
 await tree.locator('[data-dir=raw]').click();writeFileSync(join(manuscripts,'raw','02_method.md'),'# Method');await tree.locator('[data-dir=raw]').click();await tree.locator('[data-file="02_method.md"]').waitFor();console.log('PASS reopening unwatched folder reads new files');

 // Keep an earlier, empty-of-new-file answer on the wire. A second focus
 // must win even when that old request lands last.
 const raw=join(manuscripts,'raw');holdPath=raw;
 await page.evaluate(()=>window.dispatchEvent(new Event('focus')));await until(()=>oldHeld);
 writeFileSync(join(raw,'latest.md'),'# Latest');
 await page.evaluate(()=>window.dispatchEvent(new Event('focus')));
 await tree.locator('[data-file="latest.md"]').waitFor();
 const oldCount=completed.filter(p=>p===raw).length;releaseOld();
 await until(()=>completed.filter(p=>p===raw).length>oldCount);await page.waitForTimeout(200);
 await tree.locator('[data-file="latest.md"]').waitFor();console.log('PASS older listing cannot overwrite latest response');
 // A collapsed ancestor hides its remembered descendants too. Focus
 // refreshes the root, but must not list those hidden levels.
 await tree.locator('[data-dir=docs]').click();const before=reads.length;
 await page.evaluate(()=>window.dispatchEvent(new Event('focus')));
 await until(()=>reads.length>before);await page.waitForTimeout(200);
 assert(!reads.slice(before).some(p=>p===manuscripts||p===raw),'Focus read a hidden descendant');
 console.log('PASS fallback refreshes only visible branches');
 // Conversely, a reliable watch keeps the cache on focus and reopening.
 watchMode=true;watches.length=0;
 await page.reload({waitUntil:'domcontentloaded'});await page.locator(`[data-session-tabs] [data-tab-open="${id}"]`).first().click();await tree.waitFor();
 await until(()=>watches.length>0);await tree.locator('[data-dir=docs]').click();
 await tree.locator('[data-file="latest.md"]').waitFor();
 await until(()=>watches.some(d=>d.includes(raw)));await page.waitForTimeout(200);
 const count=reads.length;
 await tree.locator('[data-dir=raw]').click();await tree.locator('[data-dir=raw]').click();
 await page.evaluate(()=>window.dispatchEvent(new Event('focus')));await page.waitForTimeout(300);
 assert.equal(reads.length,count,'Reliable watch should keep its cached listings');
 await tree.locator('[data-file="latest.md"]').waitFor();console.log('PASS reliable watch keeps cache on focus and reopening');
 await page.screenshot({path:join(scratch,'tree.png')});assert.deepEqual(errors,[]);

}catch(e){console.error(e);await page?.screenshot({path:join(scratch,'failed.png'),timeout:5000}).catch(()=>{});throw e;}
finally{for(const id of sessions)await api('close_session',{id}).catch(()=>{});await browser?.close().catch(()=>{});for(const child of children)try{process.kill(-child.pid,'SIGKILL');}catch{};for(const n of ['bridge','khor'])rmSync(join(scratch,n),{force:true});await usage?.close();}
