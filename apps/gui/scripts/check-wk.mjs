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
// It has since grown three arms that are not about the shim at all and
// are here for the same reason — **they are questions only a real
// WebKit answers**: whether a blob: PDF in an iframe paints, whether
// the editor's rows sit on the painted sheet, and (2026-09-20,
// docs/SHELL.md 第 1 条) whether a wheel that runs off the end of a
// pane moves the whole app. Chrome cannot be asked the last one: its
// desktop viewport does not rubber-band, so `check-face` measures the
// declarations and this measures what they do.
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
  "/shell.html": readFileSync(join(PROBE, "shell.html")),
  "/well.html": readFileSync(join(PROBE, "well.html")),
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

// ── the two blocks that nail the shell down (docs/SHELL.md 第 1 条) ──
//
// **Lifted verbatim, never retyped.** The shell arm's whole claim is
// that *khor's* rules stop the page moving on this engine, and a copy
// of them in the probe would be this gate agreeing with itself. A block
// that cannot be found is a failure rather than an empty string: the
// arm would otherwise run with no rules at all and report「the page did
// not move」about a page that was never given anything to hold it.
const APP = readFileSync(join(here, "..", "src", "styles", "app.css"), "utf8");
const block = (text, where, opener) => {
  const at = text.indexOf(opener);
  const shut = at === -1 ? -1 : text.indexOf("}", at);
  if (at === -1 || shut === -1) {
    console.error(
      `wk gate: ${where} no longer contains the rule starting \`${opener.split("\n")[0]}\` — ` +
        "the shell arm would measure a page with no rules on it and pass",
    );
    process.exit(1);
  }
  return text.slice(at, shut + 1);
};
const shellCss = [
  block(TOKENS, "tokens.css", "html,\n  body,\n  #root {"),
  block(APP, "app.css", ".overflow-auto,"),
  // 第三轮: the tab strip draws no scrollbar. Both rules, because one
  // engine reads the property and an older one only the pseudo-element.
  block(APP, "app.css", "[data-tab-strip] {"),
  block(APP, "app.css", "[data-tab-strip]::-webkit-scrollbar {"),
].join("\n");

