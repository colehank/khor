// Real-connection check for the browser face (批⑦): a real `khor serve`,
// a real browser that is not the app, and the page it actually serves.
//
// **What is here is only what a browser can prove.** The locks
// themselves — no key, wrong key, foreign origin, a name instead of an
// address, a rotated key retiring the old one — are asserted against a
// live server in `crates/web/tests/face.rs`, where they are cheaper and
// cannot be confused with a rendering problem. What that file cannot
// reach is whether the page khor serves *is the app*, and whether the
// key survives the trip from the address bar into storage. That is this
// file.
//
//   node scripts/web-face.mjs
//
// Env: KHOR_BIN (default target/debug/khor), WEB_PORT (default 5468 —
// next door to the product's 5467, so a developer's own face can be up
// at the same time).
import { spawn, execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, readFileSync, existsSync, copyFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const repo = join(gui, "../..");
const PORT = Number(process.env.WEB_PORT ?? 5468);
// Snapshot the binary, for the reason the smoke run does: another lane's
// `cargo build` mid-run would otherwise swap the thing under test.
const scratch = mkdtempSync(join(tmpdir(), "khor-webface-"));
const KHOR = join(scratch, "khor");
copyFileSync(process.env.KHOR_BIN ?? join(repo, "target/debug/khor"), KHOR);
const HOME = join(scratch, "home");

const children = [];
let browser;
const done = [];
function ok(what) {
  done.push(what);
  console.log(`  ok  ${what}`);
}

async function until(what, ms, f) {
  const deadline = Date.now() + ms;
  // f() runs once before the deadline is consulted, so a 0ms budget
  // cannot manufacture a failure.
  for (;;) {
    const got = await f();
    if (got) return got;
    if (Date.now() > deadline) throw new Error(`timed out waiting for ${what}`);
    await new Promise((r) => setTimeout(r, 100));
  }
}

async function main() {
  const env = { ...process.env, KHOR_HOME: HOME, KHOR_NAME: "webface", KHOR_WEB_PORT: String(PORT) };
  delete env.KHOR_SESSION;
  const serve = spawn(KHOR, ["serve"], { env, stdio: ["ignore", "pipe", "pipe"], detached: true });
  children.push(serve);
  let log = "";
  serve.stdout.on("data", (d) => (log += d));
  serve.stderr.on("data", (d) => (log += d));
  await until("the face to bind", 30_000, () => log.includes(`:${PORT}`));

  const keyFile = join(HOME, ".khor", "web.key");
  const key = await until("a key on disk", 10_000, () =>
    existsSync(keyFile) ? readFileSync(keyFile, "utf8").trim() : null,
  );
  ok("serve raised the face and minted a key");

  browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server"] });
  // A phone, as closely as this machine can be one: the narrow face's
  // own breakpoint is 719px, and touch is what decides whether a
  // hover-only control exists at all.
  const phone = await browser.newContext({
    viewport: { width: 390, height: 844 },
    hasTouch: true,
    isMobile: true,
  });
  const page = await phone.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));

  // 1. The link khor printed, opened by something that is not the app.
  await page.goto(`http://127.0.0.1:${PORT}/?k=${key}`);
  const rail = await until("the app to paint", 20_000, async () =>
    (await page.locator("[data-rail], nav").count()) > 0 ? true : null,
  );
  if (!rail) throw new Error("no rail");
  ok("the page khor serves is the app, not a placeholder");

  // The app is only real if the backend answered: this machine's own
  // name comes from the node, and nothing in the bundle knows it.
  await until("this machine's own row", 20_000, async () =>
    (await page.getByText("webface", { exact: false }).count()) > 0 ? true : null,
  );
  ok("the backend answered through /api — the machine's own name is on screen");

  // 2. The key comes off the address bar and is kept.
  const url = page.url();
  if (url.includes(key)) throw new Error(`the key is still in the address bar: ${url}`);
  ok("the key is no longer in the address bar");
  const held = await page.evaluate(() => window.localStorage.getItem("khor.web.key"));
  if (held !== key) throw new Error(`storage holds ${held}, not the key`);
  ok("the key is in storage");

  // 3. The bookmark promise: the stripped address, opened cold.
  await page.goto(`http://127.0.0.1:${PORT}/`);
  await until("the app again, with no key in the link", 20_000, async () =>
    (await page.getByText("webface", { exact: false }).count()) > 0 ? true : null,
  );
  ok("the address without the key still opens — a bookmark works");

  if (errors.length) throw new Error(`the page threw: ${errors.join(" | ")}`);
  ok("nothing threw while all of that happened");

  // 4. **Locked out, and told so.** A browser that never had the key
  //    must read a sentence, not stare at a half-drawn screen. A fresh
  //    context, because the point is a device that was never given one.
  const stranger = await browser.newContext({ viewport: { width: 390, height: 844 } });
  const cold = await stranger.newPage();
  await cold.goto(`http://127.0.0.1:${PORT}/`);
  const refusal = await until("khor to say why", 20_000, async () => {
    const text = await cold.locator("body").innerText();
    return text.includes("钥匙") ? text : null;
  });
  if (!refusal.includes("khor web")) {
    throw new Error(`the refusal does not say what to do: ${refusal.slice(0, 200)}`);
  }
  ok("a browser with no key is told what to run, in a sentence");

  // ── the phone's own three, on a live terminal ──
  //
  // `cat` because it echoes: what comes back on the grid is proof the
  // bytes reached the pty, not proof that a field accepted a keystroke.
  execFileSync(KHOR, ["open", "-d", "--title", "termcat", "--", "cat"], { env });
  await page.goto(`http://127.0.0.1:${PORT}/`);
  const row = await until("the terminal session to appear", 30_000, async () => {
    const r = page.locator('[data-row]').filter({ hasText: "termcat" });
    return (await r.count()) > 0 ? r.first() : null;
  });
  await row.tap();
  await until("the terminal pane", 30_000, async () =>
    (await page.locator("[data-terminal]").count()) > 0 ? true : null,
  );

  // 5. **A phone can type into it at all.** A `div` raises no soft
  //    keyboard; only a field does, so a tap has to land focus on one.
  await page.locator("[data-terminal]").tap();
  const focused = await page.evaluate(() => ({
    tag: document.activeElement?.tagName ?? "(none)",
    isField: document.activeElement?.hasAttribute("data-term-keys") ?? false,
  }));
  if (!focused.isField) {
    throw new Error(`tapping the terminal focused ${focused.tag}, not the field`);
  }
  ok("a tap puts focus where a soft keyboard can appear");

  // 6. …and what that field receives reaches the shell. This is the
  //    event a soft keyboard actually produces: text arriving in the
  //    field, not a `keydown` anyone can name.
  const typed = `hello-from-a-phone-${process.pid}`;
  await page.evaluate((text) => {
    const field = document.querySelector("[data-term-keys]");
    field.value = text;
    field.dispatchEvent(new Event("input", { bubbles: true }));
  }, typed + "\r");
  await until("the shell to echo it back", 30_000, async () =>
    (await page.locator("[data-terminal]").innerText()).includes(typed) ? true : null,
  );
  ok("what the field receives reaches the pty — the shell echoed it back");

  // 7. **Scrollback is reachable by dragging.** `onWheel` is the only
  //    other way back through history and a touch screen never sends
  //    one, so without this the 2000 lines 批⑤ added exist and cannot
  //    be looked at from a phone.
  for (let i = 0; i < 60; i++) {
    await page.evaluate((text) => {
      const field = document.querySelector("[data-term-keys]");
      field.value = text;
      field.dispatchEvent(new Event("input", { bubbles: true }));
    }, `line-${i}\r`);
  }
  await until("the screen to fill", 30_000, async () =>
    (await page.locator("[data-terminal]").innerText()).includes("line-59") ? true : null,
  );
  const box = await page.locator("[data-terminal]").boundingBox();
  const midX = box.x + box.width / 2;
  await page.touchscreen.tap(midX, box.y + box.height / 2);
  // A drag downwards walks backwards, which is where the older lines are.
  await page.evaluate(({ x, y1, y2 }) => {
    const pane = document.querySelector("[data-terminal]");
    const touch = (type, y) =>
      pane.dispatchEvent(
        new TouchEvent(type, {
          bubbles: true,
          touches: type === "touchend" ? [] : [new Touch({ identifier: 1, target: pane, clientX: x, clientY: y })],
        }),
      );
    touch("touchstart", y1);
    touch("touchmove", y2);
    touch("touchend", y2);
  }, { x: midX, y1: box.y + 20, y2: box.y + box.height - 20 });
  await until("the pane to say it is no longer at the bottom", 10_000, async () =>
    (await page.locator("[data-term-bottom]").count()) > 0 ? true : null,
  );
  ok("dragging walks back through the scrollback");

  console.log(`\nweb face: all green (${done.length} checks)`);
}

async function cleanup() {
  if (browser) await browser.close().catch(() => {});
  for (const c of children) {
    try {
      process.kill(-c.pid, "SIGKILL");
    } catch {
      /* already gone */
    }
  }
  rmSync(scratch, { recursive: true, force: true });
}

try {
  await main();
} finally {
  await cleanup();
}
