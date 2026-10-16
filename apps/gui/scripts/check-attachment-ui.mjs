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
const scratch=process.env.CONTENT_CHECK_DIR||join(tmpdir(),'khor-attachment-ui-check');
const target=process.env.CARGO_TARGET_DIR||join(repo,'target');
const port=Number(process.env.CONTENT_VITE_PORT||1482), bridgePort=Number(process.env.CONTENT_BRIDGE_PORT||1483);
const word=key=>readFileSync(join(repo,'crates/catalog/zh.toml'),'utf8').match(new RegExp('^'+key+' = "([^"]*)"','m'))?.[1];
const children=[],sessions=[];
let browser,usage,page;
const until=async(fn,ms=20000)=>{const end=Date.now()+ms;let last;do{const failed=children.find(c=>c.exitCode!==null&&c.exitCode!==0);if(failed)throw Error('Private test service exited: '+failed.exitCode);try{last=await fn();if(last)return last;}catch(e){last=e;}await new Promise(r=>setTimeout(r,100));}while(Date.now()<end);throw Error('Timed out: '+last);};
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
    mkdirSync(dir,{recursive:true});for(let j=0;j<24;j++)writeFileSync(join(dir,'extra-'+String(j).padStart(2,'0')+'.txt'),'Neighbor '+j+'\n'+(j===15?Array.from({length:80},(_,k)=>'Line '+k).join('\n'):''));writeFileSync(join(dir,'notes.md'),`# ${i===0?'Alpha':'Beta'}\n\nSession contents.\n`);writeFileSync(join(dir,'only-'+i+'.txt'),'This file belongs to '+i+'\n');
    const id=await api('open_on',{machine:'',title:'Content '+(i===0?'Alpha':'Beta'),cwd:dir,cols:100,rows:35});sessions.push(id);
  }
  run(join(gui,'node_modules/.bin/vite'),['--port',String(port),'--strictPort'],{...process.env,KHOR_FACE_GATE:'1'});
  await until(()=>fetch(`http://localhost:${port}/`).then(r=>r.ok));
  browser=await chromium.launch({channel:'chrome',headless:process.env.CONTENT_HEADED!=='1',args:['--no-proxy-server']});
  const context=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true,reducedMotion:'reduce'});
  // The LAN HTTP face has no secure-context randomUUID API.
  await context.addInitScript(()=>Object.defineProperty(globalThis.crypto,'randomUUID',{configurable:true,value:undefined}));
  await context.addInitScript(ids=>{
    localStorage.setItem('khor.sessions.tabs',JSON.stringify({tabs:ids,active:ids[0]}));localStorage.setItem('khor.place',JSON.stringify({landing:'sessions',screen:'detail'}));localStorage.setItem('khor.lang','zh');
  },sessions);
  page=await context.newPage();page.setDefaultTimeout(12000);const errors=[];page.on('pageerror',e=>{errors.push(e.message);console.error('PAGEERROR',e.message);});
  const url=`http://localhost:${port}/?bridge=${bridgePort}`;
  await page.goto(url);
  assert(await page.evaluate(()=>typeof globalThis.crypto.randomUUID==='undefined'),'LAN compatibility probe did not run');
  const shown=()=>page.locator('[data-session-content][data-content-visible=true]');
  const out=process.env.CONTENT_UI_OUTPUT||join(scratch,'ui');mkdirSync(out,{recursive:true});const results=[],failures=[];
  const note=(name,data)=>{results.push({name,...data});console.log(name,JSON.stringify(data));if(data.error)failures.push(name+': '+data.error);};
  const step=async(name,fn)=>{try{await fn();}catch(e){note(name,{error:String(e)});await page.screenshot({path:join(out,'attachment-error-'+name+'.png')}).catch(()=>{});}};
  const folder=join(projects[0],'folder');mkdirSync(folder,{recursive:true});writeFileSync(join(folder,'child.md'),'# Child\n');
  const remote={...device,id:'a'.repeat(64),name:'audit-remote',shown:'远端 A',me:false,hop:'direct',hosts:true,paused:false};
  await page.route('**/devices',r=>r.fulfill({json:[device,remote]}));
  await page.route('**/ls',r=>{const asked=r.request().postDataJSON().path;const path=asked||projects[0];const child=path===folder;return r.fulfill({json:{path,entries:child?[{name:'child.md',dir:false,size:8,at_ms:0}]:[{name:'folder',dir:true,size:0,at_ms:0},{name:'notes.md',dir:false,size:8,at_ms:0}],truncated:false}});});
  await page.route('**/read_text',r=>r.fulfill({json:{total:8,at_ms:0,text:'# Audit\n',cut:false,binary:false,digest:'audit'}}));
  await page.route('**/stat_text',r=>r.fulfill({json:{total:8,at_ms:0,digest:'audit'}}));
  let hold=null,release;const staged=[],sent=[],forgotten=[],cancelled=[];
  await page.route(/\/term_drop_stage\?/,async r=>{const name=new URL(r.request().url()).searchParams.get('name');staged.push(name);if(hold)await hold;await r.fulfill({json:'/stage/'+staged.length+'/'+name});});
  await page.route('**/term_drop',r=>{const args=r.request().postDataJSON();sent.push(args);return r.fulfill({json:{landed:args.paths,miss:null}});});
  await page.route('**/term_drop_items',r=>{const args=r.request().postDataJSON();sent.push(args);return r.fulfill({json:{landed:args.items.map(x=>x.path),miss:null}});});
  await page.route('**/term_drop_cancel',r=>{cancelled.push(r.request().postDataJSON());return r.fulfill({json:null});});
  await page.route('**/term_drop_forget',r=>{forgotten.push(...r.request().postDataJSON().paths);return r.fulfill({json:null});});
  await page.route('**/term_drop_progress',r=>r.fulfill({json:null}));
  if(!(await page.locator('[data-deck-title]').isVisible()))await page.locator('[data-session-title]').filter({hasText:/^Content Alpha$/}).click();
  const keys=page.locator('[data-key-row-switch]:visible');if(await keys.getAttribute('aria-expanded')!=='true')await keys.click();await page.locator('[data-term-key=attach]:visible').click();await page.locator('[data-files-machine]').first().waitFor();
  const panel=()=>page.locator('[data-files-panel]');
  await step('preview-selection',async()=>{
    await page.locator('[data-files-machine="audit-remote"]').click();await page.locator('[data-files-name="notes.md"]>button').first().click();await page.locator('[data-file-preview] [data-file-view]').waitFor();
    assert.equal(await page.locator('[data-file-preview] [contenteditable=true]').count(),0);assert.equal(sent.length,0);note('attachment-preview',{editable:await page.locator('[data-file-preview] [contenteditable=true]').count(),sent:sent.length,source:await page.locator('[data-file-preview] [role=radio][aria-checked=true]').textContent()});
    await page.locator('[data-file-preview-close]').click();await page.locator('[data-files-select=folder]').click();await page.locator('[data-files-name=folder]>button').first().click();
    assert(await page.locator('[data-files-select="child.md"]').isDisabled());note('directory-inclusion',{childDisabled:await page.locator('[data-files-select="child.md"]').isDisabled()});await page.locator('[data-files-up]').click();await page.locator('[data-files-select=folder]').click();
    assert.equal(await page.locator('[data-files-select=folder]').getAttribute('aria-pressed'),'false');note('directory-deselect',{selected:await page.locator('[data-files-select=folder]').getAttribute('aria-pressed')});
    await page.locator('[data-files-select="notes.md"]').click();await page.locator('[data-files-review]').click();note('attachment-queue',{selected:await page.locator('[data-files-selected]').count()});
    await page.locator('[data-files-send]').click();await until(()=>sent.length>0);await until(()=>page.locator('[data-files-send]').isDisabled());note('attachment-send',{sent:sent.length,operation:sent[0]});
  });
  await step('cancel-immediate-retry',async()=>{
    const file=panel().locator('input[type=file]:not([webkitdirectory])');await file.setInputFiles([{name:'retry-a.txt',mimeType:'text/plain',buffer:Buffer.from('a')},{name:'retry-b.txt',mimeType:'text/plain',buffer:Buffer.from('b')}]);
    let finishCancel;const ack=new Promise(r=>finishCancel=r);
    await page.unroute('**/term_drop_cancel');await page.route('**/term_drop_cancel',async r=>{cancelled.push(r.request().postDataJSON());await ack;await r.fulfill({json:null});});
    hold=new Promise(r=>release=r);
    try{
      await page.locator('[data-files-send]').click();await until(()=>staged.length===1);
      await panel().getByRole('button',{name:word('term-drop-stop'),exact:true}).click();
      assert.equal(await page.locator('[data-files-send]').count(),0,'cancel exposed an early retry');
      await page.locator('[data-files-transfer][data-stopping]').waitFor();
      assert.equal(await panel().getByRole('button',{name:word('term-drop-stop'),exact:true}).count(),0,'cancel can be repeated during cleanup');
      release();hold=null;await until(()=>forgotten.length>0);
      assert.equal(await page.locator('[data-files-send]').count(),0,'retry unlocked before cancellation acknowledgement');
      finishCancel();await page.locator('[data-files-send]').waitFor();await page.locator('[data-files-send]').click();
      await until(()=>sent.length>=2);await until(()=>page.locator('[data-files-send]').isDisabled());
      note('cancel-retry',{staged:staged.length,sent:sent.length,cancels:cancelled.length});
    }finally{release?.();hold=null;finishCancel?.();}
  });
  await step('empty-system-change',async()=>{
    await panel().locator('input[type=file]:not([webkitdirectory])').setInputFiles([]);
    assert.equal(await panel().getByText(word('files-empty-selection'),{exact:true}).count(),0,'empty change entered empty queue');note('empty-system-change',{emptyQueue:await panel().getByText(word('files-empty-selection'),{exact:true}).count(),sources:await page.locator('[data-files-machine]').count(),header:await panel().locator('h3').first().innerText()});
  });
  writeFileSync(join(out,'attachment-results.json'),JSON.stringify({results,errors},null,2));assert.deepEqual(errors,[]);assert.deepEqual(failures,[]);console.log('attachment UI gate: clean');
}catch(e){await page?.screenshot({path:join(scratch,'failed.png')}).catch(()=>{});console.error(e);throw e;}
finally{
  for(const id of sessions)await api('close_session',{id}).catch(()=>{});
  await browser?.close().catch(()=>{});
  for(const child of children)try{process.kill(-child.pid,'SIGKILL');}catch{}
  // Keep screenshots and fixture state, not a full binary pair per run.
  for(const name of ['bridge','khor'])rmSync(join(scratch,name),{force:true});
  await usage?.close();
}
