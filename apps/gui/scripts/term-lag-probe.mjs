// Typing lag, measured on the web face (audit/TERM-LAG-BRIEF.md step 1).
// **A hand tool, not a gate**: it is in no `npm test` and opens no window
// (headless Chrome against a scratch `khor serve`).
//
//   KHOR_BIN=<target>/debug/khor node scripts/term-lag-probe.mjs [rows]
//
// It stands up a scratch node (own KHOR_HOME, no mDNS, own port), opens
// a `cat` session and `rows - 1` more idle ones (the list's and the day
// strips' reads grow with the rows), opens the `cat` terminal in the
// page and types `KEYS` letters every `EVERY` ms (`term-lag-page.mjs`),
// then again with a second face on the same session. Prints p50 / p95 /
// max key → echo and every echo over 150 ms with the page's long frames.
//
// Env: KHOR_BIN (default target/debug/khor), WEB_PORT (default 5469),
// KEYS (200), EVERY (50), SCRATCH (a directory for the copy and the home;
// default the system temp directory).
import { spawn, execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, readFileSync, existsSync, copyFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";
import { lagProbe } from "./term-lag-page.mjs";

const gui = fileURLToPath(new URL("..", import.meta.url));
const repo = join(gui, "../..");
const PORT = Number(process.env.WEB_PORT ?? 5469);
const ROWS = Number(process.argv[2] ?? 1);
const KEYS = Number(process.env.KEYS ?? 200);
const EVERY = Number(process.env.EVERY ?? 50);
const scratch = mkdtempSync(join(process.env.SCRATCH ?? tmpdir(), "khor-termlag-"));
const KHOR = join(scratch, "khor");
copyFileSync(process.env.KHOR_BIN ?? join(repo, "target/debug/khor"), KHOR);
const HOME = join(scratch, "home");
const env = { ...process.env, KHOR_HOME: HOME, KHOR_NAME: "termlag", KHOR_NO_MDNS: "1", KHOR_WEB_PORT: String(PORT), TMUX_TMPDIR: scratch };
delete env.KHOR_SESSION;
delete env.TMUX;

const children = [];
let browser;

async function until(what, ms, f) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    const v = await f();
    if (v) return v;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`timed out waiting for ${what}`);
}

async function openTerminal(context, key) {
  const page = await context.newPage();
  await page.goto(`http://127.0.0.1:${PORT}/?k=${key}`);
  const row = await until("the cat row", 30_000, async () => {
    const r = page.locator("[data-row]").filter({ hasText: "lagcat" });
    return (await r.count()) > 0 ? r.first() : null;
  });
  await row.click();
  await until("the terminal pane", 30_000, async () =>
    (await page.evaluate(() => Boolean(document.querySelector("[data-terminal]")?.__xterm))) ? true : null,
  );
  // Let the join's replay land before timing anything.
  await new Promise((r) => setTimeout(r, 2000));
  return page;
}

function report(arm, r) {
  if (r.error) return console.log(`${arm}: ${r.error}`);
  const f = (v) => (v === null ? "-" : `${Math.round(v)} ms`);
  console.log(`${arm}: ${r.echoed}/${r.keys} echoed, ${r.lost} lost · p50 ${f(r.p50)} · p95 ${f(r.p95)} · max ${f(r.max)}`);
  if (r.stalls.length) console.log(`  stalls > 150 ms (at, ms): ${r.stalls.map((s) => `${s.at}:${s.ms}`).join(" ")}`);
  const busy = [...r.longtasks, ...r.frameGaps].filter((g) => g.ms > 100);
  if (busy.length) console.log(`  page busy > 100 ms (at, ms): ${busy.map((g) => `${g.at}:${g.ms}`).join(" ")}`);
}

async function main() {
  const serve = spawn(KHOR, ["serve"], { env, stdio: ["ignore", "pipe", "pipe"], detached: true });
  children.push(serve);
  let log = "";
  serve.stdout.on("data", (d) => (log += d));
  serve.stderr.on("data", (d) => (log += d));
  await until("the face to bind", 30_000, () => log.includes(`:${PORT}`));
  const keyFile = join(HOME, ".khor", "web.key");
  const key = await until("a key on disk", 10_000, () => (existsSync(keyFile) ? readFileSync(keyFile, "utf8").trim() : null));

  execFileSync(KHOR, ["open", "-d", "--title", "lagcat", "--", "cat"], { env });
  for (let i = 1; i < ROWS; i += 1) execFileSync(KHOR, ["open", "-d", "--title", `idle-${i}`, "--", "sleep", "86400"], { env });

  browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server"] });
  const one = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await openTerminal(one, key);
  report(`web face, ${ROWS} rows, one face`, await page.evaluate(lagProbe, { keys: KEYS, every: EVERY }));

  const two = await browser.newContext({ viewport: { width: 390, height: 844 } });
  await openTerminal(two, key);
  report(`web face, ${ROWS} rows, second face attached`, await page.evaluate(lagProbe, { keys: KEYS, every: EVERY }));
}

async function cleanup() {
  if (browser) await browser.close().catch(() => {});
  try {
    execFileSync(KHOR, ["quit"], { env, stdio: "ignore", timeout: 20_000 });
  } catch {
    /* nothing to quit */
  }
  for (const c of children) {
    try {
      process.kill(-c.pid, "SIGKILL");
    } catch {
      /* already gone */
    }
  }
  rmSync(scratch, { recursive: true, force: true });
}

main()
  .catch((e) => {
    console.error(e);
    process.exitCode = 1;
  })
  .finally(cleanup);
