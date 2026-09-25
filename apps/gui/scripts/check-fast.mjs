// Fast: nothing makes a person wait, and nothing makes them lose what
// they were looking at (docs/DESIGN.md 第五节「快」, docs/SHELL.md 第十六轮).
//
// # Why this gate exists
//
// 2026-09-24:「机器页面与额度页面打开后特别慢才加载出来内容, 而且离开后
// 再返回也没有上次内容的缓存」. Two faults, and no gate could see either:
//
//   · this machine's reading was taken inside the request, one `statfs`
//     per volume, and a USB disk asleep answers its `statfs` in 3.65 s —
//     so the device table took 10–36 s. Every gate's fake home sits on
//     the build machine's healthy disks, where the same code is fast;
//   · the machine card asked for its process table afresh on every mount
//     and drew nothing until it came, so coming back to the machine page
//     was an empty card first, every time. A screenshot taken a second
//     later cannot tell 「it was there」 from 「it came back quickly」.
//
// # The claims
//
//   1. with a volume that takes ten seconds to answer under the bridge
//      (`BRIDGE_SLOW_VOLUME_MS`, a volume that does not exist — never a
//      real disk), the machine page shows this machine's row within one
//      second of being opened, and its card with readings within one
//      second of the row being pressed;
//   2. after leaving for another page and coming back, **the first
//      frame** has the card, its readings, and its process rows — read
//      in the animation frame right after the press, before anything
//      could have been fetched.
//
// # It builds nothing
//
// `check-day`'s rule: cargo's lock is shared, so a missing bridge is a
// refusal that names the command.
//
// Run: `npm run fast` (needs system Chrome and
// `cargo build -p khor-gui-core --bin bridge`).
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright-core";

import { fakeHome, fakeUsage } from "./fake-home.mjs";

const gui = fileURLToPath(new URL("..", import.meta.url));
const repo = join(gui, "..", "..");

const PORT = Number(process.env.FAST_PORT ?? 1471);
const BRIDGE_PORT = Number(process.env.FAST_BRIDGE_PORT ?? 1472);
// The slow volume's `statfs`, in ms: the ten seconds of the cargo gate,
// well past the three and a half a sleeping My Book measured.
const SLOW_MS = 10_000;
const OPEN_MS = 1_000;

const target = process.env.CARGO_TARGET_DIR ?? join(repo, "target");
const BRIDGE = join(target, "debug/bridge");

const problems = [];
const fail = (m) => problems.push(m);

if (!existsSync(BRIDGE)) {
  console.error(
    `the bridge is not built: ${BRIDGE}\n` +
      "  cargo build -p khor-gui-core --bin bridge\n" +
      "This gate does not build it for you — cargo's lock is shared, and a check " +
      "that queues behind a build is a check nobody runs.",
  );
  process.exit(1);
}

// A home of its own, and a `HOME` of its own (`fake-home.mjs`).
const home = mkdtempSync(join(tmpdir(), "khor-fast-gate-"));
const usage = await fakeUsage();
const fakeHomeDir = join(home, "home");
mkdirSync(fakeHomeDir, { recursive: true });
const faked = fakeHome(fakeHomeDir, usage.url);

let bridge, vite, browser;
const listening = async (port, what, ms) => {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    try {
      const r = await fetch(`http://127.0.0.1:${port}/`, { method: "POST", body: "{}" });
      if (r) return true;
    } catch {
      /* not up yet */
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`${what} never came up on ${port}`);
};

