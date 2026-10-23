// Real App paths with the private bridge; no supplier requests.
import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {chromium} from 'playwright-core';
const base=process.env.LAYOUT_LAB_URL??'http://localhost:23795',shots='/Volumes/disk0/khor-gui-session-layout-proof/compose-right';
mkdirSync(shots,{recursive:true});
const browser=await chromium.launch({channel:'chrome',headless:process.env.PW_HEADLESS==='1'});let page;
try{
 page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));let checks=0;
 await page.request.get(`${base}/__ai_preview/reset`);
 const selected=()=>page.evaluate(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active);
 const settle=()=>page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>Number.isFinite(a.effect?.getComputedTiming().endTime)).map(a=>a.finished.catch(()=>{}))));
 const open=async(session='gui/ai-elements-review',theme='light',keyboard='0',width=390,height=844)=>{
  await page.setViewportSize({width,height});await page.goto(`${base}/?${new URLSearchParams({bridge:new URL(base).port,labSession:session,theme,keyboard,phone:width<760?'1':'0',deck:'compose'})}`);
  await page.locator('[data-composer-text]').waitFor();if(width<760)await page.locator('[data-lab-compose-avatar]').waitFor();await settle();
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
 const avatar=()=>page.locator('[data-lab-compose-avatar] [data-deck-title]'),band=()=>page.locator('[data-lab-compose-avatar] [data-deck-band]');
 const inputBefore=await page.locator('[data-chat-box]').boundingBox();await avatar().click();await page.locator('[data-lab-session-pill]').waitFor();
 const pill=await page.locator('[data-lab-session-pill]').boundingBox(),inputAfter=await page.locator('[data-chat-box]').boundingBox();assert(pill.y+pill.height<=inputAfter.y&&pill.x>=0&&pill.x+pill.width<=390,'expanded pill floats just above input');assert(Math.abs(inputBefore.height-inputAfter.height)<1,'opening navigation does not resize the draft');
 await page.screenshot({animations:'disabled',path:`${shots}/expanded-pill.png`});await page.locator('[data-composer-text]').click();await page.locator('[data-lab-session-pill]').waitFor({state:'hidden'});assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Claude 草稿');checks++;
 await avatar().click();await page.locator('[data-lab-pill-overview]').click();await page.locator('[data-tab-sheet]').waitFor();await page.locator('[data-tab-sheet-open="gui/codex-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});
 await page.locator('[data-composer-text]').fill('保留 Codex 草稿');checks++;
 const hold=async()=>{const a=await avatar().boundingBox(),x=a.x+a.width/2,y=a.y+a.height/2;await page.mouse.move(x,y);await page.mouse.down();await page.locator('[data-deck-arc]').waitFor();return {x,y};};
 const move=async(origin,dx,dy=0)=>{await page.mouse.move(origin.x+dx,origin.y+dy,{steps:3});};
 const preview=()=>band().getAttribute('data-band-preview');
 let origin=await hold();assert.equal(await page.locator('[data-lab-outer-wheel]').count(),0,'custom outer targets removed');
 await move(origin,-12);assert.equal(await preview(),'gui/ai-elements-stream');assert.equal(await band().getAttribute('data-band-tier'),'1');
 await move(origin,-22);assert.equal(await band().getAttribute('data-band-tier'),'2');await move(origin,-31);assert.equal(await band().getAttribute('data-band-tier'),'3');
 await page.waitForFunction(()=>document.querySelector('[data-lab-compose-avatar] [data-deck-band]')?.getAttribute('data-band-preview')==='gui/ai-elements-plan');
 await move(origin,0);assert.equal(await band().getAttribute('data-band-direction'),'0');assert.equal(await selected(),'gui/codex-review','held preview never opens intermediate contents');assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Codex 草稿');
 await move(origin,12);assert.equal(await preview(),'gui/ai-elements-stream','reversal previews immediately');await move(origin,31);assert.equal(await band().getAttribute('data-band-tier'),'3','outward fast gear fits the screen edge');await move(origin,0);
 await settle();await page.screenshot({animations:'disabled',path:`${shots}/held-selector.png`});await page.mouse.up();await page.waitForFunction(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active==='gui/ai-elements-stream');await page.locator('[data-deck-selector]').waitFor({state:'hidden'});assert.equal(await page.locator('[data-lab-session-pill], [data-tab-sheet]').count(),0,'hold release does not become a tap');checks++;
 origin=await hold();await move(origin,-12);await move(origin,0);assert.equal(await band().getAttribute('data-band-direction'),'0');await page.keyboard.press('Escape');await page.mouse.up();assert.equal(await selected(),'gui/ai-elements-stream');checks++;
 origin=await hold();await band().dispatchEvent('pointercancel',{pointerId:1,isPrimary:true});await page.mouse.up();await page.locator('[data-deck-selector]').waitFor({state:'hidden'});assert.equal(await selected(),'gui/ai-elements-stream');checks++;
 origin=await hold();await band().dispatchEvent('pointerdown',{pointerId:9,isPrimary:false,pointerType:'touch',button:0});await page.mouse.up();await page.locator('[data-deck-selector]').waitFor({state:'hidden'});assert.equal(await selected(),'gui/ai-elements-stream');assert.equal(await page.locator('[data-lab-session-pill], [data-tab-sheet]').count(),0);checks++;
 origin=await hold();await move(origin,-12);await page.setViewportSize({width:430,height:844});await page.mouse.up();await page.locator('[data-deck-selector]').waitFor({state:'hidden'});assert.equal(await selected(),'gui/ai-elements-stream');checks++;
 const begin=async()=>{const a=await avatar().boundingBox(),x=a.x+a.width/2,y=a.y+a.height/2;await page.mouse.move(x,y);await page.mouse.down();return {x,y};};
 origin=await begin();await move(origin,-12,-72);await page.locator('[data-deck-overview-target][data-active]').waitFor();assert.equal(await band().getAttribute('data-band-steer-phase'),'pull','upward overview does not wait for a long press');await move(origin,0,-12);await page.mouse.up();assert.equal(await page.locator('[data-tab-sheet]').count(),0,'retract cancels direct overview');checks++;
 origin=await begin();await move(origin,-12,-72);await page.locator('[data-deck-overview-target][data-active]').waitFor();await page.screenshot({animations:'disabled',path:`${shots}/upward-overview.png`});await page.mouse.up();await page.locator('[data-tab-sheet]').waitFor();await page.locator('[data-tab-sheet-open="gui/codex-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Codex 草稿');checks++;
 await avatar().click();const fullBand=page.locator('[data-lab-session-pill] [data-deck-band]'),full=await fullBand.boundingBox();
 origin={x:full.x+full.width/2,y:full.y+full.height/2};await page.mouse.move(origin.x,origin.y);await page.mouse.down();await move(origin,-full.width*.7);assert.equal(await fullBand.getAttribute('data-band-phase'),'swipe');assert.equal(await selected(),'gui/codex-review');const swiped=await fullBand.getAttribute('data-band-preview');await page.mouse.up();await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,swiped);await page.locator('[data-lab-session-pill]').waitFor({state:'hidden'});checks++;
 await avatar().click();const f=await fullBand.boundingBox();await page.mouse.move(f.x+f.width/2,f.y+f.height/2);await page.mouse.down();await page.locator('[data-deck-arc]').waitFor();await page.mouse.move(f.x+f.width/2-20,f.y+f.height/2);assert.equal(await fullBand.getAttribute('data-band-steer-phase'),'steering');await page.keyboard.press('Escape');await page.mouse.up();await page.locator('[data-deck-selector]').waitFor({state:'hidden'});await page.locator('[data-lab-session-pill]').waitFor({state:'hidden'});checks++;
 await avatar().focus();await page.keyboard.press('Enter');await page.locator('[data-lab-session-pill]').waitFor();await page.keyboard.press('ArrowUp');await page.locator('[data-tab-sheet]').waitFor();await page.locator('[data-tab-sheet-open="gui/ai-elements-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Claude 草稿');checks++;
 // Native touch and compatibility-click ownership use DeckBand unchanged.
 const cdp=await page.context().newCDPSession(page);await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:2});
 const a=await avatar().boundingBox(),touch={x:a.x+a.width/2,y:a.y+a.height/2,id:0};
 await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[touch]});await page.locator('[data-deck-arc]').waitFor();await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{...touch,x:touch.x-12}]});const touched=await preview();await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,touched);assert.equal(await page.locator('[data-lab-session-pill], [data-tab-sheet]').count(),0);checks++;
 await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[touch]});await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await page.locator('[data-lab-session-pill]').waitFor();await page.keyboard.press('Escape');await page.locator('[data-lab-session-pill]').waitFor({state:'hidden'});await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:false});checks++;
 await page.setViewportSize({width:1450,height:1050});await page.goto(`${base}/lab/gui-session-layout.html`);assert.equal(await page.getByLabel('布局',{exact:true}).inputValue(),'compose');const frame=page.frameLocator('iframe');await frame.locator('[data-lab-compose-avatar]').waitFor();
 await frame.locator('[data-composer-text]').fill('比较时保留草稿');await page.getByLabel('布局',{exact:true}).selectOption('light');await frame.locator('[data-lab-light-header]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较时保留草稿');
 await page.getByLabel('布局',{exact:true}).selectOption('compose');await frame.locator('[data-lab-compose-avatar]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较时保留草稿');await page.screenshot({animations:'disabled',path:`${shots}/lab.png`,fullPage:true});checks++;
 await frame.locator('[data-chat-send]').click();await frame.locator('[data-chat-stop]').waitFor();await frame.locator('[data-chat-stop]').click();await frame.locator('[data-chat-stop]').waitFor({state:'hidden'});await page.getByLabel('布局',{exact:true}).selectOption('light');await frame.locator('[data-lab-light-header]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'','sending clears cross-layout draft cache');checks++;
 assert.deepEqual(errors,[]);console.log(`${checks} model header, right avatar, shared terminal gestures, cancellation, touch and draft checks passed. Screenshots: ${shots}`);
}catch(error){if(page)await page.screenshot({animations:'disabled',path:`${shots}/failure.png`});throw error;}finally{await browser.close();}
