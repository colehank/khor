// The reading gate: a conversation is the one surface in this app that
// is *read* rather than scanned, and until 2026-08-30 nothing measured
// it. It borrowed the dashboard's aux size (12.5), ran to whatever width
// the window gave it (measured: 1098 wide, 88 Chinese characters to a
// line at a 1482 window), and painted at the top of the contrast range.
//
// **The hole this fills, stated as what the other gates do not do.**
// `check-palette` measures the primary button's label, the focus ring's
// ratio and geometry, and the six state words. `check-face` measures
// weights, touch targets and reading order. `check-type` forbids a size
// literal outside the tokens. Not one of them looks at the body: its
// contrast, its size, or how far a line of it runs. Grep for
// `--foreground` and `--muted-foreground` across `scripts/` before
// widening this gate's scope — the only two hits are a sentence in
// `check-rail` and one equality reference in `smoke` (the kind mark's
// colour), neither of which is an assertion about prose.
//
// Claims:
//   0. probe: a conversation actually painted a paragraph that *wraps*.
//      Every claim below is about a wrapped line, so a one-line
//      paragraph — or none — would satisfy them by vacuum, which is
//      this repo's most-met failure;
//   1. the column is bounded, and converted with advances measured on
//      the page it lands inside the reading band for both scripts;
//   1b. and the element carrying the cap carries the reading size. The
//      cap is an `em` (`--measure`), so those two classes have to ride
//      together — split them and the column silently comes out narrower
//      than the token says while every other number here stays right;
//   2. on a narrow window the cap does not bind and nothing overflows
//      sideways — the phone must not pay for the wide window's rule;
//   3. the body ink reads in the long-form band against the ground it is
//      actually painted on, in both themes, and stays plainly above the
//      aux ink;
//   4. the reading size is larger than the shell's base — the borrowing
//      this batch ended cannot come back by one token being edited;
//   4b. and the box a person writes into is the size they are reading.
//      It was two steps under (12.5 against 15) while already matching
//      the column's *width*, which made the mismatch in size the louder
//      of the two. This one is worth a gate of its own because the
//      obvious way to fix it is the way that silently does not work —
//      a `text-read` handed down from the caller loses to a `md:`
//      prefixed size inside `cn()`, and loses only above the breakpoint;
//   5. a tool call and a blockquote are not the same picture. They were:
//      the same `border-l-2 pl-2`, the same border colour, the same
//      gutter — one shape saying two things;
//   6. and both conversation faces are held to it, not just the agent
//      one (两端对齐的是事实, docs/UX.md): the machine conversation is
//      routed in and measured by the same code;
//   7. zero page errors while measuring.
//
// **What it cannot block.** Whether 38 is a better column than 44 — the
// band is arithmetic and the point inside it is a matter of eyes, which
// is why the assertion is a band and the value lives in one token. And
// it says nothing about the *narrow* posture's character count, on
// purpose: see NARROW below.
//
// Run: `npm run read` (boots its own vite, needs system Chrome, no
// cargo lock).
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const PORT = Number(process.env.READ_PORT ?? 1454);
const WIDE = 1482;
const NARROW = 390;
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

/**
 * The reading band for Chinese characters on a line. Both ends are the
 * ordinary long-form range.
 */
const CJK_BAND = [30, 45];
/**
 * The band for Latin characters — **and the ceiling is 90, not the 75
 * these two bands are usually quoted with together.**
 *
 * Both bands were written about text in one script. These paragraphs
 * run 32% to 74% CJK, and a column asked to satisfy 30–45 CJK *and*
 * 45–75 Latin has only 30–35 characters left to it (a CJK advance is
 * 1em, Latin .465–.514em, so 75 Latin is 35–39em while 30 CJK is 30em).
 * Held there the column costs 71% of the characters on screen — measured
 * 3250 → 930 in a 755-tall pane — and for comparison Claude's own
 * reading column runs past 75 Latin characters.
 *
 * So the product gives ground on the Latin ceiling knowingly, and this
 * gate says so out loud rather than quietly passing a column it would
 * have failed. What it still refuses is an *unbounded* column, which is
 * the thing that was actually wrong.
 */
const LAT_BAND = [45, 90];
/**
 * Where long-form body ink should read. The floor is not the standard's
 * 4.5: a CJK glyph carries several times the strokes of a letter at the
 * same size, and thinning the ink closes the gaps between strokes before
 * it dims a letterform. The ceiling is the other end of the same
 * argument — text held at the top of the range glares on a dark ground.
 */
