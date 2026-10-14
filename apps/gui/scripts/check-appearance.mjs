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
  { name: "window-opacity", value: "100", choices: [], page: "appearance", section: "window" },
];
try {
  writeFileSync(entry, `import React,{useState,useEffect} from 'react';
import {createRoot} from 'react-dom/client';
import {TooltipProvider} from '../src/components/ui/tooltip';
import {SettingsProvider,SettingsRereadProvider} from '../src/settings';
import {TerminalThemeSection} from '../src/views/more/TerminalThemeSection';
import {WindowAppearanceSection} from '../src/views/more/WindowAppearanceSection';
import {TerminalPane} from '../src/views/TerminalPane';
import * as palette from '../src/lib/term-theme';
import {followWindowSurface,wearWindowChoice,windowChoice} from '../src/lib/window-surface';
import {fetchConfig} from '../src/api';
import {speakLang} from '../src/lang';
import '../src/styles/tokens.css';import '../src/styles/app.css';
speakLang('zh');palette.wearTermPalette();followWindowSurface();window.palette=palette;
function Lab(){const [rows,setRows]=useState([]);const read=()=>fetchConfig().then(setRows);useEffect(()=>{read()},[]);const surface=windowChoice(rows);useEffect(()=>{if(surface)wearWindowChoice(surface)},[surface?.material,surface?.opacity]);
return <SettingsProvider value={rows}><SettingsRereadProvider value={read}><TooltipProvider><div style={{display:'flex',height:'100vh',color:'var(--foreground)'}}><div style={{flex:1,minWidth:0,display:'flex',flexDirection:'column'}}><TerminalPane id='shell/appearance-test' active /></div><aside style={{width:400,overflow:'auto',padding:'var(--gap-base)'}}><WindowAppearanceSection/><TerminalThemeSection/></aside></div></TooltipProvider></SettingsRereadProvider></SettingsProvider>}
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
  await page.waitForFunction(() => document.querySelector("[data-terminal]").__xterm.options.theme.background === "transparent" && document.getElementById("root").style.getPropertyValue("--window-ground-clip").includes("M"));
  assert(await page.evaluate(() => document.querySelector("[data-terminal]").__xterm === window.originalTerm));
  assert.equal(calls.filter(c => c.cmd === "term_open").length, opens);
  const image = await page.screenshot();
  const pixel = await page.evaluate(async base64 => {
    const img = new Image(); img.src = "data:image/png;base64," + base64; await img.decode();
    const canvas = document.createElement("canvas"); canvas.width = img.width; canvas.height = img.height;
    const ctx = canvas.getContext("2d"); ctx.drawImage(img, 0, 0);
    const r = document.querySelector("[data-terminal]").getBoundingClientRect();
    return [...ctx.getImageData(Math.floor(r.left + 2), Math.floor(r.bottom - 2), 1, 1).data];
  }, image.toString("base64"));
  const expected = [33, 41, 62];
  pixel.slice(0, 3).forEach((c, i) => assert(Math.abs(c - expected[i]) <= 3, `single background plane: got ${pixel}, expected ${expected}`));
  assert.deepEqual(await page.evaluate(() => { const t = document.querySelector("[data-terminal]").__xterm; const b = t.buffer.active; const c = b.getLine(b.baseY + b.cursorY).getCell(0); return { color: c.getFgColor(), rgb: c.isFgRGB() }; }), rgb, "theme must not recolor program RGB");
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
  assert.deepEqual(errors, []);
  console.log("appearance: presets/DIY/JSON, cross-window sync, same PTY/buffer/scroll, background-only single-layer alpha, unsupported blur, parser bounds and narrow editor passed");
} finally { if (browser) await browser.close(); if (vite) vite.kill(); rmSync(entry, { force: true }); }
