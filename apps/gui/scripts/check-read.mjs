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
//      the page it lands inside the reading band — stated for CJK, which
//      is the script these paragraphs are mostly in, and *not* stated
//      twice: see LAT_PER_CJK for why a second band in Latin was the
//      same claim wearing different units, and what replaced it;
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
//   7. syntax colour is three roles and no more: a comment is the aux
//      ink, a string is `--code-string`, and a keyword — standing for
//      everything else — is the body ink it inherits. The fourth role is
//      the one a低色 scheme loses one rule at a time;
//   8. and the new colour is genuinely new: further in OKLab from every
//      colour this app has already given a job to than those colours are
//      from each other;
//   9. a fence that has not closed carries no highlight at all — the
//      block still arriving is both a guess and work on every chunk;
//  10. what `dangerouslySetInnerHTML` receives is this build's markup and
//      never the agent's: a block whose *content* is a script tag and an
//      event handler grows no element and comes back character for
//      character;
//  11. zero page errors while measuring.
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
 * Chinese characters on a line.
 *
 * **The ceiling is transcribed from claude.ai, measured 2026-08-30**: a
 * real conversation in a 1580 window has a 736-wide paragraph column at
 * size 16, which is 46 CJK glyphs. The person reading this app asked
 * for that face whole, so the number here follows the product it is
 * matching rather than a rule.
 *
 * **It therefore does not satisfy WCAG 2.x SC 1.4.8 (AAA), and that is
 * recorded rather than removed.** The SC caps a line at 80 characters —
 * **40 if CJK**, on the ground that a CJK glyph is about twice as wide
 * as a Latin character at the same legibility. 46 is 15% past it. The
 * reference product does not satisfy it either (46 CJK / 99.3 Latin).
 * The ceiling here has been 45, then 40 (when the standard was first
 * looked up), and now 46; **a gate whose citation gets deleted the
 * moment the product stops obeying it teaches the next reader that
 * nobody ever checked** — so the citation stays, above the value that
 * departs from it.
 *
 * **Tombstone: that Latin figure read 88.3 until 2026-08-31, and the
 * wrong number is worth more here than the right one.** 88.3 was a
 * single reading that had never been reproduced; re-measuring the live
 * product gives 99.3, with 100 counted directly on a 736 line box. The
 * reference face is 1.3% wider a character than this one, not 12%.
 * **The first attempt to re-check it concluded「the webfont is not
 * rendering, so the measurement is of a fallback」— and that was wrong
 * too**: the control deleted `anthropic-serif` from the stack, and the
 * entry sitting behind it is a `size-adjust`ed metric-matched fallback,
 * so both arms of that control were built to measure the same width. It
 * cannot come back anything but「no difference」. `document.fonts
 * .check()` is no help either; it returns `true` for a family that does
 * not exist. To prove a face is really rendering, drop the
 * metric-matched fallbacks too and show the number moves. The full
 * arithmetic is on `--measure` in `tokens.css`.
 *
 * **The floor is not from the standard and is not pretending to be.**
 * Nothing published says a line may not be too *short*, but 32 was
 * measured to cost 71% of the characters on screen (3250 → 930 in a
 * 755-tall pane), so a column that narrow is a defect this gate should
 * still catch. 30 is the bottom of the same long-form range the ceiling
 * used to come from — the one number here that is still a matter of
 * reading practice rather than a rule.
 *
 * **What this deliberately does not assert, and why it is not a hole.**
 * The same SC caps non-CJK at 80, and at the ceiling above a line holds
 * ~100 Latin characters (46 × the 2.189 measured below on 2026-08-31).
 * Both halves hold only at a cap of 36.5em or less. The standard's two
 * numbers were written for text in one script; these paragraphs run
 * 32–74% CJK, so one half has to give and it is the half for the script
 * this app is not mostly in. Asserting it would make this gate red on
 * the value the product ships, which is a gate that gets turned off
 * rather than a gate that gets obeyed — so it is written down here
 * instead, with the arithmetic, where the next person can weigh it.
 */