const INK_BAND = [10, 13.5];
/** How far body has to stay above the aux ink for that demotion to mean
    anything. Measured at the values this landed on: 12.23 over 6.36
    dark, 12.07 over 4.73 light. */
const AUX_MARGIN = 1.5;

const fail = (m) => {
  console.error(m);
  process.exitCode = 1;
};

/** The sample. Mixed on purpose — a pure-CJK or pure-Latin paragraph
    would let one script's band be satisfied while the other's was not
    even exercised, and the whole difficulty here is that one cap has to
    answer for both. */
const AGENT_TEXT = `我把这一段当成真实的 agent 回话来量：它既有中文长句，也有 English clauses，还夹着 \`file paths\` 和 identifier 之类的东西——正文行长要同时对这两种脚本成立，所以样本必须是混排的，而不是拿一段纯中文或者一段纯英文去凑一个好看的数字。

The conversation face borrowed the dashboard's size tier, which was tuned for rows of facts that never wrap. A paragraph is not a row: it wraps, and once it wraps the line length becomes the thing that decides whether the eye finds the start of the next line without having to hunt for it.

> 一段引用，用来和上面那条工具调用比形状。

\`\`\`ts
const items = useMemo(() => [...paint.past, ...paint.live], [paint]);
\`\`\``;

const frames = [
  {
    kind: "history",
    update: {
      update: {
        sessionUpdate: "user_message_chunk",
        content: { type: "text", text: "帮我看看这个会话面的排版" },
      },
    },
  },
  {
    kind: "history",
    update: { update: { sessionUpdate: "tool_call", title: "Read src/views/ChatView.tsx" } },
  },
  {
    kind: "history",
    update: {
      update: { sessionUpdate: "agent_message_chunk", content: { type: "text", text: AGENT_TEXT } },
    },
  },
  { kind: "history_end" },
];

const row = (id, kind, title) => ({
  id,
  kind,
  title,
  word: "idle",
  at_ms: Date.now(),
  unread: 0,
  source: null,
  pinned: false,
  stale: false,
  category: "claude",
  group: "",
  face: null,
  hosted: true,
  attachable: false,
  last: null,
  via: null,
  via_face: null,
  home: "gate",
});
const ROWS = [row("gui/read-gate", "gui", "read-gate-agent"), row("chat/read-gate", "chat", "read-gate-machine")];

/**
 * Everything measured off one painted conversation, in the page.
 *
 * The advances are measured here rather than written down: a font stack
 * change would move them, and a gate carrying its own copy of a number
 * the browser can answer for is the way a gate and a product drift
 * apart. The ground is composited down the ancestor chain for the same
 * reason — a bubble sits on a translucent `--muted`, and taking the
 * first non-transparent background as the ground would report a
 * contrast against a colour nothing is painted on.
 */
