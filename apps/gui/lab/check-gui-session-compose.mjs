// Real App paths with the private bridge; no supplier requests.
import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {chromium} from 'playwright-core';
const base=process.env.LAYOUT_LAB_URL??'http://localhost:23795',shots='/Volumes/disk0/khor-gui-session-layout-proof/compose';
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
  assert(a.x+a.width<=plus.x+1&&plus.x+plus.width<=field.x+1,'avatar, add, then message');
  await page.locator('[data-composer-text]').fill('这是一段换行后仍应独占完整文字空间的草稿。\n第二行继续输入。');
  await page.locator('[data-lab-compose-expanded]').waitFor();
  const expanded=await page.locator('[data-chat-box]').boundingBox(),text=await page.locator('[data-composer-text]').boundingBox(),avatar=await page.locator('[data-lab-compose-avatar]').boundingBox(),send=await page.locator('[data-chat-send]').boundingBox();
  assert(text.y+text.height<=avatar.y+1&&text.width>expanded.width*.85,'full-width text above controls');
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
 await page.locator('[data-lab-compose-avatar]').click();await page.locator('[data-tab-sheet]').waitFor();await page.locator('[data-tab-sheet-open="gui/codex-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});
 await page.locator('[data-composer-text]').fill('保留 Codex 草稿');checks++;
 const hold=async()=>{const a=await page.locator('[data-lab-compose-avatar]').boundingBox(),x=a.x+a.width/2,y=a.y+a.height/2;await page.mouse.move(x,y);await page.mouse.down();await page.locator('[data-lab-outer-wheel]').waitFor();return {x,y};};
 const preview=async id=>{const face=await page.locator(`[data-lab-outer-session="${id}"] .lab-outer-face`).boundingBox();await page.mouse.move(face.x+face.width/2,face.y+face.height/2,{steps:10});await page.locator(`[data-lab-outer-session="${id}"][data-selected]`).waitFor();};
 let origin=await hold();await preview('wide/all');assert.equal(await selected(),'gui/codex-review','preview does not navigate');assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Codex 草稿');
 for(const row of await page.locator('[data-lab-outer-session]').all()){
  const face=await row.locator('.lab-outer-face').boundingBox(),words=await row.locator('.lab-outer-words').boundingBox();assert(face.x+face.width<=words.x+1,'titles sit right of avatars');
  assert.equal(await row.evaluate(el=>getComputedStyle(el).backgroundColor),'rgba(0, 0, 0, 0)','no wheel panel');
 }
 await page.screenshot({animations:'disabled',path:`${shots}/wheel-preview.png`});await page.mouse.up();await page.waitForFunction(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active==='wide/all');await page.locator('[data-lab-outer-wheel]').waitFor({state:'hidden'});assert.equal(await page.locator('[data-tab-sheet]').count(),0);checks++;
 origin=await hold();await preview('gui/ai-elements-review');await page.mouse.move(origin.x,origin.y,{steps:5});await page.mouse.up();assert.equal(await selected(),'wide/all');checks++;
 origin=await hold();await preview('gui/codex-review');await page.keyboard.press('Escape');await page.mouse.up();assert.equal(await selected(),'wide/all');checks++;
 origin=await hold();await page.locator('[data-lab-compose-avatar]').dispatchEvent('pointercancel',{pointerId:1,isPrimary:true});await page.mouse.up();await page.locator('[data-lab-outer-wheel]').waitFor({state:'hidden'});assert.equal(await selected(),'wide/all');checks++;
 origin=await hold();await page.locator('[data-lab-compose-avatar]').dispatchEvent('pointerdown',{pointerId:9,isPrimary:false,pointerType:'touch',button:0});await page.mouse.up();await page.locator('[data-lab-outer-wheel]').waitFor({state:'hidden'});assert.equal(await selected(),'wide/all');assert.equal(await page.locator('[data-tab-sheet]').count(),0);checks++;
 origin=await hold();await page.setViewportSize({width:430,height:844});await page.mouse.up();await page.locator('[data-lab-outer-wheel]').waitFor({state:'hidden'});assert.equal(await selected(),'wide/all');checks++;
 origin=await hold();const target=await page.locator('[data-lab-all-tabs-target]').boundingBox();await page.mouse.move(target.x+target.width/2,target.y+target.height/2,{steps:12});await page.locator('[data-lab-all-tabs-target][data-selected]').waitFor();assert.equal(await selected(),'wide/all');
 await page.screenshot({animations:'disabled',path:`${shots}/all-tabs-gesture.png`});await page.mouse.up();await page.locator('[data-tab-sheet]').waitFor();await page.locator('[data-tab-sheet-open="gui/ai-elements-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Claude 草稿');checks++;
 await page.locator('[data-composer-text]').fill('第一行\n第二行\n第三行');await page.screenshot({animations:'disabled',path:`${shots}/expanded.png`});
 await page.setViewportSize({width:1450,height:1050});await page.goto(`${base}/lab/gui-session-layout.html`);assert.equal(await page.getByLabel('布局',{exact:true}).inputValue(),'compose');const frame=page.frameLocator('iframe');await frame.locator('[data-lab-compose-avatar]').waitFor();
 await frame.locator('[data-composer-text]').fill('比较时保留草稿');await page.getByLabel('布局',{exact:true}).selectOption('light');await frame.locator('[data-lab-light-header]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较时保留草稿');
 await page.getByLabel('布局',{exact:true}).selectOption('compose');await frame.locator('[data-lab-compose-avatar]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较时保留草稿');await page.screenshot({animations:'disabled',path:`${shots}/lab.png`,fullPage:true});checks++;
 assert.deepEqual(errors,[]);console.log(`${checks} model header, bottom avatar, full-width draft, outer arc and diagonal overview checks passed. Screenshots: ${shots}`);
}catch(error){if(page)await page.screenshot({animations:'disabled',path:`${shots}/failure.png`});throw error;}finally{await browser.close();}
