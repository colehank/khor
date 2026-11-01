// Production page controls + history hook; only native IPC/events are replaced.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {writeFileSync,rmSync,mkdtempSync} from 'node:fs';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright-core';
const gui=fileURLToPath(new URL('..',import.meta.url)),port=Number(process.env.PAGE_HISTORY_PORT||23934),prefix='.page-history-'+process.pid;
const out=mkdtempSync(join(process.env.KHOR_GATE_TMP||'/Volumes/disk0','khor-page-history-')),paths=[];
const put=(name,text)=>{const p=join(gui,name);paths.push(p);writeFileSync(p,text);return p;};
let vite,browser;
try {
 const api=put('scripts/'+prefix+'-api.ts',`export const inApp=true;const listeners=new Map(),nativeListeners=new Map(),pending=[];let state={back:false,forward:false};export const calls=[];
 export const onPageWord=(event,tab,fn,ready)=>{const key=event+':'+tab,set=listeners.get(key)||new Set();listeners.set(key,set);set.add(fn);queueMicrotask(()=>{if(set.has(fn))ready?.();});return()=>set.delete(fn);};
 export const listenNativeEvent=(event,fn,options={})=>{const set=nativeListeners.get(event)||new Set();nativeListeners.set(event,set);set.add(fn);queueMicrotask(()=>{if(set.has(fn))options.onReady?.();});return()=>set.delete(fn);};
 export const pageSteps=tab=>{calls.push(tab);return new Promise(done=>pending.push({tab,done}));};export const pageStep=()=>Promise.resolve();
 window.historyProbe={pending,calls,emitNative:(event,payload)=>{for(const fn of nativeListeners.get(event)||[])fn(payload);},set:value=>state=value,answer:(tab,value=state)=>{const at=pending.findIndex(p=>p.tab===tab);if(at<0)throw Error('No pending history query '+tab);pending.splice(at,1)[0].done(value);},emit:(event,tab,text)=>{if(event==='page:steps')state={back:text[0]==='1',forward:text[1]==='1'};for(const fn of listeners.get(event+':'+tab)||[])fn(text);}};`);
 put('scripts/'+prefix+'.tsx',`import {useState} from 'react';import {createRoot} from 'react-dom/client';import * as controls from '../src/browse/PageControls';import {TooltipProvider} from '../src/components/ui/tooltip';import '../src/styles/tokens.css';import '../src/styles/app.css';
 function Probe(){const [tab,setTab]=useState('a');window.selectPage=setTab;const state=controls.usePageSteps?controls.usePageSteps(tab):null;return <TooltipProvider><div data-native-page-tab={tab} data-native-page-visible="true"/><div className="glass flex" data-page-probe><controls.PageHistory tab={tab} steps={state}/></div></TooltipProvider>;}createRoot(document.getElementById('root')).render(<Probe/>);`);
 const config=put('scripts/'+prefix+'-config.ts',`import base from '../vite.config';import path from 'node:path';export default {...base,cacheDir:${JSON.stringify(join(out,'cache'))},resolve:{alias:[{find:'@/api',replacement:${JSON.stringify(api)}},{find:'@/lib/native-events',replacement:${JSON.stringify(api)}},{find:'@',replacement:path.resolve(import.meta.dirname,'../src')}]}};`);
 put(prefix+'.html',`<!doctype html><html data-theme="dark"><body><div id="root"></div><script type="module">import R from '/@react-refresh';R.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>t=>t;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/${prefix}.tsx"></script></body></html>`);
 vite=spawn('node_modules/.bin/vite',['--config',config,'--port',String(port),'--strictPort'],{cwd:gui,env:{...process.env,KHOR_FACE_GATE:'1'},stdio:['ignore','pipe','pipe']});
 let ready=false;vite.stdout.on('data',d=>ready||=String(d).includes(':'+port));for(let n=0;n<100&&!ready;n++)await new Promise(r=>setTimeout(r,100));assert(ready,'Vite did not start');
 browser=await chromium.launch({channel:'chrome',headless:true});const page=await browser.newPage();await page.goto(`http://localhost:${port}/${prefix}.html`);
 const back=page.locator('[data-page-step=back]'),forward=page.locator('[data-page-step=forward]');await back.waitFor();
 assert.equal(await back.getAttribute('aria-disabled'),'true','unknown/unavailable history must grey back');assert.equal(await forward.count(),0,'unknown/unavailable forward must be hidden');
 await page.waitForFunction(()=>historyProbe.pending.some(p=>p.tab==='a'));await page.evaluate(()=>historyProbe.answer('a',{back:false,forward:false}));
 assert.equal(await back.getAttribute('aria-disabled'),'true');assert.equal(await forward.count(),0);
 await page.evaluate(()=>historyProbe.emit('page:steps','a','10'));await page.waitForFunction(()=>!document.querySelector('[data-page-step=back]').hasAttribute('aria-disabled'));assert.equal(await forward.count(),0);
 await page.evaluate(()=>historyProbe.emit('page:steps','a','01'));await forward.waitFor();assert.equal(await back.getAttribute('aria-disabled'),'true');
 // An older native query cannot overwrite a newer event.
 await page.evaluate(()=>historyProbe.emitNative('page:navigation',{tab:'a',view:'1',navigation:'2',revision:'3',phase:'finished',estimate:1,requested_url:'https://history.test',url:'https://history.test',exit:'',proxied:false,error:null}));await page.waitForFunction(()=>historyProbe.pending.some(p=>p.tab==='a'));await page.evaluate(()=>{historyProbe.emit('page:steps','a','10');historyProbe.answer('a',{back:false,forward:true});});await page.waitForFunction(()=>!document.querySelector('[data-page-step=back]').hasAttribute('aria-disabled'));assert.equal(await forward.count(),0);
 // A tab change resets its visual state, then asks the native current state.
 await page.evaluate(()=>selectPage('b'));await page.waitForFunction(()=>historyProbe.pending.some(p=>p.tab==='b'));assert.equal(await back.getAttribute('aria-disabled'),'true');assert.equal(await forward.count(),0);await page.evaluate(()=>historyProbe.answer('b',{back:true,forward:true}));await forward.waitFor();assert.equal(await back.getAttribute('aria-disabled'),null);
 await page.evaluate(()=>historyProbe.emit('page:steps','a','00'));assert.equal(await back.getAttribute('aria-disabled'),null,'old tab event changed current tab');
 // SPA changes can be read even without a load event, only while presented.
 await page.evaluate(()=>historyProbe.set({back:false,forward:false}));await page.waitForFunction(()=>historyProbe.pending.some(p=>p.tab==='b'));await page.evaluate(()=>historyProbe.answer('b'));await page.waitForFunction(()=>document.querySelector('[data-page-step=back]').getAttribute('aria-disabled')==='true');assert.equal(await forward.count(),0);
 await page.evaluate(()=>document.querySelector('[data-native-page-tab]').setAttribute('data-native-page-visible','false'));await new Promise(r=>setTimeout(r,500));assert.equal(await page.evaluate(()=>historyProbe.pending.length),0,'hidden page kept polling');
 await page.screenshot({path:join(out,'history.png')});console.log('PASS page history: unknown, unavailable, live events, stale query, tab reset and native requery',out);
}finally{await browser?.close();vite?.kill('SIGTERM');for(const p of paths)rmSync(p,{force:true});}
