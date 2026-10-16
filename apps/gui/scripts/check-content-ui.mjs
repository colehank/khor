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
  const results=[],shots=[],failures=[],gestureTracks=[];let stopped=false;
  const verify=(name,d)=>{
    if(d.error)return false;
    if(name==='attachment-icons')return d.heading!==word('files-add')&&d.icons.length===2&&d.icons.every(x=>x.svg.width===16&&x.svg.height===16);
    if(name==='attachment-return')return d.sources>0;
    if(name==='workspace-focus')return d.placeholder===word('tree-search')&&d.width>=180&&d.leading>=12&&d.identity===0&&d.source===0&&d.tabs===0&&d.end===1;
    if(name==='workspace-back')return d.query==='notes';
    if(name==='history-delete')return d.status===200&&d.remains===0;
    if(name==='browser-menu')return d.items.length>=2&&d.menu.bottom<=d.viewport&&d.items.every(x=>x.hit);
    if(name==='browser-picker-geometry')return Math.abs(d.prompt.centerY-d.source.centerY)<1.5&&d.prompt.font===d.bodyFont&&!d.focused;
    if(name==='browser-document-layout')return !d.twoLine&&d.titleWidth>=40;
    if(name==='safe-area-keyboard')return d.panelTop>=parseFloat(d.safeTop)+7.9;
    if(name==='responsive')return d.inputWidth>=88&&!d.overflow; // Two touch widths after the exit face and clear button at 320.
  };
  const note=(name,data)=>{results.push({name,...data});console.log(name,JSON.stringify(data)); const ok=verify(name,data);if(ok===false)failures.push(name+': '+JSON.stringify(data));};
  const shot=async name=>{const path=join(out,name+'.png');await page.screenshot({path});shots.push(path);};
  const step=async(name,fn)=>{if(stopped)return;try{await fn();console.log('PASS',name);if(process.env.CONTENT_STOP_AFTER===name)stopped=true;}catch(e){note(name,{error:String(e)});await shot('error-'+name).catch(()=>{});if(process.env.CONTENT_FAIL_FAST==='1'){writeFileSync(join(out,'ui-results.json'),JSON.stringify({results,shots,errors},null,2));throw e;}}};
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
  await page.locator('[data-session-deck] [data-deck-title]').click();
  const picker=()=>page.locator(`[data-content-picker="${sessions[0]}"]:visible`);
  const visibleTitle=()=>shown().locator('[data-band-seg="cur"] .content-owner-title');
  const pickerBand=()=>picker().locator('[data-content-deck] [data-deck-band]:visible');
  const editPicker=async()=>{
    const address=picker().locator('[data-page-address]:visible');
    if(await address.count())return address;
    await pickerBand().locator('[data-band-seg="cur"] button').click();
    const input=picker().locator('input:visible');await input.waitFor();return input;
  };
  const endPicker=async()=>{const end=picker().locator('[data-content-edit-end]:visible');if(await end.count())await end.click();await pickerBand().waitFor();};
  const chooseSource=async source=>{
    await endPicker();await picker().locator('[data-content-scope]:visible').click();
    await page.locator(`[data-content-source="${source}"]`).click();
    await pickerBand().waitFor();
    assert.equal(await picker().locator('input:visible').count(),0,'choosing source opened its keyboard');
  };
  const touch=await context.newCDPSession(page);
  const drag=async(locator,share,{cancel=false,dy=0}={})=>{
    const box=await locator.boundingBox();assert(box&&box.width>0,'gesture surface is missing');
    if(process.env.CONTENT_DEBUG_GESTURES==='1')await page.evaluate(()=>{
      window.__contentGesture=[];
      if(window.__contentGestureListening)return;window.__contentGestureListening=true;
      for(const type of ['pointerdown','pointermove','pointerup','pointercancel','gotpointercapture','lostpointercapture','touchstart','touchmove','touchend','touchcancel'])window.addEventListener(type,e=>{const t=e.target instanceof Element?e.target:null,b=t?.closest('[data-deck-band]');window.__contentGesture.push({type,target:t?.tagName,band:!!b,hasCapture:b&&'pointerId'in e?b.hasPointerCapture(e.pointerId):null,selection:window.getSelection()?.toString(),x:e.clientX,y:e.clientY,prevented:e.defaultPrevented,transform:b?.querySelector('[data-band-seg=cur]')?.style.transform});},true);
    });
    const travel=Math.min(Math.abs(share)*box.width,box.width*.72)*Math.sign(share);
    const x=box.x+box.width*(travel<0?.82:.18),y=box.y+box.height/2;
    await touch.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x,y}]});
    for(const part of [.15,.35,.6,1])await touch.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:x+travel*part,y:y+dy*part}]});
    await touch.send('Input.dispatchTouchEvent',{type:cancel?'touchCancel':'touchEnd',touchPoints:[]});
    if(process.env.CONTENT_DEBUG_GESTURES==='1'){
      gestureTracks.push({share,cancel,box,events:await page.evaluate(()=>window.__contentGesture)});
      writeFileSync(join(out,'gesture-tracks.json'),JSON.stringify(gestureTracks,null,2));
    }
  };
  const resting=async locator=>{
    await until(()=>locator.evaluate(el=>[...el.querySelectorAll('[data-band-seg]')].every(s=>!s.style.transform&&!s.style.display&&!s.style.maxWidth)));
    assert.equal(await locator.locator('[data-band-seg="cur"]').count(),1);
  };
  await pickerBand().waitFor();
  await step('workspace-heading-source',async()=>{
    const head=picker().locator('.mobile-content-location');await head.waitFor();
    assert.equal(await head.locator('[data-side-face]').count(),1,'workspace header does not use its machine avatar');
    assert.equal((await head.locator('strong').textContent()).trim(),'project-alpha');
    assert(!(await head.innerText()).includes(device.name),'workspace header repeats its machine name');
    assert.equal(await picker().locator('.mobile-content-picker-head').count(),0,'fixed source segment returned');
    await picker().locator('[data-content-scope]:visible').click();
    const menu=page.getByRole('menu');await menu.waitFor();
    const options=menu.locator('[data-content-source]');assert.equal(await options.count(),2);
    for(const [i,label] of [word('workspaces-tab'),word('browser-tab')].entries()){
      assert.equal(await options.nth(i).locator('svg').count(),1,'source menu is missing its icon');
      assert((await options.nth(i).textContent()).includes(label));
    }
    const alignment=await options.evaluateAll(els=>els.map(el=>({x:el.querySelector('svg').getBoundingClientRect().x,w:el.querySelector('svg').getBoundingClientRect().width})));
    assert.equal(alignment[0].x,alignment[1].x);assert.equal(alignment[0].w,alignment[1].w);
    await page.locator('[data-content-source="workspace"]').click();
    assert.equal(await picker().locator('input:visible').count(),0);
    await shot('workspace-heading-source');
  });
  await step('workspace-focus',async()=>{
    const field=await editPicker();assert.equal(await field.getAttribute('placeholder'),word('tree-search'));
    note('workspace-focus',await field.evaluate(input=>{const pill=input.closest('.mobile-content-pill'),deck=input.closest('[data-content-deck]'),b=input.getBoundingClientRect(),p=pill.getBoundingClientRect();return {placeholder:input.placeholder,width:b.width,leading:b.x-p.x,identity:pill.querySelectorAll('.content-owner-identity').length,source:pill.querySelectorAll('[data-content-scope]').length,tabs:deck.querySelectorAll('[data-content-overview-button]').length,end:deck.querySelectorAll('[data-content-edit-end]').length};}));
    await field.fill('中文');
    await field.evaluate(el=>el.dispatchEvent(new CompositionEvent('compositionstart',{bubbles:true,data:'zhongwen'})));
    for(const init of [{isComposing:true},{keyCode:229}]){
      await field.evaluate((el,init)=>el.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true,...init})),init);
      assert(await field.evaluate(el=>document.activeElement===el),'IME confirmation blurred the directory search');
      assert.equal(await field.inputValue(),'中文');
    }
    assert.equal(await pickerBand().count(),0,'editing retained a page-swipe surface');
    await drag(field,.45);assert.equal(await field.inputValue(),'中文');
    assert.equal(await picker().locator('.mobile-content-workspace:not([hidden])').count(),1,'text selection switched the source');
    await field.evaluate(el=>el.dispatchEvent(new CompositionEvent('compositionend',{bubbles:true,data:'中文'})));
    const clear=picker().locator('[data-content-search-clear]');await clear.click();assert.equal(await field.inputValue(),'');assert(await field.evaluate(el=>document.activeElement===el));
    await field.fill('notes');await shot('workspace-editing');
    await picker().getByText('notes.md',{exact:true}).click();await shown().locator('.cm-content').waitFor();
    assert.equal(await shown().getAttribute('data-content-preview'),null,'file still requires keeping');
    await shown().locator('[data-file-back]').click();
    note('workspace-back',{query:await pickerBand().locator('[data-band-seg="cur"]').innerText()});
  });
  await step('source-pill-swipe',async()=>{
    await endPicker();
    const initial=await pickerBand().locator('[data-band-seg="cur"]').innerText();
    await drag(pickerBand(),-.65,{cancel:true});await resting(pickerBand());
    assert.equal(await pickerBand().locator('[data-band-seg="cur"]').innerText(),initial,'cancel switched the source');
    await drag(pickerBand(),-.65);
    await picker().locator('.mobile-content-browser:not([hidden])').waitFor();await resting(pickerBand());
    assert.equal(await picker().locator('input:visible').count(),0,'source swipe opened the keyboard');
    await chooseSource('workspace');await chooseSource('browser');
    await shot('browser-source-resting');
  });
  await step('orphan-exit',async()=>{
    await picker().locator('[data-place="https://orphan.example/a"]').click();
    await page.locator('[data-omni-item^="m:"]').first().waitFor();
    await page.setViewportSize({width:1100,height:844});await picker().locator('[data-desktop-picker]').waitFor();await picker().locator('[data-page-field][data-state=editing]').waitFor();
    await page.setViewportSize({width:390,height:844});await picker().locator('[data-page-address]').waitFor();
    const input=picker().locator('[data-page-address]');
    assert.equal(await input.inputValue(),orphan.place,'orphan address lost across width change');
    await page.locator(`[data-omni-item="m:${device.id}"]`).click();
    await shown().locator('[data-content-page-more]').waitFor();
    assert.equal(await visibleTitle().textContent(),'orphan.example/a');
    await shown().locator('[data-content-page-more]').click();await page.getByRole('menuitem',{name:word('content-find'),exact:true}).click();
    await picker().locator('[data-place="https://orphan.example/a"]').click();
    await picker().locator('[data-page-address]').fill('http://localhost:'+port+'/revised');
    await picker().locator('[data-page-machine-face]').tap();
    await page.locator(`[data-omni-item="m:${device.id}"]`).click();
    await shown().locator('[data-content-page-more]').waitFor();
    assert.equal(await visibleTitle().textContent(),'localhost:'+port+'/revised','exit choice opened obsolete historical address');
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
    await endPicker();
  });
  await step('browser-picker-menu',async()=>{
    const input=await editPicker();await picker().locator('[data-page-machine-face]').tap();
    await page.locator('[data-omni-item^="m:"]').first().waitFor({state:'attached'});
    note('browser-menu',await page.evaluate(()=>{const menu=document.querySelector('[data-omni-menu]'),r=menu.getBoundingClientRect();return {viewport:innerHeight,menu:{y:r.y,bottom:r.bottom,height:r.height},items:[...menu.querySelectorAll('[data-omni-item]')].map(e=>{const b=e.getBoundingClientRect(),hit=document.elementFromPoint(b.x+b.width/2,b.y+b.height/2);return {text:e.textContent,y:b.y,bottom:b.bottom,hit:!!hit&&(hit===e||e.contains(hit))};})};}));
    await shot('browser-machine-menu');await input.press('Escape');await input.press('Escape');await pickerBand().waitFor();
  });
  await step('browser-picker-geometry',async()=>{
    let input=await editPicker();await input.fill('http://localhost:'+port+'/audit');await endPicker();
    note('browser-picker-geometry',await picker().evaluate(el=>{const r=e=>{const b=e.getBoundingClientRect();return {x:b.x,y:b.y,w:b.width,h:b.height,centerY:b.y+b.height/2,font:getComputedStyle(e).fontSize};};return {prompt:r(el.querySelector('.mobile-content-browser:not([hidden]) .content-picker-prompt')),source:r(el.querySelector('.mobile-content-browser:not([hidden]) [data-content-scope]')),bodyFont:getComputedStyle(el.querySelector('.mobile-content-browser:not([hidden]) .mobile-content-pill')).fontSize,focused:el.contains(document.activeElement)&&document.activeElement.tagName==='INPUT'};}));await shot('browser-picker-address');
    input=await editPicker();await input.press('Enter');await shown().locator('[data-content-page-more]').waitFor();
    note('browser-document-layout',await shown().evaluate(el=>{const identity=el.querySelector('.mobile-file-identity .content-owner-identity');return {twoLine:!!identity,title:el.querySelector('[data-band-seg="cur"] .content-owner-title')?.textContent,titleWidth:el.querySelector('[data-band-seg="cur"] .content-owner-title')?.getBoundingClientRect().width};}));await shot('browser-document');
    assert.equal(await shown().getAttribute('data-content-preview'),null,'webpage still requires keeping');
    await shown().locator('.mobile-page-identity [data-band-seg="cur"] button').click();
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
    await page.locator(`[data-content-tab="${id}"] [role=tab][title="${redirected}"]`).waitFor({state:'attached'});
    assert.equal(await shown().locator('[data-page-address]').inputValue(),draft,'page navigation overwrote an active address draft');
    await page.setViewportSize({width:390,height:844});await shown().locator('.mobile-content-page-editor').waitFor();
    await shown().locator('[data-page-address]').press('Escape');
    assert.equal(await visibleTitle().textContent(),'localhost:'+port+'/redirected','leaving editing did not restore the latest page');
    await shown().locator('[data-content-page-more]').click();await page.getByRole('menuitem',{name:word('content-find'),exact:true}).click();
  });
  await step('safe-area-keyboard',async()=>{
    await page.setViewportSize({width:390,height:500});await page.evaluate(()=>document.documentElement.style.setProperty('--safe-top','59px'));
    await editPicker();await page.locator('[data-content-panel][data-detent=full]').waitFor();
    note('safe-area-keyboard',await page.locator('[data-content-panel]').evaluate(el=>{const b=el.getBoundingClientRect();return {height:innerHeight,panelTop:b.y,safeTop:getComputedStyle(document.documentElement).getPropertyValue('--safe-top')};}));await shot('keyboard-safe-area');
    await endPicker();await page.setViewportSize({width:390,height:844});
  });
  await step('responsive',async()=>{
    for(const width of [320,390,430]){
      await page.setViewportSize({width,height:844});await editPicker();
      note('responsive',await picker().evaluate((el,width)=>{const b=el.querySelector('.mobile-content-browser:not([hidden]) [data-page-address]').getBoundingClientRect();return {width,inputWidth:b.width,overflow:document.documentElement.scrollWidth>innerWidth};},width));await shot('browser-width-'+width);await endPicker();
    }
  });
  await step('content-pill-navigation',async()=>{
    await page.setViewportSize({width:390,height:844});
    // Opened documents remain tabs without a keep step, in overview order.
    const opened=(await api('tabs_list',{kind:'content'})).tabs.filter(t=>t.session===sessions[0]&&t.view!=='workspace');assert(opened.length>=3);
    await picker().locator('[data-content-overview-button]:visible').click();
    const overview=()=>page.locator('[data-content-overview]:not(.session-content-retained)');
    assert.equal(await overview().locator('input:visible').count(),0,'overview forced keyboard');
    await overview().locator('[data-content-overview-search]').click();
    const search=overview().getByRole('searchbox');await search.fill('notes.md');
    assert.equal(await overview().locator('[data-content-overview-tab]').count(),1,'content search did not filter');
    await search.fill('');await overview().locator('[data-content-edit-end]').click();
    const ids=await overview().locator('[data-content-overview-tab]').evaluateAll(els=>els.map(e=>e.dataset.contentOverviewTab));
    assert.deepEqual(ids,opened.map(t=>t.id),'swipe and overview order differ');
    await overview().locator(`[data-tab-sheet-open="${ids[0]}"]`).click();await shown().locator('[data-deck-band]').waitFor();
    const before=await shown().getAttribute('data-session-content');
    await drag(shown().locator('[data-deck-band]'),-.65,{cancel:true});await resting(shown().locator('[data-deck-band]'));
    assert.equal(await shown().getAttribute('data-session-content'),before,'cancel changed content');
    await drag(shown().locator('[data-deck-band]'),-.65);
    await until(async()=>await shown().getAttribute('data-session-content')===ids[1]);await resting(shown().locator('[data-deck-band]'));
    assert.equal(await shown().locator('[data-page-address]:visible').count(),0,'content swipe opened address editing');
    for(const id of ids.slice(1))await api('tabs_close',{kind:'content',id});
    await until(async()=>await shown().getAttribute('data-session-content')===ids[0]);
    await until(async()=>await shown().locator('[data-band-seg]').count()===1);
    await drag(shown().locator('[data-deck-band]'),-.65);await resting(shown().locator('[data-deck-band]'));
    assert.equal(await shown().getAttribute('data-session-content'),ids[0],'one document did not spring back');
    await shot('content-pill-single');
    await shown().locator('[data-file-back]').click();await overview().waitFor();await overview().locator('[data-tab-sheet-new]').click();await chooseSource('browser');
  });
  await step('latest-position-and-session-search',async()=>{
    const text=await pickerBand().locator('[data-band-seg="cur"]').innerText();
    await page.locator('.mobile-content-scrim').click({position:{x:12,y:12}});
    await page.locator('[data-session-deck] [data-deck-title]').click();await pickerBand().waitFor();
    assert.equal(await picker().locator('.mobile-content-browser:not([hidden])').count(),1,'reopening restored a document instead of browser entrance');
    assert.equal(await pickerBand().locator('[data-band-seg="cur"]').innerText(),text,'browser draft was lost on reopen');
    await page.locator('.mobile-content-scrim').click({position:{x:12,y:12}});
    await page.locator('[data-deck-tabs]').click();const list=page.locator('[data-content-sessions]');await list.waitFor();
    assert.equal(await list.locator('input:visible').count(),0,'session overview forced keyboard');
    const prompt=list.locator('[data-session-overview-search]');assert.equal((await prompt.innerText()).trim(),word('search-sessions'));
    await prompt.click();const search=list.getByRole('searchbox');await search.fill('Beta');
    assert.equal(await list.locator('[data-tab-sheet-row]').count(),1);assert((await list.locator('[data-tab-sheet-name]').innerText()).includes('Beta'));
    await search.fill('中文');assert.equal(await list.locator('[data-tab-sheet-row]').count(),0);
    await list.locator('[data-content-search-clear]').click();assert.equal(await list.locator('[data-tab-sheet-row]').count(),2);
    await shot('session-overview-search');
  });
  await page.evaluate(()=>document.documentElement.dataset.theme='light');await shot('browser-light');
  writeFileSync(join(out,'ui-results.json'),JSON.stringify({results,shots,errors},null,2));assert.deepEqual(errors,[]);assert.deepEqual(failures,[],failures.join('\n'));console.log('content UI gate: clean',out);
}catch(e){await page?.screenshot({path:join(scratch,'failed.png')}).catch(()=>{});console.error(e);throw e;}
finally{
  for(const id of sessions)await api('close_session',{id}).catch(()=>{});
  await browser?.close().catch(()=>{});
  for(const child of children)try{process.kill(-child.pid,'SIGKILL');}catch{}
  // Keep screenshots and fixture state, not a full binary pair per run.
  for(const name of ['bridge','khor'])rmSync(join(scratch,name),{force:true});
  await usage?.close();
}
