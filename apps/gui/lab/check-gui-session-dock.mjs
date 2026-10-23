// Real App views, controls and gestures on the isolated fixture bridge.
import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {chromium} from 'playwright-core';
const base=process.env.LAYOUT_LAB_URL??'http://localhost:23795';
const shots=process.env.LAYOUT_DOCK_SHOTS??'/Volumes/disk0/khor-gui-session-layout-proof/combined';
mkdirSync(shots,{recursive:true});
const browser=await chromium.launch({channel:'chrome',headless:process.env.PW_HEADLESS==='1'});
let page;
try{
 page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));let checks=0;
 await page.request.get(`${base}/__ai_preview/reset`);
 const open=async(session='gui/ai-elements-review',theme='light',keyboard='0',width=390,height=844)=>{
  await page.setViewportSize({width,height});
  await page.goto(`${base}/?${new URLSearchParams({bridge:new URL(base).port,labSession:session,theme,keyboard,phone:width<760?'1':'0',deck:'combined'})}`);
  await page.locator('[data-composer-text]').waitFor();
  if(width<760)await page.locator('[data-session-dock] [data-composer-text]').waitFor();
  await page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>Number.isFinite(a.effect?.getComputedTiming().endTime)).map(a=>a.finished.catch(()=>{}))));
 };
 const geometry=async()=>page.evaluate(()=>{
  const box=selector=>{const r=document.querySelector(selector).getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,bottom:r.bottom};};
  const dock=document.querySelector('[data-session-dock]');
  return {dock:box('[data-session-dock]'),nav:box('[data-session-deck]'),input:box('[data-chat-box]'),content:box('[data-session-content]'),bottom:parseFloat(getComputedStyle(dock).marginBottom),visible:visualViewport.height};
 });
 for(const session of ['gui/ai-elements-review','gui/codex-review','wide/all'])for(const theme of ['light','dark'])for(const keyboard of ['0','1']){
  console.log('layout',session,theme,keyboard);await open(session,theme,keyboard);const initial=await geometry();
  assert(initial.input.bottom<=initial.nav.y+1,'input stays above navigation');
  assert(initial.content.bottom<=initial.dock.y+1,'dock has its own room');
  assert(Math.abs(initial.dock.bottom+initial.bottom-initial.visible)<1,'dock clears home indicator or keyboard');
  assert(initial.dock.x>=0&&initial.dock.x+initial.dock.width<=390,'dock stays inside viewport');
  await page.screenshot({animations:'disabled',path:`${shots}/${session==='wide/all'?'khor':session.includes('codex')?'codex':'claude'}-${theme}-${keyboard}.png`});
  await page.locator('[data-composer-text]').fill('第一行\n第二行\n第三行\n第四行');const multiline=await geometry();
  assert(Math.abs(multiline.nav.y-initial.nav.y)<1,'navigation does not move with text growth');
  assert(multiline.dock.y<initial.dock.y,'input grows into content');assert(multiline.content.height>0);
  await page.locator('[data-lab-session-settings]').click();await page.locator('[data-lab-option="model"]').waitFor();
  const menu=await page.locator('[data-lab-preferences]').boundingBox();assert(menu.y>=0&&menu.y+menu.height<=initial.visible+1,'settings menu stays above keyboard');
  await page.keyboard.press('Escape');await page.locator('[data-lab-preferences]').waitFor({state:'hidden'});
  await page.locator('[data-chat-scroll], [data-machine-chat-scroll]').evaluate(el=>{el.scrollTop=0;el.dispatchEvent(new Event('scroll'));});
  assert.equal(await page.locator('[data-deck-strip]').count(),0,'combined dock stays available when reading');checks++;
 }
 for(const width of [320,430,667])for(const keyboard of ['0','1']){
  await open('gui/codex-review','dark',keyboard,width,width===667?375:844);
  await page.locator('[data-composer-text]').fill(Array.from({length:20},(_,i)=>`草稿第 ${i+1} 行`).join('\n'));
  const g=await geometry();assert(g.content.height>0);assert(g.dock.bottom<=g.visible);
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  await page.screenshot({animations:'disabled',path:`${shots}/width-${width}-${keyboard}.png`});checks++;
 }
 await open('gui/codex-review','light','0',1280,800);assert.equal(await page.locator('[data-session-dock]').count(),0,'wide tabs retain their existing arrangement');checks++;
 const selected=()=>page.evaluate(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active);
 const overview=async()=>{
  await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});
  const b=await page.locator('[data-deck-band]').boundingBox(),x=b.x+b.width/2,y=b.y+b.height/2;
  await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x,y-90,{steps:5});
  await page.locator('[data-deck-selector][data-overview]').waitFor();await page.mouse.up();await page.locator('[data-tab-sheet]').waitFor();
 };
 const choose=async id=>{await overview();await page.locator(`[data-tab-sheet-open="${id}"]`).click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});await page.locator('[data-session-dock] [data-composer-text]').waitFor();assert.equal(await selected(),id);};
 await open();await page.locator('[data-composer-text]').fill('Claude 的草稿');
 await choose('gui/codex-review');assert.equal(await page.locator('[data-composer-text]').inputValue(),'');await page.locator('[data-composer-text]').fill('Codex 的草稿');
 await choose('wide/all');assert.equal(await page.locator('[data-composer-text]').inputValue(),'');await page.locator('[data-composer-text]').fill('Khor 的草稿');
 await choose('gui/ai-elements-review');assert.equal(await page.locator('[data-composer-text]').inputValue(),'Claude 的草稿');
 await choose('gui/codex-review');assert.equal(await page.locator('[data-composer-text]').inputValue(),'Codex 的草稿');
 await choose('wide/all');assert.equal(await page.locator('[data-composer-text]').inputValue(),'Khor 的草稿');checks++;
 await choose('gui/ai-elements-review');const b=await page.locator('[data-deck-band]').boundingBox(),x=b.x+b.width/2,y=b.y+b.height/2;
 await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x-95,y,{steps:5});await page.mouse.up();
 await page.waitForFunction(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active!=='gui/ai-elements-review');checks++;
 await choose('gui/ai-elements-review');assert.equal(await page.locator('[data-composer-text]').inputValue(),'Claude 的草稿');
 await page.locator('[data-chat-send]').click();await page.locator('[data-chat-box] [data-chat-stop]').waitFor();await page.locator('[data-chat-box] [data-chat-stop]').click();
 await choose('gui/codex-review');await choose('gui/ai-elements-review');assert.equal(await page.locator('[data-composer-text]').inputValue(),'','send clears saved draft');checks++;
 await page.setViewportSize({width:1450,height:1050});await page.goto(`${base}/lab/gui-session-layout.html`);
 const frame=page.frameLocator('iframe');await frame.locator('[data-session-dock] [data-composer-text]').fill('比较布局的草稿\n第二行');
 for(const layout of ['top','bottom','combined']){
  await page.getByLabel('布局',{exact:true}).selectOption(layout);await frame.locator('[data-composer-text]').waitFor();
  assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较布局的草稿\n第二行');
 }
 await page.getByLabel('模拟键盘升起').check();await frame.locator('[data-lab-keyboard]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较布局的草稿\n第二行');
 await page.screenshot({animations:'disabled',path:`${shots}/lab-comparison.png`,fullPage:true});
 await page.getByLabel('会话',{exact:true}).selectOption('wide/all');await frame.locator('[data-composer-text]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'');await frame.locator('[data-composer-text]').fill('Khor 的比较草稿');
 await page.getByLabel('会话',{exact:true}).selectOption('gui/codex-review');await frame.locator('[data-composer-text]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'');
 await page.getByLabel('会话',{exact:true}).selectOption('wide/all');await frame.locator('[data-composer-text]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'Khor 的比较草稿');
 await frame.locator('[data-chat-send]').click();await frame.locator('[data-chat-box] [data-chat-stop]').waitFor();await frame.locator('[data-chat-box] [data-chat-stop]').click();
 // A sent draft must also disappear from the parent comparison cache.
 await page.getByLabel('布局',{exact:true}).selectOption('top');await frame.locator('[data-composer-text]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'');checks++;
 assert.deepEqual(errors,[]);console.log(`${checks} combined dock layout, keyboard, gestures, per-session drafts and Lab comparison checks passed. Screenshots: ${shots}`);
}catch(error){if(page)await page.screenshot({animations:'disabled',path:`${shots}/failure.png`});throw error;}finally{await browser.close();}
