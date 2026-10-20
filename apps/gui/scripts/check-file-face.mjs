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
// A notebook, and a file with one line past the wrap limit (`lib/code/
// wrap`'s `WRAP_LINE_MAX`, 64 KB) beside an ordinary one.
const notebook = JSON.stringify({nbformat:4,nbformat_minor:5,metadata:{language_info:{name:'python'}},cells:[{cell_type:'markdown',metadata:{},source:['# Notebook heading']},{cell_type:'code',execution_count:1,metadata:{},source:['1 + 1'],outputs:[{output_type:'execute_result',execution_count:1,metadata:{},data:{'text/plain':['2']}}]}]});
const longLine = `{"image/png": "${'A'.repeat(70*1024)}"}\n`;
// **A formula-dense paper, made up** (2026-10-21, the owner's real draft
// stays out of the repo): about the density that made the preview stick —
// two inline formulas a line, a display formula, a wide table and a code
// block every section. Every scroll box a preview can hold is in it.
const formulas = Array.from({length:12},(_,k)=>[
  `## 第 ${k+1} 节`,'',
  ...Array.from({length:12},(_,i)=>`记 $x_{${i}}^{(${k})}$ 为第 ${i} 个槽，$\\mathcal{H}_{t}$ 中关于它的陈述满足 $p(v\\mid Y_t)=\\frac{\\alpha_{${i}}}{1+\\beta}$。`),'',
  '$$','M_{t+1} = E\\big(F(M_t,\\varphi_t)\\big), \\qquad m_t = R(M_t,o_t,Q) + '+Array.from({length:10},(_,i)=>`\\theta_{${i}}`).join(' + '),'$$','',
  '| 方法 | 依据 | 误差 | 备注 |','|---|---|---|---|',
  ...Array.from({length:5},(_,i)=>`| 方法 ${i} | $\\hat\\theta_{${i}}$ | $\\epsilon_{${i}}$ | ${'很长的备注'.repeat(12)} |`),'',
  '```ts',`const wide${k} = [${Array.from({length:30},(_,i)=>i).join(', ')}];`,'```','',
].join('\n')).join('\n');
const commits = ['a', 'b'].map((c, i) => ({id:c.repeat(40),author:'Fixture',when_ms:Date.now()-i*60000,subject:`Revision ${i+1}`,body:''}));
let vite, browser;
const errors = [];
try {
  writeFileSync(entry, `import React from 'react'; import {createRoot} from 'react-dom/client'; import {flushSync} from 'react-dom';
import {FilePage} from '../src/views/FilePage.tsx'; import {FileView} from '../src/views/FileView.tsx';
import {TooltipProvider} from '../src/components/ui/tooltip.tsx'; import {FilePreview} from '../src/views/chat-files.tsx'; import {NARROW_QUERY} from '../src/hooks/use-narrow.ts';
import {followVisualViewport} from '../src/lib/viewport.ts';
import {ContentPosition} from '../src/views/SessionContent.tsx'; import {speakLang} from '../src/lang.ts';
import {Profiler,useState} from 'react';
import '../src/styles/tokens.css'; import '../src/styles/app.css';
followVisualViewport();
const root=createRoot(document.getElementById('root')), no=()=>{};
const posture=()=>document.documentElement.toggleAttribute('data-narrow',matchMedia(NARROW_QUERY).matches); posture(); addEventListener('resize',posture);
window.showFile=(path,embedded=false)=>flushSync(()=>root.render(<TooltipProvider><FilePage embedded={embedded} machine='fixture' path={path} here={false} onBack={no} onGone={no} onMoved={no} history /></TooltipProvider>));
// The session's content host as the shell gives it: every saved place is
// shell state, so the stub re-renders the page the way App does.
window.commits=0; window.saves=[];
function Hosted({path}){const [,bump]=useState(0);const nav={scrollOf:()=>undefined,scroll:(s,k,v)=>{window.saves.push(v);bump(n=>n+1);}};
  return <Profiler id='file' onRender={()=>{window.commits++;}}><ContentPosition session='s' position='p' show mobile={false} navigation={nav} className='flex min-h-0 flex-1 flex-col'><FilePage machine='fixture' path={path} here={false} onBack={no} onGone={no} onMoved={no} /></ContentPosition></Profiler>;}
window.showHosted=(path)=>flushSync(()=>root.render(<TooltipProvider><Hosted path={path} /></TooltipProvider>));
window.langWrites=()=>{let n=0;const o=new MutationObserver(l=>{n+=l.length;});o.observe(document.documentElement,{attributes:true,attributeFilter:['lang']});speakLang('zh');speakLang('zh');speakLang('zh');return new Promise(r=>setTimeout(()=>{o.disconnect();r(n);},50));};
speakLang('zh');
window.showBare=(path)=>flushSync(()=>root.render(<TooltipProvider><FileView key={path} machine='fixture' path={path} git={false} reading /></TooltipProvider>));
window.showLook=(path)=>flushSync(()=>root.render(<TooltipProvider><FilePreview machine='fixture' shown='' here={false} path={path} onClose={no} /></TooltipProvider>));
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
      const text=args.path==='/formulas.md'?formulas:args.path?.endsWith('.ipynb')?notebook:args.path?.endsWith('wide.json')?longLine:args.path?.endsWith('.txt')?long:args.path?.endsWith('.csv')?'a,"b,c",d\n':args.path?.endsWith('.html')?'<h1>HTML file</h1>':'# Current file\n\nText\n';
      if(cmd==='read_text') result={text,total:text.length,at_ms:Date.now(),cut:false,binary:false,digest:'0'.repeat(64)};
      else if(cmd==='git_log') result={commits,more:false};
      else if(cmd==='git_show') result={text:args.path?.endsWith('.ipynb')?notebook:`# ${args.rev===commits[0].id?'First':'Second'} revision\n`,cut:false,diff:{hunks:[],more:false}};
      else if(cmd==='file_stat') result={total:text.length,at_ms:Date.now(),digest:'0'.repeat(64)};
      else if(cmd==='reveal_here') result=false;
      else if(cmd==='about') result={hosts:true,faces:[]};
      await route.fulfill({contentType:'application/json',body:JSON.stringify(result)});
    });
    await page.route('**/__file_face_gate?**',route=>route.fulfill({contentType:'text/html',body:`<!doctype html><html data-theme="dark"><body><div id="root" class="flex h-screen flex-col"></div><script type="module">import RefreshRuntime from '/@react-refresh';RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/.file-face-gate-${process.pid}.tsx"></script></body></html>`}));
    await page.goto(`http://localhost:${port}/__file_face_gate?bridge=${port}`);
    // The last press's menu may still be closing, its items still in the
    // page: counted or pressed then, they detach under the press (red on
    // 686fb853, where 预览 moved into 「…」). Every opening waits for it.
    const settled=()=>page.locator('[data-file-more-menu]').waitFor({state:'detached'});
    const more=async()=>{await settled(); await page.locator('[data-file-more]').click(); await page.locator('[data-file-more-menu]').waitFor();};
    const toggle=async()=>{
      await settled();
      if(!await page.locator('[data-file-face-toggle]').count())await more();
      await page.locator('[data-file-face-toggle]').click();
    };
    const source=async()=>{
      await page.locator('[data-file-page][data-file-face="source"]').waitFor();
      await page.locator('.cm-editor').waitFor();
      assert.equal(await page.locator('[data-file-preview]').count(),0,'new file/revision opened rendered');
    };
    await source();
    await toggle();
    await page.locator('[data-file-preview="md"] h1').waitFor();
    assert.equal(await page.locator('[data-file-preview="md"] h1').innerText(),'Current file');
    // Same FilePage instance: changing the file must clear preview.
    for(const path of ['/data.csv','/page.html','/notes.md']) {
      await page.evaluate(path=>window.showFile(path),path); await source();
      await toggle();
      await page.locator('[data-file-preview]').waitFor();
    }
    const revision=async index=>{
      await more();
      await page.locator('[data-file-more-item="history"]').click();
      await page.locator(`[data-file-history-row="${commits[index].id}"]`).click();
      await page.locator(`[data-file-rev="${commits[index].id}"]`).waitFor();
      await source();
    };
    await revision(0);
    assert.equal(await page.locator('.cm-content').getAttribute('contenteditable'),'false','history became editable');
    await toggle();
    assert.equal(await page.locator('[data-file-preview="md"] h1').innerText(),'First revision');
    await page.locator('[data-file-back]').click(); await source();
    await revision(1); // A second version must not inherit the first one's preview.
    await toggle();
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
      // A touch's synthesized mouse selection commits after touchend.
      // Wait for that transaction before measuring the caret, not a timer.
      await page.waitForFunction(()=>{const v=document.querySelector('[data-code-view]')?.__view;return v&&v.state.doc.lineAt(v.state.selection.main.head).number>20;},null,{timeout:2500});
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
      // **A formula-dense preview scrolls like its source** (2026-10-21,
      // the owner: 预览面滚轮卡、拖滚动条不卡). Three things, each red on
      // the code before: (a) a wheel over a code block, a wide table or a
      // display formula moves the document — those boxes only scroll
      // sideways, and contained on both axes they swallowed it (Chromium
      // stopped at 642 of 6418 px); (b) a pass through the file commits
      // the hosted page a handful of times, not once per scroll event (the
      // content host saved the place per event: 62 renders a pass); (c)
      // the shell's per-render language call writes nothing when nothing
      // changed (Chromium restyles the document for a same-value root
      // attribute, ~20 ms a render here).
      assert.equal(await page.evaluate(()=>window.langWrites()),0,'speakLang rewrote <html lang> to the value it already had');
      await page.evaluate(()=>window.showHosted('/formulas.md')); await source();
      await toggle();
      const prev=page.locator('[data-file-preview="md"]');
      await prev.locator('.katex-display').first().waitFor();
      await pause(300);
      for(const [sel,label] of [['[data-md] td','table cell'],['[data-md] pre','code block'],['[data-md] .katex-display','display formula']]){
        const at=await prev.evaluate((s,sel)=>{const e=s.querySelector(sel);s.scrollTop+=e.getBoundingClientRect().top-s.getBoundingClientRect().top-200;const r=e.getBoundingClientRect();return {x:r.left+Math.min(40,r.width/2),y:r.top+r.height/2,top:s.scrollTop};},sel);
        await page.mouse.move(at.x,at.y);
        for(let i=0;i<3;i++){await page.mouse.wheel(0,200);await pause(120);}
        const moved=await prev.evaluate(s=>s.scrollTop)-at.top;
        assert(moved>=300,`a wheel over a ${label} moved the preview ${moved}px of 600 — the box swallowed it`);
      }
      await prev.evaluate(s=>{s.scrollTop=0;}); await pause(400);
      const box=await prev.boundingBox(); await page.mouse.move(box.x+box.width/2,box.y+box.height/2);
      const before=await page.evaluate(()=>{const c=window.commits;window.saves.length=0;return c;});
      for(let i=0;i<40;i++){await page.mouse.wheel(0,120);await pause(16);}
      await pause(500);
      const pass=await page.evaluate(b=>({commits:window.commits-b,saves:window.saves.length,last:window.saves.at(-1)?.top,top:document.querySelector('[data-file-preview]').scrollTop}),before);
      assert(pass.top>3000,`the wheel pass did not travel: ${JSON.stringify(pass)}`);
      assert(pass.commits<=6,`a 40-step wheel pass committed the hosted file page ${pass.commits} times (the place saved per scroll event?): ${JSON.stringify(pass)}`);
      assert.equal(pass.last,pass.top,`the place saved after the pass is not where the reader stopped: ${JSON.stringify(pass)}`);
      console.log(`file face · formula preview: wheel crosses code, table and formula boxes; 40-step pass ${JSON.stringify(pass)}`);
      await page.evaluate(()=>window.showFile('/notes.md',true)); await source();
      assert.equal(await page.locator('[data-file-deck]').count(),0,'embedded file drew a second deck');
      await revision(0);
      await page.locator('[data-file-rev-compare]').click();
      assert.equal(await page.locator('[data-file-rev-compare]').getAttribute('aria-pressed'),'true');
      await page.locator('[data-file-back]').click();await source();
      assert.equal(await page.locator('[data-file-rev]').count(),0,'embedded history could not return to current file');
      console.log('file face · embedded: source, history comparison, back to current file, no second deck');
    }
    // **A notebook opens as its preview; everything else as source**
    // (the owner, 2026-10-20). Per file, never carried from the file
    // before: md → ipynb → md on one page, and source chosen on a
    // notebook does not outlive it.
    {
      const face=()=>page.locator('[data-file-page]').getAttribute('data-file-face');
      const nb=async()=>{await page.locator('[data-file-page][data-file-face="preview"] [data-file-preview="ipynb"] [data-nb-cell]').first().waitFor({timeout:8000});};
      await page.evaluate(()=>window.showFile('/analysis.ipynb')); await nb();
      await toggle(); await source();
      await page.evaluate(()=>window.showFile('/notes.md')); await source();
      await toggle(); await page.locator('[data-file-preview="md"] h1').waitFor();
      await page.evaluate(()=>window.showFile('/again.ipynb')); await nb();
      await page.evaluate(()=>window.showFile('/notes.md')); await source();
      assert.equal(await face(),'source','md after a notebook did not open as source');
      // A notebook's revision: the same rule.
      await page.evaluate(()=>window.showFile('/analysis.ipynb')); await nb();
      await more();
      await page.locator('[data-file-more-item="history"]').click();
      await page.locator(`[data-file-history-row="${commits[0].id}"]`).click();
      await page.locator(`[data-file-rev="${commits[0].id}"] [data-file-preview="ipynb"] [data-nb-cell]`).first().waitFor({timeout:8000});
      // The attachment viewer and the bare view.
      await page.evaluate(()=>window.showLook('/attached.ipynb'));
      await page.locator('[data-file-preview-body] [data-file-preview="ipynb"] [data-nb-cell]').first().waitFor({timeout:8000});
      await page.evaluate(()=>window.showLook('/attached.md'));
      await page.locator('[data-file-preview-body] [data-file-text]').waitFor();
      assert.equal(await page.locator('[data-file-preview-body] [data-file-preview]').count(),0,'attachment md opened rendered after a notebook');
      await page.evaluate(()=>window.showBare('/bare.ipynb'));
      await page.locator('[data-file-preview="ipynb"] [data-nb-cell]').first().waitFor({timeout:8000});
      console.log(`file face · ${width}px: notebooks open as preview (page, history, attachment, bare), md still source, not carried across files`);
    }
    // **A line past 64 KB turns wrapping off; an ordinary file keeps it**
    // (the owner, 2026-10-20: a wrapped 1 MB line took 5.7 s to open).
    {
      const wraps=()=>page.evaluate(()=>document.querySelector('.cm-content')?.classList.contains('cm-lineWrapping'));
      await page.evaluate(()=>window.showFile('/wide.json')); await source();
      assert.equal(await wraps(),false,'a file with a 70 KB line still wraps');
      await page.evaluate(()=>window.showFile('/notes.md')); await source();
      assert.equal(await wraps(),true,'an ordinary file stopped wrapping');
      console.log(`file face · ${width}px: a 70 KB line opens unwrapped, an ordinary file wrapped`);
    }
    await context.close(); console.log(`file face · ${width}px: source default, explicit preview, path change, historical version reset, read-only attachment`);
  }
  assert.deepEqual(errors,[],'browser exceptions'); console.log('file face gate: clean');
} finally {await browser?.close(); vite?.kill(); rmSync(entry,{force:true});}
