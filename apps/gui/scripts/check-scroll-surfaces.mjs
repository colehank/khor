// Browser regression for the same interactive fixture used by CUA.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {lstatSync,writeFileSync,mkdirSync} from 'node:fs';
import {isAbsolute,join} from 'node:path';
import {chromium,webkit} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url)),port=Number(process.env.SCROLL_SURFACES_PORT??1494),existing=process.env.SCROLL_SURFACES_URL;
const processes=process.argv.includes('--processes');
const material=process.argv.includes('--material');
async function headerMaterialPaint(page,engine){
 const select='[data-scroll-proof="会话"] [data-band]';
 await page.locator(select).waitFor();
 const results=[],directory=process.env.SCROLL_SURFACES_MATERIAL_DIR;
 if(directory)mkdirSync(directory,{recursive:true});
 for(const theme of ['light','dark'])for(const color of ['rgb(80, 110, 140)','rgb(140, 90, 80)','rgb(70, 120, 100)']){
  await page.evaluate(({theme,color})=>{
   document.documentElement.dataset.theme=theme;
   document.documentElement.setAttribute('data-window-transparent','');
   document.documentElement.style.setProperty('--window-opacity','20%');
   document.documentElement.style.background=color;
   // A uniform underlay isolates color gain from actual blur and row glyphs.
   document.querySelectorAll('.scroll-proof-list').forEach(el=>{el.style.color='transparent';el.scrollTop=180;});
  },{theme,color});
  await page.waitForFunction(select=>document.querySelector(select).hasAttribute('data-scrolled')&&getComputedStyle(document.querySelector(select),'::before').opacity==='1',select);
  const head=page.locator(select),box=await head.boundingBox();
  const clip={x:Math.floor(box.x+box.width/2),y:Math.ceil(box.y+4),width:12,height:12};
  const actual=await page.screenshot({clip,animations:'disabled'});
  await head.evaluate(el=>el.style.setProperty('--band-blur','none'));
  const reference=await page.screenshot({clip,animations:'disabled'});
  await head.evaluate(el=>el.style.removeProperty('--band-blur'));
  const pixels=await page.evaluate(async encoded=>{
   const samples=[];
   for(const data of encoded){
    const image=new Image();image.src='data:image/png;base64,'+data;await image.decode();
    const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;
    const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);const bytes=ctx.getImageData(0,0,image.width,image.height).data;
    const mean=[0,0,0];for(let i=0;i<bytes.length;i+=4)for(let k=0;k<3;k++)mean[k]+=bytes[i+k]/(bytes.length/4);samples.push(mean);
   }
   return samples;
  },[actual.toString('base64'),reference.toString('base64')]);
  const delta=Math.max(...pixels[0].map((value,k)=>Math.abs(value-pixels[1][k])));
  results.push({engine,theme,color,actual:pixels[0],reference:pixels[1],delta});
 }
 if(directory){writeFileSync(join(directory,`${engine}-material.json`),JSON.stringify(results,null,2)+'\n');await page.screenshot({path:join(directory,`${engine}-material.png`)});}
 assert(results.every(result=>result.delta<=2),`Fixed header changes uniform backdrop color: ${JSON.stringify(results)}`);
 console.log(`header material: ${engine} ${results.length} uniform-color comparisons passed`);
}
async function processHeadingPaint(page){
 const select='[data-scroll-proof="processes"]';
 const geometry=await page.evaluate(async select=>{
   const pane=document.querySelector(select),head=pane.querySelector('[data-top-heads]'),body=pane.querySelector('[data-top-body]'),port=head.closest('[data-band-scroll]');
   assertPort();
   function assertPort(){if(!port)throw new Error('process fixture has no real band owner');}
   const frame=()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
   const textBox=element=>{const range=document.createRange();range.selectNodeContents(element);return range.getBoundingClientRect();};
   port.scrollTop=100;await frame();
   const label=textBox(head.querySelector('[data-process-heading]')),row=textBox(body.querySelector('[data-top-row="8"] > :first-child'));
   port.scrollTop+=row.top+row.height/2-label.top-label.height/2;await frame();
   const box=head.getBoundingClientRect(),ink=textBox(head.querySelector('[data-process-heading]'));
   if(!head.hasAttribute('data-sticky-held'))throw new Error('process heading never became held');
   return {clip:{x:Math.ceil(box.left),y:Math.floor(ink.top),width:Math.floor(box.width)-1,height:Math.ceil(ink.bottom)-Math.floor(ink.top)},scrollTop:port.scrollTop};
 },select);
 await page.waitForFunction(select=>getComputedStyle(document.querySelector(select+' [data-top-heads]'),'::before').opacity==='1',select);
 const snapshot=()=>page.evaluate(select=>{
   const head=document.querySelector(select+' [data-top-heads]'),body=document.querySelector(select+' [data-top-body]'),port=head.closest('[data-band-scroll]');
   const box=node=>{const r=node.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height};};
   return {head:box(head),body:box(body),scrollTop:port.scrollTop};
 },select);
 const before=await snapshot();
 const sample=async()=>{
   const visible=await page.screenshot({clip:geometry.clip,animations:'disabled'});
   let hidden;
   try{
     await page.locator(select+' [data-top-body]').evaluate(body=>{body.style.visibility='hidden';});
     assert.deepEqual(await snapshot(),before,'visibility comparison must preserve the held heading, body and scroll geometry');
     hidden=await page.screenshot({clip:geometry.clip,animations:'disabled'});
   }finally{await page.locator(select+' [data-top-body]').evaluate(body=>body.style.removeProperty('visibility'));}
   return {visible,hidden};
 };
 const compare=async pair=>page.evaluate(async images=>{
   const decode=async text=>{
     const image=new Image();image.src='data:image/png;base64,'+text;await image.decode();
     const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;
     const context=canvas.getContext('2d');context.drawImage(image,0,0);return context.getImageData(0,0,image.width,image.height);
   };
   const [a,b]=await Promise.all(images.map(decode));
   if(a.width!==b.width||a.height!==b.height)throw new Error('paint samples changed dimensions');
   let changedPixels=0,maxDelta=0;
   for(let p=0;p<a.data.length;p+=4){let delta=0;for(let c=0;c<3;c++)delta=Math.max(delta,Math.abs(a.data[p+c]-b.data[p+c]));maxDelta=Math.max(maxDelta,delta);if(delta>0)changedPixels++;}
   return {pixels:a.width*a.height,changedPixels,maxDelta};
 },[pair.visible.toString('base64'),pair.hidden.toString('base64')]);
 const actual=await sample(),actualPaint=await compare(actual);
 // Blur may legitimately sample nearby colour. Disable only that fixture
 // layer for a second pair, to isolate the production body mask's glyphs.
 const control=await page.addStyleTag({content:select+' [data-top-heads]::before { -webkit-backdrop-filter:none !important; backdrop-filter:none !important; }'});
 let direct;
 try{assert.deepEqual(await snapshot(),before);direct=await sample();}finally{await control.evaluate(style=>style.remove());}
 const directPaint=await compare(direct);
 const artifacts=process.env.SCROLL_SURFACES_ARTIFACTS;
 assert(artifacts&&isAbsolute(artifacts)&&lstatSync(artifacts).isDirectory()&&!lstatSync(artifacts).isSymbolicLink(),'explicit fresh artifact directory required');
 for(const [name,pair] of [['actual-blur',actual],['direct-mask',direct]])for(const [state,png] of Object.entries(pair))writeFileSync(join(artifacts,`processes-${name}-${state}.png`),png,{flag:'wx'});
 const report={scope:'held Processes label-band actual paint',...geometry,before,actualPaint,directPaint};
 writeFileSync(join(artifacts,'processes-paint.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});
 console.log(JSON.stringify(report));
 assert.equal(directPaint.changedPixels,0,'the body mask must suppress direct glyphs in the held Processes label band; actual blur colour differences are diagnostic only');
}
let vite,browser;
try{
 if(!existing){
  vite=spawn('node_modules/.bin/vite',['--port',String(port),'--strictPort'],{cwd:gui,stdio:['ignore','pipe','pipe']});
  let ready=false,log='';vite.stdout.on('data',b=>{ready||=String(b).includes(`:${port}`);});vite.stderr.on('data',b=>{log+=String(b);});
  for(let n=0;n<100&&!ready;n++)await new Promise(r=>setTimeout(r,100));assert(ready,log||'Vite failed to start');
 }
 const url=new URL(existing??`http://localhost:${port}/lab/scroll-surfaces.html`);if(processes)url.searchParams.set('surface','processes');
 if(material){
  for(const [driver,name] of [[chromium,'chromium'],[webkit,'webkit']]){
   browser=await driver.launch({headless:true});
   const page=await browser.newPage({viewport:{width:1200,height:800}});await page.goto(url.href);
   await headerMaterialPaint(page,name);await browser.close();browser=undefined;
  }
 }else{
 browser=await chromium.launch({channel:(process.env.PLAYWRIGHT_CHROMIUM_CHANNEL??'chrome')||undefined,headless:true});const page=await browser.newPage({viewport:{width:1200,height:800}});
 await page.goto(url.href);
 if(processes){
   await page.locator('[data-scroll-proof="processes"] [data-top-heads]').waitFor({state:'visible'});
   await processHeadingPaint(page);
 }else{
 await page.getByRole('button',{name:'检查列表',exact:true}).click();
 await page.waitForFunction(()=>document.querySelector('[data-scroll-proof-report]')?.textContent.includes('"results"'));
 const report=JSON.parse(await page.locator('[data-scroll-proof-report]').textContent());assert(report.pass,JSON.stringify(report));
 assert.equal(report.results.length,36,'the full header/theme/material/opacity matrix must run');
 for(const pane of ['会话','文件','机器']){
   const rows=report.results.filter(r=>r.pane===pane);
   assert.equal(new Set(rows.map(r=>r.range)).size,1,`${pane}: material changes altered scroll range`);
   assert(rows.every(r=>r.top===180),`${pane}: scroll position changed`);
 }
 console.log(`scroll surfaces: ${report.results.length} header/theme/material/opacity cases passed`);
 }
 }
}finally{await browser?.close();vite?.kill();}
