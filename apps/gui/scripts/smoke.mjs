// Real-connection acceptance for the GUI scaffold. No mocks anywhere:
// two homes on real UDP, the dev bridge is the real backend behind an
// HTTP skin, the browser is the system Chrome. What is asserted:
//   1. a session living on alpha shows up in beta's GUI, with source;
//   2. the word in the GUI equals the word the CLI prints (two faces,
//      one backend);
//   3. clicking the row is the seen semantics: the unread badge clears
//      AND alpha's own list turns idle — the loop closes cross-device;
//   4. faces: rows paint a real derived SVG (not a placeholder), one
//      machine is the same picture in two places on one screen, two
//      machines are two pictures, and flipping the theme moves nothing
//      inside the SVG;
//   5. the back button exists only on the narrow face (after proving
//      the detail header renders at all — negative assertions must
//      first prove the probe is alive);
//   6. zero pageerror throughout.
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
  // detached = own process group, so cleanup can kill the whole tree —
  // killing an npx wrapper alone orphans the real server underneath.
  const c = spawn(cmd, args, { env, stdio: ["ignore", "pipe", "pipe"], detached: true });
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
  const guiWord = (await row.locator("[data-word-text]").innerText()).trim();
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
    (await row.getAttribute("data-word")) === "done" && (await row.locator("[data-unread]").count()) === 1,
  );
  await row.click();
  await until("beta's row to settle idle, badge gone", 30_000, async () =>
    (await row.getAttribute("data-word")) === "idle" && (await row.locator("[data-unread]").count()) === 0,
  );
  // The loop's far end: alpha's own CLI now prints the same idle word
  // the GUI shows — the watermark travelled, and the wording matches.
  const idleGuiWord = (await row.locator("[data-word-text]").innerText()).trim();
  await until("alpha to read the seen watermark", 30_000, () => {
    const line = cli(envA, "sessions").split("\n").find((l) => l.includes("tui/cafe1"));
    return Boolean(line && line.includes(idleGuiWord));
  });

  // 4) faces. Everything below is still on the wide viewport.
  //
  //    a. every session row paints a real derived face. Stated
  //    positively on purpose: the blank branch renders no <svg> at all,
  //    so "there is an svg in every row" *is* "no row fell back to a
  //    placeholder", with no negative selector to spell wrong.
  const rowCount = await page.locator("[data-word]").count();
  const facedRows = await page.locator("[data-word] [data-face] svg").count();
  if (rowCount === 0 || facedRows !== rowCount) {
    throw new Error(`rows with a painted face: ${facedRows} of ${rowCount}`);
  }
  //    …and what it painted is the derivation, not an empty canvas: the
  //    canvas side is one of the two the core ships, and the ground rect
  //    carries a hex color from the palette.
  const rowFace = page.locator("[data-word] [data-face] svg").first();
  const viewBox = await rowFace.getAttribute("viewBox");
  if (viewBox !== "0 0 80 80" && viewBox !== "0 0 36 36") {
    throw new Error(`a face's canvas is neither 80 nor 36: ${viewBox}`);
  }
  if (!/<rect[^>]*fill="#[0-9a-f]{6}"/.test(await rowFace.innerHTML())) {
    throw new Error("a face has no ground rect in a palette color");
  }

  //    b. one machine, two places on one screen, one picture. The blur
  //    filter's id is per-instance (a document-uniqueness device, not
  //    part of the face), so it is normalized away before comparing.
  const faceOf = async (locator) =>
    (await locator.innerHTML()).replace(/av-blur-[^"')\s]+/g, "BLUR");
  await page.locator('[data-rail-item][data-landing="devices"]').click();
  await until("the device list", 10_000, async () => (await page.locator("[data-device]").count()) >= 2);
  const railFace = page.locator('nav > [data-face]');
  const betaRow = page.locator('[data-device="beta"] [data-face]');
  if ((await railFace.count()) !== 1) throw new Error("probe dead: no face at the foot of the rail");
  if ((await betaRow.count()) !== 1) throw new Error("probe dead: no face on beta's own row");
  if ((await faceOf(railFace)) !== (await faceOf(betaRow))) {
    throw new Error("this machine has two different faces on one screen");
  }

  //    c. the control: two machines are not one picture. Without this,
  //    a painter that drew the same thing for everyone would pass (b).
  const alphaRow = page.locator('[data-device="alpha"] [data-face]');
  if ((await alphaRow.count()) !== 1) throw new Error("probe dead: no face on alpha's row");
  if ((await faceOf(alphaRow)) === (await faceOf(betaRow))) {
    throw new Error("two machines were painted the same face");
  }

  //    d. a theme flip moves nothing inside the SVG.
  //
  //    Two measurements, because they fail differently. The markup
  //    catches a painter that emitted different numbers or colors. The
  //    *computed* paint catches the subtler one: markup that is
  //    byte-identical because it says `var(--something)`, which then
  //    resolves per theme — a face that reads the theme without any
  //    string ever changing.
  //
  //    The probe has to be proven alive first, or "nothing changed" is
  //    also exactly what a theme switch that never took effect looks
  //    like. So two things that *should* follow the theme are measured
  //    in the same breath: the page's ground, and the face's hairline
  //    edge — the one documented theme-aware part, which lives outside
  //    the SVG.
  const themeProbe = async () =>
    page.evaluate(() => {
      const face = document.querySelector('[data-device="beta"] [data-face]');
      const edge = face && face.querySelector("span");
      const svg = face && face.querySelector("svg");
      return {
        body: getComputedStyle(document.body).backgroundColor,
        edge: edge && getComputedStyle(edge).borderTopColor,
        // Every painted element's resolved paint, in document order
        paint:
          svg &&
          [...svg.querySelectorAll("*")]
            .map((el) => {
              const s = getComputedStyle(el);
              return [el.tagName, s.fill, s.stroke, s.mixBlendMode, s.filter].join("|");
            })
            .join("\n"),
      };
    });
  await page.emulateMedia({ colorScheme: "light" });
  const lightFace = await faceOf(betaRow);
  const light = await themeProbe();
  await page.emulateMedia({ colorScheme: "dark" });
  const darkFace = await faceOf(betaRow);
  const dark = await themeProbe();
  if (light.body === dark.body) throw new Error("probe dead: the theme flip changed no ground color");
  if (!light.edge || light.edge === dark.edge) {
    throw new Error(`probe dead: the avatar edge did not follow the theme (${light.edge} / ${dark.edge})`);
  }
  if (!light.paint) throw new Error("probe dead: read no computed paint off the face");
  if (lightFace !== darkFace) throw new Error("the face's markup changed when the theme did");
  if (light.paint !== dark.paint) throw new Error("the face's computed paint changed when the theme did");
  await page.emulateMedia({ colorScheme: null });
  await page.locator('[data-rail-item][data-landing="sessions"]').click();
  await until("back on the session list", 10_000, async () => (await page.locator("[data-word]").count()) > 0);

  // 5) faces of the shell: wide has a detail header but no back;
  //    narrow, after entering a detail, has the back button.
  await row.click();
  if ((await page.locator("[data-detail-header]").count()) !== 1) throw new Error("probe dead: no detail header");
  if ((await page.locator("[data-back]").count()) !== 0) throw new Error("back button on the wide face");
  // Shrinking mid-detail keeps the detail up (Telegram's behavior) —
  // and there, with the list genuinely off-screen, back exists.
  await page.setViewportSize({ width: 390, height: 720 });
  await until("narrow detail with back", 10_000, async () => (await page.locator("[data-back]").count()) === 1);
  await page.locator("[data-back]").click();
  await until("back to the narrow list", 10_000, async () => (await page.locator("[data-list]").count()) === 1);
  await until("rows on the narrow list", 10_000, async () => (await page.locator("[data-word]").count()) > 0);

  // 6) the page never threw.
  if (pageErrors.length) throw new Error(`pageerror: ${pageErrors.join(" | ")}`);

  if (process.env.SMOKE_SHOT) {
    await page.setViewportSize({ width: 1080, height: 720 });
    // The rail clicks above leave the pointer on a glyph, and the rail
    // labels appear on hover — park it off the rail so the shot shows
    // the resting screen rather than one mid-hover.
    await page.mouse.move(900, 600);
    await new Promise((r) => setTimeout(r, 600));
    await page.screenshot({ path: process.env.SMOKE_SHOT });
  }

  console.log("gui smoke: all green");
} finally {
  if (browser) await browser.close().catch(() => {});
  for (const c of children) {
    try {
      process.kill(-c.pid, "SIGTERM");
    } catch {}
  }
  await new Promise((r) => setTimeout(r, 500));
  for (const c of children) {
    try {
      process.kill(-c.pid, "SIGKILL");
    } catch {}
  }
}
