// Typing lag, measured in the Mac app (audit/TERM-LAG-BRIEF.md step 1).
// **A hand tool, never a gate**: it raises a window, so it refuses to
// start unless the Mac has been idle for two minutes, and it is in no
// `npm test`.
//
// It needs a scratch build whose page carries an eval hub (a loop in
// `main.tsx` that fetches script from 127.0.0.1:1477/next and posts the
// answer to /done — written into the scratch build only, never
// committed), and it runs that build under a name of its own so its
// WebKit store is not the owner's (`~/Library/WebKit/<name>`).
//
//   KHOR_GUI_BIN=<target>/debug/khor-gui KHOR_BIN=<target>/debug/khor \
//     node scripts/term-lag-app.mjs [rows] [arm…]
//
// Arms: `quiet` (the terminal alone), `sample` (the same, with a `sample`
// of the app over the run, its main thread summarised). Env: KEYS (300),
// EVERY (50), SCRATCH (where the copy and the home go).
import { spawn, execFileSync } from "node:child_process";
import { createServer } from "node:http";
import { mkdtempSync, rmSync, copyFileSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { homedir, tmpdir } from "node:os";
import { lagProbe } from "./term-lag-page.mjs";

const NAME = "khor-gui-lag";
const ROWS = Number(process.argv[2] ?? 1);
const ARMS = process.argv.slice(3).length ? process.argv.slice(3) : ["quiet"];
const KEYS = Number(process.env.KEYS ?? 300);
const EVERY = Number(process.env.EVERY ?? 50);

const idle = () => {
  const out = execFileSync("ioreg", ["-c", "IOHIDSystem"], { encoding: "utf8" });
  const m = out.match(/"HIDIdleTime" = (\d+)/);
  return m ? Number(m[1]) / 1e9 : 0;
};
if (idle() < 120) {
  console.error(`the Mac is in use (idle ${Math.round(idle())} s); this raises a window, so it waits for 120 s idle`);
  process.exit(2);
}

const scratch = mkdtempSync(join(process.env.SCRATCH ?? tmpdir(), "khor-termlag-app-"));
const GUI = join(scratch, NAME);
const KHOR = join(scratch, "khor");
copyFileSync(process.env.KHOR_GUI_BIN, GUI);
copyFileSync(process.env.KHOR_BIN, KHOR);
const HOME = join(scratch, "home");
const env = { ...process.env, KHOR_HOME: HOME, KHOR_NAME: "termlag", KHOR_NO_MDNS: "1", TMUX_TMPDIR: scratch, KHOR_SLOW_CALLS: process.env.KHOR_SLOW_CALLS ?? "20" };
delete env.KHOR_SESSION;
delete env.TMUX;

// The hub: one script out at a time, its answer back.
let waiting = null;
let pending = null;
let answer = null;
const hub = createServer((req, res) => {
  res.setHeader("access-control-allow-origin", "*");
  if (req.url === "/next") {
    if (pending) {
      res.end(pending);
      pending = null;
    } else waiting = res;
    setTimeout(() => {
      if (waiting === res) {
        waiting = null;
        res.end("");
      }
    }, 15_000);
  } else if (req.url === "/done") {
    let body = "";
    req.on("data", (d) => (body += d));
    req.on("end", () => {
      res.end("");
      answer?.(JSON.parse(body));
    });
  } else res.end("");
});
await new Promise((r) => hub.listen(1477, "127.0.0.1", r));
const evalInPage = (code, ms = 120_000) =>
  new Promise((resolve, reject) => {
    const t = setTimeout(() => reject(new Error(`no answer to ${code.slice(0, 60)}`)), ms);
    answer = (v) => {
      clearTimeout(t);
      resolve(v);
    };
    if (waiting) {
      waiting.end(code);
      waiting = null;
    } else pending = code;
  });

let app;
const slow = [];
async function until(what, ms, f) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    const v = await f().catch(() => null);
    if (v) return v;
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`timed out waiting for ${what}`);
}

function report(arm, r) {
  if (r.error) return console.log(`${arm}: ${r.error}`);
  const span = r.keys * r.every + 2000;
  const calls = slow.filter((c) => c.at >= r.beganAt - 2000 && c.at <= r.beganAt + span);
  const byCommand = new Map();
  for (const c of calls) {
    const was = byCommand.get(`${c.command} on ${c.thread}`) ?? { n: 0, ms: 0, most: 0 };
    byCommand.set(`${c.command} on ${c.thread}`, { n: was.n + 1, ms: was.ms + c.ms, most: Math.max(was.most, c.ms) });
  }
  const f = (v) => (v === null ? "-" : `${Math.round(v)} ms`);
  console.log(`${arm}: ${r.echoed}/${r.keys} echoed, ${r.lost} lost · p50 ${f(r.p50)} · p95 ${f(r.p95)} · max ${f(r.max)}`);
  if (r.stalls.length) console.log(`  stalls > 150 ms (at, ms): ${r.stalls.map((s) => `${s.at}:${s.ms}`).join(" ")}`);
  const busy = [...r.longtasks, ...r.frameGaps].filter((g) => g.ms > 100);
  if (busy.length) console.log(`  page busy > 100 ms (at, ms): ${busy.map((g) => `${g.at}:${g.ms}`).join(" ")}`);
  for (const [k, v] of byCommand) console.log(`  slow calls: ${k} ×${v.n}, ${v.ms} ms in all, longest ${v.most} ms`);
  const long = calls.filter((c) => c.ms >= 50);
  if (long.length) console.log(`  calls ≥ 50 ms (at, command, ms): ${long.map((c) => `${c.at - r.beganAt}:${c.command}:${c.ms}`).join(" ")}`);
}

