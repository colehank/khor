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
const scratch=process.env.CONTENT_CHECK_DIR||join(tmpdir(),'khor-content-ui-check');
const target=process.env.CARGO_TARGET_DIR||join(repo,'target');
const port=Number(process.env.CONTENT_VITE_PORT||1480), bridgePort=Number(process.env.CONTENT_BRIDGE_PORT||1481);
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
  const out=process.env.CONTENT_UI_OUTPUT||join(scratch,'ui');mkdirSync(out,{recursive:true});
  const results=[],shots=[],failures=[];
  const verify=(name,d)=>{
    if(d.error)return false;
    if(name==='attachment-icons')return d.heading!==word('files-add')&&d.icons.length===2&&d.icons.every(x=>x.svg.width===16&&x.svg.height===16);
    if(name==='attachment-return')return d.sources>0;
    if(name==='workspace-focus')return d.placeholder===word('tree-search')&&d.field.w>=180&&d.name.display==='none'&&d.face.display==='none'&&d.field.font===d.name.font;
    if(name==='workspace-back')return d.query==='notes';
    if(name==='history-delete')return d.status===200&&d.remains===0;
    if(name==='browser-menu')return d.items.length>=2&&d.menu.bottom<=d.viewport&&d.items.every(x=>x.hit);
    if(name==='browser-picker-geometry')return Math.abs(d.address.centerY-d.sessionFace.centerY)<1.5&&d.address.font===d.session.font;
    if(name==='browser-document-layout')return !d.twoLine&&d.titleWidth>=40;
    if(name==='safe-area-keyboard')return d.panelTop>=parseFloat(d.safeTop)+7.9;
    if(name==='responsive')return d.inputWidth>=88&&!d.overflow; // Two touch widths after the exit face and clear button at 320.
  };
  const note=(name,data)=>{results.push({name,...data});console.log(name,JSON.stringify(data)); const ok=verify(name,data);if(ok===false)failures.push(name+': '+JSON.stringify(data));};
  const shot=async name=>{const path=join(out,name+'.png');await page.screenshot({path});shots.push(path);};
  const step=async(name,fn)=>{try{await fn();}catch(e){note(name,{error:String(e)});await shot('error-'+name).catch(()=>{});}};
  const remote={...device,id:'a'.repeat(64),name:'audit-remote',shown:'远端 A',me:false,hop:'direct',hosts:true,paused:false};
  await page.route('**/devices',r=>r.fulfill({json:[device,remote]}));
  await api('recent_visit',{kind:'page',machine:device.name,place:'https://known.example/'});
  let orphanDeleted=false;page.on('response',r=>{if(r.url().endsWith('/forget_recent')&&r.ok()&&r.request().postDataJSON()?.device==='b'.repeat(64))orphanDeleted=true;});
  const orphan={device:'b'.repeat(64),name:'bbbbbbbb',place:'https://orphan.example/a',at_ms:Date.now()};
  await page.route('**/recents',async r=>{if(r.request().postDataJSON()?.kind==='page')await r.fulfill({json:[...(orphanDeleted?[]:[orphan]),...await api('recents',{kind:'page',limit:12})]});else await r.continue();});
  await page.reload();
  await page.setViewportSize({width:390,height:844});
  if(!(await page.locator('[data-deck-title]').isVisible()))await page.locator('[data-session-title]').filter({hasText:/^Content Alpha$/}).click();
  await page.locator('[data-deck-title]').waitFor();await page.evaluate(()=>document.documentElement.dataset.theme='dark');
  await step('attachments',async()=>{
    const keys=page.locator('[data-key-row-switch]:visible');if(await keys.getAttribute('aria-expanded')!=='true')await keys.click();
    await page.locator('[data-term-key=attach]:visible').click();await page.locator('[data-files-machine]').first().waitFor();
    note('attachment-icons',await page.locator('[data-files-panel]').evaluate(el=>({heading:el.querySelector('h3')?.textContent,icons:[...el.querySelectorAll('[data-files-pick]')].map(b=>({kind:b.dataset.filesPick,svg:(s=>({width:s.width,height:s.height}))(b.querySelector('svg').getBoundingClientRect())}))})));
    await shot('attachments-home');
    await page.locator('[data-files-machine]').first().click();await page.locator('[data-files-up]').waitFor();
    const homePath=await page.locator('[data-files-panel] h3+p').textContent();await page.locator('[data-files-up]').click();
    note('attachment-return',{opened:homePath,after:await page.locator('[data-files-panel] h3+p').textContent().catch(()=>null),sources:await page.locator('[data-files-machine]').count()});
    await page.locator('[data-term-key=attach]:visible').click();
  });
  await page.locator('[data-deck-title]').click();
  const picker=()=>page.locator(`[data-content-picker="${sessions[0]}"]:visible`);
  await picker().locator('[data-content-search]').waitFor();
  await step('workspace-focus',async()=>{
    const field=picker().locator('[data-content-search]');const placeholder=await field.getAttribute('placeholder');await field.focus();
    note('workspace-focus',await picker().evaluate((el,placeholder)=>{const input=el.querySelector('[data-content-search]'),name=el.querySelector('.content-owner-name'),face=el.querySelector('.content-owner-face');const r=x=>{const b=x.getBoundingClientRect();return {w:b.width,h:b.height,font:getComputedStyle(x).fontSize,display:getComputedStyle(x).display};};return {placeholder,field:r(input),name:r(name),face:r(face)};},placeholder));
    await field.fill('notes');
    for(const init of [{isComposing:true},{keyCode:229}]){
      await field.evaluate((el,init)=>el.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true,...init})),init);
      assert(await field.evaluate(el=>document.activeElement===el),'IME confirmation blurred the directory search');
      assert.equal(await field.inputValue(),'notes');
    }
    await shot('workspace-editing');
    await picker().getByText('notes.md',{exact:true}).click();await shown().locator('.cm-content').waitFor();
    await shown().locator('[data-file-back]').click();note('workspace-back',{query:await field.inputValue()});
  });
  await picker().getByRole('radio',{name:word('browser-tab'),exact:true}).click();
  await step('orphan-exit',async()=>{
    await picker().locator('[data-place="https://orphan.example/a"]').click();
    await page.locator('[data-omni-item^="m:"]').first().waitFor();
    await page.setViewportSize({width:1100,height:844});await picker().locator('[data-desktop-picker]').waitFor();await picker().locator('[data-page-field][data-state=editing]').waitFor();
    await page.setViewportSize({width:390,height:844});await picker().locator('[data-page-address]').waitFor();
    const input=picker().locator('[data-page-address]');
    assert.equal(await input.inputValue(),orphan.place,'orphan address lost across width change');
    await page.locator(`[data-omni-item="m:${device.id}"]`).click();
    await shown().locator('[data-content-page-more]').waitFor();
    assert.equal(await shown().locator('.content-owner-title').textContent(),'orphan.example/a');
    await shown().locator('[data-content-page-more]').click();await page.getByRole('menuitem',{name:word('content-find'),exact:true}).click();
    await picker().locator('[data-place="https://orphan.example/a"]').click();
    await picker().locator('[data-page-address]').fill('http://localhost:'+port+'/revised');
    await picker().locator('[data-page-machine-face]').tap();
    await page.locator(`[data-omni-item="m:${device.id}"]`).click();
    await shown().locator('[data-content-page-more]').waitFor();
    assert.equal(await shown().locator('.content-owner-title').textContent(),'localhost:'+port+'/revised','exit choice opened obsolete historical address');
    await shown().locator('[data-content-page-more]').click();await page.getByRole('menuitem',{name:word('content-find'),exact:true}).click();
  });
  await step('history-delete',async()=>{
    await picker().locator('[data-place="https://orphan.example/a"]').click();await shot('orphan-recents');
    for(const place of ['https://known.example/','https://orphan.example/a']){
      const id=place===orphan.place?orphan.device:device.id;
      const button=page.locator(`[data-forget-device="${id}"][data-forget-place="${place}"]:visible`);
      const [r]=await Promise.all([page.waitForResponse(r=>r.url().endsWith('/forget_recent')&&r.request().postDataJSON()?.device===id),button.click()]);
      const body=await r.text();if(r.ok())await button.waitFor({state:'detached'});
      note('history-delete',{place,status:r.status(),body,remains:await button.count(),alert:await picker().locator('[role=alert]').count()});
    }

  });
  await step('browser-picker-menu',async()=>{
    const input=picker().locator('[data-page-address]');await input.focus();await picker().locator('[data-page-machine-face]').tap();
    await page.locator('[data-omni-item^="m:"]').first().waitFor({state:'attached'});
    note('browser-menu',await page.evaluate(()=>{const menu=document.querySelector('[data-omni-menu]'),r=menu.getBoundingClientRect();return {viewport:innerHeight,menu:{y:r.y,bottom:r.bottom,height:r.height},items:[...menu.querySelectorAll('[data-omni-item]')].map(e=>{const b=e.getBoundingClientRect(),hit=document.elementFromPoint(b.x+b.width/2,b.y+b.height/2);return {text:e.textContent,y:b.y,bottom:b.bottom,hit:!!hit&&(hit===e||e.contains(hit))};})};}));
    await shot('browser-machine-menu');await input.press('Escape');await input.press('Escape');
  });
  await step('browser-picker-geometry',async()=>{
    const input=picker().locator('[data-page-address]');await input.focus();await input.fill('http://localhost:'+port+'/audit');await input.blur();
    note('browser-picker-geometry',await picker().evaluate(el=>{const r=e=>{const b=e.getBoundingClientRect();return {x:b.x,y:b.y,w:b.width,h:b.height,centerY:b.y+b.height/2,font:getComputedStyle(e).fontSize,lineHeight:getComputedStyle(e).lineHeight};};return {session:r(el.querySelector('.content-owner-name')),sessionFace:r(el.querySelector('.content-owner-face')),machine:r(el.querySelector('[data-page-machine-face]')),address:r(el.querySelector('[data-page-address]'))};}));await shot('browser-picker-address');
    await input.focus();await input.press('Enter');await shown().locator('[data-content-page-more]').waitFor();
    note('browser-document-layout',await shown().evaluate(el=>{const identity=el.querySelector('.mobile-file-identity .content-owner-identity');return {twoLine:!!identity,display:identity&&getComputedStyle(identity).display,rows:identity&&getComputedStyle(identity).gridTemplateRows,title:el.querySelector('.content-owner-title')?.textContent,titleWidth:el.querySelector('.content-owner-title')?.getBoundingClientRect().width};}));await shot('browser-document');
    await shown().locator('[data-content-page-more]').click();await page.getByRole('menuitem',{name:word('content-keep'),exact:true}).click();
    await until(async()=>!await shown().getAttribute('data-content-preview'));
    await shown().locator('.mobile-page-identity').click();
    await shown().locator('[data-page-address]').evaluate(el=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
    assert(await shown().locator('[data-page-address]').evaluate(el=>el.selectionStart===0&&el.selectionEnd===el.value.length),'opening the address did not select it');
    const draft='http://localhost:'+port+'/draft';await shown().locator('[data-page-address]').fill(draft);
    await page.setViewportSize({width:1100,height:844});await shown().locator('[data-content-page-bar]').waitFor();
    assert.equal(await shown().locator('[data-page-address]').inputValue(),draft,'document draft lost on desktop');
    await page.setViewportSize({width:390,height:844});await shown().locator('.mobile-content-page-editor').waitFor();
    assert.equal(await shown().locator('[data-page-address]').inputValue(),draft,'document draft lost on mobile');
    await page.setViewportSize({width:1100,height:844});await shown().locator('[data-content-page-bar]').waitFor();
    const id=await shown().getAttribute('data-session-content'),redirected='http://localhost:'+port+'/redirected';
    await api('tabs_go',{kind:'content',id,device:device.id,url:redirected,view:'page'});
    await page.locator(`[data-content-tab="${id}"] [role=tab][title="${redirected}"]`).waitFor();
    assert.equal(await shown().locator('[data-page-address]').inputValue(),draft,'page navigation overwrote an active address draft');
    await page.setViewportSize({width:390,height:844});await shown().locator('.mobile-content-page-editor').waitFor();
    await shown().locator('[data-page-address]').press('Escape');
    assert.equal(await shown().locator('.content-owner-title').textContent(),'localhost:'+port+'/redirected','leaving editing did not restore the latest page');
    await shown().locator('[data-content-page-more]').click();await page.getByRole('menuitem',{name:word('content-find'),exact:true}).click();
  });
  await step('safe-area-keyboard',async()=>{
    await page.setViewportSize({width:390,height:500});await page.evaluate(()=>document.documentElement.style.setProperty('--safe-top','59px'));
    await picker().locator('[data-page-address]').focus();await page.locator('[data-content-panel][data-detent=full]').waitFor();
    note('safe-area-keyboard',await page.locator('[data-content-panel]').evaluate(el=>{const b=el.getBoundingClientRect(),head=el.querySelector('.mobile-content-picker-head').getBoundingClientRect();return {height:innerHeight,panelTop:b.y,safeTop:getComputedStyle(document.documentElement).getPropertyValue('--safe-top'),headerTop:head.y,headerBottom:head.bottom};}));await shot('keyboard-safe-area');
    await page.setViewportSize({width:390,height:844});await picker().locator('[data-page-address]').blur();
  });
  await step('responsive',async()=>{
    for(const width of [320,390,430]){
      await page.setViewportSize({width,height:844});await picker().locator('[data-page-address]').focus();
      note('responsive',await picker().evaluate((el,width)=>{const b=el.querySelector('[data-page-address]').getBoundingClientRect();return {width,inputWidth:b.width,overflow:document.documentElement.scrollWidth>innerWidth};},width));await shot('browser-width-'+width);await picker().locator('[data-page-address]').blur();
    }
  });
  await page.evaluate(()=>document.documentElement.dataset.theme='light');await shot('browser-light');
  writeFileSync(join(out,'ui-results.json'),JSON.stringify({results,shots,errors},null,2));assert.deepEqual(errors,[]);assert.deepEqual(failures,[],failures.join('\n'));console.log('content UI gate: clean',out);
}catch(e){await page?.screenshot({path:join(scratch,'failed.png')}).catch(()=>{});console.error(e);throw e;}
finally{
  for(const id of sessions)await api('close_session',{id}).catch(()=>{});
  await browser?.close();
  for(const child of children)try{process.kill(-child.pid,'SIGKILL');}catch{}
  await usage?.close();
}
