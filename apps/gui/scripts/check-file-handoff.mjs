// Real FilePage/FileView; only native transport and the OS activity sheet are replaced.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdirSync,mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from 'playwright-core';
const quicklook=process.argv.includes('--quicklook');
const gui=fileURLToPath(new URL('..',import.meta.url)),port=23879;
const entry=join(gui,`scripts/.file-handoff-${process.pid}.tsx`);
const out=process.env.FILE_HANDOFF_OUT||mkdtempSync(join(tmpdir(),'khor-file-handoff-'));mkdirSync(out,{recursive:true});
const pause=ms=>new Promise(r=>setTimeout(r,ms));
let vite,browser,page;const errors=[];
try {
 writeFileSync(entry,`import React from 'react';import {createRoot} from 'react-dom/client';import {flushSync} from 'react-dom';
import {FilePage} from '../src/views/FilePage.tsx';import {FilePreview} from '../src/views/chat-files.tsx';import {TooltipProvider} from '../src/components/ui/tooltip.tsx';import {speakLang} from '../src/lang.ts';import {NARROW_QUERY} from '../src/hooks/use-narrow.ts';import '../src/styles/tokens.css';import '../src/styles/app.css';
speakLang('zh');document.documentElement.toggleAttribute('data-narrow',matchMedia(NARROW_QUERY).matches);
const root=createRoot(document.getElementById('root'));const no=()=>{};
window.showFile=(path='/project/large.pptx',hidden=false)=>flushSync(()=>root.render(<TooltipProvider><section className='relative flex min-h-0 flex-1 flex-col' aria-hidden={hidden||undefined} inert={hidden||undefined}><FilePage machine='remote' path={path} here={false} onBack={no} onGone={no} onMoved={no}/></section></TooltipProvider>));
window.showAttachment=()=>flushSync(()=>root.render(<TooltipProvider><FilePreview machine='remote' path='/project/large.pptx' here={false} onClose={no}/></TooltipProvider>));window.showFile();`);
 vite=spawn(join(gui,'node_modules/.bin/vite'),['--port',String(port),'--strictPort'],{cwd:gui,env:{...process.env,KHOR_FACE_GATE:'1'},stdio:['ignore','pipe','pipe']});
 let ready=false;vite.stdout.on('data',d=>ready||=String(d).includes(`:${port}`));vite.stderr.on('data',d=>process.stderr.write(d));for(let i=0;i<100&&!ready;i++){assert(vite.exitCode===null);await pause(100);}assert(ready);
 browser=await chromium.launch({channel:'chrome',headless:true,args:['--no-proxy-server']});
 const context=await browser.newContext({viewport:{width:390,height:844},hasTouch:true,isMobile:true});
 await context.addInitScript((previewEnabled)=>{
  Object.defineProperty(navigator,'platform',{get:()=> 'iPhone'});Object.defineProperty(navigator,'maxTouchPoints',{get:()=>5});
  window.calls=[];window.complete=false;window.refuse=false;window.startDelay=0;
  const wait=ms=>new Promise(r=>setTimeout(r,ms));
  window.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}},transformCallback:()=>0,unregisterCallback:()=>{},invoke:async(cmd,args={})=>{
   window.calls.push({cmd,args});const total=130*1024*1024;
   if(cmd==='file_handoff_supported')return true;
   if(cmd==='file_preview_supported')return previewEnabled;
   if(cmd==='file_preview_open'){await wait(50);if(window.refusePreview)throw Error('Cannot preview this file');return true;}
   if(cmd==='file_handoff_start'){await wait(window.startDelay);return {moved:0,total,ready:false,cancelled:false,error:null};}
   if(cmd==='file_handoff_state')return {moved:window.complete?total:1048576,total,ready:window.complete,cancelled:false,error:window.refuse?'Download failed':null};
   if(cmd==='file_handoff_open'){await wait(50);return false;}
   if(cmd==='preview')return {kind:'doc',media:'',data:'',html:'',total,cut:true,empty:false};
   if(cmd==='read_text')return {text:'# Hello\n',total:8,at_ms:0,cut:false,binary:false,digest:'0'.repeat(64)};
   if(cmd==='file_stat')return {total,at_ms:0,digest:''};
   if(cmd==='about')return {hosts:false,faces:[]};
   if(cmd==='reveal_here'||cmd==='trash_here')return false;
   if(cmd==='git_diff')return {hunks:[],binary:false};
   return null;
  }};
 },quicklook);
 page=await context.newPage();page.setDefaultTimeout(7000);page.on('pageerror',e=>errors.push(e.message));
 await page.route('**/__file_handoff',r=>r.fulfill({contentType:'text/html',body:`<!doctype html><html data-theme='dark'><head><meta name='viewport' content='width=device-width,initial-scale=1'></head><body><div id='root' class='relative flex h-screen flex-col'></div><script type='module'>import R from '/@react-refresh';R.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>t=>t;window.__vite_plugin_react_preamble_installed__=true;</script><script type='module' src='/scripts/.file-handoff-${process.pid}.tsx'></script></body></html>`}));
 await page.goto(`http://localhost:${port}/__file_handoff`);
 if(quicklook) {
  await page.locator('[data-file-native-preview]').waitFor();
  const before=await page.locator('[data-file-deck]').evaluate(e=>e.getBoundingClientRect().height);
  await page.evaluate(()=>{window.complete=true;window.refusePreview=true});
  await page.locator('[data-file-native-preview]').click();await page.getByRole('button',{name:'重试',exact:true}).waitFor();
  assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.cmd==='file_handoff_open').length),0,'preview failure opened an external app');
  await page.evaluate(()=>window.refusePreview=false);await page.getByRole('button',{name:'重试',exact:true}).click();await page.locator('[data-file-handoff-status]').waitFor({state:'detached'});
  assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.cmd==='file_preview_open').length),2,'retry did not preserve preview intent');
  assert.equal(await page.locator('[data-file-deck]').evaluate(e=>e.getBoundingClientRect().height),before,'system preview changed deck height');
  await page.locator('[data-file-native-preview]').click();await page.locator('[data-file-handoff-status]').waitFor({state:'detached'});
  assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.cmd==='file_preview_open').length),3,'closed preview could not reopen');
  assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.cmd==='preview').length),0,'Office preview still converts to Markdown');
  await page.screenshot({path:join(out,'native-preview.png')});
  await page.evaluate(()=>{window.complete=false;window.startDelay=400});await page.locator('[data-file-native-preview]').click();await page.evaluate(()=>window.showFile('/project/other.pptx'));await pause(700);
  assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.cmd==='file_preview_open').length),3,'late original opened after navigating away');
  await page.evaluate(()=>{window.startDelay=0;window.complete=true});await page.locator('[data-file-open-other]').click();await page.locator('[data-file-handoff-status]').waitFor({state:'detached'});
  assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.cmd==='file_handoff_open').length),1,'external fallback is unavailable');
  await page.evaluate(()=>window.showAttachment());await page.locator('[data-file-native-preview]').waitFor();
  assert.deepEqual(errors,[]);writeFileSync(join(out,'quicklook-results.json'),JSON.stringify({checks:['original Office preview','preview failure and retry intent','dismiss and reopen','deck preservation','no Markdown conversion','late cancellation','external fallback','attachment preview'],errors},null,2));console.log('Quick Look UI: clean',out);
 } else {
 await page.locator('[data-file-open-other]').waitFor();assert.equal(await page.locator('[data-file-fallback-name]').innerText(),'large.pptx');
 assert.match(await page.locator('[data-file-body]').innerText(),/PPTX/);assert.match(await page.locator('[data-file-body]').innerText(),/130/);
 const geometry=()=>page.locator('[data-file-deck]').evaluate(e=>({height:e.getBoundingClientRect().height,folded:e.closest('[data-file-page]').dataset.folded??null}));const before=await geometry();
 // Cancel before start responds: that late response must not open a file.
 await page.evaluate(()=>window.startDelay=400);await page.locator('[data-file-open-other]').click();await page.getByRole('button',{name:'取消',exact:true}).click();await pause(700);
 assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.cmd==='file_handoff_open').length),0);assert(await page.evaluate(()=>window.calls.some(c=>c.cmd==='file_handoff_cancel')));
 // A failed download has an explicit retry, with an independent job id.
 await page.evaluate(()=>{window.startDelay=0;window.refuse=true});await page.locator('[data-file-open-other]').click();await page.getByRole('button',{name:'重试',exact:true}).waitFor();
 const failedId=await page.evaluate(()=>window.calls.filter(c=>c.cmd==='file_handoff_start').at(-1).args.id);
 await page.evaluate(()=>{window.refuse=false;window.complete=true});await page.getByRole('button',{name:'重试',exact:true}).click();await page.locator('[data-file-handoff-status]').waitFor({state:'detached'});
 const opened=await page.evaluate(()=>window.calls.filter(c=>c.cmd==='file_handoff_open').at(-1));assert(opened);assert.notEqual(opened.args.id,failedId);assert.deepEqual(await geometry(),before,'native sheet changed the file deck');
 // Neither a new path nor an inactive retained tab may open the previous file.
 await page.evaluate(()=>{window.complete=false;window.startDelay=400});await page.locator('[data-file-open-other]').click();await page.evaluate(()=>window.showFile('/project/other.pptx'));await pause(700);
 assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.cmd==='file_handoff_open').length),1,'old path opened after navigation');
 await page.evaluate(()=>window.startDelay=0);await page.locator('[data-file-open-other]').click();await page.evaluate(()=>{window.showFile('/project/other.pptx',true);window.complete=true});await pause(500);
 assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.cmd==='file_handoff_open').length),1,'hidden tab opened an OS sheet');
 // Leaving the app during download cancels it without opening on return.
 await page.evaluate(()=>{window.showFile('/project/background.pptx');window.complete=false});await page.locator('[data-file-open-other]').click();
 await page.evaluate(()=>{Object.defineProperty(document,'hidden',{configurable:true,value:true});document.dispatchEvent(new Event('visibilitychange'));window.complete=true;});await pause(400);
 assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.cmd==='file_handoff_open').length),1,'background download opened an OS sheet');
 await page.evaluate(()=>{delete document.hidden;window.complete=false;});
 // Normal files keep the same original-file action inside More.
 await page.evaluate(()=>{window.showFile('/project/notes.md');window.complete=false});await page.locator('.cm-content').waitFor();await page.locator('[data-file-more]').click();await page.locator('[data-file-more-item="open-other"]').waitFor();assert.equal(await page.locator('[data-file-more-item="open-main"]').count(),0);
 await page.locator('[data-file-more-item="open-other"]').click();await page.getByRole('progressbar').waitFor();await page.locator('[data-file-more-menu]').waitFor({state:'detached'});
 assert(await page.evaluate(()=>window.calls.some(c=>c.cmd==='file_handoff_start'&&c.args.path==='/project/notes.md')));
 assert.equal(await page.evaluate(()=>window.calls.filter(c=>c.cmd==='pull_temp').length),0,'phone used desktop temporary copies');
 assert.equal(await page.evaluate(()=>document.documentElement.clientWidth),390,'mobile viewport did not apply');
 assert(await page.locator('[data-file-handoff-status]').evaluate(e=>e.scrollWidth<=innerWidth),'transfer status overflows the phone');
 await page.screenshot({path:join(out,'progress.png')});await page.getByRole('button',{name:'取消',exact:true}).click();
 await page.evaluate(()=>window.showFile('/project/large.pptx'));await page.locator('[data-file-open-other]').waitFor();await page.screenshot({path:join(out,'large-file.png')});
 await page.setViewportSize({width:320,height:740});await page.evaluate(()=>window.showFile('/project/'+('很长的文件名_'.repeat(16))+'.pptx'));await page.locator('[data-file-open-other]').waitFor();
 assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),'long file name overflows narrow screen');await page.screenshot({path:join(out,'narrow-long-name.png')});
 await page.setViewportSize({width:390,height:844});
 await page.evaluate(()=>window.showAttachment());await page.locator('[data-file-preview-body] [data-file-open-other]').waitFor();
 assert.equal(await page.locator('[data-file-preview-share]').count(),0,'attachment still shares limited preview bytes');
 assert.deepEqual(errors,[]);writeFileSync(join(out,'results.json'),JSON.stringify({checks:['original fallback metadata','late start cancellation','retry','OS sheet dismissal preserves deck','path change cancellation','hidden retained tab','background cancellation','narrow long filename','normal file More','attachment original bytes'],errors},null,2));console.log('file handoff: clean',out);
 }
} catch(error){await page?.screenshot({path:join(out,'failed.png')}).catch(()=>{});throw error;}
finally{await browser?.close().catch(()=>{});if(vite?.exitCode===null){vite.kill('SIGTERM');await Promise.race([new Promise(r=>vite.once('exit',r)),pause(2000)]);if(vite.exitCode===null)vite.kill('SIGKILL');}rmSync(entry,{force:true});}
