// Runs the production hook against real browser DOM. Native adapter messages
// are simulated; this does not validate UIKit gesture-recognizer ordering.
import {build} from 'esbuild';
import {chromium,webkit} from 'playwright-core';
import {createServer} from 'node:http';
import {mkdirSync,writeFileSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {tmpdir} from 'node:os';
const gui=resolve(process.env.PAN_GUI_ROOT||'.'),out=process.env.PAN_OUTPUT||join(tmpdir(),'khor-content-pan-check');mkdirSync(out,{recursive:true});
const stub=join(out,'api.js');writeFileSync(stub,`export function onPageWord(_,tab,run){window.nativeHear=run;return()=>{window.nativeHear=null}};export function pageSheet(tab,options){(window.nativeConfigs??=[]).push({tab,options});return Promise.resolve(null)}`);
const entry=join(out,'entry.tsx');writeFileSync(entry,`
import React,{useState}from'react';import{createRoot}from'react-dom/client';import{useContentSheet}from ${JSON.stringify(join(gui,'src/hooks/use-content-sheet.ts'))};
function Probe(){const[el,seat]=useState(null),[detent,set]=useState('half');useContentSheet(el,set,()=>window.closedBySheet=true,()=>({height:800,foot:0}));return <div ref={seat} id="sheet" data-detent={detent}><div id="scroll"><div id="body">content</div></div><input id="edit"/><div data-native-page-tab="probe"/></div>};createRoot(document.getElementById('root')).render(<Probe/>);
`);
const built=await build({entryPoints:[entry],bundle:true,write:false,format:'iife',platform:'browser',jsx:'automatic',nodePaths:[join(gui,'node_modules')],alias:{'@/api':stub,'@':join(gui,'src')},define:{'process.env.NODE_ENV':'"production"'}});
const html=`<!doctype html><style>:root{--place-sheet-half:.5;--place-sheet-full:.92}#sheet{height:var(--content-sheet-half-height,400px);display:flex;flex-direction:column;width:360px}#sheet[data-detent=full]{height:var(--content-sheet-full-height)}#scroll{overflow:auto;flex:1;min-height:0}#body{height:1800px;flex:none}input{flex:none}</style><div id="root"></div><script>${built.outputFiles[0].text}</script>`;
const server=createServer((q,r)=>{r.setHeader('Content-Type','text/html');r.end(html)});await new Promise(r=>server.listen(0,'127.0.0.1',r));let browser;
try{for(const[name,engine]of[['chromium',chromium],['webkit',webkit]]){
 browser=await engine.launch(name==='chromium'?{channel:'chrome',headless:true}:{headless:true});const page=await browser.newPage({viewport:{width:390,height:800},hasTouch:true});await page.goto('http://127.0.0.1:'+server.address().port);await page.locator('#sheet').waitFor();await page.waitForTimeout(100);
 const result=await page.evaluate(async()=>{
  const sheet=document.querySelector('#sheet'),scroll=document.querySelector('#scroll'),body=document.querySelector('#body'),h=()=>sheet.getBoundingClientRect().height;
  const touch=(type,y,target=body)=>{const e=new Event(type,{bubbles:true,cancelable:true});Object.defineProperty(e,'touches',{value:type==='touchend'||type==='touchcancel'?[]:[{identifier:1,target,clientX:100,clientY:y}]});return target.dispatchEvent(e);};
  const check=(yes,msg)=>{if(!yes)throw Error(msg)},frame=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
  scroll.scrollTop=120;const start=h();touch('touchstart',400);touch('touchmove',300);check(Math.abs(h()-start-100)<1,'half sheet must expand even with an existing directory scroll position');check(scroll.scrollTop===120,'expansion moved the directory');touch('touchcancel',300);check(!sheet.style.height,'cancel left intermediate height');
  scroll.scrollTop=120;touch('touchstart',400);touch('touchmove',40);touch('touchend',40);await frame();check(sheet.dataset.detent==='full','upward drag did not settle full');
  scroll.scrollTop=100;touch('touchstart',200);touch('touchmove',230);check(!sheet.hasAttribute('data-sheet-pulling'),'full sheet took down scroll before list top');scroll.scrollTop=0;touch('touchmove',260);check(h()<735,'same gesture did not hand off at list top');touch('touchcancel',260);
  const edit=document.querySelector('#edit'),height=h();touch('touchstart',300,edit);touch('touchmove',420,edit);check(h()===height,'input drag resized the sheet');touch('touchcancel',420,edit);
  await frame();check(window.nativeConfigs?.some(x=>x.tab==='probe'&&x.options?.full>0),'native page was never configured');window.nativeHear('start,650,-500');check(h()===650,'native page pan did not resize the sheet');window.nativeHear('cancel,0,0');check(!sheet.style.height,'native cancel left intermediate height');window.nativeHear('end,650,-500');check(!sheet.style.height,'late native end restarted a cancelled gesture');window.dispatchEvent(new Event('blur'));await frame();check(window.nativeConfigs.some(x=>x.options===null),'blur retained native ownership');return{start,full:h(),nativeBound:true};
 });await page.screenshot({path:join(out,name+'.png')});console.log('PASS',name,JSON.stringify(result));await browser.close();browser=null;
}}finally{await browser?.close();await new Promise(r=>server.close(r));}
