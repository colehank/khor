// Browser behavior of the real views, independent configuration and avatar wheel.
import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {chromium} from 'playwright-core';
const base=process.env.LAYOUT_LAB_URL??'http://localhost:23795',shots='/Volumes/disk0/khor-gui-session-layout-proof/light';
mkdirSync(shots,{recursive:true});
const browser=await chromium.launch({channel:'chrome',headless:process.env.PW_HEADLESS==='1'});let page;
try{
 page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));let checks=0;
 await page.request.get(`${base}/__ai_preview/reset`);
 const selected=()=>page.evaluate(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active);
 const settle=()=>page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>Number.isFinite(a.effect?.getComputedTiming().endTime)).map(a=>a.finished.catch(()=>{}))));
 const open=async(session='gui/ai-elements-review',theme='light',keyboard='0',width=390,height=844)=>{
  await page.setViewportSize({width,height});await page.goto(`${base}/?${new URLSearchParams({bridge:new URL(base).port,labSession:session,theme,keyboard,phone:width<760?'1':'0',deck:'light'})}`);
  await page.locator('[data-composer-text]').waitFor();if(width<760)await page.locator('[data-lab-light-header]').waitFor();await settle();
 };
 for(const session of ['gui/ai-elements-review','gui/codex-review','wide/all'])for(const theme of ['light','dark'])for(const keyboard of ['0','1']){
  await open(session,theme,keyboard);
  assert.equal(await page.locator('[data-lab-light-header] .glass').count(),1,'only avatar has permanent glass');
  assert.equal(await page.locator('[data-lab-session-identity] [data-lab-config-trigger]').count(),0,'configuration is a sibling of identity');
  const header=await page.locator('[data-lab-light-header]').boundingBox(),content=await page.locator('[data-session-content]').boundingBox(),composer=await page.locator('[data-chat-box]').boundingBox();
  assert(header.y+header.height<=content.y+1&&content.y+content.height<=composer.y+1);
  for(const selector of ['[data-lab-header-back]','[data-lab-session-identity]','[data-lab-config-trigger]']){
   const material=await page.locator(selector).evaluate(el=>({background:getComputedStyle(el).backgroundColor,shadow:getComputedStyle(el).boxShadow}));assert.equal(material.background,'rgba(0, 0, 0, 0)');assert.equal(material.shadow,'none');
  }
  await page.locator('[data-composer-text]').fill('第一行\n第二行\n第三行');assert(Math.abs((await page.locator('[data-lab-light-header]').boundingBox()).y-header.y)<1);
  await page.locator('[data-lab-config-trigger]').click();await page.locator('[data-lab-option="model"]').waitFor();
  assert.equal(await page.locator('[data-tab-sheet]').count(),0,'configuration does not open navigation');
  const menu=await page.locator('[data-lab-preferences]').boundingBox(),visible=await page.evaluate(()=>visualViewport.height);assert(menu.y>=0&&menu.y+menu.height<=visible+1);
  await page.keyboard.press('Escape');await page.locator('[data-lab-preferences]').waitFor({state:'hidden'});
  await page.locator('[data-composer-text]').fill('');await page.screenshot({animations:'disabled',path:`${shots}/${session==='wide/all'?'khor':session.includes('codex')?'codex':'claude'}-${theme}-${keyboard}.png`});checks++;
 }
 for(const [width,height] of [[320,844],[430,932],[667,375]]){
  await open('gui/codex-review','dark','1',width,height);assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  const title=await page.locator('[data-lab-header-title]').boundingBox(),settings=await page.locator('[data-lab-config-trigger]').boundingBox();assert(title.x+title.width<=settings.x);
  await page.screenshot({animations:'disabled',path:`${shots}/width-${width}.png`});checks++;
 }
 await open('gui/codex-review','light','0',1280,800);assert.equal(await page.locator('[data-lab-light-header]').count(),0,'wide tabs remain unchanged');checks++;
 await open();await page.locator('[data-composer-text]').fill('保留 Claude 草稿');
 await page.locator('[data-lab-session-identity]').click();await page.locator('[data-tab-sheet]').waitFor();await page.locator('[data-tab-sheet-open="gui/codex-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});assert.equal(await selected(),'gui/codex-review');
 await page.locator('[data-composer-text]').fill('保留 Codex 草稿');checks++;
 const hold=async()=>{const a=await page.locator('[data-lab-header-avatar]').boundingBox(),x=a.x+a.width/2,y=a.y+a.height/2;await page.mouse.move(x,y);await page.mouse.down();await page.locator('[data-lab-session-wheel]').waitFor();return {x,y};};
 const preview=async(id,origin)=>{
  const first=await page.locator('[data-lab-wheel-session]').first().boundingBox(),target=await page.locator(`[data-lab-wheel-session="${id}"]`).boundingBox();
  const fx=first.x+first.width/2,fy=first.y+first.height/2,tx=target.x+target.width/2,ty=target.y+target.height/2;
  await page.mouse.move(fx,fy,{steps:5});const radius=Math.hypot(fx-origin.x,fy-origin.y),from=Math.atan2(fy-origin.y,fx-origin.x),to=Math.atan2(ty-origin.y,tx-origin.x);
  for(let i=1;i<=8;i++){const angle=from+(to-from)*i/8;await page.mouse.move(origin.x+radius*Math.cos(angle),origin.y+radius*Math.sin(angle));}
  await page.locator(`[data-lab-wheel-session="${id}"][data-selected]`).waitFor();
 };
 let origin=await hold();await preview('wide/all',origin);assert.equal(await selected(),'gui/codex-review','preview does not navigate');assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Codex 草稿');
 await page.screenshot({animations:'disabled',path:`${shots}/wheel-preview.png`});await page.mouse.up();await page.waitForFunction(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active==='wide/all');await page.locator('[data-lab-session-wheel]').waitFor({state:'hidden'});assert.equal(await page.locator('[data-tab-sheet]').count(),0,'release does not also click overview');checks++;
 origin=await hold();await preview('gui/ai-elements-review',origin);await page.mouse.move(origin.x,origin.y,{steps:5});await page.mouse.up();await page.locator('[data-lab-session-wheel]').waitFor({state:'hidden'});assert.equal(await selected(),'wide/all','return to origin cancels');checks++;
 origin=await hold();await preview('gui/codex-review',origin);await page.keyboard.press('Escape');await page.mouse.up();await page.locator('[data-lab-session-wheel]').waitFor({state:'hidden'});assert.equal(await selected(),'wide/all');checks++;
 origin=await hold();await page.locator('[data-lab-session-identity]').dispatchEvent('pointercancel',{pointerId:1,isPrimary:true});await page.mouse.up();await page.locator('[data-lab-session-wheel]').waitFor({state:'hidden'});assert.equal(await selected(),'wide/all');checks++;
 origin=await hold();await page.locator('[data-lab-session-identity]').dispatchEvent('pointerdown',{pointerId:9,isPrimary:false,pointerType:'touch',button:0});await page.mouse.up();await page.locator('[data-lab-session-wheel]').waitFor({state:'hidden'});assert.equal(await selected(),'wide/all');assert.equal(await page.locator('[data-tab-sheet]').count(),0,'second touch cancels without clicking overview');checks++;
 origin=await hold();await page.setViewportSize({width:430,height:844});await page.mouse.up();await page.locator('[data-lab-session-wheel]').waitFor({state:'hidden'});assert.equal(await selected(),'wide/all');checks++;
 await page.locator('[data-lab-session-identity]').focus();await page.keyboard.press('Enter');await page.locator('[data-tab-sheet]').waitFor();await page.locator('[data-tab-sheet-open="gui/ai-elements-review"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});assert.equal(await page.locator('[data-composer-text]').inputValue(),'保留 Claude 草稿');checks++;
 await page.setViewportSize({width:1450,height:1050});await page.goto(`${base}/lab/gui-session-layout.html`);await page.getByLabel('布局',{exact:true}).selectOption('light');const frame=page.frameLocator('iframe');await frame.locator('[data-lab-light-header]').waitFor();assert.equal(await page.getByLabel('布局',{exact:true}).inputValue(),'light');
 await frame.locator('[data-composer-text]').fill('比较时保留草稿');await page.getByLabel('布局',{exact:true}).selectOption('combined');await frame.locator('[data-session-dock]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较时保留草稿');
 await page.getByLabel('布局',{exact:true}).selectOption('light');await frame.locator('[data-lab-light-header]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较时保留草稿');await page.screenshot({animations:'disabled',path:`${shots}/lab.png`,fullPage:true});checks++;
 assert.deepEqual(errors,[]);console.log(`${checks} light header, independent configuration, angular wheel, cancellation and draft checks passed. Screenshots: ${shots}`);
}catch(error){if(page)await page.screenshot({animations:'disabled',path:`${shots}/failure.png`});throw error;}finally{await browser.close();}
