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
import{CodeEditor}from ${JSON.stringify(join(gui,'src/components/CodeEditor.tsx'))};
function Probe(){const[el,seat]=useState(null),[detent,set]=useState('half');useContentSheet(el,set,()=>window.closedBySheet=true,()=>({height:800,foot:0}));return <div ref={seat} id="sheet" data-detent={detent}><div id="scroll"><div id="body">content<pre id="code" style={{overflowX:'auto',whiteSpace:'pre'}}>{'wide '.repeat(80)}</pre><div id="wide" style={{overflowX:'auto'}}><table style={{display:'block',overflowX:'auto'}}><tbody><tr><td id="cell" style={{whiteSpace:'nowrap'}}>{'cell '.repeat(80)}</td></tr></tbody></table></div><span id="formula" style={{display:'block',overflowX:'auto',overflowY:'hidden'}}>{'x '.repeat(200)}</span></div></div><div id="editor"><CodeEditor path="probe.ts" text={Array.from({length:60},(_,n)=>'const line'+n+'=1;').join('\\n')} editable/></div><input id="edit"/><div data-native-page-tab="probe"/></div>};createRoot(document.getElementById('root')).render(<Probe/>);
`);
const built=await build({entryPoints:[entry],bundle:true,write:false,format:'iife',platform:'browser',jsx:'automatic',nodePaths:[join(gui,'node_modules')],alias:{'@/api':stub,'@':join(gui,'src')},define:{'process.env.NODE_ENV':'"production"','import.meta.env.DEV':'false'}});
const html=`<!doctype html><style>:root{--place-sheet-half:.5;--place-sheet-full:.92}#sheet{height:var(--content-sheet-half-height,400px);display:flex;flex-direction:column;width:360px}#sheet[data-detent=full]{height:var(--content-sheet-full-height)}#scroll{overflow:auto;flex:1;min-height:0}#body{height:1800px;flex:none}input{flex:none}#editor{height:160px;flex:none;overflow:hidden}#editor>[data-code-view]{height:100%}</style><div id="root"></div><script>${built.outputFiles[0].text}</script>`;
const server=createServer((q,r)=>{r.setHeader('Content-Type','text/html; charset=utf-8');r.end(html)});await new Promise(r=>server.listen(0,'127.0.0.1',r));let browser;
try{for(const[name,engine]of[['chromium',chromium],['webkit',webkit]]){
 browser=await engine.launch(name==='chromium'?{channel:'chrome',headless:true}:{headless:true});const page=await browser.newPage({viewport:{width:390,height:800},hasTouch:true});await page.goto('http://127.0.0.1:'+server.address().port);await page.locator('#sheet').waitFor();await page.waitForTimeout(100);
 const result=await page.evaluate(async()=>{
  const sheet=document.querySelector('#sheet'),scroll=document.querySelector('#scroll'),body=document.querySelector('#body'),h=()=>sheet.getBoundingClientRect().height;
  const touch=(type,y,target=body)=>{const e=new Event(type,{bubbles:true,cancelable:true});Object.defineProperty(e,'touches',{value:type==='touchend'||type==='touchcancel'?[]:[{identifier:1,target,clientX:100,clientY:y}]});return target.dispatchEvent(e);};
  const check=(yes,msg)=>{if(!yes)throw Error(msg)},frame=()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
  const code=sheet.querySelector('.cm-content'),beforeCode=h();touch('touchstart',400,code);touch('touchmove',320,code);check(Math.abs(h()-beforeCode-80)<1,'editable CodeMirror content must share vertical pans');touch('touchcancel',320,code);
  scroll.scrollTop=120;const start=h();touch('touchstart',400);touch('touchmove',300);check(Math.abs(h()-start-100)<1,'half sheet must expand even with an existing directory scroll position');check(scroll.scrollTop===120,'expansion moved the directory');touch('touchcancel',300);check(!sheet.style.height,'cancel left intermediate height');
  scroll.scrollTop=120;touch('touchstart',400);touch('touchmove',40);touch('touchend',40);await frame();check(sheet.dataset.detent==='full','upward drag did not settle full');
  scroll.scrollTop=100;touch('touchstart',200);touch('touchmove',230);check(!sheet.hasAttribute('data-sheet-pulling'),'full sheet took down scroll before list top');scroll.scrollTop=0;touch('touchmove',232);check(h()<735,'same gesture did not hand off at list top');touch('touchcancel',232);
  // A code block, a wide table or a display formula scrolls only sideways;
  // pressed on one, the document under it decides the hand-off. Each is a
  // computed overflow-y:auto box whose scrollTop is always 0.
  scroll.scrollTop=600;const settled=h();
  for(const id of ['code','cell','formula']){const box=document.querySelector('#'+id);touch('touchstart',200,box);touch('touchmove',260,box);check(!sheet.hasAttribute('data-sheet-pulling')&&h()===settled,'drag down from #'+id+' pulled the sheet before the document top');touch('touchcancel',260,box);
   const wheel=new WheelEvent('wheel',{deltaY:-60,bubbles:true,cancelable:true});box.dispatchEvent(wheel);check(!wheel.defaultPrevented&&!sheet.hasAttribute('data-sheet-pulling'),'wheel up over #'+id+' pulled the sheet before the document top');}
  await new Promise(r=>setTimeout(r,250));check(sheet.dataset.detent==='full'&&!sheet.style.height,'a refused wheel left the sheet moved');
  scroll.scrollTop=0;
  const edit=document.querySelector('#edit'),height=h();touch('touchstart',300,edit);touch('touchmove',420,edit);check(h()===height,'input drag resized the sheet');touch('touchcancel',420,edit);
  await frame();check(window.nativeConfigs?.some(x=>x.tab==='probe'&&x.options?.full>0),'native page was never configured');window.nativeHear('start,650,-500');check(h()===650,'native page pan did not resize the sheet');window.nativeHear('cancel,0,0');check(!sheet.style.height,'native cancel left intermediate height');window.nativeHear('end,650,-500');check(!sheet.style.height,'late native end restarted a cancelled gesture');window.dispatchEvent(new Event('blur'));await frame();check(window.nativeConfigs.some(x=>x.options===null),'blur retained native ownership');return{start,full:h(),nativeBound:true};
 });await page.screenshot({path:join(out,name+'.png')});console.log('PASS',name,JSON.stringify(result));await browser.close();browser=null;
}}finally{await browser?.close();await new Promise(r=>server.close(r));}
