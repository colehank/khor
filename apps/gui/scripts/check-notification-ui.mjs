import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {createServer} from 'vite';
import {chromium} from 'playwright-core';

const gui=fileURLToPath(new URL('..',import.meta.url));
const out=process.env.NOTIFICATION_UI_DIR;
assert(out,'Set NOTIFICATION_UI_DIR to an isolated evidence directory');
mkdirSync(out,{recursive:true});
let server,browser;
const results=[];
try {
  server=await createServer({root:gui,configFile:join(gui,'vite.config.ts'),cacheDir:join(out,'vite'),
    optimizeDeps:{entries:[join(gui,'lab/notifications.html')]},server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'error'});
  await server.listen();
  const url=server.resolvedUrls.local[0]+'lab/notifications.html';
  browser=await chromium.launch({channel:(process.env.PLAYWRIGHT_CHROMIUM_CHANNEL??'chrome')||undefined,headless:true});
  for(const width of [1100,390]) {
    const page=await browser.newPage({viewport:{width,height:820}}),errors=[];
    page.on('pageerror',e=>errors.push(String(e)));
    await page.goto(url);await page.locator('[data-notification-list]').waitFor();
    assert.equal(await page.locator('[data-notification]').count(),3);
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false);
    assert.deepEqual(errors,[]);
    await page.screenshot({path:join(out,`notifications-${width}.png`)});
    results.push(`layout ${width}`);await page.close();
  }
  const context=await browser.newContext({viewport:{width:390,height:820}}),page=await context.newPage();
  await page.goto(url);
  const pending=()=>page.locator('[data-notification][data-pending="true"]');
  await pending().waitFor();
  await page.getByRole('button',{name:'全部已读',exact:true}).click();
  assert.equal(await pending().count(),1);
  assert.equal(await page.locator('[data-notification][data-read="false"]').count(),0);
  results.push('reading preserves pending input');
  await page.getByRole('button',{name:/Khor-main/}).click();
  assert.match(await page.locator('[data-lab-opened]').innerText(),/gui\/khor@aaaa/);
  results.push('exact session and execution home');
  await page.locator('[data-lab-resolve]').click();
  assert.equal(await pending().count(),0);await page.getByText('已处理',{exact:true}).waitFor();
  results.push('host resolution');
  const count=await page.locator('[data-notification]').count();
  await page.reload();await page.locator('[data-notification-list]').waitFor();
  assert.equal(await page.locator('[data-notification]').count(),count);assert.equal(await pending().count(),0);
  results.push('persistent replay and stale resolution');
  const second=await context.newPage();await second.goto(url);await second.locator('[data-notification-list]').waitFor();
  await page.locator('[data-lab-complete]').click();
  await second.locator('[data-notification][data-read="false"]').waitFor();
  await second.getByRole('button',{name:'全部已读',exact:true}).click();
  await page.waitForFunction(()=>!document.querySelector('[data-notification][data-read="false"]'));
  results.push('cross-window inbox and read receipts');
  writeFileSync(join(out,'results.json'),JSON.stringify(results,null,2));
  console.log('notification UI:',results);
} finally {
  if(browser)await browser.close();
  if(server)await server.close();
}
