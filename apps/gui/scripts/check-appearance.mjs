// Real settings components + TerminalPane/xterm. Only transport is a fixture.
// Checks paint updates preserve the attachment, buffer and scroll position;
// native desktop material itself is checked separately in the real app.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { readFileSync, writeFileSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
const gui = fileURLToPath(new URL("..", import.meta.url));
const port = Number(process.env.APPEARANCE_PORT ?? 1493);
const pause = ms => new Promise(r => setTimeout(r, ms));
const entry = `${gui}/scripts/.appearance-gate-${process.pid}.tsx`;
let bootStyle = readFileSync(`${gui}/index.html`, "utf8").match(/<style>([\s\S]*?)<\/style>/)?.[1];
assert(bootStyle, "first-frame styles missing");
const calls = [], errors = [];
let vite, browser, blur = true;
let rows = [
  { name: "window-material", value: "solid", choices: ["solid", "clear", "frost"].map(value => ({ value, label: value })), page: "appearance", section: "window" },
  { name: "window-outline", value: "on", choices: [], page: "appearance", section: "window" },
  { name: "window-opacity", value: "100", choices: [], page: "appearance", section: "window" },
];
try {
  writeFileSync(entry, `import React,{useState,useEffect} from 'react';
import {createRoot} from 'react-dom/client';
import {TooltipProvider} from '../src/components/ui/tooltip';
import {SettingsProvider,SettingsRereadProvider} from '../src/settings';
import {TerminalThemeSection} from '../src/views/more/TerminalThemeSection';
import {WindowAppearanceSection} from '../src/views/more/WindowAppearanceSection';
import {TerminalPane,EndedPane} from '../src/views/TerminalPane';
import {FaceMark} from '../src/components/FaceMark';
import * as palette from '../src/lib/term-theme';
import {followWindowSurface,wearWindowChoice,windowChoice} from '../src/lib/window-surface';
import {fetchConfig} from '../src/api';
import {speakLang} from '../src/lang';
import '../src/styles/tokens.css';import '../src/styles/app.css';
speakLang('zh');palette.wearTermPalette();followWindowSurface();window.palette=palette;window.windowChoice=windowChoice;window.wearWindowChoice=wearWindowChoice;
function Lab(){const [kept,setKept]=useState(false),[front,setFront]=useState(false),[concealed,setConcealed]=useState(false);window.hideTerminals=setConcealed;window.keepTerminal=()=>setKept(true);window.closeKept=()=>setKept(false);window.frontKept=setFront;const [ended,setEnded]=useState(false);window.showEnded=()=>setEnded(true);const [rows,setRows]=useState([]);const read=()=>fetchConfig().then(setRows);useEffect(()=>{read()},[]);const surface=windowChoice(rows);useEffect(()=>{if(surface)wearWindowChoice(surface)},[surface?.material,surface?.opacity,surface?.outline]);
return <SettingsProvider value={rows}><SettingsRereadProvider value={read}><TooltipProvider><div style={{display:'flex',height:'100vh',color:'var(--foreground)'}}><div style={{flex:1,minWidth:0,display:'flex',flexDirection:'column',position:'relative'}}><div data-retained-front className={front||concealed?'pointer-events-none invisible absolute inset-0 flex flex-col':'contents'}>{ended?<EndedPane id='shell/appearance-test' at={1}/>:<TerminalPane id='shell/appearance-test' active={!front} />}</div>{kept&&<div data-retained-back className={front&&!concealed?'contents':'pointer-events-none invisible absolute inset-0 flex flex-col'} aria-hidden={!front||undefined}><TerminalPane id='shell/appearance-kept' active={front}/></div>}</div><aside style={{width:400,overflow:'auto',padding:'var(--gap-base)'}}><div data-halo-lab style={{display:'flex',gap:40,padding:32}}><FaceMark face={{machine:null}} halo="done"/><div data-tab-bar><FaceMark face={{machine:null}} halo="done" size="tab"/></div></div><WindowAppearanceSection/><TerminalThemeSection/></aside></div></TooltipProvider></SettingsRereadProvider></SettingsProvider>}
createRoot(document.getElementById('root')).render(<Lab/>);
`);
  vite = spawn("node_modules/.bin/vite", ["--port", String(port), "--strictPort"], { cwd: gui, stdio: ["ignore", "pipe", "pipe"] });
  let ready = false, viteLog = ""; vite.stdout.on("data", b => { ready ||= String(b).includes(`:${port}`); }); vite.stderr.on("data", b => { viteLog += String(b); });
  for (let n = 0; n < 100 && !ready; n++) await pause(100);
  assert(ready, "vite did not start");
  browser = await chromium.launch({ channel: "chrome", headless: process.env.APPEARANCE_HEADLESS === "1", args: ["--no-proxy-server", "--disable-features=LocalNetworkAccessChecks"] });
  const context = await browser.newContext({ viewport: { width: 1400, height: 900 }, reducedMotion: "reduce" });
  await context.addInitScript(() => {
    window.__TAURI_INTERNALS__ = { invoke: async () => 0, transformCallback: () => 0, unregisterCallback: () => {} };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    window.__KHOR_WINDOW_SURFACE__ = true;
  });
  await context.route(`http://127.0.0.1:${port}/**`, async route => {
    const cmd = new URL(route.request().url()).pathname.slice(1), args = route.request().postDataJSON() ?? {};
    calls.push({ cmd, args });
    let result = null;
    if (cmd === "window_appearance_support") result = { supported: true, blur };
    if (cmd === "config") result = rows;
    if (cmd === "set_config") { rows = rows.map(r => r.name === args.name ? { ...r, value: args.value } : r); result = rows; }
    if (cmd === "term_attach") result = { bytes: Buffer.from(Array.from({ length: 140 }, (_, i) => `row ${i} 中文\r\n`).join("") + "\x1b[31mred\x1b[0m\r\n\x1b[38;2;12;34;56mtruecolor\x1b[0m").toString("base64"), seq: 1 };
    if (["term_wait", "term_stream"].includes(cmd)) { await pause(120); result = { bytes: "", seq: 1, gone: false, reset: false, stall: null }; }
    if (cmd === "term_ended") result = {ended:{exit:0},recent:[],screen:{cols:80,rows:24,at_ms:1,bytes:Buffer.from('              \r\n\x1b[40m              \x1b[0m\r\n\x1b[48;2;64;64;96m              \x1b[0m\r\n\x1b[44m              \x1b[0m\r\n\x1b[7m              \x1b[0m\r\n\x1b[38;2;255;255;255m██████████████\x1b[0m').toString('base64')}};
    if (cmd === "save_terminal_palette") result = true;
    await route.fulfill({ contentType: "application/json", body: JSON.stringify(result) });
  });
  await context.route("**/__appearance_gate?**", route => route.fulfill({ contentType: "text/html", body: `<!doctype html><html data-theme="dark" style="background:#336699"><head><meta charset="utf-8"></head><body><style>${bootStyle}</style><div id="root"></div><script type="module">import RefreshRuntime from '/@react-refresh';RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/.appearance-gate-${process.pid}.tsx"></script></body></html>` }));
  const page = await context.newPage(); page.on("pageerror", e => errors.push(e.message));
  const startupErrors = []; page.on("requestfailed", r => startupErrors.push(`${r.url()}: ${r.failure()?.errorText}`)); page.on("response", r => { if (r.status() >= 400) startupErrors.push(`${r.status()} ${r.url()}`); });
  await page.goto(`http://localhost:${port}/__appearance_gate?bridge=${port}`);
  await page.locator("[data-terminal]").waitFor({ timeout: 30000 }).catch(async e => { throw new Error(`${e.message}\n${errors.join("\n")}\n${startupErrors.join("\n")}\n${viteLog}\n${(await page.locator("body").innerHTML()).slice(0,1200)}`); });
  // Teeth: recreate the old extra body background after stylesheet load.
  // This must fail the screenshot assertion rather than silently pass.
  if (process.env.APPEARANCE_BOOT_DOUBLE === "1") await page.addStyleTag({ content: ":root[data-window-transparent] body { background: color-mix(in srgb, var(--boot-ground) var(--window-opacity), transparent) !important; }" });
  await page.waitForFunction(() => document.querySelector("[data-terminal]")?.__xterm?.buffer.active.length > 100);
  const baseline = await page.evaluate(() => {
    const t = document.querySelector("[data-terminal]").__xterm;
    window.originalTerm = t; t.scrollLines(-40);
    return { length: t.buffer.active.length, y: t.buffer.active.viewportY, cols: t.cols, rows: t.rows };
  });
  const rgb = await page.evaluate(() => { const t = document.querySelector("[data-terminal]").__xterm; const b = t.buffer.active; const c = b.getLine(b.baseY + b.cursorY).getCell(0); return { color: c.getFgColor(), rgb: c.isFgRGB() }; });
  assert(rgb.rgb && rgb.color === 0x0c2238, "fixture needs a truecolor cell");
  const opens = calls.filter(c => c.cmd === "term_open").length;
  await page.locator('[data-setting-select="term-theme"]').selectOption("mocha");
  await page.waitForFunction(() => document.querySelector("[data-terminal]").__xterm.options.theme.foreground === "#cdd6f4");
  assert.deepEqual(await page.evaluate(() => { const t = document.querySelector("[data-terminal]").__xterm; return { length: t.buffer.active.length, y: t.buffer.active.viewportY, cols: t.cols, rows: t.rows }; }), baseline);
  assert(await page.evaluate(() => document.querySelector("[data-terminal]").__xterm === window.originalTerm));
  await page.locator('[data-setting-select="window-material"]').selectOption("clear");
  await page.getByRole("slider", { name: "背景不透明度" }).fill("85");
  await page.waitForFunction(() => document.documentElement.style.getPropertyValue("--window-opacity") === "85%");
  assert(await page.evaluate(()=>document.documentElement.hasAttribute('data-window-outline')),'transparent window lost its default outline');
  const outline=()=>page.getByRole('switch',{name:'窗口轮廓',exact:true});
  await outline().click();await page.waitForFunction(()=>!document.documentElement.hasAttribute('data-window-outline'));
  assert.equal(await page.evaluate(()=>getComputedStyle(document.getElementById('root'),'::after').content),'none');
  await outline().click();await page.waitForFunction(()=>document.documentElement.hasAttribute('data-window-outline'));
  assert.equal(await page.evaluate(()=>getComputedStyle(document.getElementById('root'),'::after').pointerEvents),'none');

  await page.waitForFunction(() => document.getElementById("root").style.getPropertyValue("--window-ground-clip").includes("M"));
  assert(await page.evaluate(() => document.querySelector("[data-terminal]").__xterm === window.originalTerm));
  assert.equal(calls.filter(c => c.cmd === "term_open").length, opens);
  const halos = await page.evaluate(() => [...document.querySelectorAll('[data-halo-lab] [data-halo]')].map(el => {const r=el.getBoundingClientRect();const s=getComputedStyle(el);return {right:r.right,y:r.top+r.height/2,gap:parseFloat(s.getPropertyValue('--halo-gap')),width:parseFloat(s.getPropertyValue('--halo-w')),ink:s.getPropertyValue('--halo-color').trim()};}));
  const image = await page.screenshot();
  if (process.env.APPEARANCE_SHOT) { writeFileSync(process.env.APPEARANCE_SHOT, image); await page.locator("[data-halo-lab]").screenshot({path:process.env.APPEARANCE_SHOT.replace(/\.png$/, "-rings.png")}); }
  const pixel = await page.evaluate(async base64 => {
    const img = new Image(); img.src = "data:image/png;base64," + base64; await img.decode();
    const canvas = document.createElement("canvas"); canvas.width = img.width; canvas.height = img.height;
    const ctx = canvas.getContext("2d"); ctx.drawImage(img, 0, 0);
    const r = document.querySelector("[data-terminal]").getBoundingClientRect();
    const screen = document.querySelector("[data-terminal] .xterm-screen").getBoundingClientRect();
    return { edge: [...ctx.getImageData(Math.floor(r.left + 2), Math.floor(r.bottom - 2), 1, 1).data], canvas: [...ctx.getImageData(Math.floor(screen.right - 20), Math.floor(screen.top + 10), 1, 1).data] };
  }, image.toString("base64"));
  const haloPixels = await page.evaluate(async ({base64,halos}) => {const img=new Image();img.src='data:image/png;base64,'+base64;await img.decode();const c=document.createElement('canvas');c.width=img.width;c.height=img.height;const ctx=c.getContext('2d');ctx.drawImage(img,0,0);return halos.map(h=>{const sample=x=>[...ctx.getImageData(Math.floor(x),Math.floor(h.y),1,1).data];return {gap:sample(h.right+0.5),outside:sample(h.right+h.gap+h.width+3),stroke:sample(h.right+h.gap+h.width-0.5)};});},{base64:image.toString('base64'),halos});
  for (const [i,h] of haloPixels.entries()) {assert.deepEqual(h.gap,h.outside,`halo ${i}: transparent gap must reveal the same background as outside the ring`);assert(h.stroke.slice(0,3).some((c,k)=>Math.abs(c-h.outside[k])>10),`halo ${i}: ring stroke must remain visible`);}
  const expected = [33, 41, 62];
  for (const [where, rgba] of Object.entries(pixel)) rgba.slice(0, 3).forEach((c, i) => assert(Math.abs(c - expected[i]) <= 3, `single background plane ${where}: got ${rgba}, expected ${expected}`));
  // DetailPane retains inactive terminals at full size under visibility:hidden.
  // Their rectangles must not toggle the shared background hole back to filled.
  await page.evaluate(() => window.keepTerminal());
  await page.waitForFunction(() => document.querySelector('[data-retained-back] [data-terminal]')?.__xterm);
  async function retainedPixel(front) {
    await page.evaluate(front => window.frontKept(front), front);
    await page.waitForFunction(front => getComputedStyle(document.querySelector(front ? '[data-retained-back] [data-terminal]' : '[data-retained-front] [data-terminal]')).visibility === 'visible', front);
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    const shot = await page.screenshot();
    return page.evaluate(async ({base64,front}) => {
      const img=new Image();img.src='data:image/png;base64,'+base64;await img.decode();
      const c=document.createElement('canvas');c.width=img.width;c.height=img.height;const ctx=c.getContext('2d');ctx.drawImage(img,0,0);
      const r=document.querySelector((front?'[data-retained-back]':'[data-retained-front]')+' .xterm-screen').getBoundingClientRect();
      return [...ctx.getImageData(Math.floor(r.right-20),Math.floor(r.top+10),1,1).data];
    },{base64:shot.toString('base64'),front});
  }
  for (const front of [false,true,false]) {
    const rgba=await retainedPixel(front);
    rgba.slice(0,3).forEach((c,i)=>assert(Math.abs(c-expected[i])<=3,`retained terminal ${front}: got ${rgba}, expected ${expected}; inactive terminals cannot add a second background`));
  }
  // Changing to a GUI tab hides both terminals but does not resize or
  // remove them. Visibility changes alone must restore the app ground.
  await page.evaluate(() => window.hideTerminals(true));
  await page.waitForFunction(() => (document.getElementById('root').style.getPropertyValue('--window-ground-clip').match(/M/g) ?? []).length === 1);
  const hiddenImage = await page.screenshot();
  const hiddenPixel = await page.evaluate(async base64 => {
    const img=new Image();img.src='data:image/png;base64,'+base64;await img.decode();
    const c=document.createElement('canvas');c.width=img.width;c.height=img.height;const ctx=c.getContext('2d');ctx.drawImage(img,0,0);
    const r=document.querySelector('[data-retained-front] .xterm-screen').getBoundingClientRect();
    return [...ctx.getImageData(Math.floor(r.right-20),Math.floor(r.top+10),1,1).data];
  },hiddenImage.toString('base64'));
  hiddenPixel.slice(0,3).forEach((c,i)=>assert(Math.abs(c-[8,15,23][i])<=3,`hidden terminals leave app ground: ${hiddenPixel}`));
  await page.evaluate(() => window.hideTerminals(false));
  const restored = await retainedPixel(false);
  restored.slice(0,3).forEach((c,i)=>assert(Math.abs(c-expected[i])<=3,`terminal ground restores after GUI tab: ${restored}`));
  await page.evaluate(() => window.closeKept());
  await page.waitForFunction(() => document.querySelectorAll('[data-terminal]').length === 1);
  assert.deepEqual(await page.evaluate(() => { const t = document.querySelector("[data-terminal]").__xterm; const b = t.buffer.active; const c = b.getLine(b.baseY + b.cursorY).getCell(0); return { color: c.getFgColor(), rgb: c.isFgRGB() }; }), rgb, "theme must not recolor program RGB");
  assert.equal(await page.evaluate(() => window.windowChoice([{name:'window-material',value:'clear'},{name:'window-opacity',value:'0'}]).opacity), 0, "zero is an opacity, not a missing preference");
  await page.getByRole("slider", { name: "背景不透明度" }).fill("0");
  await page.waitForFunction(() => document.documentElement.style.getPropertyValue("--window-opacity") === "0%");
  const zeroImage = await page.screenshot();
  const zeroPixel = await page.evaluate(async base64 => { const img = new Image(); img.src = "data:image/png;base64," + base64; await img.decode(); const c = document.createElement("canvas"); c.width=img.width;c.height=img.height;const ctx=c.getContext("2d");ctx.drawImage(img,0,0);const r=document.querySelector("[data-terminal] .xterm-screen").getBoundingClientRect();return [...ctx.getImageData(Math.floor(r.right-20),Math.floor(r.top+10),1,1).data]; },zeroImage.toString("base64"));
  assert.deepEqual(zeroPixel,[51,102,153,255], "at zero the actual terminal canvas reveals the desktop, without a black layer");
  assert.equal(await page.locator("[data-terminal] .xterm").evaluate(el => getComputedStyle(el).opacity), "1", "terminal text keeps its opacity");
  assert(await page.evaluate(() => document.querySelector("[data-terminal]").__xterm === window.originalTerm));
  await page.getByRole("slider", { name: "背景不透明度" }).fill("85");
  await page.waitForFunction(() => document.documentElement.style.getPropertyValue("--window-opacity") === "85%");

  await page.locator("[data-term-theme-edit]").click();
  await page.getByRole("textbox", { name: "名称", exact: true }).fill("我的配色");
  await page.getByRole("textbox", { name: "背景 HEX", exact: true }).fill("#102030");
  await page.getByRole("textbox", { name: "亮红 HEX", exact: true }).fill("#ffaa99");
  await page.getByRole("textbox", { name: "红 HEX", exact: true }).fill("bad");
  assert(await page.locator("[data-term-theme-save]").isDisabled());
  assert.equal(await page.locator("[data-term-theme-editor] [data-term-theme-preview]").evaluate(el => el.__xterm.options.theme.background), "#102030", "invalid hex keeps the last preview");
  await page.getByRole("textbox", { name: "红 HEX", exact: true }).fill("#ff8888");
  await page.locator("[data-term-theme-editor] summary").click();
  await page.locator("[data-term-theme-extended]").fill("#123456 #abcdef");
  await page.locator("[data-term-theme-save]").click();
  await page.waitForFunction(() => document.querySelector("[data-terminal]").__xterm.options.theme.brightRed === "#ffaa99");
  assert.deepEqual(await page.evaluate(() => document.querySelector("[data-terminal]").__xterm.options.theme.extendedAnsi), ["#123456", "#abcdef"]);
  await page.locator("[data-term-theme-export]").click();
  const exported = JSON.parse(calls.findLast(c => c.cmd === "save_terminal_palette").args.text);
  assert.equal(exported.name, "我的配色"); assert.equal(exported.theme.brightRed, "#ffaa99");
  await page.locator("[data-term-theme-file]").setInputFiles({ name: "bad.json", mimeType: "application/json", buffer: Buffer.from('{"background":"oops"}') });
  assert.equal(await page.locator('[data-setting-select="term-theme"]').inputValue(), await page.evaluate(() => window.palette.termTheme().selected));
  await page.locator("[data-term-theme-file]").setInputFiles({ name: "roundtrip.json", mimeType: "application/json", buffer: Buffer.from(JSON.stringify(exported)) });
  await page.waitForFunction(() => window.palette.termTheme().customs.length === 2);
  await page.locator("[data-term-theme-edit]").click(); await page.getByRole("textbox", { name: "背景 HEX", exact: true }).fill("#aa0000"); await page.keyboard.press("Escape");
  assert.equal(await page.evaluate(() => document.querySelector("[data-terminal]").__xterm.options.theme.foreground), exported.theme.foreground);
  const other = await context.newPage(); await other.goto(`http://localhost:${port}/__appearance_gate?bridge=${port}&face=settings`);
  await other.locator('[data-setting-select="term-theme"]').selectOption("nord");
  await page.waitForFunction(() => document.querySelector("[data-terminal]").__xterm.options.theme.foreground === "#d8dee9");
  assert.equal(await other.locator("html").getAttribute("data-window-transparent"), null, "settings window remains opaque");
  await other.close();
  await page.reload(); await page.locator('[data-setting-select="term-theme"]').waitFor();
  assert.equal(await page.locator('[data-setting-select="term-theme"]').inputValue(), "nord");
  await page.locator('[data-setting-select="window-material"]').selectOption("solid");
  await page.waitForFunction(() => !document.documentElement.hasAttribute("data-window-transparent"));
  blur = false; await page.reload(); await page.locator('[data-setting-select="window-material"]').waitFor();
  assert.equal(await page.locator('[data-setting-select="window-material"] option[value="frost"]').count(), 0, "Linux does not offer native blur");
  await page.setViewportSize({ width: 760, height: 900 }); await page.locator("[data-term-theme-edit]").click();
  assert(await page.locator("[data-term-theme-editor]").evaluate(el => el.getBoundingClientRect().right <= innerWidth));
  const validation = await page.evaluate(() => {
    const p = window.palette;
    const base = p.PRESETS[0].theme;
    const check = input => { try { p.readPalette(input, base); return false; } catch { return true; } };
    return { invalid: check({ red: 'url(https://example.com)' }), tooMany: check({ extendedAnsi: Array(241).fill('#123456') }), alpha: p.readPalette({ selectionBackground: 'rgba(10, 20, 30, 0.5)' }, base).selectionBackground, short: p.colorHex('#abc'), count: p.PRESETS.length };
  });
  assert.deepEqual(validation, { invalid: true, tooMany: true, alpha: '#0a141e80', short: '#aabbcc', count: 9 });
  // A failed local write still applies this window's choice. Retrying
  // Save updates that same custom palette, instead of accumulating copies.
  await page.getByRole("textbox", { name: "名称", exact: true }).fill("重试保存");
  await page.evaluate(() => { window.originalSetItem = Storage.prototype.setItem; Storage.prototype.setItem = function(key, value) { if (key === "khor.term-theme") throw new DOMException("blocked", "SecurityError"); return window.originalSetItem.call(this, key, value); }; });
  const beforeRetry = await page.evaluate(() => window.palette.termTheme().customs.length);
  await page.locator("[data-term-theme-save]").click();
  await page.getByRole("status").filter({ hasText: "本窗口已应用" }).first().waitFor();
  await page.locator("[data-term-theme-save]").click();
  assert.equal(await page.evaluate(() => window.palette.termTheme().customs.length), beforeRetry + 1);
  await page.evaluate(() => { Storage.prototype.setItem = window.originalSetItem; });
  await page.locator("[data-term-theme-save]").click();
  await page.locator("[data-term-theme-editor]").waitFor({ state: "hidden" });

  await page.setViewportSize({width:1400,height:900});
  // Same ANSI bytes in WebGL and its actual lost-context DOM fallback.
  // Blank cells detect double tinting; a white block detects faded text.
  await page.evaluate(async () => {
    window.wearWindowChoice({material:'clear',opacity:50},false);
    const t=document.querySelector('[data-terminal]').__xterm;
    t.options.cursorInactiveStyle='none';
    await new Promise(done=>t.write('\x1b[?25l\x1b[2J\x1b[H' +
      '              \r\n\x1b[40m              \x1b[0m\r\n'+
      '\x1b[48;2;64;64;96m              \x1b[0m\r\n'+
      '\x1b[44m              \x1b[0m\r\n'+
      '\x1b[7m              \x1b[0m\r\n'+
      '\x1b[38;2;255;255;255m██████████████\x1b[0m',done));
    t.scrollToBottom();
  });
  const sampleBackgrounds = async () => {
    await pause(150);
    const image=await page.screenshot();
    return page.evaluate(async b64=>{
      const t=document.querySelector('[data-terminal], [data-term-ended-screen]').__xterm;
      const s=t.element.querySelector('.xterm-screen').getBoundingClientRect();
      const d=t._core._renderService.dimensions.css.cell;
      const img=new Image();img.src='data:image/png;base64,'+b64;await img.decode();
      const c=document.createElement('canvas');c.width=img.width;c.height=img.height;
      const ctx=c.getContext('2d');ctx.drawImage(img,0,0);
      return Array.from({length:6},(_,i)=>[...ctx.getImageData(Math.floor((s.left+5.5*d.width)*img.width/innerWidth),Math.floor((s.top+(i+0.5)*d.height)*img.width/innerWidth),1,1).data]);
    },image.toString('base64'));
  };
  const checkBackgrounds = async renderer => {
    for (const alpha of [0.5,0,0.85,1,0.5]) {
      await page.evaluate(alpha=>window.wearWindowChoice({material:'clear',opacity:alpha*100},false),alpha);
      const samples=await sampleBackgrounds();
      const colors=await page.evaluate(()=>{
        const c=document.querySelector('[data-terminal], [data-term-ended-screen]').__xterm._core._themeService.colors;
        return [c.background.rgba>>>8, c.ansi[0].rgba>>>8, 0x404060, c.ansi[4].rgba>>>8, c.foreground.rgba>>>8];
      });
      for (let i=0;i<5;i++) {
        const rgb=[colors[i]>>>16,(colors[i]>>>8)&255,colors[i]&255];
        const expected=rgb.map((v,k)=>Math.round(v*alpha+[51,102,153][k]*(1-alpha)));
        samples[i].slice(0,3).forEach((v,k)=>assert(Math.abs(v-expected[k])<=3,`${renderer} row ${i} at ${alpha}: ${samples[i]}, expected ${expected}`));
      }
      assert(samples[5].slice(0,3).every(v=>v>245),`${renderer}: truecolor foreground remains opaque: ${samples[5]}`);
    }
    await page.evaluate(()=>{
      window.wearWindowChoice({material:'clear',opacity:0},false);
      const t=document.querySelector('[data-terminal], [data-term-ended-screen]').__xterm;
      t.select(2,t.buffer.active.viewportY,7);
    });
    const selected=(await sampleBackgrounds())[0];
    assert(selected.slice(0,3).some((v,k)=>Math.abs(v-[51,102,153][k])>10),`${renderer}: selection must remain visible at zero background opacity`);
    await page.evaluate(()=>document.querySelector('[data-terminal], [data-term-ended-screen]').__xterm.clearSelection());

  };
  await checkBackgrounds('WebGL');
  const cdp=await context.newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride',{width:1400,height:900,deviceScaleFactor:2,mobile:false});
  await checkBackgrounds('WebGL retina');
  await page.evaluate(()=>{
    const gl=document.querySelector('[data-terminal]').__xterm._core._renderService._renderer.value._gl;
    if(!gl)throw Error('WebGL fixture was not loaded');
    gl.getExtension('WEBGL_lose_context').loseContext();
  });
  await page.locator('[data-terminal] .xterm-rows').waitFor();
  await checkBackgrounds('DOM fallback retina');
  await cdp.send('Emulation.setDeviceMetricsOverride',{width:1400,height:900,deviceScaleFactor:1,mobile:false});
  await checkBackgrounds('DOM fallback');
  await page.evaluate(()=>document.dispatchEvent(new Event('visibilitychange')));
  await page.waitForFunction(()=>!!document.querySelector('[data-terminal]').__xterm._core._renderService._renderer.value._gl);
  await checkBackgrounds('WebGL recovered');
  await page.evaluate(()=>window.showEnded());
  await page.locator('[data-term-ended-drawn]').waitFor();
  await checkBackgrounds('ended snapshot');
  await page.evaluate(()=>window.wearWindowChoice({material:'frost',opacity:0},false));
  assert.equal(await page.locator('html').evaluate(el=>el.style.getPropertyValue('--window-opacity')),'0%','frost tint also reaches zero');
  if(process.env.APPEARANCE_SHOT) await page.screenshot({path:process.env.APPEARANCE_SHOT.replace(/\.png$/, '-ansi.png')});
  assert.deepEqual(errors, []);
  console.log("appearance: presets/DIY/JSON, cross-window sync, same PTY/buffer/scroll, retained/hidden tab backgrounds, background-only single-layer alpha in WebGL/DOM/recovery at 1x/2x, unsupported blur, parser bounds and narrow editor passed");
} finally { if (browser) await browser.close(); if (vite) vite.kill(); rmSync(entry, { force: true }); }
