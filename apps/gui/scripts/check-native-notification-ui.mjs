// Real notification UI/state. Only native presentation IPC is replaced.
import assert from 'node:assert/strict';
import {mkdtempSync,writeFileSync,appendFileSync,mkdirSync,rmSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';
import {chromium} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url)),out=process.env.NOTIFICATION_UI_DIR;
assert(out);mkdirSync(out,{recursive:true});
const dir=mkdtempSync(join(gui,'.native-notice-')),name=dir.slice(gui.length),put=(file,text)=>{const p=join(dir,file);writeFileSync(p,text);return p;};
let server,browser;
try{
 const api=put('api.ts',`export const inApp=true;const listeners=new Map();export const calls=[];
 export const hear=(name,fn)=>{let set=listeners.get(name);if(!set)listeners.set(name,set=new Set());set.add(fn);return()=>set.delete(fn);};
 export const emit=(name,value)=>listeners.get(name)?.forEach(fn=>fn(value));
 export const showMenu=async request=>{calls.push({command:'show',request});emit('menu:rows',request);return true;};
 export const sizeMenu=async(id,h)=>{calls.push({command:'size',id,h});};
 export const menuInput=async(id,event)=>{calls.push({command:'input',id,event});emit('menu:input',{id,event});};
 export const hideMenu=async id=>{calls.push({command:'hide',id});};
 window.nativeNotice={calls,emit};`);
 put('entry.tsx',`import './api';import React,{useEffect,useState} from 'react';import{hear}from'@/api';import{NativeNotificationFace}from'@/components/NativeNotificationFace';import{createRoot}from'react-dom/client';import{NotificationBanners}from'@/views/NotificationBanners';import{receiveNotifications,useNotifications}from'@/lib/notifications';import{TooltipProvider}from'@/components/ui/tooltip';import'@/styles/tokens.css';import'@/styles/app.css';
 const row={id:'gui/test',title:'Native notice',kind:'gui',category:'khor',word:'idle',execution_home:'host',home:'Mac',face:null,imported:false,observation_owner:null,activity:[],last:'The same session preview',doing:null};
 receiveNotifications([row],()=>false);row.activity=[{id:'done',kind:'turn_completed',at_ms:Date.now(),resolved:false,text:null,duration_ms:null,exit:null}];receiveNotifications([row],()=>false);
 function Face(){const state=useNotifications(),[mirror,setMirror]=useState(null);useEffect(()=>hear('menu:rows',setMirror),[]);return <TooltipProvider><div data-native-page-visible="true" style={{position:'fixed',inset:0,zIndex:100,background:'black',color:'white'}}>Native page stand-in</div><NotificationBanners state={state} sessions={[row]} narrow={false} onOpen={()=>{window.opened=true;return true;}}/>{mirror&&<div data-menu-native style={{position:'fixed',left:mirror.at.x,top:mirror.at.y,zIndex:200,width:mirror.at.w}}><NativeNotificationFace key={mirror.id} state={mirror}/></div>}</TooltipProvider>;}createRoot(document.getElementById('root')).render(<React.StrictMode><Face/></React.StrictMode>);`);
 put('index.html',`<!doctype html><html data-theme="dark"><body><div id="root"></div><script type="module" src="/${name}/entry.tsx"></script></body></html>`);
 server=await createServer({plugins:[{name:'notice-diagnostic',enforce:'pre',transform(source,id){
  if(id.endsWith('/native-notification-dom.js'))return source.replace('export function notificationDOM(scope, emit) {',"let traceCount=0;const trace=(s)=>{if(traceCount++<30)console.info('NATIVE_NOTICE_DOM',s)};export function notificationDOM(scope, emit) {").replace('    update(html) {',"    update(html) { trace('update:start');").replace('size();\n    },',"size();trace('update:end');\n    },");
  if(id.endsWith('/use-native-notifications.ts'))return source.replace('export function useNativeNotifications',"let traceCount=0;const trace=(s:string)=>{if(traceCount++<30)console.info('NATIVE_NOTICE_OWNER',s)};export function useNativeNotifications").replace('frame=0;if(!alive)return;',"frame=0;trace('frame:start');if(!alive)return;").replace('const fingerprint=JSON.stringify([html,geometry]);',"trace('snapshot:done');const fingerprint=JSON.stringify([html,geometry]);");
 }}],root:gui,configFile:join(gui,'vite.config.ts'),cacheDir:join(out,'native-vite'),optimizeDeps:{entries:[join(dir,'index.html')]},resolve:{alias:[{find:'@/api',replacement:api},{find:'@',replacement:join(gui,'src')}]},server:{host:'127.0.0.1',port:0,hmr:false},logLevel:'error'});await server.listen();
 browser=await chromium.launch({channel:(process.env.PLAYWRIGHT_CHROMIUM_CHANNEL??'chrome')||undefined,headless:true});const page=await browser.newPage({viewport:{width:1100,height:820}});const errors=[];page.on('pageerror',e=>errors.push(String(e)));page.on('console',message=>{if(message.text().startsWith('NATIVE_NOTICE'))appendFileSync(join(out,'native-notification-trace.txt'),message.text()+'\n');});
 const debuggerSession=await page.context().newCDPSession(page);await debuggerSession.send('Debugger.enable');
 debuggerSession.on('Debugger.paused',data=>{
  writeFileSync(join(out,'native-notification-stack.json'),JSON.stringify(data.callFrames.map(frame=>({functionName:frame.functionName,url:frame.url,location:frame.location})),null,2));
  const frame=data.callFrames[0];
  if(frame)void debuggerSession.send('Debugger.evaluateOnCallFrame',{callFrameId:frame.callFrameId,returnByValue:true,expression:`JSON.stringify({url:location.href,source:document.querySelector('[data-notification-banners]')?.outerHTML.slice(0,400),calls:window.nativeNotice?.calls.slice(-4).map(c=>({command:c.command,event:c.event,id:c.request?.id})),count:window.nativeNotice?.calls.length})`}).then(value=>writeFileSync(join(out,'native-notification-paused.json'),JSON.stringify(value,null,2))).finally(()=>debuggerSession.send('Debugger.resume').catch(()=>{}));
 });
 await page.goto(server.resolvedUrls.local[0]+name+'/index.html');await page.locator('[data-notification-banner]').waitFor();
 try{await page.waitForFunction(()=>nativeNotice.calls.some(call=>call.command==='show'),null,{timeout:5000});}
 catch(error){await page.screenshot({path:join(out,'native-notification-red.png')});writeFileSync(join(out,'native-notification-errors.json'),JSON.stringify({errors,calls:await page.evaluate(()=>nativeNotice.calls)},null,2));throw error;}
 const request=await page.evaluate(()=>nativeNotice.calls.find(call=>call.command==='show').request);
 assert(request.html?.includes('The same session preview'),'The native document carries the actual rendered banner');
 const mirror=page.frameLocator('iframe[data-native-notification-face]'),banner=mirror.locator('[data-notification-banner]');
 await banner.waitFor();await page.waitForFunction(()=>document.querySelector('[data-notification-banners]').hasAttribute('data-notification-native'));
 await page.waitForFunction(()=>nativeNotice.calls.some(call=>call.command==='size'&&call.h>0));
 if(process.env.NATIVE_NOTIFICATION_SNAPSHOT_OUT){
  await page.waitForTimeout(300);
  const html=await page.evaluate(()=>nativeNotice.calls.filter(call=>call.command==='show').at(-1).request.html);
  writeFileSync(process.env.NATIVE_NOTIFICATION_SNAPSHOT_OUT,html.replace(/data-notification-banner="[^"]*"/g,'data-notification-banner="m:local"'));
 }
 const originalKey=await banner.getAttribute('data-notification-banner');
 try{await banner.hover({timeout:2500});}
 catch(error){
  await Promise.race([debuggerSession.send('Debugger.pause'),new Promise(resolve=>setTimeout(resolve,2000))]);
  await page.waitForTimeout(500);
  const diagnostic=await page.evaluate(()=>{
   const box=node=>node?{rect:node.getBoundingClientRect().toJSON(),style:node.getAttribute('style'),attributes:[...node.attributes].map(a=>[a.name,a.value]),animations:node.getAnimations({subtree:true}).map(a=>({state:a.playState,time:a.currentTime}))}:null;
   const f=document.querySelector('iframe'),doc=f?.contentDocument;
   return {errors:window.noticeErrors,source:box(document.querySelector('[data-notification-banners]')),banner:box(document.querySelector('[data-notification-banner]')),frame:box(f),mirror:box(doc?.querySelector('[data-notification-banners]')),mirroredBanner:box(doc?.querySelector('[data-notification-banner]')),calls:nativeNotice.calls.slice(-8).map(c=>({...c,request:c.request?{...c.request,html:c.request.html?.slice(0,100)}:undefined})),totalCalls:nativeNotice.calls.length};
  });
  writeFileSync(join(out,'native-notification-hover.json'),JSON.stringify({errors,diagnostic},null,2));await page.screenshot({path:join(out,'native-notification-hover.png')});throw error;
 }
 await page.waitForTimeout(150);
 assert(await page.locator('[data-notification-banner]').first().getAttribute('data-notification-engaged'),'Native hover reaches the owner');
 // Pointer capture remains in the private frame while the owner updates its rendered transform.
 const rect=await banner.boundingBox();await page.mouse.move(rect.x+rect.width/2,rect.y+rect.height*.8);await page.mouse.down();await page.mouse.move(rect.x+rect.width/2,rect.y-rect.height,{steps:10});await page.mouse.up();
 await page.waitForFunction(()=>document.querySelector('[data-notification-closing]'),null,{timeout:1000});
 await page.locator('[data-notification-banner]').first().waitFor({state:'detached'});
 assert.equal(await page.evaluate(()=>window.opened===true),false,'Swipe does not open a session');
 assert((await page.evaluate(()=>nativeNotice.calls.filter(call=>call.command==='input').map(call=>call.event.key))).every(key=>key===originalKey),'Inputs preserve the original lease candidate');
 assert.deepEqual(errors,[]);writeFileSync(join(out,'native-notification-results.json'),JSON.stringify({nativePresenter:true,sharedRenderedContent:true,hoverForwarding:true,swipeDismissal:true,noClickAfterSwipe:true,errors},null,2));
}finally{await browser?.close();await server?.close();rmSync(dir,{recursive:true,force:true});}
