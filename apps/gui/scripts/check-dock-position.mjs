// Production App, controlled transport and platform insets; no user data.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
const gui = fileURLToPath(new URL("..", import.meta.url));
const port = Number(process.env.DOCK_PORT ?? 23790);
const shots = process.env.DOCK_SHOTS ?? "/Volumes/disk0/khor-deck-bottom-proof";
let server, browser;
const pause = (ms) => new Promise((done) => setTimeout(done, ms));
const iphone = "Mozilla/5.0 (iPhone; CPU iPhone OS 26_0 like Mac OS X) AppleWebKit/605.1.15 Version/26.0 Mobile/15E148 Safari/604.1";
const android = "Mozilla/5.0 (Linux; Android 16; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Mobile Safari/537.36";
try {
  server = spawn("node_modules/.bin/vite", ["--config", "scripts/ai-elements-preview.vite.ts", "--host", "127.0.0.1", "--port", String(port), "--strictPort"], { cwd: gui, env: { ...process.env, KHOR_FACE_GATE: "1" }, stdio: ["ignore", "pipe", "pipe"] });
  let ready = false, diagnostics = ""; server.stdout.on("data", (b) => { ready ||= String(b).includes(`:${port}`); }); server.stderr.on("data", (b) => { diagnostics += b; });
  for (let n = 0; n < 150 && !ready && server.exitCode === null; n++) await pause(100);
  assert(ready, `preview startup: ${diagnostics}`);
  browser = await chromium.launch({ channel: "chrome", headless: !process.env.DOCK_HEADED }); mkdirSync(shots, { recursive: true });
  for (const [platform, width, height, safe, wanted] of [["ios",430,932,34,22],["ios",375,667,0,8],["ios",667,375,21,16],["android",430,932,24,32],["android",430,932,48,56],["desktop",430,800,0,16]]) {
    const touch = platform !== "desktop";
    const context = await browser.newContext({ viewport: { width, height }, hasTouch: touch, isMobile: touch, ...(touch ? { userAgent: platform === "ios" ? iphone : android } : {}) });
    await context.addInitScript(() => { localStorage.setItem("khor.place", JSON.stringify({ landing: "sessions", screen: "list" })); localStorage.setItem("khor.sessions.tabs", JSON.stringify({ tabs: [], active: null })); });
    const page = await context.newPage(), errors = []; page.on("pageerror", (e) => errors.push(e.message));
    await page.goto(`http://localhost:${port}/?bridge=${port}`);
    await page.addStyleTag({ content: `:root:not([data-keyboard]){--safe-bottom:${safe}px}` });
    const nav = page.locator("[data-float-nav]");
    if (width < 760) {
      await nav.waitFor(); const box = await nav.boundingBox();
      assert(Math.abs(height - box.y - box.height - wanted) < 1, `${platform} ${width}: bottom ${height-box.y-box.height}, expected ${wanted}`);
      const clear = await page.locator("[data-pane-body]").first().evaluate((el) => parseFloat(getComputedStyle(el).paddingBottom));
      assert(clear >= wanted + box.height, "last list row cannot clear the floating bar");
      assert(box.x >= 16 && box.x+box.width <= width-16, "horizontal gutters collapsed");
    } else assert.equal(await nav.count(), 0, "wide layout acquired mobile navigation");
    if (platform === "ios" && width === 430) {
      await page.screenshot({ animations: "disabled", path: `${shots}/list.png` });
      await page.evaluate(() => document.documentElement.setAttribute("data-theme", "dark"));
      await page.screenshot({ animations: "disabled", path: `${shots}/list-dark.png` });
      await page.evaluate(() => document.documentElement.setAttribute("data-theme", "light"));
      await page.getByRole("button", { name: /^AI Elements · 组件总览 / }).click();
      const back = page.locator("[data-session-deck] [data-back]"); await back.waitFor();
      let box = await back.boundingBox(); assert(Math.abs(height-box.y-box.height-22)<1, `session bottom ${height-box.y-box.height}`);
      await page.screenshot({ animations: "disabled", path: `${shots}/session.png` });
      await page.evaluate(() => { Object.defineProperty(window.visualViewport, "height", { configurable: true, get: () => 550 }); window.visualViewport.dispatchEvent(new Event("resize")); });
      await page.waitForFunction(() => document.documentElement.hasAttribute("data-keyboard"));
      box = await back.boundingBox(); assert(Math.abs(550-box.y-box.height-8)<1, "keyboard acquired a second safe-area gap");
      await page.evaluate(() => { delete window.visualViewport.height; window.visualViewport.dispatchEvent(new Event("resize")); });
      await page.waitForFunction(() => !document.documentElement.hasAttribute("data-keyboard"));
      box = await back.boundingBox(); assert(Math.abs(height-box.y-box.height-22)<1, "keyboard dismissal did not restore dock baseline");
    }
    assert.deepEqual(errors, []); console.log(`${platform} ${width}×${height}: inset ${safe}, bottom ${wanted}, layout passed`); await context.close();
  }
} finally { await browser?.close(); server?.kill("SIGTERM"); }
