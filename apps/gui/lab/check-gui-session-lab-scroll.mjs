import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {chromium} from 'playwright-core';
const base=process.env.LAYOUT_LAB_URL??'http://localhost:23795',shots='/Volumes/disk0/khor-gui-session-layout-proof/outer-arcs';mkdirSync(shots,{recursive:true});
const browser=await chromium.launch({channel:'chrome',headless:process.env.PW_HEADLESS==='1'});let checks=0;
try{
 const page=await browser.newPage();
 for(const [width,height] of [[320,844],[390,844],[667,375]]){
  await page.setViewportSize({width,height});await page.goto(base+'/lab/gui-session-layout.html');await page.getByRole('heading',{name:'会话操作区'}).waitFor();await page.frameLocator('iframe').locator('[data-lab-fan-avatar]').waitFor();
  const stage=page.locator('.layout-lab-stage'),before=await stage.boundingBox();await page.mouse.move(6,Math.min(400,height-20));await page.mouse.wheel(0,1100);await page.waitForFunction(()=>window.scrollY>100);const after=await stage.boundingBox();assert(after.y<before.y-100,'the actual wheel moves from controls toward preview');checks++;
  await page.getByRole('link',{name:'查看预览 ↓',exact:true}).click();await page.waitForFunction(()=>{const r=document.querySelector('.layout-lab-stage').getBoundingClientRect();return r.y>=-1&&r.y<30;});assert(await page.getByRole('link',{name:'打开全屏 ↗',exact:true}).getAttribute('href'));checks++;
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,'the Lab does not force the entire phone sideways');
  assert.equal(await page.frameLocator('iframe').locator('html').evaluate(e=>getComputedStyle(e).overflowY),'hidden','scrolling remains locked inside the actual App');assert.equal(await page.frameLocator('iframe').locator('html').getAttribute('data-layout-lab-shell'),null);checks++;
  await page.screenshot({path:`${shots}/lab-scroll-${width}.png`});
 }
 await page.setViewportSize({width:390,height:844});await page.goto(base+'/lab/gui-session-layout.html');await page.getByRole('heading',{name:'会话操作区'}).waitFor();const cdp=await page.context().newCDPSession(page);await cdp.send('Emulation.setTouchEmulationEnabled',{enabled:true,maxTouchPoints:1});
 await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x:6,y:650,id:0}]});for(let y=610;y>=170;y-=40)await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x:6,y,id:0}]});await cdp.send('Input.dispatchTouchEvent',{type:'touchEnd',touchPoints:[]});await page.waitForFunction(()=>window.scrollY>100);checks++;
 console.log(`${checks} narrow Lab wheel/touch scroll, preview jump, width and App isolation checks passed. Screenshots: ${shots}`);
}finally{await browser.close();}
