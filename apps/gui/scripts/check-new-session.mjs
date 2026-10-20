// Real App + private node/PTYs. Transport is the existing dev bridge;
// native page overlay/borrow rendering requires desktop app acceptance.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdirSync,writeFileSync,readFileSync,copyFileSync,rmSync} from 'node:fs';
import {join,resolve} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright-core';
import {fakeHome,fakeUsage} from './fake-home.mjs';
const gui=process.env.CONTENT_UI_ROOT||fileURLToPath(new URL('..',import.meta.url));
const repo=resolve(gui,'../..');
const scratch=process.env.CONTENT_CHECK_DIR||join(tmpdir(),'khor-new-session-check');
const target=process.env.CARGO_TARGET_DIR||join(repo,'target');
const port=Number(process.env.CONTENT_VITE_PORT||23862), bridgePort=Number(process.env.CONTENT_BRIDGE_PORT||23863);
const word=key=>readFileSync(join(repo,'crates/catalog/zh.toml'),'utf8').match(new RegExp('^'+key+' = "([^"]*)"','m'))?.[1];
const children=[],sessions=[];
let browser,usage,page;
const until=async(fn,ms=20000)=>{const end=Date.now()+ms;let last;do{const failed=children.find(c=>c.exitCode!==null&&c.exitCode!==0);if(failed)throw Error('Private test service exited: '+failed.exitCode);try{last=await fn();if(last)return last;}catch(e){last=e;}await new Promise(r=>setTimeout(r,100));}while(Date.now()<end);throw Error('Timed out: '+last);};
const run=(cmd,args,env,cwd=gui)=>{const c=spawn(cmd,args,{env,cwd,detached:true,stdio:['ignore','pipe','pipe']});children.push(c);c.stderr.on('data',d=>process.stderr.write(d));return c;};
const api=async(cmd,args={})=>{const r=await fetch(`http://127.0.0.1:${bridgePort}/${cmd}`,{signal:AbortSignal.timeout(5000),method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(args)});const value=await r.json();if(!r.ok)throw Error(JSON.stringify(value));return value;};
try{
 rmSync(scratch,{recursive:true,force:true});mkdirSync(scratch,{recursive:true});usage=await fakeUsage();
 const env={...process.env,...fakeHome(join(scratch,'home'),usage.url),KHOR_HOME:join(scratch,'node'),KHOR_NAME:'toolbar-check',KHOR_NO_MDNS:'1',BRIDGE_PORT:String(bridgePort)};delete env.KHOR_SESSION;
 for(const n of ['bridge','khor'])copyFileSync(join(target,'debug',n),join(scratch,n));run(join(scratch,'bridge'),[],env);await until(()=>api('devices').then(r=>r.length));
 const device=(await api('devices')).find(d=>d.me),root=join(scratch,'project');mkdirSync(root,{recursive:true});writeFileSync(join(root,'notes.md'),'# Notes\n\nKept content.');mkdirSync(join(root,'child'));writeFileSync(join(root,'child','second.txt'),'Second file');
 const id=await api('open_on',{machine:'',title:'Toolbar checks',cwd:root,cols:100,rows:35});sessions.push(id);await api('recent_visit',{kind:'page',machine:device.name,place:usage.url});
 run(join(gui,'node_modules/.bin/vite'),['--port',String(port),'--strictPort'],{...process.env,KHOR_FACE_GATE:'1'});await until(()=>fetch(`http://localhost:${port}/`).then(r=>r.ok));
 browser=await chromium.launch({channel:'chrome',headless:false,args:['--no-proxy-server']});const context=await browser.newContext({viewport:{width:1350,height:900},hasTouch:false,isMobile:false});
 await context.addInitScript(ids=>{localStorage.setItem('khor.sessions.tabs',JSON.stringify({tabs:ids,active:ids[0]}));localStorage.setItem('khor.place',JSON.stringify({landing:'sessions',screen:'detail'}));localStorage.setItem('khor.lang','zh');},sessions);
 page=await context.newPage();page.setDefaultTimeout(10000);const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const remote={...device,id:'a'.repeat(64),name:'toolbar-remote',shown:'远端 A',me:false,hop:'direct',paused:false};await page.route('**/devices',r=>r.fulfill({json:[device,remote]}));


 const submissions=[];
 await page.route('**/open_shell',async r=>{submissions.push({kind:'shell',...r.request().postDataJSON()});await r.fulfill({status:400,json:'Private verification: create intercepted'});});
 await page.route('**/open_session',async r=>{submissions.push({kind:'agent',...r.request().postDataJSON()});await r.fulfill({status:400,json:'Private verification: create intercepted'});});
 await page.route('**/launch_presets',r=>r.fulfill({json:[{id:'example',name:'研究环境',command:'claude --resume'}]}));
 await page.goto(`http://localhost:${port}/?bridge=${bridgePort}`,{waitUntil:'domcontentloaded',timeout:45000});await page.bringToFront();
 await page.locator('[data-session-tabs] [data-tab-new]:visible').first().click();const form=page.locator('[data-new-session-form]');await form.waitFor();
 await form.locator('[data-new-session-launch] textarea').click();await page.getByRole('option').filter({hasText:'Claude · 终端'}).click();
 const gap=await form.evaluate(el=>{const label=[...el.querySelectorAll('[data-form-label]')].find(e=>e.textContent==='启动方式').getBoundingClientRect(),input=el.querySelector('[data-new-session-launch] textarea').getBoundingClientRect();return Math.abs(label.y+label.height/2-input.y-input.height/2);});
 assert(gap<1,`Launch label is ${gap}px below input center`);
 const name=form.locator('[data-new-session-name]');await name.waitFor();assert.equal(await name.inputValue(),'');
 assert(await form.evaluate(el=>el.lastElementChild.matches('[data-new-session-name]')),'Name is not the last row');
 const create=page.locator('[data-new-session-create]');const count=async()=>{const n=submissions.length;await create.click();await until(()=>submissions.length>n);await until(()=>create.isEnabled());return submissions.at(-1);};
 await name.fill('  我的测试会话  ');let sent=await count();assert.equal(sent.title,'我的测试会话');const first=sent;sent=await count();assert.equal(sent.token,first.token,'Retry must reuse the creation token');await name.fill('改名后创建');sent=await count();assert.notEqual(sent.token,first.token,'A different name is a different creation intent');const beforeIme=submissions.length;await name.dispatchEvent('keydown',{key:'Enter',isComposing:true});assert.equal(submissions.length,beforeIme,'IME confirmation must not create');
 await name.fill('   ');sent=await count();assert.equal(sent.title,'','Blank name keeps node default');
 const launch=form.locator('[data-new-session-launch] textarea');await launch.click();await page.getByRole('option').filter({hasText:'研究环境'}).click();sent=await count();assert.equal(sent.title,'研究环境','Blank custom preset keeps preset name');
 await name.fill('自定义覆盖');sent=await count();assert.equal(sent.title,'自定义覆盖','Typed name overrides preset');
 await launch.click();await page.getByRole('option').filter({hasText:'Claude · 对话'}).click();sent=await count();assert.equal(sent.title,'自定义覆盖');assert.equal(sent.kind,'agent');
 await name.fill('');sent=await count();assert.equal(sent.title,'','Blank agent name keeps node directory fallback');
 await page.screenshot({path:join(scratch,'desktop.png')});console.log('PASS desktop label alignment, optional/trimmed names, shell/agent requests and preset precedence');
 await page.setViewportSize({width:390,height:844});await name.waitFor();await name.fill('窄屏会话');sent=await count();assert.equal(sent.title,'窄屏会话');
 assert(await form.evaluate(el=>el.scrollWidth<=el.clientWidth),'Mobile form overflows');assert(await name.evaluate(el=>{const b=el.getBoundingClientRect();return b.width>100&&b.x>=0&&b.right<=innerWidth;}));await page.screenshot({path:join(scratch,'mobile.png')});console.log('PASS mobile name input and layout');
 assert.deepEqual(errors,[]);console.log('new session verification clean');
}catch(e){console.error(e);await page?.screenshot({path:join(scratch,'failed.png'),timeout:5000}).catch(()=>{});throw e;}
finally{for(const id of sessions)await api('close_session',{id}).catch(()=>{});await browser?.close().catch(()=>{});for(const child of children)try{process.kill(-child.pid,'SIGKILL');}catch{};for(const n of ['bridge','khor'])rmSync(join(scratch,n),{force:true});await usage?.close();}