const probe = () => {
  const lin = (c) => {
    c /= 255;
    return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  };
  const lum = ([r, g, b]) => 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
  const parse = (s) => (s.match(/[\d.]+/g) || []).map(Number);
  const over = (fg, bg) => {
    const a = fg.length > 3 ? fg[3] : 1;
    return [0, 1, 2].map((i) => fg[i] * a + bg[i] * (1 - a));
  };
  const groundOf = (el) => {
    const stack = [];
    for (let n = el; n; n = n.parentElement) {
      const c = parse(getComputedStyle(n).backgroundColor);
      if (c.length >= 4 && c[3] === 0) continue;
      stack.push(c);
      if (c.length < 4 || c[3] === 1) break;
    }
    let g = [255, 255, 255];
    for (let i = stack.length - 1; i >= 0; i--) g = over(stack[i], g);
    return g;
  };
  const ratio = (fg, bg) => {
    const [a, b] = [lum(fg), lum(bg)].sort((x, y) => y - x);
    return (a + 0.05) / (b + 0.05);
  };
  const contrastOf = (el) => {
    const g = groundOf(el);
    return +ratio(over(parse(getComputedStyle(el).color), g), g).toFixed(2);
  };
  const advance = (cs, text) => {
    const s = document.createElement("span");
    s.style.font = `${cs.fontStyle} ${cs.fontWeight} ${cs.fontSize} / normal ${cs.fontFamily}`;
    s.style.whiteSpace = "pre";
    s.style.position = "absolute";
    s.style.visibility = "hidden";
    s.textContent = text;
    document.body.appendChild(s);
    const w = s.getBoundingClientRect().width / [...text].length;
    s.remove();
    return w;
  };
  const CJK = "这个应用的字号阶是仪表盘的阶密紧为一屏塞下很多事实";
  const LAT = "The conversation is a reading surface, not a dashboard row.";

  const column = document.querySelector("[data-chat-column]");
  const md = document.querySelector("[data-md]");
  // The two faces write their bodies differently and neither is wrong:
  // an agent writes markdown and gets a `p`, a machine's message is the
  // characters somebody typed and stays a pre-wrapped run (`ChatView`'s
  // own rule about the user's bubble). What is being measured is a
  // wrapped line of prose, which both of those are.
  const p = md?.querySelector("p") ?? document.querySelector("[data-msg-text]");
  const scroller = document.querySelector("[data-chat-scroll]") ?? column?.parentElement;

  // The tool line and the blockquote, found the same way: by walking the
  // painted conversation. A negative claim about one of them is only
  // worth anything if the other — which is known to be there — is found
  // by the identical walk.
  const inChat = [...(scroller?.querySelectorAll("*") ?? [])];
  const quote = inChat.find((e) => e.tagName === "BLOCKQUOTE") ?? null;
  const tool = inChat.find(
    (e) => e.childElementCount === 0 && /^Read src\/views\//.test(e.textContent ?? ""),
  );
  const shapeOf = (el) =>
    el
      ? {
          borderLeft: parseFloat(getComputedStyle(el).borderLeftWidth),
          mono: /mono|Menlo|Consolas|ui-monospace/i.test(getComputedStyle(el).fontFamily),
          padLeft: parseFloat(getComputedStyle(el).paddingLeft),
        }
      : null;

  // The shell's own base, read off the page rather than repeated here.
  const base = getComputedStyle(document.body).fontSize;

  // The box this conversation is written into. The two faces name theirs
  // differently — one goes through the shared control, one is a bare
  // textarea — so both are asked for and whichever is on screen answers.
  const box = document.querySelector("[data-chat-input], [data-chat-composer]");
  const boxCs = box ? getComputedStyle(box) : null;

  if (!p || !column || !scroller) {
    return { drew: false, hasColumn: !!column, hasP: !!p, hasScroller: !!scroller };
  }
  const cs = getComputedStyle(p);
  const w = p.getBoundingClientRect().width;
  const lh = parseFloat(cs.lineHeight);
  const colCs = getComputedStyle(column);
  // The aux ink taken from the token and laid on the *body's own*
  // ground, rather than found as some element that happens to wear it.
  // The claim is about two inks on one paper, and one face has no
  // element wearing the aux ink at all — a lookup would have made this
  // comparison quietly disappear on that face while every other number
  // stayed right.
  const auxDecl = getComputedStyle(document.documentElement)
    .getPropertyValue("--muted-foreground")
    .trim();
  const paper = groundOf(p);
  const auxContrast = auxDecl ? +ratio(over(parse(auxDecl), paper), paper).toFixed(2) : null;
  return {
    drew: true,
    lines: Math.round(p.getBoundingClientRect().height / lh),
    colWidth: +w.toFixed(1),
    colMaxWidth: colCs.maxWidth,
    colFontSize: colCs.fontSize,
    bodyFontSize: cs.fontSize,
    baseFontSize: base,
    boxFontSize: boxCs ? boxCs.fontSize : null,
    lineHeight: lh,
    cjkPerLine: +(w / advance(cs, CJK)).toFixed(1),
    latPerLine: +(w / advance(cs, LAT)).toFixed(1),
    bodyContrast: contrastOf(p),
    auxContrast,
    // What a narrow window has to answer: the cap must not bind, and
    // the pane must not have gained a sideways scroll.
    available: +(
      scroller.clientWidth -
      parseFloat(getComputedStyle(scroller).paddingLeft) -
      parseFloat(getComputedStyle(scroller).paddingRight)
    ).toFixed(1),
    overflows: scroller.scrollWidth > scroller.clientWidth,
    quote: shapeOf(quote),
    tool: shapeOf(tool),
  };
};

