// Real file page and historical file viewer, with transport-only fixtures.
// Run: node scripts/check-file-face.mjs (system Chrome).
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { writeFileSync, rmSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';
const gui = fileURLToPath(new URL('..', import.meta.url));
const port = Number(process.env.FILE_FACE_PORT ?? 1469);
const entry = `${gui}/scripts/.file-face-gate-${process.pid}.tsx`;
const pause = ms => new Promise(r => setTimeout(r, ms));
// Eighty lines: enough that a tap low on a phone's screen is a line the
// keyboard will cover.
const long = Array.from({length:80},(_,i)=>`line ${i+1}: some text on it`).join('\n')+'\n';
const commits = ['a', 'b'].map((c, i) => ({id:c.repeat(40),author:'Fixture',when_ms:Date.now()-i*60000,subject:`Revision ${i+1}`,body:''}));
let vite, browser;
const errors = [];
try {
  writeFileSync(entry, `import React from 'react'; import {createRoot} from 'react-dom/client'; import {flushSync} from 'react-dom';
import {FilePage} from '../src/views/FilePage.tsx'; import {FileView} from '../src/views/FileView.tsx';
import {TooltipProvider} from '../src/components/ui/tooltip.tsx'; import {NARROW_QUERY} from '../src/hooks/use-narrow.ts';
import {followVisualViewport} from '../src/lib/viewport.ts';
import '../src/styles/tokens.css'; import '../src/styles/app.css';
followVisualViewport();
const root=createRoot(document.getElementById('root')), no=()=>{};
const posture=()=>document.documentElement.toggleAttribute('data-narrow',matchMedia(NARROW_QUERY).matches); posture(); addEventListener('resize',posture);
window.showFile=(path,embedded=false)=>flushSync(()=>root.render(<TooltipProvider><FilePage embedded={embedded} machine='fixture' path={path} here={false} onBack={no} onGone={no} onMoved={no} history /></TooltipProvider>));
window.showBare=(path)=>flushSync(()=>root.render(<TooltipProvider><FileView key={path} machine='fixture' path={path} git={false} reading /></TooltipProvider>));
window.showFile('/notes.md');`);
  vite = spawn('node_modules/.bin/vite', ['--port', String(port), '--strictPort'], {cwd:gui,stdio:['ignore','pipe','pipe']});
  let ready=false; vite.stdout.on('data',d=>{ready ||= String(d).includes(`:${port}`);});
  for(let i=0;i<100&&!ready;i++) await pause(100);
  assert(ready, 'Vite did not start');
  browser = await chromium.launch({channel:'chrome',headless:true,args:['--no-proxy-server','--disable-features=LocalNetworkAccessChecks']});
  for(const width of [1200,390]) {
    const context=await browser.newContext({viewport:{width,height:844},hasTouch:width<700});
    const page=await context.newPage(); page.on('pageerror',e=>errors.push(e.message));
    // A phone's keyboard is the visual viewport going short while the
    // layout viewport does not (lib/viewport); Chrome has none, so a
    // stand-in, installed before the app's scripts (check-face bar k).
    if(width<700) await page.addInitScript(()=>{
      const fake=new EventTarget();
      Object.assign(fake,{height:window.innerHeight,width:window.innerWidth,scale:1,offsetTop:0,offsetLeft:0});
      Object.defineProperty(window,'visualViewport',{configurable:true,value:fake});
      window.__keyboard=covered=>{fake.height=window.innerHeight-covered; fake.dispatchEvent(new Event('resize'));};
    });
    await page.route(`http://127.0.0.1:${port}/**`,async route=>{
      const cmd=new URL(route.request().url()).pathname.slice(1), args=route.request().postDataJSON()??{};
      let result=null;
      const text=args.path?.endsWith('.txt')?long:args.path?.endsWith('.csv')?'a,"b,c",d\n':args.path?.endsWith('.html')?'<h1>HTML file</h1>':'# Current file\n\nText\n';
      if(cmd==='read_text') result={text,total:text.length,at_ms:Date.now(),cut:false,binary:false,digest:'0'.repeat(64)};
      else if(cmd==='git_log') result={commits,more:false};
      else if(cmd==='git_show') result={text:`# ${args.rev===commits[0].id?'First':'Second'} revision\n`,cut:false,diff:{hunks:[],more:false}};
      else if(cmd==='file_stat') result={total:text.length,at_ms:Date.now(),digest:'0'.repeat(64)};
      else if(cmd==='reveal_here') result=false;
      else if(cmd==='about') result={hosts:true,faces:[]};
      await route.fulfill({contentType:'application/json',body:JSON.stringify(result)});
    });
    await page.route('**/__file_face_gate?**',route=>route.fulfill({contentType:'text/html',body:`<!doctype html><html data-theme="dark"><body><div id="root" class="flex h-screen flex-col"></div><script type="module">import RefreshRuntime from '/@react-refresh';RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/.file-face-gate-${process.pid}.tsx"></script></body></html>`}));
    await page.goto(`http://localhost:${port}/__file_face_gate?bridge=${port}`);
    const source=async()=>{
      await page.locator('[data-file-face-toggle="source"]').waitFor();
      await page.locator('.cm-editor').waitFor();
      assert.equal(await page.locator('[data-file-preview]').count(),0,'new file/revision opened rendered');
    };
    await source();
    await page.locator('[data-file-face-toggle]').click();
    await page.locator('[data-file-preview="md"] h1').waitFor();
    assert.equal(await page.locator('[data-file-preview="md"] h1').innerText(),'Current file');
    // Same FilePage instance: changing the file must clear preview.
    for(const path of ['/data.csv','/page.html','/notes.md']) {
      await page.evaluate(path=>window.showFile(path),path); await source();
      await page.locator('[data-file-face-toggle]').click();
      await page.locator('[data-file-preview]').waitFor();
    }
    const revision=async index=>{
      await page.locator('[data-file-more]').click();
      await page.locator('[data-file-more-item="history"]').click();
      await page.locator(`[data-file-history-row="${commits[index].id}"]`).click();
      await page.locator(`[data-file-rev="${commits[index].id}"]`).waitFor();
      await source();
    };
    await revision(0);
    assert.equal(await page.locator('.cm-content').getAttribute('contenteditable'),'false','history became editable');
    await page.locator('[data-file-face-toggle]').click();
    assert.equal(await page.locator('[data-file-preview="md"] h1').innerText(),'First revision');
    await page.locator('[data-file-back]').click(); await source();
    await revision(1); // A second version must not inherit the first one's preview.
    await page.locator('[data-file-face-toggle]').click();
    assert.equal(await page.locator('[data-file-preview="md"] h1').innerText(),'Second revision');
    await page.evaluate(()=>window.showBare('/attachment.md'));
    await page.locator('[data-file-text]').waitFor();
    assert.equal(await page.locator('[data-file-preview]').count(),0,'bare attachment viewer defaulted to preview');
    if(width<700) {
      // **A tap low in the file, then the keyboard** (the owner,
      // 2026-10-14: 点屏幕靠下的行，键盘弹出后不会把这一行顶到键盘上方).
      // The line must end above what covers the foot — the keys, and the
      // deck that stands on them — while the keyboard rises, once it has
      // settled, and as the person types.
      await page.evaluate(()=>window.showFile('/long.txt'));
      await page.locator('.cm-content[contenteditable="true"]').waitFor(); await pause(300);
      const seen=label=>page.evaluate(label=>{
        const v=document.querySelector('[data-code-view]').__view, c=v.coordsAtPos(v.state.selection.main.head);
        const s=v.scrollDOM.getBoundingClientRect(), deck=document.querySelector('[data-file-deck]').getBoundingClientRect();
        const floor=Math.min(window.visualViewport.height,s.bottom,deck.top);
        return {label,line:v.state.doc.lineAt(v.state.selection.main.head).number,top:Math.round(c.top),bottom:Math.round(c.bottom),from:Math.round(s.top),floor:Math.round(floor),shown:c.top>=s.top&&c.bottom<=floor};
      },label);
      await page.touchscreen.tap(120,700);
      const tapped=await seen('tapped');
      assert(tapped.shown&&tapped.line>20,`keyboard · the tap did not put a low line under the caret: ${JSON.stringify(tapped)}`);
      for(const k of [80,180,280,336]) {await page.evaluate(k=>window.__keyboard(k),k); await pause(75);}
      await pause(1200);
      const risen=await seen('keyboard up');
      assert(risen.shown,`keyboard · the tapped line is under the keyboard or the deck once it rose: ${JSON.stringify(risen)}`);
      await page.keyboard.insertText('x'); await pause(200);
      const typed=await seen('typed');
      assert(typed.shown,`keyboard · the line typed on is under the keyboard or the deck: ${JSON.stringify(typed)}`);
      await page.evaluate(()=>window.__keyboard(0)); await pause(300);
      console.log(`file face · ${width}px: keyboard over a low line — ${JSON.stringify([tapped,risen,typed])}`);
    }
    if(width===1200) {
      await page.evaluate(()=>window.showFile('/notes.md',true)); await source();
      assert.equal(await page.locator('[data-file-deck]').count(),0,'embedded file drew a second deck');
      await revision(0);
      await page.locator('[data-file-rev-compare]').click();
      assert.equal(await page.locator('[data-file-rev-compare]').getAttribute('aria-pressed'),'true');
      await page.locator('[data-file-back]').click();await source();
      assert.equal(await page.locator('[data-file-rev]').count(),0,'embedded history could not return to current file');
      console.log('file face · embedded: source, history comparison, back to current file, no second deck');
    }
    await context.close(); console.log(`file face · ${width}px: source default, explicit preview, path change, historical version reset, read-only attachment`);
  }
  assert.deepEqual(errors,[],'browser exceptions'); console.log('file face gate: clean');
} finally {await browser?.close(); vite?.kill(); rmSync(entry,{force:true});}
