// Real App paths with the private bridge; no supplier requests.
import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {chromium} from 'playwright-core';
const base=process.env.LAYOUT_LAB_URL??'http://localhost:23795',shots='/Volumes/disk0/khor-gui-session-layout-proof/left-fan';
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
 const candidates=()=>page.locator('[data-lab-fan-session]').evaluateAll(es=>es.map(e=>e.dataset.labFanSession));
 const pick=async id=>{await avatar().click();await page.locator('[data-lab-pill-overview]').click();await page.locator('[data-tab-sheet]').waitFor();await page.locator(`[data-tab-sheet-open="${id}"]`).click();await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,id);await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});await avatar().waitFor();};
 await page.evaluate(()=>sessionStorage.setItem('khor.lab.session-recents',JSON.stringify(['gui/ai-elements-review'])));
 await pick('gui/codex-review');await page.locator('[data-composer-text]').fill('保留 Codex 草稿');await pick('wide/all');await pick('gui/ai-elements-plan');await pick('gui/ai-elements-review');
 assert.deepEqual(await page.evaluate(()=>JSON.parse(sessionStorage.getItem('khor.lab.session-recents')).slice(0,4)),['gui/ai-elements-review','gui/ai-elements-plan','wide/all','gui/codex-review']);checks++;
 const begin=async()=>{const a=await avatar().boundingBox(),x=a.x+a.width/2,y=a.y+a.height/2;await page.mouse.move(x,y);await page.mouse.down();return {x,y};};
 const hold=async()=>{const p=await begin();await fan().waitFor();assert.equal(await fan().getAttribute('data-fan-phase'),'fan');return p;};
 const move=async(p,angle,r=80)=>{await page.mouse.move(p.x-r*Math.cos(angle*Math.PI/180),p.y-r*Math.sin(angle*Math.PI/180),{steps:5});};
 const hot=()=>fan().getAttribute('data-fan-selected');
 let origin=await hold();const frozen=await candidates();assert.deepEqual(frozen,['gui/ai-elements-plan','wide/all','gui/codex-review']);
 await move(origin,8);assert.equal(await hot(),frozen[0]);await move(origin,33);assert.equal(await hot(),frozen[1]);await move(origin,58);assert.equal(await hot(),frozen[2]);
 assert.equal(await selected(),'gui/ai-elements-review');assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Claude 草稿');
 await page.evaluate(async()=>{const selected=document.querySelector('[data-lab-left-fan]').getAttribute('data-fan-selected'),started=performance.now();await new Promise((resolve,reject)=>{const check=()=>{if(document.querySelector('[data-lab-left-fan]')?.getAttribute('data-fan-selected')!==selected)return reject(Error('stationary fan must not auto-repeat'));if(performance.now()-started>1500)resolve();else requestAnimationFrame(check);};check();});});
 const label=await page.locator('[data-lab-fan-title]').boundingBox();for(const face of await page.locator('[data-lab-fan-session]').all()){const b=await face.boundingBox();assert(label.x+label.width<=b.x||label.y+label.height<=b.y||label.y>=b.y+b.height,'title avoids candidate faces');}
 await page.screenshot({animations:'disabled',path:`${shots}/fan-preview.png`});await page.mouse.up();await page.waitForFunction(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active==='gui/codex-review');await fan().waitFor({state:'hidden'});assert.equal(await page.locator('[data-lab-session-pill], [data-tab-sheet]').count(),0);assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Codex 草稿');checks++;
 origin=await hold();assert.deepEqual(await candidates(),['gui/ai-elements-review','gui/ai-elements-plan','wide/all'],'actual focus updates recency for the next contact');await move(origin,33);await page.mouse.move(origin.x,origin.y,{steps:5});await page.mouse.up();assert.equal(await selected(),'gui/codex-review');checks++;
 origin=await hold();const snapshot=await candidates();await page.evaluate(()=>sessionStorage.setItem('khor.lab.session-recents',JSON.stringify(['wide/all','gui/ai-elements-plan','gui/ai-elements-review'])));await move(origin,33);assert.deepEqual(await candidates(),snapshot,'history changes never move a held candidate');await page.keyboard.press('Escape');await page.mouse.up();assert.equal(await selected(),'gui/codex-review');checks++;
 origin=await hold();await move(origin,8);await page.mouse.move(origin.x,origin.y-85,{steps:5});assert.equal(await fan().getAttribute('data-fan-lane'),'fan');assert.equal(await page.locator('[data-lab-fan-overview][data-active]').count(),0);await page.mouse.up();assert.equal(await selected(),'gui/codex-review');checks++;
 origin=await begin();await page.mouse.move(origin.x,origin.y-75,{steps:4});await page.locator('[data-lab-fan-overview][data-active]').waitFor();await page.mouse.move(origin.x-80,origin.y-40,{steps:5});assert.equal(await fan().getAttribute('data-fan-lane'),'overview');assert.equal(await hot(),null);await page.mouse.up();assert.equal(await page.locator('[data-tab-sheet]').count(),0);checks++;
 origin=await begin();await page.mouse.move(origin.x,origin.y-75,{steps:4});await page.locator('[data-lab-fan-overview][data-active]').waitFor();await page.mouse.move(origin.x,origin.y-8);await page.mouse.up();assert.equal(await page.locator('[data-tab-sheet]').count(),0);checks++;
 origin=await begin();await page.mouse.move(origin.x,origin.y-75,{steps:4});await page.locator('[data-lab-fan-overview][data-active]').waitFor();await page.screenshot({animations:'disabled',path:`${shots}/upward-overview.png`});await page.mouse.up();await page.locator('[data-tab-sheet]').waitFor();await page.locator('[data-tab-sheet-open="gui/ai-elements-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Claude 草稿');checks++;
 origin=await hold();await page.mouse.move(origin.x,origin.y-75,{steps:4});await page.locator('[data-lab-fan-overview][data-active]').waitFor();await page.mouse.up();await page.locator('[data-tab-sheet]').waitFor();await page.locator('[data-tab-sheet-open="gui/codex-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});checks++;
 origin=await hold();await move(origin,33);await page.keyboard.press('Escape');await page.mouse.up();assert.equal(await selected(),'gui/codex-review');assert.equal(await page.locator('[data-lab-session-pill], [data-tab-sheet]').count(),0);checks++;
 origin=await hold();await move(origin,33);await avatar().dispatchEvent('pointercancel',{pointerId:1,isPrimary:true});await page.mouse.up();await fan().waitFor({state:'hidden'});assert.equal(await selected(),'gui/codex-review');checks++;
 origin=await hold();await move(origin,33);await avatar().dispatchEvent('pointerdown',{pointerId:9,isPrimary:false,pointerType:'touch',button:0});await page.mouse.up();await fan().waitFor({state:'hidden'});assert.equal(await selected(),'gui/codex-review');checks++;
 origin=await hold();await move(origin,33);await page.setViewportSize({width:430,height:844});await page.mouse.up();await fan().waitFor({state:'hidden'});assert.equal(await selected(),'gui/codex-review');checks++;
 origin=await hold();await page.mouse.move(origin.x+30,origin.y);assert.equal(await hot(),null,'outward motion does not select');await page.mouse.up();assert.equal(await selected(),'gui/codex-review');checks++;
 for(const history of [[],['gui/ai-elements-review'],['gui/ai-elements-review','wide/all']]){
  await page.evaluate(history=>sessionStorage.setItem('khor.lab.session-recents',JSON.stringify(history)),history);origin=await hold();assert.deepEqual(await candidates(),history);
  if(!history.length)assert.equal(await page.locator('[data-lab-fan-title]').textContent(),'暂无最近会话');await page.mouse.move(origin.x,origin.y);await page.mouse.up();assert.equal(await selected(),'gui/codex-review');checks++;
 }
 await page.evaluate(()=>sessionStorage.setItem('khor.lab.session-recents',JSON.stringify(['gui/ai-elements-review','wide/all','gui/ai-elements-plan'])));
 const inputBefore=await page.locator('[data-chat-box]').boundingBox();await avatar().click();await page.locator('[data-lab-session-pill]').waitFor();const pill=await page.locator('[data-lab-session-pill]').boundingBox(),inputAfter=await page.locator('[data-chat-box]').boundingBox();assert(pill.y+pill.height<=inputAfter.y&&Math.abs(inputBefore.height-inputAfter.height)<1);await page.screenshot({animations:'disabled',path:`${shots}/expanded-pill.png`});await page.keyboard.press('Escape');await page.locator('[data-lab-session-pill]').waitFor({state:'hidden'});await avatar().waitFor();checks++;
 await avatar().click();const fullBand=page.locator('[data-lab-session-pill] [data-deck-band]'),b=await fullBand.boundingBox();await page.mouse.move(b.x+b.width/2,b.y+b.height/2);await page.mouse.down();await page.mouse.move(b.x-b.width*.2,b.y+b.height/2,{steps:6});const swiped=await fullBand.getAttribute('data-band-preview');await page.mouse.up();await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,swiped);checks++;
 await avatar().focus();await page.keyboard.press('Enter');await page.locator('[data-lab-session-pill]').waitFor();await page.keyboard.press('ArrowUp');await page.locator('[data-tab-sheet]').waitFor();await page.locator('[data-tab-sheet-open="gui/ai-elements-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});checks++;
 const cdp=await page.context().newCDPSession(page);await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:2});
 let a=await avatar().boundingBox(),t={x:a.x+a.width/2,y:a.y+a.height/2,id:0};await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[t]});await fan().waitFor();await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{...t,x:t.x-67,y:t.y-44}]});const touched=await hot();assert(touched);await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,touched);assert.equal(await page.locator('[data-lab-session-pill], [data-tab-sheet]').count(),0);checks++;
 a=await avatar().boundingBox();t={x:a.x+a.width/2,y:a.y+a.height/2,id:0};await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[t]});await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await page.locator('[data-lab-session-pill]').waitFor();await page.keyboard.press('Escape');await page.locator('[data-lab-session-pill]').waitFor({state:'hidden'});await avatar().waitFor();checks++;
 await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:false});
 for(const [width,height,keyboard] of [[320,844,'1'],[430,932,'1'],[667,375,'1']]){
  await open('gui/ai-elements-review','dark',keyboard,width,height);await page.evaluate(()=>sessionStorage.setItem('khor.lab.session-recents',JSON.stringify(['gui/codex-review','wide/all','gui/ai-elements-plan'])));origin=await hold();await move(origin,33);
  const panel=await fan().boundingBox(),cap=await page.locator('[data-chat-box]').boundingBox();assert(panel.y+panel.height<=cap.y+1,'selector leaves input and keyboard in place');
  for(const selector of ['[data-lab-fan-session]','[data-lab-fan-overview]','[data-lab-fan-title]'])for(const item of await page.locator(selector).all()){const r=await item.boundingBox();assert(r.x>=0&&r.x+r.width<=width+1&&r.y>=panel.y&&r.y+r.height<=cap.y+1,'fan labels and faces fit the visible content');}
  assert.equal(await fan().getAttribute('data-compact'),width===667?'true':null);await page.screenshot({animations:'disabled',path:`${shots}/fan-width-${width}.png`});await page.keyboard.press('Escape');await page.mouse.up();checks++;
 }
 await page.setViewportSize({width:1450,height:1050});await page.goto(`${base}/lab/gui-session-layout.html`);assert.equal(await page.getByLabel('布局',{exact:true}).inputValue(),'fan');const frame=page.frameLocator('iframe');await frame.locator('[data-lab-fan-avatar]').waitFor();await frame.locator('[data-composer-text]').fill('比较时保留草稿');
 await page.getByLabel('布局',{exact:true}).selectOption('compose');await frame.locator('.lab-avatar-band').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较时保留草稿');await page.getByLabel('布局',{exact:true}).selectOption('fan');await frame.locator('[data-lab-fan-avatar]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较时保留草稿');await page.screenshot({animations:'disabled',path:`${shots}/lab.png`,fullPage:true});checks++;
 assert.deepEqual(errors,[]);console.log(`${checks} left fan, actual recent visits, held order, direction locking, touch, cancellation, keyboard and layout checks passed. Screenshots: ${shots}`);
}catch(error){if(page)await page.screenshot({animations:'disabled',path:`${shots}/failure.png`});throw error;}finally{await browser.close();}
