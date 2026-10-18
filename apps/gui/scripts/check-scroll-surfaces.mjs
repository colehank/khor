// Browser regression for the same interactive fixture used by CUA.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url)),port=Number(process.env.SCROLL_SURFACES_PORT??1494);
let vite,browser;
try{
 vite=spawn('node_modules/.bin/vite',['--port',String(port),'--strictPort'],{cwd:gui,stdio:['ignore','pipe','pipe']});
 let ready=false,log='';vite.stdout.on('data',b=>{ready||=String(b).includes(`:${port}`);});vite.stderr.on('data',b=>{log+=String(b);});
 for(let n=0;n<100&&!ready;n++)await new Promise(r=>setTimeout(r,100));assert(ready,log||'Vite failed to start');
 browser=await chromium.launch({channel:'chrome',headless:true});const page=await browser.newPage({viewport:{width:1200,height:800}});
 await page.goto(`http://localhost:${port}/lab/scroll-surfaces.html`);
 await page.getByRole('button',{name:'检查列表',exact:true}).click();
 await page.waitForFunction(()=>document.querySelector('[data-scroll-proof-report]')?.textContent.includes('"results"'));
 const report=JSON.parse(await page.locator('[data-scroll-proof-report]').textContent());assert(report.pass,JSON.stringify(report));
 for(const pane of ['会话','文件','机器']){
   const rows=report.results.filter(r=>r.pane===pane);
   assert.equal(new Set(rows.map(r=>r.range)).size,1,`${pane}: material changes altered scroll range`);
   assert(rows.every(r=>r.top===180),`${pane}: scroll position changed`);
 }
 console.log(`scroll surfaces: ${report.results.length} header/theme/material/opacity cases passed`);
}finally{await browser?.close();vite?.kill();}