const CJK_BAND = [30, 46];
/**
 * **There is no Latin band any more, and removing it is not the same as
 * widening it.**
 *
 * It was 45–75, then 90 to let a wider column through. A third number
 * would have been the tell: an assertion that has to be edited every
 * time the thing it measures is edited is not holding anything.
 *
 * The reason it kept needing to move is that it was never a second
 * claim. A CJK glyph's advance is exactly 1em and a Latin character's is
 * a fixed fraction of one, so **once the cap is fixed the Latin count is
 * the CJK count times a font constant** — measured 2.189 on 2026-08-31
 * (it read 2.17, undated, until then). Asserting both is asserting the
 * same fact twice in different units, and the two copies can only ever
 * disagree.
 *
 * **That 2.189 is the illustration; the band below is the assertion.**
 * The constant belongs to the font stack, not to any decision made
 * here, so it drifts whenever the stack does — which is why the band is
 * wide and why the figure above carries a date. It is also never worth
 * trusting from a comment: the printed line already carries both counts
 * on every run, so the live ratio is one division away. **Changing the
 * font stack is not a reason to come and edit that number.**
 *
 * What a Latin reading *can* say on its own is something about the
 * font, not about the column: if a future stack drew Latin much
 * narrower, the same cap would put far more of it on a line. That is
 * asserted below, directly, as the ratio — and it does not move when the
 * cap moves, which is the property the old band did not have.
 *
 * The count is still printed. It is worth knowing; it is not a claim.
 */
const LAT_PER_CJK = [1.85, 2.45];
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

/**
 * A block whose *content* is markup. The painted output reaches the DOM
 * through `dangerouslySetInnerHTML`, so this is the input that decides
 * whether that is safe: highlight.js escapes before it wraps, and this
 * gate reads back both the element count and the text rather than taking
 * that on trust. A JSON block because JSON is on the whitelist, so this
 * text goes through the painter rather than around it.
 */
const EVIL = `{"marker": "escape-probe", "a": "<script>alert(1)</script>", "b": "<img src=x onerror=alert(2)>"}`;

/** The sample. Mixed on purpose — a pure-CJK or pure-Latin paragraph
    would let one script's band be satisfied while the other's was not
    even exercised, and the whole difficulty here is that one cap has to
    answer for both. */
