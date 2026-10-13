// Real map, matrix and machine page; only transport is mocked. No user store.
// Run: node scripts/check-connectivity.mjs (system Chrome).
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { writeFileSync, rmSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
const gui = fileURLToPath(new URL("..", import.meta.url));
const port = Number(process.env.CONNECTIVITY_PORT ?? 1468);
const entry = `${gui}/scripts/.connectivity-gate-${process.pid}.tsx`;
const shots = process.env.CONNECTIVITY_SHOTS ?? "/Users/zgh/khor-drop/connectivity";
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const id = n => String(n).repeat(64);
const face = color => ({ canvas: 100, background: color, radius_ratio: .5, paint: { variant: "marble", blur: 20, veins: [] }, mark: { ink: "dark", veil: 0 } });
const device = (n, name, extra = {}) => ({ id: id(n), name, shown: name, label: null, me: false, paused: false, face: face(["", "#c8c99c", "#93cc82", "#edbca0", "#b1c7d9", "#d5b4c8"][n]), pinned: false, vitals: null, hop: "direct", heard_ms: 100, rtt_ms: 3, relay: null, via: null, down_ms: null, last_rtt_ms: 3, since_ms: 100, last_hop: "direct", roads: [], sponsor: null, watch: [], watched: null, seat: { left: 50, top: 50 }, hosts: true, ...extra });
const road = (n, ms, extra = {}) => ({ to: id(n), road: "direct", relay: null, ms, via: null, broken: null, ...extra });
const devices = [
  device(1, "MacLab", { me: true, hop: null, rtt_ms: null, roads: [road(2, 3), road(3, 29, { via: id(2) }), road(4, 18), road(5, null, { road: "none" })] }),
  device(2, "turing", { roads: [road(1, 4), road(3, 26), road(4, 17), road(5, null, { road: "none" })] }),
  device(3, "hinton", { hop: "forwarded", via: "turing", rtt_ms: 29, roads: [road(1, 31, { via: id(2) }), road(2, 27), road(4, 19), road(5, 8)] }),
  device(4, "lecun", { rtt_ms: 18, roads: [road(1, 20), road(2, 19), road(3, 23), road(5, null, { road: "none" })] }),
  // The phone's old claim conflicts deliberately with the live reporter:
  // being unreachable from MacLab cannot delete hinton's own observation.
  device(5, "my iphone", { hosts: false, hop: "quiet", down_ms: 240000, heard_ms: 250000, rtt_ms: null, last_rtt_ms: 73, roads: [road(1, 71), road(2, 74), road(3, 73), road(4, 75)] }),
];
let vite, browser;
const errors = [], diagnostics = [], calls = [];
try {
  writeFileSync(entry, `import React from 'react';
import {createRoot} from 'react-dom/client'; import {flushSync} from 'react-dom';
import {MyKhorPage} from '../src/views/more/MyKhorPage.tsx';
import {MachineCard} from '../src/views/MachineCard.tsx';
import {TooltipProvider} from '../src/components/ui/tooltip.tsx';
import {NARROW_QUERY} from '../src/hooks/use-narrow.ts';
import '../src/styles/tokens.css'; import '../src/styles/app.css';
const rows=${JSON.stringify(devices)}; const no=()=>{}; const root=createRoot(document.getElementById('root'));
const posture=()=>{document.documentElement.toggleAttribute('data-narrow',matchMedia(NARROW_QUERY).matches); document.documentElement.toggleAttribute('data-coarse',matchMedia('(pointer:coarse)').matches);}; posture(); addEventListener('resize',posture);
window.showMap=()=>flushSync(()=>root.render(<TooltipProvider><MyKhorPage key='map' devices={rows} rows={[]} onClose={no} onOpenMachine={no} onNewSession={no} onOpenWorkspaces={no} onOpenBlocked={no} onPin={async()=>{}} onForget={async()=>{}} onAdd={no} onOpenRelays={no} onChanged={no} onReset={no}/></TooltipProvider>));
window.showMachine=(index)=>flushSync(()=>root.render(<TooltipProvider><MachineCard key={index} row={rows[index]} rows={rows} narrow={matchMedia(NARROW_QUERY).matches} onBack={no} onRecognition={no}/></TooltipProvider>));
window.showMap();`);
  vite = spawn("node_modules/.bin/vite", ["--port", String(port), "--strictPort"], { cwd: gui, stdio: ["ignore", "pipe", "pipe"] });
  let ready = false, exited = false;
  vite.stdout.on("data", d => { ready ||= String(d).includes(`:${port}`); });
  vite.stderr.on("data", d => diagnostics.push(String(d)));
  vite.on("exit", () => { exited = true; });
  for (let n = 0; n < 100 && !ready && !exited; n++) await pause(100);
  assert(ready, "vite did not start");
  browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server", "--disable-features=LocalNetworkAccessChecks"] });
  mkdirSync(shots, { recursive: true });
  for (const width of [1200, 390, 375]) {
    const context = await browser.newContext({ viewport: { width, height: width > 700 ? 800 : 844 }, hasTouch: width < 700 });
    const page = await context.newPage();
    page.on("pageerror", e => errors.push(`${width}: ${e.message}`));
    page.on("console", e => { if (e.type() === "error") diagnostics.push(e.text()); });
    await page.route(`http://127.0.0.1:${port}/**`, async route => {
      const cmd = new URL(route.request().url()).pathname.slice(1);
      calls.push(cmd);
      let result = null;
      if (cmd === "about") result = { id: id(1), name: "MacLab", version: "0.1.14", paused: false, faces: [], hosts: true };
      else if (cmd === "relays") result = { in_use: [], custom: [] };
      else if (["workspaces", "waiting", "load"].includes(cmd)) {
        if (cmd === "load") { await route.fulfill({ status: 503, body: "fixture has no process reading" }); return; }
        result = [];
      } else if (cmd === "journal") result = Array.from({ length: 8 }, (_, i) => ({ at: Date.now() + i, when: "19:54:12", machine: "hinton", text: `fixture connection ${i} established through turing`, trouble: false }));
      await route.fulfill({ contentType: "application/json", body: JSON.stringify(result) });
    });
    await page.route("**/__connectivity_gate?**", route => route.fulfill({ contentType: "text/html", body: `<!doctype html><html data-theme="dark"><head><meta charset="utf-8"></head><body><div id="root" class="flex h-screen min-w-0 flex-col"></div><script type="module">import RefreshRuntime from '/@react-refresh';RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/.connectivity-gate-${process.pid}.tsx"></script></body></html>` }));
    await page.goto(`http://localhost:${port}/__connectivity_gate?bridge=${port}`);
    await page.locator('[data-dial-seat][data-me]').waitFor({ timeout: 15000 }).catch(async e => { throw new Error(`${e.message}\n${errors.concat(diagnostics).join("\n")}\n${await page.locator("body").innerText()}`); });
    await pause(150);
    assert.equal(await page.locator('[data-dial-segment] button').count(), 2, "MyKhor must have only map/matrix segments");
    const edgeRead = () => page.locator('[data-dial-edge]').evaluateAll(els => els.filter(el => {
      let p = el; while (p) { const s = getComputedStyle(p); if (s.display === 'none' || s.visibility === 'hidden' || Number(s.opacity) === 0) return false; p = p.parentElement; } return true;
    }).map(el => ({ key: el.getAttribute('data-dial-edge'), road: el.getAttribute('data-road'), dash: getComputedStyle(el).strokeDasharray })));
    const initial = await edgeRead();
    assert.equal(initial.length, 4, `${width}: default map must show exactly four local edges`);
    assert(initial.every(e => e.key.split('|').includes(id(1))), "default map leaked remote pair");
    const relayEdge = initial.find(e => e.key.split('|').includes(id(3)));
    assert(relayEdge && relayEdge.dash !== "none" && relayEdge.dash !== "0px", "peer relay must use one dashed logical edge");
    const relayBadge = page.locator('[data-dial-relay]').first();
    await relayBadge.waitFor();
    assert(await relayBadge.locator('svg').count() > 0, "relay badge must use the actual machine avatar");
    assert.match(await relayBadge.innerText(), /29/, "relay badge lost RTT");
    await page.screenshot({ path: `${shots}/map-${width}-dark.png` });
    const phone = page.locator(`[data-dial-seat="${id(5)}"]`);
    if (width > 700) await phone.hover(); else await phone.tap();
    await pause(200);
    const stale = await edgeRead();
    assert.equal(stale.length, 4, `${width}: phone focus must have four outgoing observations`);
    assert(stale.every(e => e.key.split('|').includes(id(5)) && e.road === 'none'), `${width}: offline phone's historic direct roads still appear live: ${JSON.stringify(stale)}`);
    await page.screenshot({ path: `${shots}/map-offline-${width}.png` });
    const hinton = page.locator(`[data-dial-seat="${id(3)}"]`);
    if (width > 700) await hinton.hover(); else await hinton.tap();
    await pause(100);
    assert((await edgeRead()).some(e => e.key.split('|').includes(id(5)) && e.road === 'direct'), "live reverse reporter was erased by local unreachability");
    await page.locator('button[data-dial-view="matrix"]').click();
    const matrix = page.locator('[data-dial-matrix]');
    await matrix.waitFor();
    assert.equal(await page.locator('[data-dial-matrix-title]').count(), 0, "redundant matrix heading returned");
    assert(!/从\s*↓|到\s*→|往返\s*\/\s*ms|行机器观测列机器|过期/.test(await matrix.innerText()), "matrix has rejected explanatory labels");
    assert.equal(await page.locator('[data-matrix-cell]').count(), 25, "matrix fixture incomplete");
    assert.equal(await page.locator(`[data-matrix-cell="${id(5)}>${id(3)}"]`).innerText(), "不通", "offline observation label");
    const backgrounds = await page.locator('[data-matrix-cell]:not([data-self])').evaluateAll(els => [...new Set(els.map(el => getComputedStyle(el).backgroundColor))]);
    assert.equal(backgrounds.length, 1, `matrix cells must not use latency heatmap fills: ${backgrounds}`);
    const overflow = await matrix.evaluate(el => ({ scroll: el.scrollWidth, width: el.clientWidth, document: document.documentElement.scrollWidth, viewport: innerWidth }));
    assert(overflow.scroll <= overflow.width + 1 && overflow.document <= overflow.viewport + 1, `${width}: matrix overflows horizontally: ${JSON.stringify(overflow)}`);
    await page.locator(`[data-matrix-cell="${id(2)}>${id(1)}"]`).click();
    await page.locator('[data-matrix-detail]').waitFor();
    const reverse = page.locator('[data-matrix-reverse]');
    await reverse.waitFor();
    assert.match(await reverse.innerText(), /3/, "reverse readout must use MacLab's own 3ms, not turing's 4ms");
    await page.screenshot({ path: `${shots}/matrix-${width}-dark.png` });
    await page.evaluate(() => { document.documentElement.dataset.theme = "light"; });
    await page.screenshot({ path: `${shots}/matrix-${width}-light.png` });
    await reverse.click();
    assert.match(await page.locator('[data-matrix-detail]').innerText(), /MacLab → turing/, "reverse button did not change the observation source");
    await page.evaluate(() => window.showMachine(2));
    const info = page.locator('[data-connection-info]'); await info.waitFor();
    assert.equal(await page.locator('[data-connection-details]').count(), 0, "connection details must be disclosed, not always visible");
    assert.equal(await page.locator('[data-journal]').count(), 0, "journal escaped the info disclosure");
    await info.click();
    const details = page.locator('[data-connection-details]'); await details.waitFor();
    await details.locator('[data-road-line]').waitFor();
    await details.locator('[data-journal-line]').first().waitFor();
    assert.equal(await details.locator('[data-journal-line]').count(), 8, 'connection journal must show all returned records');
    assert.match(await details.innerText(), /turing/, "connection details lost relay path");
    assert.equal(await page.getByRole('button', { name: /报告/ }).count(), 0, "retired report export control returned");
    await pause(300); // Wait for the sheet's entrance before measuring its bounds.
    const detailBounds = await details.evaluate(el => { const r = el.getBoundingClientRect(); return { top: r.top, bottom: r.bottom, viewport: innerHeight, height: r.height }; });
    assert(detailBounds.top >= 0 && detailBounds.bottom <= detailBounds.viewport + 1 && detailBounds.height > 40, `${width}: connection detail is clipped outside the viewport: ${JSON.stringify(detailBounds)}`);
    await page.screenshot({ path: `${shots}/machine-details-${width}.png` });
    await page.evaluate(() => window.showMachine(0));
    await page.locator('[data-machine-here]').waitFor();
    assert.equal(await page.locator('[data-connection-info]').count(), 0, "local machine must not have connection info button");
    await context.close();
    console.log(`connectivity · ${width}px: map direction/stale report/relay avatar, compact matrix/reverse, machine disclosure`);
  }
  assert(calls.includes("journal"), "journal fixture was never requested");
  assert.deepEqual(errors, [], "browser exceptions");
  console.log(`connectivity gate: clean; screenshots ${shots}`);
} finally {
  await browser?.close(); vite?.kill(); rmSync(entry, { force: true });
}
