import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {createServer} from 'vite';
import {chromium} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url));
const out=process.env.NOTIFICATION_UI_DIR;
assert(out,'Set NOTIFICATION_UI_DIR to an isolated evidence directory');mkdirSync(out,{recursive:true});
let server,browser;const results=[];
try {
 server=await createServer({root:gui,configFile:join(gui,'vite.config.ts'),cacheDir:join(out,'vite'),optimizeDeps:{entries:[join(gui,'lab/notifications.html')]},server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'error'});
 await server.listen();const url=server.resolvedUrls.local[0]+'lab/notifications.html';
 browser=await chromium.launch({channel:(process.env.PLAYWRIGHT_CHROMIUM_CHANNEL??'chrome')||undefined,headless:true});
 for(const width of [1100,390,320])for(const theme of ["light","dark"]){
  const page=await browser.newPage({viewport:{width,height:820},isMobile:width<500,hasTouch:width<500});const errors=[];page.on('pageerror',e=>errors.push(String(e)));
  await page.clock.install();await page.goto(url+"?theme="+theme);await page.locator('[data-lab-burst]').click();
  const banners=page.locator('[data-notification-banner]');await banners.first().waitFor();
  assert.equal(await banners.count(),width<500?1:3);
  const bounds=await page.locator('[data-notification-banners]').boundingBox();assert(bounds.y<40);assert(bounds.x>=(width<500?0:width/2));assert(bounds.x+bounds.width<=width);
  if(width<500){const inset=await page.evaluate(()=>parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--gap-row")));assert.ok(Math.abs(bounds.x-inset)<1&&Math.abs(width-bounds.x-bounds.width-inset)<1,"Phone banner follows the actual App safe horizontal inset");}
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  assert.equal(await page.locator('[data-notification-list]').count(),0,'No notification history UI');
  await page.screenshot({path:join(out,`notifications-${width}-${theme}.png`)});
  assert.match(await banners.first().innerText(),/已完成修改/,'Second line contains the same recent content as the session list');
  await page.locator('[data-notification-open]').last().click();
  assert.match(await page.locator('[data-lab-opened]').innerText(),width<500?/gui\/khor@aaaa/:/shell\/build@bbbb/);
  const clear=async()=>{await page.locator('[data-lab-clear]').click();await banners.waitFor({state:'detached'});};
  const complete=async()=>{await page.locator('[data-lab-complete]').click();await banners.waitFor();await page.clock.runFor(300);};
  await clear();await complete();
  await banners.hover();await page.clock.fastForward(9000);assert.equal(await banners.count(),1,'Hover pauses auto dismissal');
  await page.mouse.move(0,800);await page.clock.fastForward(9000);assert.equal(await banners.count(),0,'Visible notice expires after pause');
  await complete();
  const key=await banners.getAttribute('data-notification-banner');
  await page.locator('[data-lab-complete]').click();assert.equal(await banners.count(),1);assert.equal(await banners.getAttribute('data-notification-banner'),key,'Same session updates its banner in place');
  const opened=await page.locator('[data-lab-opened]').innerText();
  const cdp=width<500?await page.context().newCDPSession(page):null;
  const drag=async(fraction,cancel=false)=>{
   const box=await banners.boundingBox(),x=box.x+box.width/2,y=box.y+box.height*.85,dy=box.height*fraction;
   if(cdp){
    await page.mouse.move(0,800);
    await cdp.send('Input.dispatchTouchEvent',{type:'touchStart',touchPoints:[{x,y}]});
    await cdp.send('Input.dispatchTouchEvent',{type:'touchMove',touchPoints:[{x,y:y-dy}]});
    assert.equal(await banners.count(),1,'Touch holds the visible notice');
    await page.clock.fastForward(9000);assert.equal(await banners.count(),1,'Touch pauses dismissal');
    await cdp.send('Input.dispatchTouchEvent',{type:cancel?'touchCancel':'touchEnd',touchPoints:[]});
   }else{
    await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x,y-dy,{steps:6});
    await page.mouse.up();
   }
   await page.clock.runFor(400);
  };
  await drag(.15);assert.equal(await banners.count(),1,'Short swipe springs back');
  assert.equal(await page.locator('[data-lab-opened]').innerText(),opened,'Drag is never a click through to a session');
  const rest=await banners.evaluate(node=>getComputedStyle(node).transform);assert.ok(rest==='none'||rest==='matrix(1, 0, 0, 1, 0, 0)',rest);
  if(cdp){await drag(.7,true);assert.equal(await banners.count(),1,'OS cancellation does not dismiss');}
  await drag(.7);await banners.waitFor({state:'detached'});assert.equal(await page.locator('[data-lab-opened]').innerText(),opened,'Upward dismissal cannot navigate');
  await complete();await banners.locator('[data-notification-dismiss]').click();await banners.waitFor({state:'detached'});
  await page.locator('[data-lab-mute]').click();await page.locator('[data-lab-complete]').click();assert.equal(await banners.count(),0);
  await page.locator('[data-lab-follow]').click();await page.locator('[data-lab-summary]').click();
  await banners.waitFor();assert.equal(await banners.count(),1);assert.match(await banners.innerText(),/8 个会话有更新/);
  await page.screenshot({path:join(out,`notification-summary-${width}-${theme}.png`)});
  await banners.locator('[data-notification-open]').click();assert.equal(await page.locator('[data-lab-opened]').innerText(),'sessions');
  await clear();await page.emulateMedia({reducedMotion:'reduce'});await complete();
  assert.equal(await banners.evaluate(node=>getComputedStyle(node).animationName),'none','Reduced motion removes entrance travel');
  const stored=await page.evaluate(()=>Object.fromEntries(Object.entries(localStorage).filter(([key])=>key.startsWith('khor.notification'))));
  assert(!('khor.notification-inbox' in stored));assert(!stored['khor.notification-preferences'].includes('Khor-main'));assert(!stored['khor.notification-preferences'].includes('items'));
  await page.reload();await page.locator('[data-lab-complete]').waitFor();assert.equal(await banners.count(),0,'Reload never restores banner content');
  assert.deepEqual(errors,[]);results.push({width,theme,position:bounds,expiry:true,pause:true,mute:true,exactTarget:true,noHistory:true,swipe:true,coalescing:true,summary:true,preview:true,reducedMotion:true});await page.close();
 }
 writeFileSync(join(out,'results.json'),JSON.stringify(results,null,2));console.log('Transient notification UI:',results);
}finally{await browser?.close();await server?.close();}
