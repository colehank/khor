// The WebKit arm of the shim gate: **what a real engine does with
// `page_shim.js`**, which `check-shim.mjs` structurally cannot answer.
//
// `check-shim.mjs` runs the shim in a `vm` realm furnished by this repo
// and answers 「the rules are written correctly」. Two questions are
// WebKit's to answer, not the shim's, and getting either one wrong makes
// the whole shim a file that does nothing:
//
//   1. does `delete window.WebTransport` take, on an engine where these
//      globals are real interface objects rather than plain properties;
//   2. which realms a `forMainFrameOnly: false` user script reaches.
//
// It loads a page in a bare `WKWebView` and again in one carrying the
// shim, and reads five realms in each. **The bare arm is the point.**
// Without it, 「WebTransport is gone」 and 「this WebKit never shipped
// it」 produce the same green — the shape docs/… calls 一个不会失败的
// 对照组.
//
// # It is not khor's webview
//
// Same engine, same `WKUserScript(forMainFrameOnly: false)`, same shim
// text — but built here rather than by tauri. What it therefore cannot
// answer: whether tauri hands the script to a page webview at all. That
// path is source-checked in `check-shim.mjs` and measured nowhere.
//
// # What it found, and why two readings are printed rather than asserted
//
// A `Worker` global gets no user script, and a scripted `srcdoc` iframe
// read **before its load event** is still the pre-navigation document,
// which gets none either. Both are real escapes, both are printed and
// **neither is asserted**: asserting a hole stays open turns a WebKit
// fix into a red, and a gate that goes red on good news is a gate people
// switch off.
//
// Run: `npm run wk` — macOS only, needs `swift` and a GUI session (a
// `WKWebView` wants a window server). Elsewhere it says so and stops.
import { execFileSync, spawn } from "node:child_process";
import { createServer } from "node:http";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join, dirname } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const SHIM = join(here, "../core/src/page_shim.js");
const PROBE = join(here, "wk-probe");

let bad = 0;
const fail = (m) => {
  console.error(m);
  bad += 1;
};

if (process.platform !== "darwin") {
  console.log(`wk gate: skipped — this measures WebKit and the platform is ${process.platform}. Nothing here is claimed on it.`);
  process.exit(0);
}
try {
  execFileSync("which", ["swift"], { stdio: "ignore" });
} catch {
  console.error("wk gate: `swift` is not on PATH — this gate compiles a WKWebView probe and cannot run without it. Not a pass.");
  process.exit(1);
}

