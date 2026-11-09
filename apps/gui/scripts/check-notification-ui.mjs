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
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
  assert.equal(await page.locator('[data-notification-list]').count(),0,'No notification history UI');
  await page.screenshot({path:join(out,`notifications-${width}-${theme}.png`)});
  await page.locator('[data-notification-open]').last().click();assert.match(await page.locator('[data-lab-opened]').innerText(),/shell\/build@bbbb/);
  await page.locator('[data-lab-resolve]').click();assert.equal(await banners.filter({hasText:'Khor-main'}).count(),0,'Host resolution closes pending input');
  await page.locator('[data-lab-complete]').click();
  const active=banners.filter({hasText:'Khor-main'});await active.waitFor();
  await active.hover();await page.clock.fastForward(9000);assert.equal(await active.count(),1,'Hover pauses auto dismissal');
  await page.mouse.move(0,800);await page.clock.fastForward(9000);assert.equal(await banners.count(),0,'All transient banners expire');
  await page.locator('[data-lab-complete]').click();await banners.waitFor();await banners.locator('[data-notification-dismiss]').click();assert.equal(await banners.count(),0);
  await page.locator('[data-lab-mute]').click();await page.locator('[data-lab-complete]').click();assert.equal(await banners.count(),0);
  await page.locator('[data-lab-follow]').click();await page.locator('[data-lab-complete]').click();await banners.waitFor();
  const stored=await page.evaluate(()=>Object.fromEntries(Object.entries(localStorage).filter(([key])=>key.startsWith('khor.notification'))));
  assert(!('khor.notification-inbox' in stored));assert(!stored['khor.notification-preferences'].includes('Khor-main'));assert(!stored['khor.notification-preferences'].includes('items'));
  await page.reload();await page.locator('[data-lab-complete]').waitFor();assert.equal(await banners.count(),0,'Reload never restores banner content');
  assert.deepEqual(errors,[]);results.push({width,theme,position:bounds,expiry:true,pause:true,mute:true,exactTarget:true,noHistory:true});await page.close();
 }
 writeFileSync(join(out,'results.json'),JSON.stringify(results,null,2));console.log('Transient notification UI:',results);
}finally{await browser?.close();await server?.close();}
