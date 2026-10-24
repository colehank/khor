// Real App, actual xterm and actual DeckBand; all bytes stay in the fixture bridge.
import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {chromium} from 'playwright-core';
const base=process.env.LAYOUT_LAB_URL??'http://localhost:23795',shots='/Volumes/disk0/khor-gui-session-layout-proof/outer-arcs';mkdirSync(shots,{recursive:true});
const ids=['shell/layout-lab','gui/ai-elements-review','gui/codex-review','wide/all','gui/ai-elements-plan'];
const browser=await chromium.launch({channel:'chrome',headless:process.env.PW_HEADLESS==='1'});let page;
try{
 page=await browser.newPage({viewport:{width:390,height:844}});const errors=[];page.on('pageerror',e=>errors.push(e.message));let checks=0;
 const band=()=>page.locator('[data-session-bar] [data-deck-band]'),fan=()=>page.locator('[data-lab-left-fan]'),active=()=>page.evaluate(()=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active);
 const settle=()=>page.evaluate(()=>Promise.all(document.getAnimations().filter(a=>Number.isFinite(a.effect?.getComputedTiming().endTime)).map(a=>a.finished.catch(()=>{}))));
 const open=async(theme='dark',keyboard='0',width=390,height=844)=>{
  await page.setViewportSize({width,height});await page.goto(base+'/?'+new URLSearchParams({bridge:new URL(base).port,labSession:ids[0],labTabs:ids.join(','),theme,keyboard,phone:'1',deck:'fan'}));await band().waitFor();
  await page.waitForFunction(()=>{const t=document.querySelector('[data-kept-terminal][data-active] [data-terminal]')?.__xterm;return t?.buffer.active.getLine(0)?.translateToString().includes('layout-lab');});await settle();
  await page.evaluate(()=>window.__labTerminal=document.querySelector('[data-terminal]').__xterm);
 };
 const hold=async()=>{const r=await band().boundingBox(),p={x:r.x+r.width/2,y:r.y+r.height/2};await page.mouse.move(p.x,p.y);await page.mouse.down();await fan().waitFor();assert.equal(await fan().getAttribute('data-fan-placement'),'bottom');return p;};
 const arc=(p,angle,r=35)=>page.mouse.move(p.x+(angle-90)/30*56,p.y-(r>=140?60:0),{steps:4});
 const parked=()=>page.waitForFunction(()=>{const e=document.querySelector('[data-lab-left-fan]');return e&&Number(getComputedStyle(e).opacity)>.99&&Math.abs(Number(e.dataset.fanPosition)-Math.round(Number(e.dataset.fanPosition)))<.001;});
 const hot=()=>fan().getAttribute('data-fan-selected');
 const bounds=async()=>{
  const panel=await fan().boundingBox();
  const rows=await page.locator('[data-lab-fan-session]').evaluateAll(es=>es.map(e=>{const r=e.getBoundingClientRect();return {left:r.left,right:r.right,top:r.top,bottom:r.bottom};}));
  for(const r of rows)assert(r.left>=panel.x-1&&r.right<=panel.x+panel.width+1&&r.top>=panel.y-1&&r.bottom<=panel.y+panel.height+1,'title groups fit the visible terminal');
  const points=await page.locator('.lab-fan-guide path').evaluate(e=>{const n=e.getTotalLength(),rect=e.ownerSVGElement.getBoundingClientRect();return Array.from({length:101},(_,i)=>{const p=e.getPointAtLength(n*i/100);return {x:p.x+rect.x,y:p.y+rect.y};});});
  for(const p of points)for(const r of rows)assert(p.x<r.left||p.x>r.right||p.y<r.top||p.y>r.bottom,'outer control track never crosses any avatar/title');
 };
 for(const theme of ['light','dark'])for(const keyboard of ['0','1']){
  await open(theme,keyboard);assert.equal(await page.locator('[data-lab-model-header]').count(),0);assert.equal(await band().getAttribute('data-band-size'),'5');assert.equal(await page.locator('[data-key-row-switch]').count(),1,'phone preview includes the terminal key-row control');if(keyboard==='1')await page.locator('[data-term-keys]').waitFor();
  let p=await hold();await arc(p,100);assert.equal(await hot(),ids[1]);await arc(p,90);await parked();assert.equal(await active(),ids[0]);assert.equal(await page.locator('[data-lab-fan-title]').count(),5);assert((await page.locator('[data-lab-fan-title]').allTextContents()).every(Boolean));
  const sizes=await page.locator('[data-lab-fan-session]').evaluateAll(es=>es.map(e=>({on:e.hasAttribute('data-selected'),scale:new DOMMatrix(getComputedStyle(e).transform).a,opacity:Number(getComputedStyle(e).opacity)})));const current=sizes.find(s=>s.on);for(const s of sizes.filter(s=>!s.on)){assert(current.scale>s.scale);assert(current.opacity>s.opacity);}
  for(const row of await page.locator('[data-lab-fan-session]').all()){const face=await row.locator('.lab-fan-face').boundingBox(),label=await row.locator('.lab-fan-label').boundingBox();assert(label.y>=face.y+face.height-1,'bottom title stays below avatar');}
  await bounds();assert(await page.evaluate(()=>window.__labTerminal===document.querySelector('[data-terminal]').__xterm));await page.screenshot({path:`${shots}/terminal-${theme}-${keyboard}.png`});await page.keyboard.press('Escape');await page.mouse.up();assert.equal(await active(),ids[0]);checks++;
 }
 await open();let p=await hold();await arc(p,120);assert.equal(await fan().getAttribute('data-fan-tier'),'3');await page.waitForFunction(()=>Number(document.querySelector('[data-lab-left-fan]')?.dataset.fanPulse)>=5);await arc(p,90);assert.equal(await hot(),ids[0]);await parked();assert(Number(await fan().getAttribute('data-fan-position'))>=4.99,'fast detents animate forward through the wrap');await page.mouse.up();await fan().waitFor({state:'hidden'});assert.equal(await active(),ids[0]);checks++;
 p=await hold();await arc(p,60);assert.equal(await hot(),ids.at(-1));assert.equal(await fan().getAttribute('data-fan-direction'),'-1');await page.keyboard.press('Escape');await page.mouse.up();checks++;
 p=await hold();await arc(p,100);const preview=await hot();await arc(p,90,145);await page.locator('[data-lab-fan-overview][data-active]').waitFor();assert.equal(await fan().getAttribute('data-fan-direction'),'0');await arc(p,90,100);assert.equal(await hot(),preview);assert.equal(await page.locator('[data-lab-fan-overview][data-active]').count(),0);await page.keyboard.press('Escape');await page.mouse.up();checks++;
 p=await hold();await arc(p,90,145);await page.locator('[data-lab-fan-overview][data-active]').waitFor();await parked();await page.screenshot({path:`${shots}/terminal-overview.png`});await page.mouse.up();await page.locator('[data-tab-sheet-open="'+ids[0]+'"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});checks++;
 // A native quick horizontal swipe still uses the production direct model.
 const b=await band().boundingBox();await page.mouse.move(b.x+b.width/2,b.y+b.height/2);await page.mouse.down();await page.mouse.move(b.x+b.width*.95,b.y+b.height/2,{steps:5});const swiped=await band().getAttribute('data-band-preview');assert.notEqual(swiped,ids[0]);await page.mouse.up();await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,swiped);checks++;
 await open();p=await hold();await arc(p,100);await arc(p,90);await page.mouse.up();await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,ids[1]);await page.locator('[data-lab-fan-avatar]').waitFor();await page.locator('[data-composer-text]').fill('从终端切回仍保留的草稿');
 await page.locator('[data-lab-fan-avatar]').click();await page.locator('[data-lab-pill-overview]').click();await page.locator('[data-tab-sheet-open="'+ids[0]+'"]').click();await band().waitFor();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});await settle();assert(await page.evaluate(()=>window.__labTerminal===document.querySelector('[data-kept-terminal][data-active] [data-terminal]').__xterm));
 p=await hold();await arc(p,100);await arc(p,90);await page.mouse.up();await page.locator('[data-composer-text]').waitFor();assert.equal(await page.locator('[data-composer-text]').inputValue(),'从终端切回仍保留的草稿');checks++;
 // Expanded GUI pill uses the same titled bottom presentation and horizontal gestures.
 await page.locator('[data-lab-fan-avatar]').click();const expanded=page.locator('[data-lab-session-pill] [data-deck-band]');await expanded.waitFor();const pr=await expanded.boundingBox(),pp={x:pr.x+pr.width/2,y:pr.y+pr.height/2};await page.mouse.move(pp.x,pp.y);await page.mouse.down();await fan().waitFor();assert.equal(await fan().getAttribute('data-fan-placement'),'bottom');await page.mouse.move(pp.x+16,pp.y);await page.mouse.move(pp.x,pp.y);await parked();assert.equal(await active(),ids[1]);assert.equal(await page.locator('[data-lab-fan-title]').count(),5);for(const row of await page.locator('[data-lab-fan-session]').all()){const f=await row.locator('.lab-fan-face').boundingBox(),t=await row.locator('.lab-fan-label').boundingBox();assert(t.y>=f.y+f.height-1);}await page.screenshot({path:`${shots}/expanded-pill-shared.png`});await page.keyboard.press('Escape');await page.mouse.up();await fan().waitFor({state:'hidden'});assert.equal(await active(),ids[1]);checks++;
 for(const interruption of ['pointer','multi','resize','blur']){
  await open();p=await hold();await arc(p,100);
  if(interruption==='pointer')await band().dispatchEvent('pointercancel',{pointerId:1,isPrimary:true});
  else if(interruption==='multi')await band().dispatchEvent('pointerdown',{pointerId:8,isPrimary:false,pointerType:'touch',button:0});
  else if(interruption==='resize')await page.setViewportSize({width:430,height:844});
  else await page.evaluate(()=>window.dispatchEvent(new Event('blur')));
  await page.mouse.up();await fan().waitFor({state:'hidden'});assert.equal(await active(),ids[0]);checks++;
 }
 await open();const keyTarget=await band().locator('button').count()?band().locator('button').first():band();await keyTarget.focus();assert(await keyTarget.evaluate(e=>e===document.activeElement));await page.keyboard.press('ArrowUp');await page.locator('[data-tab-sheet-open="'+ids[0]+'"]').click();await page.locator('[data-tab-sheet]').waitFor({state:'hidden'});checks++;
 const cdp=await page.context().newCDPSession(page);await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:2});const r=await band().boundingBox(),touch={x:r.x+r.width/2,y:r.y+r.height/2,id:0};
 await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[touch]});await fan().waitFor();await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{...touch,x:touch.x+16,y:touch.y}]});const touched=await hot();assert.notEqual(touched,ids[0]);await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await page.waitForFunction(id=>JSON.parse(localStorage.getItem('khor.sessions.tabs')).active===id,touched);assert.equal(await page.locator('[data-tab-sheet]').count(),0);await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:false});checks++;
 for(const [width,height] of [[320,844],[430,932],[667,375]]){await open('dark','1',width,height);p=await hold();await arc(p,100);await arc(p,90);await parked();await bounds();await page.screenshot({path:`${shots}/terminal-width-${width}.png`});await page.keyboard.press('Escape');await page.mouse.up();checks++;}
 await page.setViewportSize({width:1450,height:1050});await page.goto(base+'/lab/gui-session-layout.html');const frame=page.frameLocator('iframe');await frame.locator('[data-lab-fan-avatar]').waitFor();await frame.locator('[data-composer-text]').fill('比较终端时保留草稿');await page.getByLabel('会话',{exact:true}).selectOption(ids[0]);await frame.locator('[data-session-bar] [data-deck-band]').waitFor();await frame.locator('[data-terminal]').waitFor();await page.getByLabel('会话',{exact:true}).selectOption(ids[1]);await frame.locator('[data-composer-text]').waitFor();assert.equal(await frame.locator('[data-composer-text]').inputValue(),'比较终端时保留草稿');checks++;
 assert.deepEqual(errors,[]);console.log(`${checks} actual xterm, vertical titles, horizontal pills, outer track, cycle/reverse, overview, swipe, touch, cancellation and comparison checks passed. Screenshots: ${shots}`);
}catch(e){if(page)await page.screenshot({path:shots+'/terminal-failure.png'});throw e;}finally{await browser.close();}