const AGENT_TEXT = `我把这一段当成真实的 agent 回话来量：它既有中文长句，也有 English clauses，还夹着 \`file paths\` 和 identifier 之类的东西——正文行长要同时对这两种脚本成立，所以样本必须是混排的，而不是拿一段纯中文或者一段纯英文去凑一个好看的数字。

The conversation face borrowed the dashboard's size tier, which was tuned for rows of facts that never wrap. A paragraph is not a row: it wraps, and once it wraps the line length becomes the thing that decides whether the eye finds the start of the next line without having to hunt for it.

> 一段引用，用来和上面那条工具调用比形状。

\`\`\`ts
// a comment, which is the aux ink
const items = useMemo(() => [...paint.past, ...paint.live], [paint]);
const where = "a string literal, which is the one new colour";
\`\`\`

\`\`\`json
${EVIL}
\`\`\`

\`\`\`rust
fn still_arriving() {`;

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
  // **The reading column is the agent's side, and since 2026-08-31 the
  // user's bubble is markdown too — so it has to be stepped over.**
  //
  // This used to take the document's first `[data-md]`, which was the
  // agent's because the bubble was a pre-wrapped run of the characters
  // somebody typed. Once the bubble started rendering markdown it came
  // first in DOM order and the gate measured *it*: on a 390 phone it
  // reported a 192-wide column, which was simply how wide that
  // sentence was — a bubble shrink-wraps its text and is capped at 85%
  // besides, so it was never the column this gate is about.
  //
  // Not a widened band and not an excused failure: the probe was
  // pointing at the wrong element, and the number it read was true of
  // the element it found.
  const mds = [...document.querySelectorAll("[data-md]")];
  const prose = mds.find((el) => !el.closest("[data-said]")) ?? md;
  const p = prose?.querySelector("p") ?? document.querySelector("[data-msg-text]");
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

  // ── the code blocks ──────────────────────────────────────────────
  // Found by their content rather than by their order, so a change to
  // the sample above cannot silently point an assertion at a different
  // block than the one it was written about.
  const blocks = [...(scroller?.querySelectorAll("pre code") ?? [])];
  const withText = (needle) => blocks.find((b) => (b.textContent ?? "").includes(needle)) ?? null;
  const closed = withText("useMemo");
  const arriving = withText("still_arriving");
  // Found by a marker that stays *text* whatever happens: if escaping
  // broke, `onerror` becomes an attribute and leaves `textContent`, so a
  // finder keyed on it would lose the block exactly when the bug appears
  // and report「never drew」instead of「escaping failed」.
  const evil = withText("escape-probe");
  const inkOf = (el, sel) => {
    const t = sel ? el?.querySelector(sel) : el;
    return t ? getComputedStyle(t).color : null;
  };
  const root = getComputedStyle(document.documentElement);
  // **Through the engine, not by reading the declaration.** A token is
  // written however it was convenient — `#005a6c`, `rgba(…)`, or a
  // `var()` that resolves to either — while what comes off an element is
  // always `rgb()`. Comparing the two forms is how this gate was green
  // for a value that had been set to a state word's own colour: the hex
  // went through a number-scraping parse, which found `005`, `6` and `c`
  // and computed a distance from noise. Handing the value back to the
  // engine as a colour is the only normalisation that cannot drift from
  // what the page actually paints.
  const probeEl = document.createElement("span");
  document.body.appendChild(probeEl);
  const token = (name) => {
    const raw = root.getPropertyValue(name).trim();
    if (!raw) return "";
    probeEl.style.color = "";
    probeEl.style.color = raw;
    const out = getComputedStyle(probeEl).color;
    // An unparseable value leaves the element at its inherited colour,
    // which would read as a perfectly ordinary rgb() and quietly compare
    // as「some other colour」. Say so instead.
    return probeEl.style.color === "" ? `unparseable:${raw}` : out;
  };
  // Flattened onto the ground they are all seen against, because that is
  // the comparison a person makes: two inks with different alphas over
  // one paper, not two declarations.
  const onPaper = (colour, paper) =>
    `rgb(${over(parse(colour), paper).map((n) => Math.round(n)).join(", ")})`;
  // Shiki paints roles as colour on anonymous spans, never as a class,
  // so a role is found by the text it painted: the fixture's own
  // string, comment and keyword (`turn`, below).
  const spans = (el) => (el ? el.querySelectorAll('span[style*="--shiki"]').length : -1);
  const byText = (el, want) => {
    if (!el) return null;
    for (const sp of el.querySelectorAll('span[style*="--shiki"]')) {
      if ((sp.textContent ?? "").includes(want)) return getComputedStyle(sp).color;
    }
    return null;
  };
  const code = {
    closedFound: closed !== null,
    closedSpans: spans(closed),
    // A run only the literal contains — `hi` is also inside `which`,
    // in the comment two lines up.
    stringInk: byText(closed, "a string literal"),
    // The slashes, not the words: the grammar cuts `//` from what
    // follows it, and both halves wear the comment's ink.
    commentInk: byText(closed, "//"),
    keywordInk: byText(closed, "const"),
    bodyInk: inkOf(closed, null),
    arrivingFound: arriving !== null,
    arrivingSpans: spans(arriving),
    evilFound: evil !== null,
    // Anything that would be an *element* rather than text. If escaping
    // failed anywhere, one of these is how it shows.
    evilElements: evil ? evil.querySelectorAll("script, img, iframe, object, embed, svg").length : -1,
    // The trailing newline is the fence's own, not the agent's.
    evilText: evil ? (evil.textContent ?? "").replace(/\n$/, "") : null,
    spokenFor: Object.fromEntries(
      ["busy", "blocked", "done", "errored", "failed", "idle", "strain", "lq3"].map((n) => [
        n,
        (() => {
          const c = token(n === "strain" || n === "lq3" ? `--${n}` : `--state-${n}`);
          return c.startsWith("rgb") && closed ? onPaper(c, groundOf(closed)) : c;
        })(),
      ]),
    ),
  };
  probeEl.remove();

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
    code,
  };
};

/** Perceptual distance between two CSS colours, in OKLab. Used for the
    one claim a plain inequality cannot carry: a *new* colour has to be
    further from every colour this app already uses than those colours
    are from each other, and `!==` is satisfied by one bit. */