let vite = null;
let browser = null;
try {
  vite = spawn("node_modules/.bin/vite", ["--port", String(PORT), "--strictPort"], {
    cwd: gui,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let gone = null;
  vite.on("exit", (code) => (gone = `vite exited with ${code} — is ${PORT} already taken?`));
  let ready = false;
  vite.stdout.on("data", (d) => (ready ||= String(d).includes(`:${PORT}`)));
  for (let i = 0; i < 150 && !ready; i++) {
    if (gone) throw new Error(gone);
    await wait(100);
  }
  if (!ready) throw new Error(`vite never came up on ${PORT}`);

  browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server"] });

  /** One painted conversation, measured. `which` picks the face. */
  const look = async ({ scheme, width, which }) => {
    const ctx = await browser.newContext({
      viewport: { width, height: 900 },
      colorScheme: scheme,
      deviceScaleFactor: 1,
    });
    const page = await ctx.newPage();
    const errs = [];
    page.on("pageerror", (e) => errs.push(String(e)));
    const json = (b) => (r) =>
      r.fulfill({ contentType: "application/json", body: JSON.stringify(b) });
    await page.route("**/sessions", json(ROWS));
    await page.route(
      "**/devices",
      json([
        {
          id: "e".repeat(64),
          name: "gate",
          me: true,
          face: null,
          pinned: false,
          vitals: null,
          hop: null,
          sponsor: null,
          seat: { left: 50, top: 50 },
        },
      ]),
    );
    await page.route("**/hooks_state", json({ installed: false }));
    await page.route("**/quota", json({ quota: null, trouble: null }));
    await page.route("**/dir_pins", json([]));
    await page.route("**/web_pins", json([]));
    await page.route("**/recents", json([]));
    await page.route("**/chat_open", json(true));
    // The machine conversation's whole readout. Its body is the same
    // sample, so both faces are measured on one text and a difference
    // in the numbers is a difference in the faces.
    await page.route(
      "**/machine_log",
      json({
        broken: 0,
        messages: [
          {
            id: "m1",
            at_ms: Date.now() - 60_000,
            from: "gate",
            from_me: false,
            text: AGENT_TEXT,
            files: null,
            unknown: null,
            retracted: false,
            edited: false,
            word: null,
            delivery: null,
          },
        ],
      }),
    );
    let served = false;
    await page.route("**/chat_poll", (r) => {
      const body = served
        ? { frames: [], next: 4, gone: false, lost: false }
        : { frames, next: 4, gone: false, lost: false };
      served = true;
      r.fulfill({ contentType: "application/json", body: JSON.stringify(body) });
    });
    await page.goto(`http://localhost:${PORT}/?bridge=${PORT}`, { waitUntil: "load" });
    await wait(900);
    const nth = which === "machine" ? 1 : 0;
    await page.locator("[data-list] [data-row-open]").nth(nth).click();
    await wait(1200);
    const out = await page.evaluate(probe);
    await ctx.close();
    return { ...out, errs, scheme, width, which };
  };

  const say = (m) => `${m.which} · ${m.scheme} · ${m.width}`;

  // ── the wide posture: both themes, both faces ──────────────────────
  for (const which of ["agent", "machine"]) {
    for (const scheme of ["dark", "light"]) {
      const m = await look({ scheme, width: WIDE, which });
      console.log(
        `${say(m)}: ${m.colWidth} wide, ${m.cjkPerLine} cjk / ${m.latPerLine} latin, ` +
          `${m.bodyFontSize}/${m.lineHeight}, contrast ${m.bodyContrast}`,
      );
      if (m.errs.length) fail(`${say(m)}: the page threw while being measured — ${m.errs[0]}`);

      // 0) the probe, before every claim below.
      if (!m.drew) {
        fail(
          `${say(m)}: probe dead — no conversation paragraph drew ` +
            `(column ${m.hasColumn}, paragraph ${m.hasP}, scroller ${m.hasScroller})`,
        );
        continue;
      }
      if (m.lines < 2) {
        fail(
          `${say(m)}: probe dead — the sample paragraph came out on ${m.lines} line(s), ` +
            `so nothing below is measuring a wrapped line`,
        );
        continue;
      }

      // 1) the column is bounded, and reads inside the band.
      if (m.colMaxWidth === "none") {
        fail(`${say(m)}: the reading column has no cap — this is the state the gate was written about`);
      }
      if (m.cjkPerLine < CJK_BAND[0] || m.cjkPerLine > CJK_BAND[1]) {
        fail(
          `${say(m)}: ${m.cjkPerLine} Chinese characters to a line, outside ${CJK_BAND.join("–")}`,
        );
      }
      if (m.latPerLine < LAT_BAND[0] || m.latPerLine > LAT_BAND[1]) {
        fail(`${say(m)}: ${m.latPerLine} Latin characters to a line, outside ${LAT_BAND.join("–")}`);
      }
      // 1b) the cap and the size ride together.
      if (m.colFontSize !== m.bodyFontSize) {
        fail(
          `${say(m)}: the element carrying the em cap computes ${m.colFontSize} while the body ` +
            `reads ${m.bodyFontSize} — the cap is resolving against the wrong size`,
        );
      }

      // 3) the ink.
      if (m.bodyContrast < INK_BAND[0] || m.bodyContrast > INK_BAND[1]) {
        fail(`${say(m)}: body ink measures ${m.bodyContrast}, outside ${INK_BAND.join("–")}`);
      }
      if (m.auxContrast === null) {
        fail(`${say(m)}: probe dead — no aux-ink element found to compare the body against`);
      } else if (m.bodyContrast < m.auxContrast * AUX_MARGIN) {
        fail(
          `${say(m)}: body ${m.bodyContrast} is not clear of aux ${m.auxContrast} ` +
            `(wants ${AUX_MARGIN}×) — the demotion has stopped saying anything`,
        );
      }

      // 4) the reading size is above the shell's base.
      if (parseFloat(m.bodyFontSize) <= parseFloat(m.baseFontSize)) {
        fail(
          `${say(m)}: the conversation reads at ${m.bodyFontSize} against a shell base of ` +
            `${m.baseFontSize} — the borrowing this batch ended is back`,
        );
      }

      // 4b) and the box is the size of what is being read.
      if (m.boxFontSize === null) {
        fail(`${say(m)}: probe dead — this face drew no box to write into`);
      } else if (m.boxFontSize !== m.bodyFontSize) {
        fail(
          `${say(m)}: the box writes at ${m.boxFontSize} while the conversation reads at ` +
            `${m.bodyFontSize} — a person is typing in a size they are not reading in`,
        );
      }

      // 5) two shapes, two meanings. Only the agent face paints a tool
      //    line; the machine one has no tools to call.
      if (which === "agent") {
        if (!m.quote) {
          fail(`${say(m)}: probe dead — no blockquote found, so the shape claim has no positive half`);
        } else if (!(m.quote.borderLeft > 0)) {
          fail(`${say(m)}: probe dead — the blockquote carries no left rule, so this walk is not finding what it thinks it is`);
        } else if (!m.tool) {
          fail(`${say(m)}: probe dead — no tool line found by the same walk that found the blockquote`);
        } else if (m.tool.borderLeft > 0 && !m.tool.mono) {
          fail(
            `${say(m)}: a tool call and a blockquote are one picture again — both a left rule of ` +
              `${m.tool.borderLeft} and no other difference in shape`,
          );
        }
      }
    }
  }

  // ── the narrow posture ────────────────────────────────────────────
  // **No character-count claim here, and that is a judgment.** A 390
  // window leaves 358 for text; 30 Chinese characters on that line needs
  // a body of 11.9 or under — smaller than this app's aux size. The
  // wide band is unreachable on a phone at any size a person can read,
  // so asserting it here would be a red that can only ever be waived,
  // and a gate kept alive by waivers is not a gate. What the phone does
  // have to answer is that it pays nothing for the wide window's rule.
  const n = await look({ scheme: "dark", width: NARROW, which: "agent" });
  console.log(
    `${say(n)}: column ${n.colWidth} of ${n.available} available, ${n.cjkPerLine} cjk / ${n.latPerLine} latin`,
  );
  if (n.errs.length) fail(`${say(n)}: the page threw while being measured — ${n.errs[0]}`);
  if (!n.drew) fail(`${say(n)}: probe dead — no conversation paragraph drew`);
  else {
    if (n.colWidth < n.available - 1) {
      fail(
        `${say(n)}: the cap is binding on a phone — the column is ${n.colWidth} where ` +
          `${n.available} is available, so the wide window's rule is costing the narrow one`,
      );
    }
    if (n.overflows) {
      fail(`${say(n)}: the conversation scrolls sideways at ${NARROW}`);
    }
  }

  if (!process.exitCode) console.log("read gate: the conversation reads");
} catch (e) {
  fail(String(e));
} finally {
  if (browser) await browser.close();
  if (vite) {
    vite.kill("SIGKILL");
    try {
      process.kill(vite.pid, "SIGKILL");
    } catch {
      // already gone
    }
  }
  await wait(200);
}