/** The app's main thread over a `sample`: the heaviest frames under
    `Thread_…  DispatchQueue_1: com.apple.main-thread`, by sample count. */
function mainThread(file) {
  const text = readFileSync(file, "utf8");
  const start = text.indexOf("com.apple.main-thread");
  if (start < 0) return "no main thread in the sample";
  const rest = text.slice(start);
  const end = rest.search(/\n {4}\d+ Thread_/);
  const block = end > 0 ? rest.slice(0, end) : rest;
  const hits = new Map();
  for (const line of block.split("\n")) {
    const m = line.match(/^\s*[+!:| ]*(\d+) (\S*(?:khor|term_|day_strips|sessions|invoke|tauri)\S*)/);
    if (m) hits.set(m[2], Math.max(hits.get(m[2]) ?? 0, Number(m[1])));
  }
  return [...hits.entries()].sort((a, b) => b[1] - a[1]).slice(0, 25).map(([f, n]) => `    ${n} ms  ${f}`).join("\n");
}

async function main() {
  execFileSync(KHOR, ["open", "-d", "--title", "lagcat", "--", "cat"], { env });
  for (let i = 1; i < ROWS; i += 1) execFileSync(KHOR, ["open", "-d", "--title", `idle-${i}`, "--", "sleep", "86400"], { env });
  app = spawn(GUI, [], { env, stdio: ["ignore", "ignore", "pipe"] });
  // The app's `KHOR_SLOW_CALLS` lines, stamped as they arrive: a call
  // began its own length before its line.
  let rest = "";
  app.stderr.on("data", (d) => {
    const lines = (rest + d).split("\n");
    rest = lines.pop();
    for (const line of lines) {
      const m = line.match(/slow call: (\S+) held (\S+) for (\d+) ms/);
      if (m) slow.push({ at: Date.now() - Number(m[3]), command: m[1], thread: m[2], ms: Number(m[3]) });
    }
  });
  await until("the page's hub", 60_000, () => evalInPage("1", 15_000));
  await until("the cat row", 60_000, () =>
    evalInPage(`(() => { const r = [...document.querySelectorAll("[data-row]")].find((e) => e.textContent.includes("lagcat")); if (!r) return false; (r.querySelector("[data-row-open]") ?? r).click(); return true; })()`),
  );
  await until("the terminal pane", 60_000, () => evalInPage(`Boolean(document.querySelector("[data-terminal]")?.__xterm)`));
  await new Promise((r) => setTimeout(r, 3000));
  const probe = `(${lagProbe.toString()})({ keys: ${KEYS}, every: ${EVERY} })`;
  for (const arm of ARMS) {
    if (arm === "sample") {
      const file = join(scratch, "sample.txt");
      const seconds = Math.ceil((KEYS * EVERY) / 1000) + 1;
      const s = spawn("sample", [String(app.pid), String(seconds), "-file", file], { stdio: "ignore" });
      const r = await evalInPage(probe, 300_000);
      await new Promise((res) => s.on("exit", res));
      report(`app, ${ROWS} rows, sampled`, r);
      console.log(`  main thread, heaviest frames over ${seconds} s:\n${mainThread(file)}`);
      copyFileSync(file, join(process.env.SAMPLE_OUT ?? scratch, `term-lag-sample-${ROWS}.txt`));
    } else report(`app, ${ROWS} rows, ${arm}`, await evalInPage(probe, 300_000));
  }
}

function cleanup() {
  hub.close();
  if (app) {
    try {
      process.kill(app.pid, "SIGTERM");
    } catch {
      /* gone */
    }
  }
  try {
    execFileSync(KHOR, ["quit"], { env, stdio: "ignore", timeout: 20_000 });
  } catch {
    /* nothing to quit */
  }
  for (const dir of [join(homedir(), "Library", "WebKit", NAME), join(homedir(), "Library", "Caches", NAME)]) {
    rmSync(dir, { recursive: true, force: true });
  }
  if (!process.env.KEEP) rmSync(scratch, { recursive: true, force: true });
}

main()
  .catch((e) => {
    console.error(e);
    process.exitCode = 1;
  })
  .finally(() => {
    cleanup();
    process.exit();
  });
