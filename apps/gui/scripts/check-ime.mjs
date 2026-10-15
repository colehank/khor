// Actual TerminalPane input chain; only the PTY transport is replaced.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { writeFileSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
const gui=fileURLToPath(new URL("..",import.meta.url));
const port=1498,entry=`${gui}/scripts/.ime-${process.pid}.tsx`;
const pause=ms=>new Promise(r=>setTimeout(r,ms));
let vite,browser;const errors=[],sent=[];
try {
 writeFileSync(entry,`import React,{useState} from 'react';import {createRoot} from 'react-dom/client';import {TooltipProvider} from '../src/components/ui/tooltip';import {TerminalPane} from '../src/views/TerminalPane';import '../src/styles/tokens.css';import '../src/styles/app.css';function Lab(){const [n,set]=useState(0);window.nextTerm=()=>set(n=>n+1);return <TooltipProvider><div style={{height:'100vh',display:'flex',flexDirection:'column'}}><TerminalPane key={n} id={'shell/ime-'+n} active/></div></TooltipProvider>}createRoot(document.getElementById('root')).render(<Lab/>);`);
 vite=spawn('node_modules/.bin/vite',['--port',String(port),'--strictPort'],{cwd:gui,stdio:['ignore','pipe','pipe']});let ready=false,log='';vite.stdout.on('data',b=>{ready ||= String(b).includes(`:${port}`)});vite.stderr.on('data',b=>log+=b);
 for(let i=0;i<100&&!ready;i++)await pause(100);assert(ready,log);
 browser=await chromium.launch({channel:'chrome',headless:true,args:['--no-proxy-server','--disable-features=LocalNetworkAccessChecks']});
 const context=await browser.newContext({viewport:{width:430,height:800},isMobile:true,hasTouch:true});
 await context.addInitScript(()=>{window.__TAURI_INTERNALS__={invoke:async()=>0,transformCallback:()=>0,unregisterCallback:()=>{}};window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};});
 await context.route(`http://127.0.0.1:${port}/**`,async route=>{
  const cmd=new URL(route.request().url()).pathname.slice(1);let result=null;
  if(cmd==='term_attach')result={bytes:'',seq:1};
  if(cmd==='term_key')sent.push(route.request().postDataJSON().keys);
  if(cmd==='term_wait'||cmd==='term_stream'){await pause(100);result={bytes:'',seq:1,gone:false,reset:false,stall:null};}
  await route.fulfill({contentType:'application/json',body:JSON.stringify(result)});
 });
 await context.route('**/__ime?**',route=>route.fulfill({contentType:'text/html',body:`<!doctype html><meta charset=utf-8><div id=root></div><script type=module>import R from '/@react-refresh';R.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>t=>t;window.__vite_plugin_react_preamble_installed__=true;</script><script type=module src='/scripts/.ime-${process.pid}.tsx'></script>`}));
 const page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));await page.goto(`http://localhost:${port}/__ime?bridge=${port}`);
 async function run(name,kind,flags=0) {
  await page.waitForFunction(()=>!!document.querySelector('[data-terminal]')?.__xterm&&!document.querySelector('[data-term-opening]'));
  const from=sent.length;
  const result=await page.evaluate(async ({kind,flags})=>{
   const t=document.querySelector('[data-terminal]').__xterm,field=t.textarea;
   const wait=()=>new Promise(r=>setTimeout(r,8));
   await new Promise(r=>t.write(flags?'\x1b[>'+flags+'u':'',r));if(kind==='option-meta')t.options.macOptionIsMeta=true;field.focus();field.value='';
   const data=[],heard=t.onData(s=>data.push(s));
   const key=(type,k,code,composing=false)=>{const e=new KeyboardEvent(type,{key:k,code:k==='Backspace'?'Backspace':k==='Enter'?'Enter':k===' '?'Space':'Key'+k.toUpperCase(),isComposing:composing,bubbles:true,cancelable:true});Object.defineProperty(e,'keyCode',{value:code});Object.defineProperty(e,'which',{value:code});field.dispatchEvent(e);return e.defaultPrevented;};
   const comp=(type,data)=>field.dispatchEvent(new CompositionEvent(type,{data,bubbles:true}));
   const input=(type,data,value,composing)=>{const b=new InputEvent('beforeinput',{inputType:type,data,isComposing:composing,bubbles:true,cancelable:true,composed:true});field.dispatchEvent(b);if(b.defaultPrevented)return;field.value=value;field.setSelectionRange(value.length,value.length);field.dispatchEvent(new InputEvent('input',{inputType:type,data,isComposing:composing,bubbles:true,composed:true}));};
   key('keydown','n',229);comp('compositionstart','');comp('compositionupdate','ni');input('insertCompositionText','ni','ni',true);await wait();key('keyup','n',78,true);
   if(kind==='backspace-flagged'||kind==='backspace-unflagged') {
    key('keydown','Backspace',8,kind==='backspace-flagged');input('deleteContentBackward',null,'n',kind==='backspace-flagged');comp('compositionupdate','n');key('keyup','Backspace',8,kind==='backspace-flagged');
   }
   if(kind==='plain-code-letter'){key('keydown','h',72,true);comp('compositionupdate','nih');input('insertCompositionText','nih','nih',true);key('keyup','h',72,true);}
   if(kind==='raw-input')input('insertText','ni','ni',false);
   if(kind==='candidate-space')key('keydown',' ',32,false);
   await wait();const before=data.join(''),draft=field.value;
   if(kind==='cancel') {
    field.setAttribute('inputmode','none');await wait();field.setAttribute('inputmode','text');
    key('keydown','a',229);await wait();input('insertText','a','a',false);key('keyup','a',65);await wait();heard.dispose();
    return {before,draft,after:data.join(''),helper:t._core._compositionHelper._isComposing};
   }
   const commit=kind==='latin-candidate'?'hello':'你';
   comp('compositionend',commit);input('insertText',commit,commit,false);
   await wait();if(kind==='candidate-space')key('keyup',' ',32,false);
   await wait();
   if(kind==='late-punctuation'){key('keydown',' ',229);await wait();input('insertText','，',field.value+'，',false);key('keyup',' ',32);await wait();}
   if(kind==='twice'){
    key('keydown','h',229);comp('compositionstart','');comp('compositionupdate','hao');input('insertCompositionText','hao',field.value+'hao',true);await wait();key('keyup','h',72,true);
    comp('compositionend','好');input('insertText','好','你好',false);await wait();
   }
   heard.dispose();return {before,draft,after:data.join(''),helper:t._core._compositionHelper._isComposing};
  },{kind,flags});
  assert.equal(result.before,'',`${name}: preedit escaped: ${JSON.stringify(result.before)}`);
  const want=({cancel:'a',twice:'你好','late-punctuation':'你，','latin-candidate':'hello'})[kind]??'你';
  assert.equal(result.after,want,`${name}: commit must arrive once`);
  assert.equal(result.helper,false,`${name}: composition stuck`);
  const draft=kind.startsWith('backspace-')?'n':kind==='plain-code-letter'?'nih':'ni';
  assert.equal(result.draft,draft,`${name}: native preedit edit must remain intact`);
  await pause(80);
  assert.equal(sent.slice(from).join(''),want,`${name}: PTY transport must match the committed text`);
  if(kind==='normal') {
    await page.evaluate(()=>{const t=document.querySelector('[data-terminal]').__xterm;window.trailing=[];t.onData(s=>window.trailing.push(s));t.focus();});
    await page.keyboard.type('x');await page.keyboard.press('Control+c');
    assert.equal(await page.evaluate(()=>window.trailing.join('')),'x\x03','typing and Ctrl+C after a commit must remain usable');
  }
  console.log('ime:',name,'passed');await page.evaluate(()=>window.nextTerm());await pause(80);
 }
 for(const [name,kind,flags] of [
  ['delete during composition','backspace-flagged',0],['delete with missing marker','backspace-unflagged',0],['letter with ordinary keyCode','plain-code-letter',0],['insertText while preedit active','raw-input',0],['candidate Space with Kitty protocol','candidate-space',31],['ordinary Chinese commit','normal',0],['Option-as-Meta enabled','option-meta',0],['English IME candidate','latin-candidate',0],['composition then delayed punctuation','late-punctuation',0],['keyboard lowered mid-composition','cancel',0],['two consecutive compositions','twice',0],
 ])await run(name,kind,flags);
 await page.waitForFunction(()=>!!document.querySelector('[data-terminal]')?.__xterm&&!document.querySelector('[data-term-opening]'));
 await page.evaluate(()=>{const t=document.querySelector('[data-terminal]').__xterm;window.plain=[];t.onData(d=>window.plain.push(d));t.focus();});
 await page.keyboard.type('abc');await pause(100);
 assert.equal(await page.evaluate(()=>window.plain.join('')),'abc','ordinary English keyboard remains unchanged');
 assert.deepEqual(errors,[]);console.log('ime: only committed text reaches the terminal');
}finally{if(browser)await browser.close();if(vite)vite.kill();rmSync(entry,{force:true});}