function apart(a, b) {
  const rgb = (s) => (s.match(/[\d.]+/g) || []).slice(0, 3).map(Number);
  const oklab = ([r, g, bb]) => {
    const f = (c) => {
      c /= 255;
      return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
    };
    const [R, G, B] = [f(r), f(g), f(bb)];
    const l = Math.cbrt(0.4122214708 * R + 0.5363325363 * G + 0.0514459929 * B);
    const m = Math.cbrt(0.2119034982 * R + 0.6806995451 * G + 0.1073969566 * B);
    const s = Math.cbrt(0.0883024619 * R + 0.2817188376 * G + 0.6299787005 * B);
    return [
      0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
      1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
      0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
    ];
  };
  const [A, B] = [oklab(rgb(a)), oklab(rgb(b))];
  return Math.hypot(A[0] - B[0], A[1] - B[1], A[2] - B[2]);
}

/**
 * How far a new colour has to sit from every colour already spoken for.
 *
 * **Not a round number picked to pass.** The closest pair this app
 * already ships as two different colours is 完成/中断 at .090 in the
 * light theme (完成/the brand green is .024, but those two are meant to
 * be the same family). A colour arriving now has to be more clearly new
 * than two old neighbours are from each other, so the floor is above
 * that, and the values it was written for clear it with room: .176 in
 * the light theme, .126 in the dark.
 */
const APART = 0.1;

