import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import path from 'node:path';

export async function checkLongHistory(browser,{port,report,mobile=false,short=false,engine='chromium'}){
 const name=`long-history-${engine}-${mobile?'mobile':'desktop'}-${short?'small-pages':'normal-pages'}`;
 const context=await browser.newContext({viewport:mobile?{width:390,height:844}:{width:1280,height:900},isMobile:mobile,hasTouch:mobile});
 const page=await context.newPage(),errors=[];page.setDefaultTimeout(15000);page.on('pageerror',e=>errors.push(String(e)));
 const stats=()=>page.evaluate(async()=>{const response=await fetch('/api/__ai_preview/history-stats',{method:'POST',headers:{'content-type':'application/json'},body:'{}'});return response.json();});
 const geometry=()=>page.locator('[data-chat-scroll]').evaluate(el=>({top:el.scrollTop,height:el.scrollHeight,viewport:el.clientHeight,items:el.querySelectorAll('[data-history-key]').length}));
 let fill,read,atLatest;
 try{
  await page.goto(`http://127.0.0.1:${port}/app/?scene=gui&machines=3&history=long${short?'&short-pages=1':''}`);
  const scroller=page.locator('[data-chat-scroll]:visible');await scroller.waitFor();
  await page.waitForFunction(()=>document.querySelector('[data-chat-column]')?.textContent.includes('第240轮'),null,{timeout:15000});
  await page.waitForFunction(()=>{const e=document.querySelector('[data-chat-scroll]');return e&&e.scrollHeight>e.clientHeight+32;},null,{timeout:10000});
  fill={...await geometry(),io:await stats()};
  assert(fill.items<=512,'Rendered history stays bounded');
  await page.screenshot({path:path.join(report,name+'-initial.png')});
  const bounds=await scroller.boundingBox();await page.mouse.move(bounds.x+bounds.width/2,bounds.y+bounds.height/2);
  for(let i=0;i<5;i++){await page.mouse.wheel(0,-Math.max(600,bounds.height));await page.waitForTimeout(160);}
  await page.waitForFunction(async before=>{const r=await fetch('/api/__ai_preview/history-stats',{method:'POST',headers:{'content-type':'application/json'},body:'{}'});return (await r.json()).backward>before;},fill.io.backward);
  read={...await geometry(),io:await stats()};assert(read.height-read.viewport-read.top>32,'Older loading keeps the reader away from the latest tail');
  const bottom=page.locator('[data-chat-bottom]');await bottom.waitFor();assert.equal(await bottom.innerText(),'');assert.equal(await bottom.getAttribute('data-glass-circle'),'');
  const place=await bottom.evaluate(node=>{const b=node.getBoundingClientRect(),owner=node.offsetParent.getBoundingClientRect();return {right:owner.right-b.right,bottom:owner.bottom-b.bottom,expectedRight:parseFloat(getComputedStyle(node).getPropertyValue('--gap-row'))};});
  assert(Math.abs(place.right-place.expectedRight)<1,'Return glass is at the right edge');assert(place.bottom>=0);
  await page.screenshot({path:path.join(report,name+'-older.png')});
  await bottom.click();await page.waitForFunction(()=>{const e=document.querySelector('[data-chat-scroll]');return e&&Math.abs(e.scrollHeight-e.clientHeight-e.scrollTop)<2&&e.textContent.includes('第240轮');});
  atLatest={...await geometry(),io:await stats()};assert.equal(await bottom.count(),0);assert(atLatest.items<=512);
 }catch(error){errors.push(String(error));await writeFile(path.join(report,name+'-body.txt'),(await page.locator('body').innerText()).slice(0,20000));}
 finally{await page.screenshot({path:path.join(report,name+'.png')});await writeFile(path.join(report,name+'-diagnosis.json'),JSON.stringify({fill,read,atLatest,current:await geometry().catch(()=>null),io:await stats().catch(()=>null),errors},null,2));await context.close();}
 return {name,fill,read,atLatest,errors};
}
