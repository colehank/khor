// Real viewport module and browser layout; only OS viewport notifications are controlled.
import assert from 'node:assert/strict';
import {build} from 'esbuild';
import {chromium} from 'playwright-core';
const built=await build({entryPoints:['src/lib/viewport.ts'],bundle:true,write:false,format:'iife',globalName:'viewport',define:{'import.meta.env.DEV':'false'}});
const browser=await chromium.launch({headless:true});
try{
 for(const event of ['visibilitychange','pageshow','focus','khor:foreground']){
  const page=await browser.newPage({viewport:{width:390,height:844}});
  try{
   await page.setContent('<input id="draft" value="Keep my input"><div style="height:100vh"></div>');
   await page.evaluate(()=>{
    const value=new EventTarget();Object.assign(value,{height:844,width:390,offsetTop:0,offsetLeft:0,scale:1});
    Object.defineProperty(window,'visualViewport',{configurable:true,value});window.viewportFixture=value;
   });
   await page.addScriptTag({content:built.outputFiles[0].text+';window.stopViewport=viewport.followVisualViewport();'});
   await page.locator('#draft').focus();
   await page.evaluate(()=>{viewportFixture.height=500;viewportFixture.dispatchEvent(new Event('resize'));});
   await page.waitForFunction(()=>viewport.keyboardShown());
   assert.equal(await page.evaluate(()=>document.documentElement.style.getPropertyValue('--app-h')),'500px');
   await page.evaluate(()=>{viewportFixture.height=844;}); // OS resumes without a resize event.
   assert.equal(await page.evaluate(()=>document.documentElement.style.getPropertyValue('--app-h')),'500px','The stale pre-background stamp is reproduced');
   await page.evaluate(event=>(event==='visibilitychange'?document:window).dispatchEvent(new Event(event)),event);
   await page.waitForFunction(()=>!document.documentElement.hasAttribute('data-keyboard')&&!viewport.keyboardShown());
   assert.equal(await page.locator('#draft').inputValue(),'Keep my input');
   assert.equal(await page.evaluate(()=>document.activeElement.id),'draft','Re-measuring never blurs or refocuses the input');
   await page.evaluate(()=>{stopViewport();viewportFixture.height=420;window.dispatchEvent(new Event('khor:foreground'));});
   assert.equal(await page.evaluate(()=>document.documentElement.style.getPropertyValue('--app-h')),'','The retired listener cannot mutate the next viewport owner');
   console.log('PASS viewport resume without OS resize:',event);
  }finally{await page.close();}
 }
}finally{await browser.close();}
