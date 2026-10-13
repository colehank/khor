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
const commits = ['a', 'b'].map((c, i) => ({id:c.repeat(40),author:'Fixture',when_ms:Date.now()-i*60000,subject:`Revision ${i+1}`,body:''}));
let vite, browser;
const errors = [];
try {
  writeFileSync(entry, `import React from 'react'; import {createRoot} from 'react-dom/client'; import {flushSync} from 'react-dom';
import {FilePage} from '../src/views/FilePage.tsx'; import {FileView} from '../src/views/FileView.tsx';
import {TooltipProvider} from '../src/components/ui/tooltip.tsx'; import {NARROW_QUERY} from '../src/hooks/use-narrow.ts';
import '../src/styles/tokens.css'; import '../src/styles/app.css';
const root=createRoot(document.getElementById('root')), no=()=>{};
const posture=()=>document.documentElement.toggleAttribute('data-narrow',matchMedia(NARROW_QUERY).matches); posture(); addEventListener('resize',posture);
window.showFile=(path)=>flushSync(()=>root.render(<TooltipProvider><FilePage machine='fixture' path={path} here={false} onBack={no} onGone={no} onMoved={no} history /></TooltipProvider>));
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
    await page.route(`http://127.0.0.1:${port}/**`,async route=>{
      const cmd=new URL(route.request().url()).pathname.slice(1), args=route.request().postDataJSON()??{};
      let result=null;
      const text=args.path?.endsWith('.csv')?'a,"b,c",d\n':args.path?.endsWith('.html')?'<h1>HTML file</h1>':'# Current file\n\nText\n';
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
    await context.close(); console.log(`file face · ${width}px: source default, explicit preview, path change, historical version reset, read-only attachment`);
  }
  assert.deepEqual(errors,[],'browser exceptions'); console.log('file face gate: clean');
} finally {await browser?.close(); vite?.kill(); rmSync(entry,{force:true});}