// ── the streaming decision, measured on the functions themselves ─────
//
// The DOM claims below prove the *outcome* on a finished turn. This
// proves the two things a finished page cannot show: that a fence which
// has not closed is still seen as open at every size the text passes
// through on its way in, and that a block which has closed is painted
// once rather than once per remaining chunk.
//
// Measured rather than reasoned: node runs this module directly (it
// strips the types), and the module counts its own grammar runs
// (`measured.paints`, `lib/highlight.ts`) — Shiki's highlighter is a
// closure nothing outside the module can wrap, where highlight.js's
// entry point could be. The count is still of real runs.
{
  const { paint, openFenceAt, languageOf, measured } = await import("../src/lib/highlight.ts");
  let calls = 0;

  // The probe first: a build where nothing ever paints satisfies both
  // 「zero while open」and「one when closed」by doing nothing at all.
  const sample = 'const s = "hi"; // c';
  if (languageOf("tsx") === null) fail("probe dead: the whitelist resolves nothing, so nothing below is exercised");
  else if ((await paint(sample, "typescript")) === null) fail("probe dead: the painter answered nothing for a whitelisted language");
  else if (measured.paints === 0) fail("probe dead: the module counted no run, so the count below means nothing");
  else {
    const first = measured.paints;
    await paint(sample, "typescript");
    await paint(sample, "typescript");
    await paint(sample, "typescript");
    calls = measured.paints;
    if (calls !== first) {
      fail(
        `a closed block is re-highlighted: ${calls - first} extra run(s) for the same text. ` +
          `Closing a fence is not the end of a turn — the prose after it keeps arriving, and each ` +
          `chunk re-renders the whole turn`,
      );
    }
  }

  // Every prefix a streaming turn passes through, at the size the
  // protocol delivers. While the fence is open it must *stay* open —
  // one prefix that reads as closed is one paint of a half-written
  // block, and a guess at a language that is still being typed.
  const turn = '前面一段话。\n\n```ts\n// c\nconst s = "hi";\nconst t = [1, 2, 3];\n```\n\n收尾的一段话，它在围栏封口之后还在继续到达。';
  const fenceAt = turn.indexOf("```");
  const closeAt = turn.indexOf("```", fenceAt + 3) + 3;
  let openWhileArriving = 0;
  let leaked = 0;
  for (let n = fenceAt + 3; n < closeAt; n += 8) {
    openWhileArriving += 1;
    if (openFenceAt(turn.slice(0, n)) === null) leaked += 1;
  }
  if (openWhileArriving === 0) fail("probe dead: no prefix landed inside the fence, so nothing was measured");
  if (leaked > 0) {
    fail(`${leaked} of ${openWhileArriving} prefixes inside an open fence read as closed — those are paints of a block still being typed`);
  }
  if (openFenceAt(turn) !== null) {
    fail("a turn whose fences are all closed still reads as open — nothing in it would ever be painted");
  }
  console.log(
    `stream: ${openWhileArriving} prefixes inside the fence, ${leaked} painted; ` +
      `a closed block costs ${calls} call(s) however many chunks follow it`,
  );
}

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
          watch: [],
          watched: null,
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
      // Not「how many Latin characters」— that follows from the cap. This
      // is the font stack's two scripts still standing in the same
      // relation to each other, which is the only thing the Latin
      // reading can say that the CJK one does not.
      const perCjk = m.latPerLine / m.cjkPerLine;
      if (perCjk < LAT_PER_CJK[0] || perCjk > LAT_PER_CJK[1]) {
        fail(
          `${say(m)}: a line holds ${perCjk.toFixed(2)} Latin characters per Chinese one, outside ` +
            `${LAT_PER_CJK.join("–")} — the two scripts have moved relative to each other, so a cap ` +
            `set for one of them is no longer the cap the other gets`,
        );
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

      // 7) syntax colour: three roles, and only where the fence closed.
      //    Agent face only — a machine's message is not markdown.
      if (which === "agent") {
        const c = m.code;
        // The probe, and it is the whole of section 8's positive half:
        // if nothing painted, "the arriving block is not painted" and
        // "the string ink is not a state colour" both pass by vacuum.
        if (!c.closedFound) {
          fail(`${say(m)}: probe dead — the closed code block never drew`);
        } else if (c.closedSpans === 0) {
          fail(
            `${say(m)}: the closed fence carries no highlight at all — either the whitelist stopped ` +
              `resolving or the painter is never being reached`,
          );
        } else if (c.stringInk === null || c.commentInk === null || c.keywordInk === null) {
          fail(
            `${say(m)}: probe dead — the painted block has no ` +
              `${c.stringInk === null ? "string" : c.commentInk === null ? "comment" : "keyword"} token to read a role off`,
          );
        } else {
          // Three roles, three values. Stated as inequality between the
          // three *rendered* inks rather than against the tokens, so a
          // rule that failed to reach the page reads as one role, not as
          // a token mismatch.
          if (c.stringInk === c.bodyInk) {
            fail(`${say(m)}: a string reads as body ink (${c.bodyInk}) — the string role is not landing`);
          }
          if (c.commentInk === c.bodyInk) {
            fail(`${say(m)}: a comment reads as body ink (${c.bodyInk}) — the comment role is not landing`);
          }
          if (c.commentInk === c.stringInk) {
            fail(`${say(m)}: comment and string are one colour (${c.stringInk})`);
          }
          // **A keyword is its own colour since 2026-09-15.** This gate
          // used to hold the opposite line — three roles, a keyword in
          // body ink — until the user ruled 「我说的语法高亮，是有颜色
          // 的」 and then 「做到 gh web 预览代码的那种高亮」; the theme
          // is GitHub's own now, and a keyword that fell back to body
          // ink would mean the theme did not reach the page.
          if (c.keywordInk === c.bodyInk) {
            fail(`${say(m)}: a keyword reads as body ink (${c.bodyInk}) — the theme is not landing`);
          }
          // The string's distance from the state words is printed, not
          // gated: the colour is GitHub's and not this app's to move,
          // and a person reads code inside a code block, where a state
          // word never appears.
          const near = Object.entries(c.spokenFor)
            .filter(([, colour]) => colour.startsWith("rgb"))
            .map(([name, colour]) => `${name} ${apart(c.stringInk, colour).toFixed(3)}`);
          console.log(`  string ink ${c.stringInk} vs the spoken-for colours: ${near.join(" · ")}`);
        }

        // 8) and the block that is still arriving is left alone.
        if (!c.arrivingFound) {
          fail(`${say(m)}: probe dead — the unterminated fence never drew, so claim 8 has nothing to look at`);
        } else if (c.arrivingSpans !== 0) {
          fail(
            `${say(m)}: the unterminated fence carries ${c.arrivingSpans} highlight span(s) — a block that ` +
              `is still arriving is being painted, which is both a guess and work on every chunk`,
          );
        }

        // 9) painted markup is this build's, never the agent's.
        if (!c.evilFound) {
          fail(`${say(m)}: probe dead — the block carrying markup never drew`);
        } else {
          if (c.evilElements !== 0) {
            fail(
              `${say(m)}: ${c.evilElements} element(s) grew out of a code block's *text* — escaping is ` +
                `not holding and the agent's words are reaching the DOM as markup`,
            );
          }
          if (c.evilText !== EVIL) {
            fail(
              `${say(m)}: a code block's text came back changed.\n  wanted: ${EVIL}\n  got:    ${c.evilText}`,
            );
          }
        }
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