try {
  bridge = spawn(BRIDGE, [], {
    env: {
      ...process.env,
      ...faked,
      KHOR_HOME: home,
      KHOR_NAME: "fast-gate",
      BRIDGE_PORT: String(BRIDGE_PORT),
      BRIDGE_SLOW_VOLUME_MS: String(SLOW_MS),
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  bridge.on("exit", (code) => {
    if (code !== null && code !== 0) fail(`the bridge exited with ${code} — probably a taken port`);
  });
  await listening(BRIDGE_PORT, "the bridge", 60_000);

  vite = spawn("node_modules/.bin/vite", ["--port", String(PORT), "--strictPort"], {
    cwd: gui,
    stdio: ["ignore", "pipe", "pipe"],
  });
  vite.stderr.on("data", (b) => process.stderr.write(`[vite] ${b}`));
  await new Promise((ok, no) => {
    const t = setTimeout(() => no(new Error("vite never said it was ready")), 60_000);
    vite.stdout.on("data", (b) => {
      if (String(b).includes("ready in")) {
        clearTimeout(t);
        ok();
      }
    });
  });

  browser = await chromium.launch({
    channel: "chrome",
    headless: true,
    // A proxy in front of localhost turns every call into a timeout.
    args: ["--no-proxy-server"],
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  const thrown = [];
  page.on("pageerror", (e) => thrown.push(String(e)));
  await page.goto(`http://localhost:${PORT}/?bridge=${BRIDGE_PORT}`, { waitUntil: "load" });

  /** Polls `f` until it holds or `ms` passes; answers the time it took,
      or null. */
  const within = async (ms, f) => {
    const t = Date.now();
    while (Date.now() - t < ms) {
      if (await f()) return Date.now() - t;
      await new Promise((r) => setTimeout(r, 25));
    }
    return null;
  };
  const rail = (landing) => page.locator(`[data-rail-item][data-landing="${landing}"]`).first();
  const count = (sel) => page.locator(sel).count();

  const railUp = await within(20_000, async () => (await rail("devices").count()) > 0);
  if (railUp === null) throw new Error("probe dead: the rail never drew, so nothing below measured anything");

  // ── 1. the first visit ─────────────────────────────────────────────
  await rail("devices").click();
  const rowIn = await within(SLOW_MS + 2_000, async () => (await count("[data-list] [data-row-open]")) > 0);
  if (rowIn === null) {
    fail(`probe dead: the machine page never listed this machine, even after ${SLOW_MS + 2_000} ms`);
  } else if (rowIn > OPEN_MS) {
    fail(`the machine page took ${rowIn} ms to list this machine (limit ${OPEN_MS} ms) — the device table is waiting on a disk`);
  }
  if (rowIn !== null) {
    await page.locator("[data-list] [data-row-open]").first().click();
    const cardIn = await within(SLOW_MS + 2_000, async () => (await count("[data-machine-card] [data-vitals]")) > 0);
    if (cardIn === null) fail("probe dead: this machine's card never showed a reading");
    else if (cardIn > OPEN_MS) fail(`this machine's card took ${cardIn} ms to show its readings (limit ${OPEN_MS} ms)`);
    // The process rows are the half the return must not lose; they have
    // to have arrived once for claim 2 to mean anything.
    const topIn = await within(15_000, async () => (await count("[data-machine-card] [data-top-row]")) > 0);
    if (topIn === null) fail("probe dead: this machine's card never drew a process row, so claim 2 has nothing to keep");

    // ── 2. leave, come back: the first frame ─────────────────────────
    await rail("sessions").click();
    const gone = await within(5_000, async () => (await count("[data-machine-card]")) === 0);
    if (gone === null) fail("probe void: the machine card never left, so coming back proves nothing");
    const first = await page.evaluate(
      () =>
        new Promise((done) => {
          document.querySelector('[data-rail-item][data-landing="devices"]').click();
          requestAnimationFrame(() =>
            done({
              card: document.querySelectorAll("[data-machine-card]").length,
              vitals: document.querySelectorAll("[data-machine-card] [data-vitals]").length,
              top: document.querySelectorAll("[data-machine-card] [data-top-row]").length,
            }),
          );
        }),
    );
    if (first.card === 0) fail("coming back to the machine page, the first frame had no machine card");
    else if (first.vitals === 0) fail("coming back to the machine page, the first frame's card had no readings");
    else if (first.top === 0) {
      fail("coming back to the machine page, the first frame's card had no process rows — it asked again and drew nothing meanwhile");
    }
    console.log(`fast: row ${rowIn} ms, card ${cardIn} ms; return frame ${JSON.stringify(first)}`);
  }

  if (thrown.length) fail(`the page threw: ${thrown.join(" | ")}`);
} catch (e) {
  fail(String(e?.stack ?? e));
} finally {
  if (browser) await browser.close();
  for (const p of [vite, bridge]) if (p && !p.killed) p.kill("SIGTERM");
  await new Promise((r) => setTimeout(r, 300));
  for (const p of [vite, bridge]) if (p && p.exitCode === null) p.kill("SIGKILL");
  rmSync(home, { recursive: true, force: true });
  await usage.close();
}

if (problems.length) {
  for (const p of problems) console.error(`✗ ${p}`);
  process.exit(1);
}
console.log("✓ fast: the machine page is up within a second over a sleeping disk, and comes back with what it had");
