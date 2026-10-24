// Real App paths with the private bridge; no supplier requests.
import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {chromium} from 'playwright-core';
const base=process.env.LAYOUT_LAB_URL??'http://localhost:23795',shots='/Volumes/disk0/khor-gui-session-layout-proof/fan-arc';
mkdirSync(shots,{recursive:true});
const browser=await chromium.launch({channel:'chrome',headless:process.env.PW_HEADLESS==='1'});let page;
try{
 page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));let checks=0;
 await page.request.get(`${base}/__ai_preview/reset`);
 const selected=()=>page.evaluate(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active);
 const settle=()=>page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>Number.isFinite(a.effect?.getComputedTiming().endTime)).map(a=>a.finished.catch(()=>{}))));
 const open=async(session='gui/ai-elements-review',theme='light',keyboard='0',width=390,height=844)=>{
  await page.setViewportSize({width,height});await page.goto(`${base}/?${new URLSearchParams({bridge:new URL(base).port,labSession:session,theme,keyboard,phone:width<760?'1':'0',deck:'fan'})}`);
  await page.locator('[data-composer-text]').waitFor();if(width<760)await page.locator('[data-lab-fan-avatar]').waitFor();await settle();
 };
 for(const session of ['gui/ai-elements-review','gui/codex-review','wide/all'])for(const theme of ['light','dark'])for(const keyboard of ['0','1']){
  await open(session,theme,keyboard);
  assert.equal(await page.locator('[data-lab-model-header] .glass').count(),1,'new session is the sole header glass');
  assert.equal(await page.locator('[data-lab-model-header] [data-lab-header-title]').count(),0,'session title leaves the header');
  const material=await page.locator('[data-lab-model-trigger]').evaluate(el=>({background:getComputedStyle(el).backgroundColor,shadow:getComputedStyle(el).boxShadow}));assert.equal(material.background,'rgba(0, 0, 0, 0)');assert.equal(material.shadow,'none');
  assert.equal(await page.locator('[data-lab-compose-avatar] .glass').count(),0,'avatar shares the capsule');
  const head=await page.locator('[data-lab-model-header]').boundingBox(),content=await page.locator('[data-session-content]').boundingBox(),cap=await page.locator('[data-chat-box]').boundingBox();
  assert(head.y+head.height<=content.y+1&&content.y+content.height<=cap.y+1);
  const a=await page.locator('[data-lab-compose-avatar]').boundingBox(),plus=await page.locator('[data-lab-add]').boundingBox(),field=await page.locator('[data-composer-text]').boundingBox();
  assert(plus.x+plus.width<=field.x+1&&field.x+field.width<=a.x+1,'add and message lead; avatar occupies the far right');
  await page.locator('[data-composer-text]').fill('这是一段换行后仍应独占完整文字空间的草稿。\n第二行继续输入。');
  await page.locator('[data-lab-compose-expanded]').waitFor();
  const expanded=await page.locator('[data-chat-box]').boundingBox(),text=await page.locator('[data-composer-text]').boundingBox(),avatar=await page.locator('[data-lab-compose-avatar]').boundingBox(),send=await page.locator('[data-chat-send]').boundingBox();
  assert(text.y+text.height<=avatar.y+1&&text.width>expanded.width*.85,'full-width text above controls');assert(send.x+send.width<=avatar.x+1,'send sits before the right avatar');
  assert(Math.abs(avatar.y+avatar.height-send.y-send.height)<2&&Math.abs(cap.y+cap.height-expanded.y-expanded.height)<2,'bottom controls stay anchored');
  await page.locator('[data-composer-text]').fill('');await page.locator('[data-lab-compose-expanded]').waitFor({state:'hidden'});
  await page.locator('[data-lab-model-trigger]').click();await page.locator('[data-lab-option="model"]').waitFor();
  assert.equal(await page.locator('[data-tab-sheet]').count(),0,'model menu is independent of navigation');
  const menu=await page.locator('[data-lab-preferences]').boundingBox(),visible=await page.evaluate(()=>visualViewport.height);assert(menu.y>=0&&menu.y+menu.height<=visible+1);
  await page.keyboard.press('Escape');await page.locator('[data-lab-preferences]').waitFor({state:'hidden'});
  await page.screenshot({animations:'disabled',path:`${shots}/${session==='wide/all'?'khor':session.includes('codex')?'codex':'claude'}-${theme}-${keyboard}.png`});checks++;
 }
 for(const [width,height] of [[320,844],[430,932],[667,375]]){
  await open('gui/codex-review','dark','1',width,height);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  await page.locator('[data-composer-text]').fill('长草稿'.repeat(40));await page.locator('[data-lab-compose-expanded]').waitFor();
  const composer=await page.locator('[data-chat-box]').boundingBox(),visible=await page.evaluate(()=>visualViewport.height);assert(composer.y>=0&&composer.y+composer.height<=visible+1);
  await page.screenshot({animations:'disabled',path:`${shots}/width-${width}.png`});checks++;
 }
 await page.setViewportSize({width:1280,height:800});await page.locator('[data-lab-compose-input]').waitFor({state:'hidden'});
 assert.equal(await page.locator('[data-chat-box]').evaluate(el=>getComputedStyle(el).display),'flex');assert.equal(await page.locator('[data-lab-composer-session-slot]').count(),0);assert.equal(await page.locator('[data-composer-text]').inputValue(),'长草稿'.repeat(40));checks++;
 await open('gui/codex-review','light','0',1280,800);assert.equal(await page.locator('[data-lab-model-header], [data-lab-compose-input], [data-lab-composer-session-slot]').count(),0,'wide composer and tabs remain unchanged');checks++;
 await open();assert.equal(await page.locator('[data-lab-effort]').textContent(),'标准');
 await page.locator('[data-lab-model-trigger]').click();await page.getByRole('menuitem',{name:/思考方式/}).click();await page.locator('[data-option-value="deep"]').click();await page.locator('[data-lab-preferences]').waitFor({state:'hidden'});
 await page.waitForFunction(()=>document.querySelector('[data-lab-effort]')?.textContent==='深入');checks++;
 await open('wide/all');await page.waitForFunction(()=>document.querySelector('[data-lab-effort]')?.textContent==='中');
 await page.locator('[data-lab-model-trigger]').click();await page.getByRole('menuitem',{name:/思考强度/}).click();await page.locator('[data-option-value="high"]').click();await page.keyboard.press('Escape');
 await page.waitForFunction(()=>document.querySelector('[data-lab-effort]')?.textContent==='高');checks++;
 await page.locator('[data-lab-new-session]').click();await page.locator('[data-new-session-page]').waitFor();checks++;



 await open();await page.locator('[data-composer-text]').fill('保留 Claude 草稿');
 const avatar=()=>page.locator('[data-lab-fan-avatar]'),fan=()=>page.locator('[data-lab-left-fan]');
 const pick=async id=>{await avatar().click();await page.locator('[data-lab-pill-overview]').click();await page.locator('[data-tab-sheet-open="'+id+'"]').click();await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,id);await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});await avatar().waitFor();};
 await pick('gui/codex-review');await page.locator('[data-composer-text]').fill('保留 Codex 草稿');await pick('gui/ai-elements-review');checks++;
 const begin=async()=>{const a=await avatar().boundingBox(),x=a.x+a.width/2,y=a.y+a.height/2;await page.mouse.move(x,y);await page.mouse.down();return {x,y};};
 const hold=async()=>{const p=await begin();await fan().waitFor();assert.equal(await fan().getAttribute('data-fan-phase'),'fan');await page.mouse.move(p.x-35/Math.SQRT2,p.y-35/Math.SQRT2);return p;};
 const arc=async(p,angle,radius=80)=>page.mouse.move(p.x-radius*Math.cos(angle*Math.PI/180),p.y-radius*Math.sin(angle*Math.PI/180),{steps:3});
 const move=async(p,dx)=>arc(p,45-Math.sign(dx)*Math.min(40,Math.abs(dx)/14*16),35);
 const hot=()=>fan().getAttribute('data-fan-selected');
 const parked=()=>page.waitForFunction(()=>{const e=document.querySelector('[data-lab-left-fan]');return e&&Math.abs(Number(e.dataset.fanPosition)-Math.round(Number(e.dataset.fanPosition)))<.001;});
 const noLateCommit=async id=>{await page.evaluate(async()=>{const at=performance.now();await new Promise(resolve=>{const tick=()=>performance.now()-at>350?resolve():requestAnimationFrame(tick);tick();});});assert.equal(await selected(),id);};
 const ordered=await page.evaluate(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).tabs);
 await page.evaluate(()=>sessionStorage.setItem('khor.lab.session-recents',JSON.stringify(['wide/all','gui/codex-review'])));
 let origin=await hold();assert.equal(Number(await fan().getAttribute('data-fan-total')),ordered.length);assert.equal(await hot(),ordered[0]);
 await move(origin,-14,4);assert.equal(await hot(),ordered[1]);assert.equal(await fan().getAttribute('data-fan-tier'),'1');assert.equal(await fan().getAttribute('data-fan-interval'),'1400');await move(origin,0);await parked();
 assert.equal(await fan().getAttribute('data-fan-direction'),'0');assert.equal(await selected(),ordered[0]);
 assert.equal(await page.locator('.lab-fan-title-meta,.lab-fan-index').count(),0);assert(!(await fan().innerText()).includes('切换会话'));
 const titles=await page.locator('[data-lab-fan-title]').allTextContents();assert.equal(titles.length,5,'five identities all carry their own title');assert(titles.every(Boolean));
 const scales=await page.locator('[data-lab-fan-session]').evaluateAll(es=>es.map(e=>({current:e.hasAttribute('data-selected'),scale:new DOMMatrix(getComputedStyle(e).transform).a})));assert(scales.find(s=>s.current).scale>Math.max(...scales.filter(s=>!s.current).map(s=>s.scale)));
 await page.evaluate(async()=>{const e=document.querySelector('[data-lab-left-fan]'),target=e.dataset.fanSelected,at=performance.now();await new Promise((resolve,reject)=>{const tick=()=>{if(e.dataset.fanSelected!==target)return reject(Error('neutral must pause indefinitely'));if(performance.now()-at>1550)resolve();else requestAnimationFrame(tick);};tick();});});checks++;
 await page.screenshot({path:`${shots}/three-titles.png`});
 await move(origin,-60,20);assert.equal(await fan().getAttribute('data-fan-tier'),'3');await page.waitForFunction(()=>Number(document.querySelector('[data-lab-left-fan]')?.getAttribute('data-fan-pulse'))>=3);await move(origin,0);await parked();const firstTarget=await hot();assert.equal(await avatar().locator('[data-lab-fan-avatar-preview]').getAttribute('data-lab-fan-avatar-preview'),firstTarget);assert.equal(await selected(),ordered[0]);checks++;
 await page.mouse.up();await page.locator('[data-lab-left-fan][data-fan-phase="settling"]').waitFor();assert.equal(await selected(),ordered[0]);await page.locator('[data-lab-left-fan][data-fan-phase="fading"]').waitFor();assert.equal(await selected(),ordered[0]);await fan().waitFor({state:'hidden'});await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,firstTarget);assert.equal(await page.locator('[data-lab-session-pill], [data-tab-sheet]').count(),0);checks++;
 await pick('gui/codex-review');assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Codex 草稿');await pick('gui/ai-elements-review');assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Claude 草稿');checks++;
 // Holding inward keeps circling, including the actually open session.
 origin=await hold();await move(origin,-60,20);await page.waitForFunction(n=>Number(document.querySelector('[data-lab-left-fan]')?.getAttribute('data-fan-pulse'))>=n,ordered.length);await move(origin,0);assert.equal(await hot(),ordered[0]);await page.mouse.up();await fan().waitFor({state:'hidden'});await noLateCommit(ordered[0]);checks++;
 // Reverse is reachable with the small travel left at the right screen edge.
 origin=await hold();await move(origin,14);assert.equal(await hot(),ordered.at(-1));assert.equal(await fan().getAttribute('data-fan-direction'),'-1');await move(origin,0);await parked();await page.screenshot({path:`${shots}/reverse-wrap.png`});await page.keyboard.press('Escape');await page.mouse.up();await noLateCommit(ordered[0]);checks++;
 // The shared arc interpolates cyclically instead of jumping across the set.
 origin=await hold();await move(origin,-14);await page.waitForFunction(()=>{const e=document.querySelector('[data-lab-left-fan]');return e&&Math.abs(Number(e.dataset.fanPosition)-Math.round(Number(e.dataset.fanPosition)))>.05;});await page.keyboard.press('Escape');await page.mouse.up();checks++;
 for(const phase of ['settling','fading']){
  origin=await hold();await move(origin,-14);
  await page.evaluate(phase=>{window.__fanCancelled=false;const el=document.querySelector('[data-lab-left-fan]'),observer=new MutationObserver(()=>{if(el.dataset.fanPhase===phase){observer.disconnect();window.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true}));window.__fanCancelled=true;}});observer.observe(el,{attributes:true,attributeFilter:['data-fan-phase']});},phase);
  await page.mouse.up();await page.waitForFunction(()=>window.__fanCancelled);await fan().waitFor({state:'hidden'});await noLateCommit(ordered[0]);checks++;
 }
 // Y motion at the same X must reverse angular steering.
 origin=await hold();await page.mouse.move(origin.x-70,origin.y-40);assert.equal(await fan().getAttribute('data-fan-direction'),'-1');await page.mouse.move(origin.x-70,origin.y-120);assert.equal(await fan().getAttribute('data-fan-direction'),'1');await page.keyboard.press('Escape');await page.mouse.up();checks++;
 // Outward travel remains available after selecting, with reversible hysteresis.
 origin=await hold();await arc(origin,61);const retained=await hot();await arc(origin,45,145);await page.locator('[data-lab-fan-overview][data-active]').waitFor();assert.equal(await fan().getAttribute('data-fan-direction'),'0');await arc(origin,45,130);assert.equal(await page.locator('[data-lab-fan-overview][data-active]').count(),1);await arc(origin,45,100);assert.equal(await page.locator('[data-lab-fan-overview][data-active]').count(),0);assert.equal(await hot(),retained);await arc(origin,29);assert.equal(await fan().getAttribute('data-fan-direction'),'-1');await page.keyboard.press('Escape');await page.mouse.up();checks++;
 origin=await begin();await arc(origin,45,145);await page.locator('[data-lab-fan-overview][data-active]').waitFor();await arc(origin,45,80);await page.mouse.up();assert.equal(await page.locator('[data-tab-sheet]').count(),0);checks++;
 origin=await begin();await arc(origin,45,145);await page.locator('[data-lab-fan-overview][data-active]').waitFor();await page.waitForFunction(()=>{const e=document.querySelector('[data-lab-left-fan]');return e&&Number(getComputedStyle(e).opacity)>.99;});await page.screenshot({path:`${shots}/outward-overview.png`});await page.mouse.up();await page.locator('[data-tab-sheet-open="gui/ai-elements-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});checks++;
 origin=await hold();await arc(origin,61);await arc(origin,45,145);await page.locator('[data-lab-fan-overview][data-active]').waitFor();await page.mouse.up();await page.locator('[data-tab-sheet-open="gui/ai-elements-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});checks++;
 origin=await hold();await page.mouse.move(origin.x,origin.y-120);assert.equal(await page.locator('[data-lab-fan-overview][data-active]').count(),0);await page.keyboard.press('Escape');await page.mouse.up();checks++;
 for(const interruption of ['pointer','multi','resize','blur']){
  origin=await hold();await move(origin,-60,20);
  if(interruption==='pointer')await avatar().dispatchEvent('pointercancel',{pointerId:1,isPrimary:true});
  else if(interruption==='multi')await avatar().dispatchEvent('pointerdown',{pointerId:9,isPrimary:false,pointerType:'touch',button:0});
  else if(interruption==='resize')await page.setViewportSize({width:430,height:844});
  else await page.evaluate(()=>window.dispatchEvent(new Event('blur')));
  await page.mouse.up();await fan().waitFor({state:'hidden'});await noLateCommit(ordered[0]);checks++;
 }
 const inputBefore=await page.locator('[data-chat-box]').boundingBox();await avatar().click();await page.locator('[data-lab-session-pill]').waitFor();const pill=await page.locator('[data-lab-session-pill]').boundingBox(),inputAfter=await page.locator('[data-chat-box]').boundingBox();assert(pill.y+pill.height<=inputAfter.y&&Math.abs(inputBefore.height-inputAfter.height)<1);await page.screenshot({path:`${shots}/expanded-pill.png`});await page.keyboard.press('Escape');await page.locator('[data-lab-session-pill]').waitFor({state:'hidden'});checks++;
 await avatar().click();const fullBand=page.locator('[data-lab-session-pill] [data-deck-band]'),b=await fullBand.boundingBox();await page.mouse.move(b.x+b.width/2,b.y+b.height/2);await page.mouse.down();await page.mouse.move(b.x-b.width*.2,b.y+b.height/2,{steps:6});const swiped=await fullBand.getAttribute('data-band-preview');await page.mouse.up();await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,swiped);checks++;
 await avatar().focus();await page.keyboard.press('Enter');await page.locator('[data-lab-session-pill]').waitFor();await page.keyboard.press('ArrowUp');await page.locator('[data-tab-sheet-open="gui/ai-elements-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});checks++;
 const cdp=await page.context().newCDPSession(page);await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:2});
 let a=await avatar().boundingBox(),t={x:a.x+a.width/2,y:a.y+a.height/2,id:0};await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[t]});await fan().waitFor();await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{...t,x:t.x-35/Math.SQRT2,y:t.y-35/Math.SQRT2}]});await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{...t,x:t.x-40,y:t.y-69.2820323}]});const touched=await hot();assert(touched&&touched!==ordered[0]);await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,touched);assert.equal(await page.locator('[data-lab-session-pill], [data-tab-sheet]').count(),0);checks++;
 a=await avatar().boundingBox();t={x:a.x+a.width/2,y:a.y+a.height/2,id:0};await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[t]});await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await page.locator('[data-lab-session-pill]').waitFor();await page.keyboard.press('Escape');await page.locator('[data-lab-session-pill]').waitFor({state:'hidden'});checks++;
 await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:false});
 const known=['gui/ai-elements-review','gui/codex-review','wide/all','gui/ai-elements-plan','gui/ai-elements-stream'];
 for(const count of [1,2,3,4,5]){
  await page.setViewportSize({width:390,height:844});await page.goto(`${base}/?${new URLSearchParams({bridge:new URL(base).port,labSession:known[0],labTabs:known.slice(0,count).join(','),phone:'1',deck:'fan',theme:'dark'})}`);await avatar().waitFor();await settle();origin=await hold();assert.equal(Number(await fan().getAttribute('data-fan-total')),count);
  if(count===1){assert.equal(await page.locator('[data-lab-fan-session]').count(),1);await move(origin,-60);assert.equal(await fan().getAttribute('data-fan-direction'),'0');}
  else{await move(origin,-60,20);await page.waitForFunction(n=>Number(document.querySelector('[data-lab-left-fan]')?.getAttribute('data-fan-pulse'))>=n,count);await move(origin,0);assert.equal(await hot(),known[0],'full revolution returns to the current session');await parked();assert.equal(new Set(await page.locator('[data-lab-fan-session]').evaluateAll(es=>es.map(e=>e.dataset.labFanSession))).size,await page.locator('[data-lab-fan-session]').count(),'small cycles never duplicate identities');}
  await page.screenshot({path:`${shots}/windows-${count}.png`});await page.mouse.up();await noLateCommit(known[0]);checks++;
 }
 await page.emulateMedia({reducedMotion:'reduce'});await open();origin=await hold();await move(origin,-14);const instant=await hot();await page.mouse.up();await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,instant);assert.equal(await fan().count(),0);await page.emulateMedia({reducedMotion:'no-preference'});checks++;
 for(const [width,height,keyboard] of [[320,844,'1'],[430,932,'1'],[667,375,'1']]){
  await open('gui/ai-elements-review','dark',keyboard,width,height);origin=await hold();await move(origin,-14,4);await move(origin,0);await parked();
  const panel=await fan().boundingBox(),cap=await page.locator('[data-chat-box]').boundingBox();assert(panel.y+panel.height<=cap.y+1);
  const bounds=await page.locator('[data-lab-fan-session],.lab-fan-face,.lab-fan-label,[data-lab-fan-overview]').evaluateAll(es=>es.map(e=>{const r=e.getBoundingClientRect();return {kind:e.className,x:r.x,y:r.y,right:r.right,bottom:r.bottom};}));
  for(const r of bounds)assert(r.x>=-1&&r.right<=width+1&&r.y>=panel.y-1&&r.bottom<=cap.y+1,`${r.kind} fits at ${width}: ${JSON.stringify(r)}`);
  for(const row of await page.locator('[data-lab-fan-session]').all()){const face=await row.locator('.lab-fan-face').boundingBox(),title=await row.locator('.lab-fan-label').boundingBox();assert(title.x+title.width<=face.x+1,'right-hand title sits left of the avatar');}
  await page.screenshot({path:`${shots}/fan-width-${width}.png`});await page.keyboard.press('Escape');await page.mouse.up();checks++;
 }
 await open('gui/ai-elements-review','dark','1');await page.locator('[data-composer-text]').fill('很长的多行草稿'.repeat(60));await page.locator('[data-lab-compose-expanded]').waitFor();origin=await hold();await move(origin,-14,4);await move(origin,0);await parked();
 const draftCap=await page.locator('[data-chat-box]').boundingBox(),draftPanel=await fan().boundingBox();
 for(const row of await page.locator('[data-lab-fan-session]').all()){const r=await row.boundingBox();assert(r.x>=-1&&r.x+r.width<=391&&r.y>=draftPanel.y-1&&r.y+r.height<=draftCap.y+1,'long draft keeps the fan in visible space');}
 await page.screenshot({path:`${shots}/long-draft-fan.png`});await page.keyboard.press('Escape');await page.mouse.up();checks++;
 await page.setViewportSize({width:1450,height:1050});await page.goto(`${base}/lab/gui-session-layout.html`);const frame=page.frameLocator('iframe');await frame.locator('[data-lab-fan-avatar]').waitFor();await frame.locator('[data-composer-text]').fill('比较时保留草稿');
 for(const count of ['3','4','5']){await page.getByLabel('窗口数量',{exact:true}).selectOption(count);await frame.locator('[data-lab-fan-avatar]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较时保留草稿');}checks++;
 await page.getByLabel('布局',{exact:true}).selectOption('compose');await frame.locator('.lab-avatar-band').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较时保留草稿');await page.getByLabel('布局',{exact:true}).selectOption('fan');await frame.locator('[data-lab-fan-avatar]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较时保留草稿');await page.screenshot({path:`${shots}/lab.png`,fullPage:true});checks++;
 assert.deepEqual(errors,[]);console.log(`${checks} angular fan, outward/retractable overview, titles, gears/pause/reverse, settle/fade, touch and layout checks passed. Screenshots: ${shots}`);
}catch(error){if(page)await page.screenshot({path:`${shots}/failure.png`});throw error;}finally{await browser.close();}
