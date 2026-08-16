// Real-connection acceptance for the GUI scaffold. No mocks anywhere:
// two homes on real UDP, the dev bridge is the real backend behind an
// HTTP skin, the browser is the system Chrome. What is asserted:
//   1. a session living on alpha shows up in beta's GUI, with source;
//   2. the word in the GUI equals the word the CLI prints (two faces,
//      one backend);
//   3. clicking the row is the seen semantics: the unread badge clears
//      AND alpha's own list turns idle — the loop closes cross-device;
//   4. the back button exists only on the narrow face (after proving
//      the detail header renders at all — negative assertions must
//      first prove the probe is alive);
//   5. zero pageerror throughout.
// Every wait has a deadline; cleanup runs in finally and kills by pid.
import { execFileSync, spawn } from "node:child_process";
import { mkdirSync, rmSync, existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const repo = join(gui, "../..");
const KHOR = join(repo, "target/debug/khor");
const BRIDGE = join(repo, "target/debug/bridge");
const SCRATCH = process.env.SMOKE_DIR ?? join(repo, "target/gui-smoke");
const A = join(SCRATCH, "alpha");
const B = join(SCRATCH, "beta");
const BRIDGE_PORT = 1431;
const VITE_PORT = 1430;

const envA = { ...process.env, KHOR_HOME: A, KHOR_NAME: "alpha" };
const envB = { ...process.env, KHOR_HOME: B, KHOR_NAME: "beta" };
delete envA.KHOR_SESSION;
delete envB.KHOR_SESSION;

const children = [];
function run(cmd, args, env, name) {
  const c = spawn(cmd, args, { env, stdio: ["ignore", "pipe", "pipe"] });
  c.stderr.on("data", (d) => process.stderr.write(`[${name}] ${d}`));
  children.push(c);
  return c;
}
function cli(env, ...args) {
  return execFileSync(KHOR, args, { env, encoding: "utf8", timeout: 30_000 });
}
function feedHook(env, event, extra = "") {
  const payload = `{"session_id":"cafe1","cwd":"/tmp/proj","hook_event_name":"${event}"${extra}}`;
  execFileSync(KHOR, ["state", "--hook"], { env, input: payload, timeout: 15_000 });
}
async function until(what, ms, f) {
  const deadline = Date.now() + ms;
  let last;
  while (Date.now() < deadline) {
    try {
      last = await f();
      if (last) return last;
    } catch (e) {
      last = e;
    }
    await new Promise((r) => setTimeout(r, 400));
  }
  throw new Error(`timed out waiting for: ${what} (last: ${last})`);
}

let browser;
try {
  rmSync(SCRATCH, { recursive: true, force: true });
  mkdirSync(A, { recursive: true });
  mkdirSync(B, { recursive: true });

  // alpha: serve + a hooked agent session (register → busy).
  run(KHOR, ["serve"], envA, "serve-a");
  await until("alpha endpoint.json", 15_000, () => existsSync(join(A, ".khor/endpoint.json")));
  const ticket = cli(envA, "invite").trim().split("\n").pop().trim();
  cli(envB, "pair", ticket);
  feedHook(envA, "SessionStart");
  feedHook(envA, "UserPromptSubmit");
  if (!cli(envA, "sessions").includes("tui/cafe1")) throw new Error("alpha row missing");

  // beta: the bridge is the app backend — embedded serve pumps sync.
  run(BRIDGE, [], { ...envB, BRIDGE_PORT: String(BRIDGE_PORT) }, "bridge");
  const vite = run("npx", ["vite", "--port", String(VITE_PORT), "--strictPort"], { ...process.env, PATH: process.env.PATH }, "vite");
  vite.stdout.on("data", () => {});
  await until("vite up", 30_000, async () => {
    const r = await fetch(`http://localhost:${VITE_PORT}/`).catch(() => null);
    return r?.ok;
  });

  browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server"] });
  const page = await browser.newPage({ viewport: { width: 1080, height: 720 } });
  const pageErrors = [];
  page.on("pageerror", (e) => pageErrors.push(String(e)));
  await page.goto(`http://localhost:${VITE_PORT}/?bridge=${BRIDGE_PORT}`);

  // 1) the reported row reaches beta's GUI, busy, with its source.
  const row = page.locator('[data-word]', { hasText: "proj" });
  await until("the alpha row in beta's GUI", 30_000, async () => (await row.count()) === 1);
  if ((await row.getAttribute("data-word")) !== "busy") throw new Error("row should be busy");
  const guiWord = (await row.locator(".word").innerText()).trim();
  if (!(await row.innerText()).includes("alpha")) throw new Error("reported row must show its source");

  // 2) two faces, one wording: the CLI line for the same row carries
  //    the same display word. No Chinese literal in this script — the
  //    catalog owns the text, the comparison owns the check.
  const cliLine = cli(envB, "sessions").split("\n").find((l) => l.includes("tui/cafe1"));
  if (!cliLine || !cliLine.includes(guiWord)) {
    throw new Error(`CLI and GUI disagree on the word: ${guiWord} vs ${cliLine}`);
  }

  // 3) turn ends on alpha → done + unread on beta; clicking the row is
  //    seen; the loop closes: badge clears here, alpha turns idle there.
  feedHook(envA, "Stop");
  await until("done + unread badge", 30_000, async () =>
    (await row.getAttribute("data-word")) === "done" && (await row.locator(".unread").count()) === 1,
  );
  await row.click();
  await until("beta's row to settle idle, badge gone", 30_000, async () =>
    (await row.getAttribute("data-word")) === "idle" && (await row.locator(".unread").count()) === 0,
  );
  // The loop's far end: alpha's own CLI now prints the same idle word
  // the GUI shows — the watermark travelled, and the wording matches.
  const idleGuiWord = (await row.locator(".word").innerText()).trim();
  await until("alpha to read the seen watermark", 30_000, () => {
    const line = cli(envA, "sessions").split("\n").find((l) => l.includes("tui/cafe1"));
    return Boolean(line && line.includes(idleGuiWord));
  });

  // 4) faces: wide has a detail header but no back; narrow, after
  //    entering a detail, has the back button.
  if ((await page.locator(".detail-header").count()) !== 1) throw new Error("probe dead: no detail header");
  if ((await page.locator(".back-btn").count()) !== 0) throw new Error("back button on the wide face");
  // Shrinking mid-detail keeps the detail up (Telegram's behavior) —
  // and there, with the list genuinely off-screen, back exists.
  await page.setViewportSize({ width: 390, height: 720 });
  await until("narrow detail with back", 10_000, async () => (await page.locator(".back-btn").count()) === 1);
  await page.locator(".back-btn").click();
  await until("back to the narrow list", 10_000, async () => (await page.locator(".list").count()) === 1);
  await until("rows on the narrow list", 10_000, async () => (await page.locator("[data-word]").count()) > 0);

  // 5) the page never threw.
  if (pageErrors.length) throw new Error(`pageerror: ${pageErrors.join(" | ")}`);

  console.log("gui smoke: all green");
} finally {
  if (browser) await browser.close().catch(() => {});
  for (const c of children) {
    try {
      c.kill("SIGTERM");
    } catch {}
  }
  await new Promise((r) => setTimeout(r, 500));
  for (const c of children) {
    try {
      c.kill("SIGKILL");
    } catch {}
  }
}