// **Spawned, not `spawnSync`.** The probe fetches its pages from the
// server above, and a synchronous spawn blocks this process's event
// loop — the server would never answer, and the probe would report a
// page that never loaded. That is not a hypothetical: it is what the
// first run of this gate said, and it read exactly like a broken shim.
let raw;
try {
  const ran = await new Promise((done) => {
    const child = spawn("swift", [join(PROBE, "probe.swift"), SHIM, String(port), codeSize, codeLeading, shellCss], {
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

// ── The face editor's colour slot: the OS picker, not a text box ────
//
// 「点击单个配色展开系统的颜色设置」 (the user, 2026-09-20). The editor
// leans on this engine turning `<input type="color">` into macOS's own
// panel; if it degraded to a text field the five wells would silently
// become five places to type a hex string, and every gate that only
// counts elements would go on passing.
{
  const wl = said.well ?? {};
  if (wl.loadFailed) {
    fail(`the colour-well page did not load: ${wl.loadFailed}`);
  } else {
    if (wl.type !== "color") {
      fail(
        `on this WebKit a colour slot is an <input type=${JSON.stringify(wl.type)}> — ` +
          "the editor's five wells would be five text boxes (views/FaceSettings ColorWell)",
      );
    }
    // Laid out at the size khor gives it: a control the engine collapses
    // is one nobody can press, and the well is the one control in this
    // app whose target and its picture are the same thing.
    if (wl.w !== 44 || wl.h !== 44) {
      fail(
        `a colour slot lays out ${wl.w}×${wl.h} where khor asked for 44×44 — ` +
          "the well *is* the target (tokens.css --touch)",
      );
    }
    // **The half khor depends on**: what the panel hands back is what
    // gets written to the device table, so the value has to survive the
    // engine's own parser. Compared against the normalised form rather
    // than the literal, or an engine that stored the string without
    // understanding it would pass.
    if (wl.took !== "#0055aa" || wl.caseFolded !== "#0055aa") {
      fail(
        `a colour slot read back ${JSON.stringify(wl.took)} / ${JSON.stringify(wl.caseFolded)} ` +
          "for #0055aa — what the picker hands back is what khor writes to the device table",
      );
    }
    console.log(
      `wk: colour well ${wl.type} ${wl.w}×${wl.h}, ${JSON.stringify(wl.was)} → ` +
        `${JSON.stringify(wl.took)}, showPicker=${wl.hasShowPicker}`,
    );
    // ⚠️ **Not asserted, and the sentence is the reason.** Opening the
    // panel needs a user gesture; a probe has none. So whether the
    // macOS colour panel really appears is 未量 here and stays the eye's
    // — the same rule this file keeps for the worker realm: a hole is
    // printed, never asserted shut.
    console.log(
      "--- the panel itself is 未量: opening it wants a user gesture, so this arm measures the " +
        "control and the value, and a person measures the panel ---",
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

// ── The shell under a wheel: 整个app不随滑轮动 (第 1 条) ────────────
//
// 「我在工作区的文件树拖动, 整个app都会下移, 正确的应该是整个app是不
// 允许随滑轮变化的」— the user, 2026-09-20, about this engine.
//
// **This is the arm `check-face` cannot have.** Chrome's desktop
// viewport does not rubber-band and its document, in khor's layout,
// never had anywhere to go — so over there the readings below are true
// whatever the stylesheet says. Here they are not: the bare arm's page
// is taller than its window, and with no rules on it the rail slides
// out of the corner the window's three buttons stand in.
{
  const bare = said.shellBare ?? {};
  const nailed = said.shellNailed ?? {};
  if (bare.loadFailed || nailed.loadFailed) {
    fail(
      `wk probe could not run the shell arm (bare: ${bare.loadFailed ?? "ok"}, nailed: ${nailed.loadFailed ?? "ok"}) — ` +
        "nothing about 整个app不随滑轮动 was measured",
    );
  } else {
    console.log(`wk: shell bare   ${JSON.stringify(bare)}`);
    console.log(`wk: shell nailed ${JSON.stringify(nailed)}`);
    // The control, in three parts: this WebKit understands the
    // property at all, the pane really did reach its end, and the page
    // really can move when nothing is holding it.
    if (bare.supports !== true) {
      fail(
        "this WebKit answers CSS.supports('overscroll-behavior','none') = false — khor's shell rule is inert " +
          "on it and every reading below proves nothing (src/styles/tokens.css base layer)",
      );
    }
    if (bare.paneAtEnd !== true) {
      fail(
        `probe dead: the pane did not reach its end in the bare arm (${bare.paneAtEnd}) — the gesture this ` +
          "arm is about never happened",
      );
    }
    if (!(bare.docTop > 0) || bare.railTop === bare.railWas) {
      fail(
        `probe dead: with no rules the page went to ${bare.docTop} and the rail stayed at ${bare.railTop} — ` +
          "this arm cannot fail, so its green below says nothing (scripts/wk-probe/shell.html)",
      );
    }
    // …and khor's own two blocks, on the same page.
    if (nailed.rootOverflowY !== "hidden") {
      fail(
        `with khor's rules the document's overflow-y is ${nailed.rootOverflowY} on this WebKit — ` +
          "「整个app是不允许随滑轮变化的」 (src/styles/tokens.css base layer)",
      );
    }
    if (nailed.rootOverscrollY !== "none") {
      fail(
        `with khor's rules the document's overscroll-behavior-y is ${nailed.rootOverscrollY} on this WebKit — ` +
          "this is the declaration that stops the elastic band, and it is the half Chrome cannot measure",
      );
    }
    if (nailed.paneOverscrollY !== "contain") {
      fail(
        `a pane wearing tailwind's overflow class computes ${nailed.paneOverscrollY} on this WebKit — ` +
          "khor's one containment selector did not reach it (src/styles/app.css)",
      );
    }
    if (nailed.docTop !== 0) {
      fail(`with khor's rules the page still scrolled to ${nailed.docTop} — 「整个app都会下移」`);
    }
    // 第三轮: a tab strip past its edge spends no height on a scrollbar.
    // **Only red-able where this WebKit draws a bar that takes room** —
    // with overlay scrollbars (a trackpad Mac) the bare arm reads 0 too,
    // and then this is said to be unmeasured rather than passed.
    if (!(bare.stripBar > 0)) {
      console.log(
        `wk: tab strip   未量 — with no rules the strip's bar takes ${bare.stripBar}px on this WebKit ` +
          "(overlay scrollbars), so 「不画滚动条」 cannot go red here",
      );
    } else if (nailed.stripBar !== 0) {
      fail(
        `with khor's rules a tab strip past its edge still spends ${nailed.stripBar}px on a scrollbar on this ` +
          "WebKit — 「标签会被移动条顶出窗口」 (src/styles/app.css [data-tab-strip])",
      );
    }
    if (nailed.railTop !== nailed.railWas) {
      fail(
        `with khor's rules the rail went ${nailed.railWas} → ${nailed.railTop} — the window's three buttons ` +
          "sit in that corner (第 58 条)",
      );
    }
  }
}

// ── A page follows the window's 深浅 (docs/SHELL.md 第四轮 第 6 条) ──
//
// 「浏览器访问地址的时候, 传递app的外观主题(深色、浅色, 似乎已经做到了,
// 检查一下)」. The app sets its window's appearance and a browsing tab's
// page is a view inside that window; this reads `prefers-color-scheme` in
// such a page as the window turns dark, light, dark. The web face cannot
// do this at all — a page there is the visitor's own browser's, and its
// 深浅 is that browser's to decide.
{
  const t = said.theme ?? {};
  console.log(`wk: theme       ${JSON.stringify(t)}`);
  if (t.loadFailed) {
    fail(`wk probe could not run the theme arm (${t.loadFailed}) — nothing about 外观主题 was measured`);
  } else if (JSON.stringify(t.reads) !== JSON.stringify(["true", "false", "true"])) {
    fail(
      `a page inside the window read prefers-color-scheme: dark as ${JSON.stringify(t.reads)} while the window ` +
        "went dark, light, dark — 「传递app的外观主题」 (src-tauri wear_appearance)",
    );
  }
}

// ⚠️ 墓碑 (2026-09-23, docs/SHELL.md 第十轮): a section here drove a real
// AppKit window through five relayouts and demanded khor's three buttons
// stay squeezed into the rail. **The squeeze is gone**: the window has a
// row of its own now and macOS's buttons keep the SDK's own size and
// spacing inside it (`trafficLightPosition`), so there is nothing here
// to hold in place and nothing to measure — what the row does is the
// face gate's (`check-face.mjs`, 第十轮).

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
      "and the editor's rows sit on the sheet's, and a wheel past the end of a pane moves neither the " +
      "page nor the rail",
  );
}
process.exitCode = bad ? 1 : 0;