// The probe pages are served over http on purpose: a borrowed page is an
// http origin, and user script injection is not obliged to behave the
// same for `file:` or `data:`.
const pages = {
  "/page.html": readFileSync(join(PROBE, "page.html")),
  "/kid.html": readFileSync(join(PROBE, "kid.html")),
  "/pdf.html": readFileSync(join(PROBE, "pdf.html")),
  "/sheet.html": readFileSync(join(PROBE, "sheet.html")),
};
const server = createServer((req, res) => {
  const body = pages[req.url];
  if (!body) {
    res.writeHead(404).end();
    return;
  }
  res.writeHead(200, { "content-type": "text/html; charset=utf-8" }).end(body);
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const port = server.address().port;

// The code face's two typographic tokens, read out of tokens.css so the
// sheet arm measures khor's leading and not a number of its own. A
// token that cannot be found is a failure, not a default: the arm would
// otherwise pass on whatever it happened to be handed.
const TOKENS = readFileSync(join(here, "..", "src", "styles", "tokens.css"), "utf8");
const token = (name) => {
  const m = TOKENS.match(new RegExp(`^\\s*${name.replace(/[-]/g, "\\-")}:\\s*([^;]+);`, "m"));
  if (!m) {
    console.error(`wk gate: ${name} is not in tokens.css — the sheet arm has nothing to measure`);
    process.exit(1);
  }
  return m[1].trim();
};
const codeSize = token("--text-code");
const codeLeading = token("--text-code--line-height");

// **Spawned, not `spawnSync`.** The probe fetches its pages from the
// server above, and a synchronous spawn blocks this process's event
// loop — the server would never answer, and the probe would report a
// page that never loaded. That is not a hypothetical: it is what the
// first run of this gate said, and it read exactly like a broken shim.
let raw;
try {
  const ran = await new Promise((done) => {
    const child = spawn("swift", [join(PROBE, "probe.swift"), SHIM, String(port), codeSize, codeLeading], {
      stdio: ["ignore", "pipe", "pipe"],
    });
    let out = "";
    let err = "";
    child.stdout.on("data", (d) => (out += d));
    child.stderr.on("data", (d) => (err += d));
    const bell = setTimeout(() => child.kill("SIGKILL"), 180_000);
    child.on("close", (status) => {
      clearTimeout(bell);
      done({ status, out, err });
    });
  });
  if (ran.status !== 0) {
    fail(`wk probe exited ${ran.status}: ${ran.err.trim().split("\n").slice(-6).join(" / ") || "(said nothing)"}`);
  }
  raw = ran.out.trim().split("\n").filter((l) => l.startsWith("{")).pop();
} finally {
  server.close();
}

if (!raw) {
  fail("wk probe printed no JSON — every assertion below would pass by reading nothing");
  process.exit(1);
}
const said = JSON.parse(raw);

/** The five realms both arms read, and what each one is. */
const REALMS = [
  ["top", "the page itself"],
  ["blankParsed", "an `about:blank` iframe the parser made"],
  ["kidParsed", "a real http child frame"],
  ["blankScripted", "an empty iframe a script made after load"],
];

// ── The bare arm, before the shimmed one means anything ──────────────
if (said.bare?.loadFailed) {
  fail(`wk probe could not load the page without the shim (${said.bare.loadFailed}) — nothing below was measured`);
} else {
  for (const [key, what] of REALMS) {
    if (said.bare?.[key] !== "function / function") {
      fail(
        `probe dead: without the shim, ${what} answered \`${said.bare?.[key]}\` rather than \`function / function\` — ` +
          `this WebKit does not have both globals there, so the shim removing them below would prove nothing`,
      );
    }
  }
}

// ── The shim, in the same five realms ────────────────────────────────
if (said.shimmed?.loadFailed) {
  fail(`wk probe could not load the page with the shim (${said.shimmed.loadFailed})`);
} else {
  for (const [key, what] of REALMS) {
    if (said.shimmed?.[key] !== "undefined / undefined") {
      fail(
        `${what} still has \`${said.shimmed?.[key]}\` (WebTransport / PublicKeyCredential) with the shim installed — ` +
          `a borrowed page reaching either of those leaves through this device's own network while the page came in through another machine`,
      );
    }
  }
  const after = said.shimmed?.srcdocAfterLoad ?? "";
  if (!after.includes('body="hi"')) {
    fail(`the srcdoc arm never reached the srcdoc document (${after}) — its reading below is about some other document`);
  } else if (!after.endsWith("undefined / undefined")) {
    fail(`a scripted srcdoc iframe still has ${after.split(", ").pop()} after its load event — the shim is not reaching a realm the page can simply make`);
  }
}

// ── The PDF arm (工作区批⑤) ──────────────────────────────────────────
//
// 「pdf 你咋转成字符了,应该直接预览」(the user, 2026-09-14). khor's file
// view hands the bytes to an `<iframe>` as a `blob:` and lets the engine
// draw them — and whether that engine *has* a PDF viewer is the one
// question deciding whether the face does that or falls back to the
// document's words (`khor_node::preview::PDF_HERE`, true on macOS/iOS
// WebKit and false on WebKitGTK).
//
// **The reading is the picture, because nothing else can answer.**
// WebKit draws a PDF with its own viewer and gives the page no document
// to query — `contentDocument` is null whether it rendered or not. So
// the probe snapshots the webview and counts dark pixels inside the
// frame's box, over a white page, with a PDF whose one page is a black
// square. An empty page and a page that never rendered are both white;
// only a drawn one is dark.
if (said.pdf?.loadFailed) {
  fail(`wk probe could not run the PDF arm (${said.pdf.loadFailed}) — nothing about PDFs was measured`);
} else {
  const [scheme, how, viewer] = String(said.pdf?.frame ?? "").split("|");
  console.log(`wk: pdf         frame=${said.pdf?.frame}, dark=${said.pdf?.dark}%`);
  if (scheme !== "blob:") {
    fail(
      `the PDF arm made a ${scheme} URL rather than a blob: — a data: URL is what this road was ` +
        "chosen against (length ceiling, and frames have blocked it before)",
    );
  }
  if (how !== "loaded") {
    fail(`a blob: PDF in an iframe never fired load on this WebKit (${how}) — khor's file view puts one there`);
  }
  if (viewer !== "true") {
    fail(
      `this WebKit says navigator.pdfViewerEnabled=${viewer} — khor hands it PDF bytes on the ` +
        "strength of that being true (`khor_node::preview::PDF_HERE`)",
    );
  }
  const dark = Number(said.pdf?.dark);
  if (!Number.isFinite(dark)) {
    fail(`the PDF arm could not read the picture (${said.pdf?.dark}) — 「it drew」 is unmeasured`);
  } else if (dark < 5) {
    fail(
      `the frame's box is ${dark}% dark over a PDF whose one page is a black square — the blob ` +
        "loaded and nothing was painted, which is the failure this arm exists for",
    );
  }
}

// ── The editor over the sheet: one leading, not two ─────────────────
// WebKit snaps a <textarea>'s rows to whole pixels and leaves a <pre>'s
// alone, so a leading of 21.6px is 21 on one layer and 21.6 on the
// other, and the caret walks off its letter one row in thirty. Chrome
// does not, which is why `check-face` never saw it (2026-09-18).
{
  const sh = said.sheet ?? {};
  if (sh.loadFailed) {
    fail(`the sheet page did not load: ${sh.loadFailed}`);
  } else if (sh.first) {
    fail(
      `on this WebKit the editor's rows stop matching the painted sheet at row ${sh.first.row} ` +
        `(editor ${sh.first.editH}px, sheet ${sh.first.sheetH}px, leading ${codeLeading}) — ` +
        "the caret sits on a neighbour's letter from there; a leading that is not a whole " +
        "number of pixels is two leadings here (tokens.css --text-code--line-height)",
    );
  } else if (sh.sheetH !== sh.editH) {
    fail(`sheet ${sh.sheetH}px against editor ${sh.editH}px over ${sh.rows} rows — the two layers do not agree`);
  } else {
    console.log(`wk: sheet       ${sh.rows} rows at ${sh.leading}: editor and sheet both ${sh.sheetH}px`);
  }
}

// ── Printed, not asserted: the two ways out that are still open ──────
console.log(`wk: bare        ${JSON.stringify(said.bare)}`);
console.log(`wk: shimmed     ${JSON.stringify(said.shimmed)}`);
// A field the probe never filled must not print like a reading: an
// absent `worker` renders as the string `undefined`, which is also a
// perfectly good answer to 「what is `typeof WebTransport` there」.
const reading = (v) => (v === undefined ? "未量 (the probe never got this far)" : v);
console.log(
  `wk: still open  worker=${reading(said.shimmed?.worker)} (a worker global gets no user script), ` +
    `srcdoc read before its load event=${reading(said.shimmed?.srcdocScripted?.split(" / ")[0])} (that read is the pre-navigation document)`,
);
if (!bad) {
  console.log(
    "wk gate: on this WebKit the delete takes, and the script reaches the page, both parsed frames, " +
      "a scripted blank frame and a srcdoc frame once loaded — a blob: PDF in an iframe really paints, " +
      "and the editor's rows sit on the sheet's",
  );
}
process.exitCode = bad ? 1 : 0;
