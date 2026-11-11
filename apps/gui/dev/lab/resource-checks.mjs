import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import path from 'node:path';

export async function checkResourceFiles(browser,{port,report,mobile=false,engine='chromium'}) {
 const name=`file-resources-${engine}-${mobile?'mobile':'desktop'}`,errors=[];
 const context=await browser.newContext({viewport:mobile?{width:390,height:844}:{width:1280,height:900},hasTouch:mobile,isMobile:mobile});
 await context.addInitScript(()=>{
  window.__mediaRetired=[];
  for(const method of ['pause','load']){const original=HTMLMediaElement.prototype[method];HTMLMediaElement.prototype[method]=function(...args){window.__mediaRetired.push({method,hasSource:this.hasAttribute('src')});return original.apply(this,args);};}
 });
 const page=await context.newPage();const requests=[];
 page.on('request',request=>{if(request.method()==='POST')requests.push({command:request.url().split('/').pop(),body:request.postDataJSON()});});
 async function open(file){await page.goto(`http://127.0.0.1:${port}/app/?scene=workspace&machines=3&file=media/${file}`);}
 try{
  await open('preview.html');
  await page.locator('[data-file-more]:visible').first().click();
  await page.locator('[data-file-more-item="face"]:visible').click();
  const frame=page.frameLocator('[data-html-frame]');
  await frame.locator('html[data-resources-ready=yes]').waitFor();
  assert.match(await frame.locator('#result').innerText(),/JavaScript 模块、相对样式和 JSON 均已加载/);
  await frame.locator('#counter').click();assert.equal(await frame.locator('#counter').innerText(),'点击计数：1');
  assert.equal(await frame.locator('body').evaluate(node=>getComputedStyle(node).fontFamily.includes('system-ui')),true);
  assert.equal(await frame.locator('body').evaluate(()=>{try{return parent.document.body===null;}catch{return true;}}),true,'A preview document cannot read the App DOM');
  await page.screenshot({path:path.join(report,name+'-html.png')});
  for(const [file,tag] of [['clip.mp4','video'],['audio.wav','audio']]){
   await open(file);const player=page.locator(`[data-resource-preview=${tag}] ${tag}`);
   await player.waitFor();await player.evaluate(node=>{node.muted=true;});
   await writeFile(path.join(report,name+'-'+tag+'-before-play.json'),JSON.stringify(await player.evaluate(node=>({src:node.getAttribute('src'),current:node.currentSrc,ready:node.readyState,network:node.networkState,error:node.error?.code,duration:node.duration})),null,2));
   await player.click();await player.evaluate(node=>Promise.race([node.play(),new Promise((_,reject)=>setTimeout(()=>reject(new Error('play() did not settle: '+JSON.stringify({src:node.getAttribute('src'),current:node.currentSrc,ready:node.readyState,network:node.networkState,error:node.error?.code}))),10_000))]));
   await player.evaluate(node=>{if(node.readyState>=1)return;return new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(new Error('Media metadata did not arrive')),10_000);node.addEventListener('loadedmetadata',()=>{clearTimeout(timer);resolve();},{once:true});node.addEventListener('error',()=>{clearTimeout(timer);reject(new Error('Media decode failed'));},{once:true});});});
   const facts=await player.evaluate(node=>({controls:node.controls,duration:node.duration,src:node.currentSrc}));
   assert(facts.controls&&facts.duration>4.8&&facts.duration<5.2,`${file} decodes through native browser controls`);
   await player.evaluate(node=>{node.currentTime=2;});
   await page.waitForFunction(tag=>{const el=document.querySelector(`[data-resource-preview=${tag}] ${tag}`);return el&&el.currentTime>=1.9&&el.currentTime<3;},tag);
   await page.screenshot({path:path.join(report,name+'-'+tag+'.png')});
   // Leave the workspace through its real navigation. The component has to
   // retire its player and lease; destroying the browser is not the test.
   for(let i=0;i<3&&await player.count();i++){
    const back=page.locator('[data-ws-back]:visible,[data-ws-list-back]:visible,[data-back]:visible').first();
    await back.click();await page.waitForTimeout(100);
   }
   await player.waitFor({state:'detached'});
   await page.waitForFunction(async()=>{const r=await fetch('/api/__ai_preview/resource-state',{method:'POST',headers:{'content-type':'application/json'},body:'{}'});return (await r.json()).active===0;});
   assert(await page.evaluate(()=>window.__mediaRetired.some(event=>event.method==='load'&&!event.hasSource)),'Leaving clears the source and unloads the native player');
   assert(!requests.some(r=>['preview','read_text'].includes(r.command)&&String(r.body?.path).endsWith(file)),'Media never enters the whole-file base64/text pipeline');
  }
 }catch(error){errors.push(String(error));await writeFile(path.join(report,name+'-body.txt'),(await page.locator('body').innerText()).slice(0,16000));await writeFile(path.join(report,name+'-failure.json'),JSON.stringify({error:String(error),players:await page.locator('audio,video').evaluateAll(nodes=>nodes.map(node=>({tag:node.tagName,ready:node.readyState,network:node.networkState,error:node.error?{code:node.error.code,message:node.error.message}:null,current:node.currentTime,paused:node.paused,duration:node.duration})))},null,2));}
 finally{await page.screenshot({path:path.join(report,name+'.png')});await context.close();}
 return {name,errors};
}
