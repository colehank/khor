// The rail, measured — everything about it that only a browser can see.
//
// Three claims live here, and they share one page load because they are
// three halves of one complaint ("界面很丑、侧边栏太窄、icon 看不清"):
//
//   1. a resting glyph is legible — its filled half against whatever it
//      actually sits on, in both themes;
//   2. the way into the mesh is its own group — it sits at the foot,
//      wearing this machine's face, a whole `--gap-group` further from
//      the settings glyph than the landings sit from each other, on
//      both faces;
//   3. being the open one is said the same way by all five items.
//
// **Why none of this is a static check.** Every one of those facts is a
// product of things written in different files: an alpha on the ink
// (`tokens.css`), a `fill-opacity` on the solid piece (`app.css`), a
// utility class on the button (`App.tsx`), a margin on one item, a
// stylesheet layer that decides who wins, and whichever ancestor happens
// to be the last one that paints a background. Reading any one of them
// tells you nothing, and a script that combined them itself would be a
// second implementation of the cascade and the layout — green whenever
// *it* was right, rather than whenever the screen was. So this asks
// Chrome for computed values and box positions, and composites exactly
// what Chrome would put on the glass.
//
// The contrast floor is 3.0, the ratio a non-text graphic needs against
// its own background to be made out at all. It was 2.30 light / 2.71
// dark before this gate existed: the button wore an ink that already
// carried alpha and the body took a further .6 **of it**, so the two
// multiplied and the filled half of every glyph went to fog while its
// outline stayed crisp. Nothing was red. Nothing could be — no test in
// this repo, and no type, can see a colour or a gap.
//
// What it blocks, in the order these have gone wrong before:
//   - an ink re-dimmed for a reason about *text* (the original bug: the
//     rail borrowed `--muted-foreground`, which is tuned for paragraphs
//     on a card and knows nothing about a 1.4/24 stroke);
//   - a second alpha appearing anywhere in the chain, since this
//     measures the product and never the factors;
//   - the two densities quietly collapsing into one — a body pushed to
//     full opacity would *pass* a contrast floor and lose the thing the
//     floor was protecting, so "body is lighter than line" is asserted
//     next to it;
//   - a grouping that exists only in a comment. `App.tsx` argued for
//     years that the mark is not a fifth landing while it sat at exactly
//     a landing's distance from them;
//   - one item saying "I am the open one" in a way the others cannot —
//     the mark used to be the only item with a filled block behind it,
//     and it is the one item that is not a place to go;
//   - the open block drifting off the brand green. It is 「侧边栏还是
//     高亮品牌绿好了」 (the user, 2026-08-30) and no floor here can
//     hold that half of it: a neutral of the same weight clears every
//     number below, which is exactly what this block was until that
//     sentence. So the colour is asserted as a colour;
//   - the whole rail failing to paint, which is why nothing here is
//     asserted before a probe has proved it can see a glyph at all.
//
// Run: `npm run rail` (boots its own vite, needs system Chrome).
// No `pregen` hook on purpose: this measures paint and position, `gen`
// takes cargo's artifact lock, and a check that queues behind a build is
// a check nobody runs. A missing `src/gen` fails here as a dead probe
// rather than as a quiet pass — which is the only property that matters
// when the step is skipped.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));

// Its own port, not the dev server's 1430. Borrowing a running vite
// would be borrowing whatever source *that* one is serving, and a
// measurement of somebody else's tree reported as a measurement of this
// one is worse than no measurement. `--strictPort` makes the collision
// an error instead of a silent hop to the next port.
const PORT = Number(process.env.RAIL_PORT ?? 1449);

// The floor for the solid half of a glyph, against whatever it sits on.
const FLOOR = 3.0;
// How much heavier the open glyph has to be than a resting one.
//
// **This replaced `BLOCK_FLOOR` on 2026-08-31**, when the open item
// stopped wearing a block at all. That constant was 1.15 — a floor for
//「there is something there」on a plate nobody reads. The complaint it
// came from was「选中态几乎没有实体表达」and the answer was a block;
// the complaint that replaced it was「选中是整个 icon 变色,而不是
// icon 的背景变色」, so the same demand for a substantial open state
// now has to be met by the ink.
//
// **2.0 is a ratio of ratios, not a contrast**: the open glyph is
// asserted to clear a resting one by that margin, measured against the
// same ground. Today it clears by ~4.4 (13.12 against 8.76 dark,
// 13.60 against 6.59 light), so the margin is not tight — it is the
// level below which two items on one rail stop being distinguishable
// now that nothing else tells them apart.
const GLYPH_MARGIN = 2.0;
// And the outline's own floor: what it measured *before* the ink was
// given its own token (4.73 light, 5.51 dark), rounded down a step. The
// body's problem must not be paid for out of the layer that was already
// fine. Per theme, because the two were never at the same level — and a
// single number here would have let one theme fund the other.
const LINE_FLOOR = { light: 4.7, dark: 5.5 };

/** sRGB → relative luminance (WCAG 2.1). */
function luminance([r, g, b]) {
  const lin = (c) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function contrast(fg, bg) {
  const [a, b] = [luminance(fg), luminance(bg)].sort((x, y) => y - x);
  return (a + 0.05) / (b + 0.05);
}

/** `over` composited onto `under`, both `[r, g, b, a]`. */
function flatten(over, under) {
  const a = over[3];
  return [0, 1, 2].map((i) => over[i] * a + under[i] * (1 - a));
}

/**
 * The three forms this file actually meets, all to `[r, g, b, a]` in 0..255.
 *
 * Neither of the two extra branches is housekeeping — each was a wrong
 * number first:
 *
 *   `#rrggbb`   `--lq3` is read straight off the custom property to compare
 *               against, and the number regex below finds digits inside
 *               `#93b04d`, so a hex fell through as a colour rather than as
 *               an error.
 *   `color(srgb r g b / a)`  what Chrome hands back for a `color-mix`, with
 *               channels in **0..1**. Read on the 0..255 scale it is a
 *               near-black carrying the right alpha — which composites to a
 *               perfectly plausible ratio and asserts the wrong thing. It
 *               read 2.41 for a block that measures 1.30.
 */
function parse(css) {
  const t = String(css).trim();
  if (t.startsWith("#")) {
    const h = t.slice(1);
    const parts = h.length === 3 ? h.split("").map((c) => c + c) : [h.slice(0, 2), h.slice(2, 4), h.slice(4, 6)];
    return [...parts.map((c) => parseInt(c, 16)), 1];
  }
  const n = t.match(/-?[\d.]+/g);
  if (!n) throw new Error(`unreadable colour: ${css}`);
  const scale = t.startsWith("color(") ? 255 : 1;
  return [Number(n[0]) * scale, Number(n[1]) * scale, Number(n[2]) * scale, n[3] === undefined ? 1 : Number(n[3])];
}

/** The same paint, whatever alpha each of the two carries. */
const same = (a, b) => {
  const [x, y] = [parse(a), parse(b)];
  return [0, 1, 2].every((i) => Math.abs(x[i] - y[i]) < 1);
};

const fail = (m) => {
  console.error(m);
  process.exitCode = 1;
};

/**
 * Everything painted behind one element, and the element's own two
 * layers, read off the live page.
 *
 * The backdrop is walked rather than assumed: the rail paints no
 * background of its own, so what is behind a glyph is whichever
 * ancestor does — today the body, tomorrow whatever a layout change
 * puts in between. Naming a token here instead would make this gate
 * agree with itself rather than with the screen.
 */
const READ = (sel) => `(() => {
  const el = document.querySelector(${JSON.stringify(sel)});
  if (!el) return null;
  const layers = [];
  for (let n = el.parentElement; n; n = n.parentElement) {
    layers.push(getComputedStyle(n).backgroundColor);
  }
  const own = getComputedStyle(el);
  // Any opacity between the glyph and the page multiplies into both of
  // its layers, so it is collected rather than assumed to be 1.
  let dim = Number(own.opacity);
  for (let n = el.parentElement; n; n = n.parentElement) dim *= Number(getComputedStyle(n).opacity);
  return {
    fill: own.fill,
    fillOpacity: Number(own.fillOpacity),
    stroke: own.stroke,
    strokeOpacity: Number(own.strokeOpacity),
    dim,
    layers,
  };
})()`;

function measure(read) {
  // The backdrop, composited from the furthest ancestor inwards. White
  // is the seed only so a page that paints nothing at all still yields
  // a number; in practice `html`/`body` paint and it never shows.
  let backdrop = [255, 255, 255];
  for (const css of [...read.layers].reverse()) {
    const c = parse(css);
    if (c[3] > 0) backdrop = flatten(c, backdrop);
  }
  const ink = parse(read.fill);
  const line = parse(read.stroke);
  return {
    backdrop,
    // The product this whole gate exists for: the ink's own alpha, the
    // fill-opacity on top of it, and anything dimming the subtree.
    body: contrast(flatten([ink[0], ink[1], ink[2], ink[3] * read.fillOpacity * read.dim], backdrop), backdrop),
    line: contrast(flatten([line[0], line[1], line[2], line[3] * read.strokeOpacity * read.dim], backdrop), backdrop),
  };
}

/**
 * Flips the desktop's scheme **and waits for the face to wear it**.
 *
 * ⚠️ 批 D2 put the dark palette under `:root[data-theme="dark"]` rather
 * than a media query (`tokens.css` says why: the alternative was a
 * second copy of the whole palette). So the stamp is one JS hop behind
 * the emulation — `src/theme.ts`'s matchMedia listener re-stamping the
 * root — and a colour read straight after `emulateMedia` is the theme
 * being *left*.
 *
 * **This waits for the stamp and nothing else.** The settle each caller
 * keeps afterwards is a different wait for a different thing: these
 * glyphs have colour transitions, and a reading taken the instant the
 * palette changes is bit-for-bit the colour it is leaving (the same
 * lesson `smoke` §26 wrote down about an animating menu item). Folding
 * the two into one wait is how this gate went red measuring ink at 1.08
 * against a ground that had already turned.
 */
async function flipTheme(page, scheme) {
  await page.emulateMedia({ colorScheme: scheme });
  const deadline = Date.now() + 5_000;
  for (;;) {
    const worn = await page.evaluate(() =>
      document.documentElement.getAttribute("data-theme"),
    );
    if (worn === scheme) return;
    if (Date.now() >= deadline) {
      throw new Error(
        `the face never wore the ${scheme} scheme (data-theme=${JSON.stringify(worn)}) — ` +
          "批 D2's stamp is what repaints, and without it every colour below is the other theme's",
      );
    }
    await new Promise((r) => setTimeout(r, 50));
  }
}

const wait = (ms) => new Promise((r) => setTimeout(r, ms));
const say = (n) => n.toFixed(2);

const LANDINGS = ["sessions", "devices", "workspaces", "quota", "browser"];

let vite;
let browser;
try {
  vite = spawn("node_modules/.bin/vite", ["--port", String(PORT), "--strictPort"], {
    cwd: gui,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let gone = null;
  vite.on("exit", (code) => (gone = `vite exited with ${code} — is ${PORT} already taken?`));
  let ready = false;
  vite.stdout.on("data", (d) => (ready ||= String(d).includes(`:${PORT}`)));
  for (let i = 0; i < 100 && !ready; i++) {
    if (gone) throw new Error(gone);
    await wait(100);
  }
  if (!ready) throw new Error(`vite never came up on ${PORT}`);

  browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--no-proxy-server"] });
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
  const thrown = [];
  page.on("pageerror", (e) => thrown.push(String(e)));

  // `?bridge=` only to get past the web face's "no key" screen: this
  // measures paint and position, and every poll in this app swallows its
  // own errors, so a bridge that answers nothing still renders the rail.
  // Nothing here reads data — a gate that needed a whole mesh to measure
  // a colour would be a gate nobody runs.
  await page.goto(`http://localhost:${PORT}/?bridge=${PORT}`, { waitUntil: "load" });

  const item = (tab) => page.locator(`[data-rail-item][data-landing="${tab}"]`);
  // **The rail item that opens the mark's place**, whatever picture it
  // is wearing. It was the app's own logo at the head of the rail until
  // 2026-08-29 and it is this machine's face at the foot now; what this
  // gate measures — that it is a control, that it stands apart from the
  // landings — did not change with the picture, so neither did the
  // attribute or this locator.
  const mesh = page.locator("[data-rail-item]", { has: page.locator("[data-rail-mark]") });
  const bgOf = (loc) => loc.evaluate((el) => getComputedStyle(el).backgroundColor);
  const boxOf = async (loc) => {
    const b = await loc.boundingBox();
    if (!b || b.width === 0 || b.height === 0) throw new Error("probe dead: an element with no box");
    return b;
  };

  // ── 1) legibility, both themes ────────────────────────────────────
  for (const scheme of ["light", "dark"]) {
    await flipTheme(page, scheme);
    await wait(400);

    // The probe, before any assertion. An empty rail reads exactly like
    // a rail that passed: `querySelector` returns null just as honestly
    // for "renamed" as for "absent", and a measurement of nothing has no
    // failing value to report.
    const items = await page.locator("[data-rail-item]").count();
    const bodies = await page.locator("[data-rail-item] svg [data-body]").count();
    if (items < 5 || bodies < 4) {
      throw new Error(`probe dead in ${scheme}: ${items} rail items, ${bodies} filled glyphs`);
    }

    // A landing that is not the open one — the resting state is the one
    // that was unreadable, and the only one every glyph in the rail is
    // in most of the time.
    const at = await page.evaluate(READ('[data-rail-item][data-on="false"] svg [data-body]'));
    if (!at) throw new Error(`probe dead in ${scheme}: no unselected glyph to measure`);

    const m = measure(at);
    console.log(
      `${scheme}: body ${say(m.body)}, line ${say(m.line)} ` +
        `(on rgb(${m.backdrop.map((c) => Math.round(c)).join(", ")}))`,
    );

    if (m.body < FLOOR) {
      fail(`${scheme}: the filled half of a resting glyph is ${say(m.body)}, under ${FLOOR}`);
    }
    if (m.line < LINE_FLOOR[scheme]) {
      fail(`${scheme}: the outline fell to ${say(m.line)}, under the ${LINE_FLOOR[scheme]} it already had`);
    }
    // One colour at two depths. Without this the floor above is
    // satisfiable by deleting the idea it protects — a body pushed to
    // full opacity is maximally legible and says nothing.
    if (!(m.body < m.line)) {
      fail(`${scheme}: body ${say(m.body)} is not lighter than line ${say(m.line)} — the two densities are gone`);
    }
  }
  await flipTheme(page, "light");

  // ── 2) the way into the mesh is its own group, at the foot ────────
  //
  // Measured as positions, never as "it looks separated". The amount is
  // read off the page's own `--gap-group` rather than written here: the
  // claim is "the distance that was meant is the distance on screen",
  // and a number copied into this file would keep passing after somebody
  // changed the token.
  //
  // **This used to measure the head of the rail**, where the app's mark
  // sat above the landings; 「取而代之,用左下角的点击作为原来的 khor
  // 页面」 (the user, 2026-08-29) moved the door to the foot and gave it
  // this machine's face. The claim is the same sentence with one word
  // changed — *this is not a fifth landing* — so it is measured in the
  // same place it always was: the gap either side of the group.
  //
  // **And it is measured on both faces now, which it was not.** The
  // foot half used to be wide-only, with the reason written down: the
  // narrow rail had no face on it. That is no longer true, and a
  // wide-only check on the one door a phone has would be the exact
  // shape this repo keeps paying for — a ruling verified on one of two
  // screens showing one thing.
  const wanted = await page.evaluate(() =>
    Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--gap-group")),
  );
  if (!(wanted > 0)) throw new Error("probe dead: --gap-group is not a length");

  for (const [face, viewport] of [
    ["wide", { width: 1400, height: 900 }],
    // A phone, not merely "under the breakpoint". The narrow rail is a
    // row that has to hold everything at once, so it is the only place
    // the rail can run out of room — measuring it at a comfortable
    // width would be measuring the case that was never in danger.
    ["narrow", { width: 390, height: 720 }],
  ]) {
    await page.setViewportSize(viewport);
    await wait(300);
    const vertical = face === "wide";
    // Which way the rail runs decides which edge is a gap. Read off the
    // nav, not assumed from the viewport: the two are supposed to agree,
    // and a gate that assumed it would not notice the day they stop.
    const dir = await page.locator("nav").evaluate((el) => getComputedStyle(el).flexDirection);
    if (vertical !== dir.startsWith("column")) {
      throw new Error(`probe dead: the ${face} rail runs ${dir}`);
    }
    const near = (b) => (vertical ? b.y : b.x);
    const far = (b) => (vertical ? b.y + b.height : b.x + b.width);

    // Nothing in the rail has been squeezed to nothing, and the rail
    // does not overflow. Both halves matter and they fail together: a
    // row with no room left shrinks whatever has no intrinsic width
    // first, which here is the mark's `<img>` — it goes to zero and
    // takes the app's own icon off the screen **without overflowing**,
    // so a glance at the layout shows nothing wrong. It cost a whole
    // smoke run to find that once; it costs this much to keep.
    const flat = await page.locator("[data-rail-item] svg, [data-rail-item] img").evaluateAll((els) =>
      els
        .map((e) => ({ w: e.getBoundingClientRect().width, h: e.getBoundingClientRect().height }))
        .filter((b) => b.w < 1 || b.h < 1).length,
    );
    if (flat > 0) fail(`${face}: ${flat} glyph(s) in the rail have been shrunk to nothing`);
    const over = await page.locator("nav").evaluate((el) => el.scrollWidth - el.clientWidth);
    if (over > 0) fail(`${face}: the rail overflows its own box by ${over}`);

    // The seats this face actually shows: wide is every landing, narrow
    // is the first four and the「…」fold that holds the rest
    // (2026-09-02, App.tsx `NARROW_SHOWN`).
    const seats = vertical
      ? LANDINGS.map((tab) => item(tab))
      : [...LANDINGS.slice(0, 4).map((tab) => item(tab)), page.locator("[data-rail-more]")];
    const boxes = [];
    for (const seat of seats) boxes.push(await boxOf(seat));

    // Inside the group first: the seats at one distance. Without this,
    // "the mark is further away" would also be satisfied by a rail with
    // random gaps in it.
    const inside = [];
    for (let i = 1; i < boxes.length; i++) {
      inside.push(near(boxes[i]) - far(boxes[i - 1]));
    }
    const spread = Math.max(...inside) - Math.min(...inside);
    if (spread > 1) {
      fail(`${face}: the rail's seats are not evenly spaced — gaps ${inside.map((g) => g.toFixed(1))}`);
    }
    const within = Math.min(...inside);

    // The glyph just before the door: 设置 on the wide column, the
    // 「…」fold on the narrow bar (设置 lives inside it there).
    const settings = page.locator("[data-rail-item]:not([data-landing])", { hasNot: page.locator("[data-rail-mark]") });
    if ((await settings.count()) !== 1) throw new Error("probe dead: cannot find the glyph before the door");
    if ((await mesh.count()) !== 1) throw new Error("probe dead: no way into the mesh in the rail");
    // **It is wearing a face, and either form of one counts.** The
    // derived SVG when a backend answered, the blank when none did —
    // this gate deliberately runs without a mesh behind it, and the two
    // wear the same classes and occupy the same box. Asserted rather
    // than assumed: a door that lost its picture would still pass every
    // measurement below, since a box is a box.
    const wearsFace = await mesh.locator("[data-face], [data-face-blank]").count();
    if (wearsFace !== 1) {
      fail(`${face}: the way into the mesh is not wearing this machine's face`);
    }
    const gap = near(await boxOf(mesh)) - far(await boxOf(settings));
    console.log(
      `${face}: landings ${within.toFixed(1)}px apart, the mesh door ${gap.toFixed(1)}px past settings`,
    );
    // A tolerance of one pixel, not a fraction of the amount: the claim
    // is that a whole `--gap-group` was added, and subpixel layout is
    // the only thing allowed to eat into it.
    if (gap - within < wanted - 1) {
      fail(
        `${face}: the mesh door sits ${gap.toFixed(1)}px past settings, only ` +
          `${(gap - within).toFixed(1)}px more than the landings sit from each other — ${wanted}px was meant`,
      );
    }
  }
  await page.setViewportSize({ width: 1400, height: 900 });
  await wait(300);

  // ── 3) one way of saying "this is the open one" ───────────────────
  //
  // Five items, five presses, one reading. The mesh door is in the list
  // on purpose: it is the item that used to carry the only filled block
  // in the rail, and the one whose picture is not a stroke glyph — so
  // it is the one an "open" look drawn as a text colour would silently
  // skip.
  //
  // **Both themes, which this was not.** It ran on the light theme only
  // for as long as the block was neutral ink: the two themes were then
  // the same alpha of the same theme's own ink, and reading one told you
  // the other. The block is the brand green now (`tokens.css`), and
  // green is one colour rather than a mirrored pair — so the two themes
  // are no longer each other's reflection, and on the dark one the two
  // assertions below actually pull against each other: the block lifts
  // the ground the open glyph stands on, so making the block heavier
  // dims the glyph it is lighting. A ceiling that only exists on the
  // theme nobody measured is the shape this repo keeps paying for.
  const openable = [["mesh", mesh], ...LANDINGS.map((tab) => [tab, item(tab)])];
  const away = () => page.mouse.move(1200, 800);

  for (const scheme of ["light", "dark"]) {
    await flipTheme(page, scheme);
    await away();
    await wait(400);

    const resting = await bgOf(item("devices"));
    const lit = [];
    for (const [name, loc] of openable) {
      await loc.click();
      await wait(300);
      if ((await loc.getAttribute("data-on")) !== "true") {
        throw new Error(`probe dead in ${scheme}: pressing ${name} did not light it`);
      }
      // **Pointer away before reading.** A click leaves the pointer on
      // the item, so this used to read「open + hovered」and call it the
      // open block — invisible for as long as the open block outranked
      // hover, and the first thing to surface when the block went away.
      await away();
      await wait(200);
      lit.push([name, await bgOf(loc)]);
    }
    const [, first] = lit[0];
    console.log(`${scheme}: open ${first}   resting: ${resting}`);
    for (const [name, bg] of lit) {
      if (bg !== first) {
        fail(`${scheme}: ${name} says it is open with ${bg}, while the others say it with ${first}`);
      }
    }

    // **There is no block, and that is the assertion now** (2026-08-31).
    //
    // This section used to require one: that it be visible against the
    // ground, and that it be the brand green. Both are gone with it —
    // the person using this app said the icon should change, not the
    // icon's background, and green is legible as a plate and not as a
    // mark (`--lq3` on the light ground measures 2.22, under the 3.0
    // SC 1.4.11 asks of a graphic; the tombstone with the rest of the
    // numbers is on `--rail-on` in `tokens.css`).
    //
    // Asserted rather than merely dropped: a block creeping back is
    // exactly the shape of「someone reads this rail as needing one」,
    // and it would land on the very reading that was overruled.
    if (first !== resting) {
      fail(
        `${scheme}: the open item paints ${first} where a resting one paints ${resting} — ` +
          `the rail says open on the glyph now, not behind it`,
      );
    }

    // **So the whole statement is on the glyph, and this is what holds
    // it up.** Two depths of one colour: the line goes from `--rail-ink`
    // to `--foreground` and the body fills from `--icon-body` to solid.
    // Without a block to lean on, an open item that failed this would be
    // indistinguishable from its neighbours — there is nothing else left
    // to tell them apart.
    const openLine = measure(await page.evaluate(READ('[data-rail-item][data-on="true"] svg [data-body]')));
    const restLine = measure(await page.evaluate(READ('[data-rail-item][data-on="false"] svg [data-body]')));
    console.log(`${scheme}: open glyph line ${say(openLine.line)} against resting ${say(restLine.line)}`);
    if (!(openLine.line > restLine.line + GLYPH_MARGIN)) {
      fail(
        `${scheme}: the open glyph measures ${say(openLine.line)} against a resting ${say(restLine.line)} — ` +
          `under a ${GLYPH_MARGIN} margin, and it is the only thing saying which item is open`,
      );
    }

    // **Being pointed at and being open now live on different channels**
    // — the pointer paints the ground, open paints the glyph — so the
    // question is no longer "which block is heavier" but "does hovering
    // erase the answer". It must not: a person dragging the pointer
    // down the rail would otherwise lose track of where they are.
    await away();
    await wait(300);
    const cold = await bgOf(item("devices"));
    await item("devices").hover();
    await wait(300);
    const hovered = await bgOf(item("devices"));
    // The probe, before the claim: a resting item must answer a hover,
    // or "hovering changed nothing" below would mean nothing.
    if (hovered === cold) {
      throw new Error(
        `probe dead in ${scheme}: a resting glyph shows no hover, so nothing measured here means anything`,
      );
    }
    const hoveredLine = measure(
      await page.evaluate(READ('[data-rail-item][data-on="false"]:hover svg [data-body]')),
    );
    if (hoveredLine.line !== null && hoveredLine.line >= openLine.line) {
      fail(
        `${scheme}: a hovered resting glyph measures ${say(hoveredLine.line)} against the open ` +
          `${say(openLine.line)} — pointing at an item must not make it look like the open one`,
      );
    }

    // **And the open item still reads as open while it is pointed at.**
    //
    // The first spelling of this asserted that hovering leaves the open
    // glyph's number *unchanged*, and it failed: 13.60 → 12.20 light.
    // Nothing moved the ink — the hover wash moved the **ground the
    // glyph is measured against**, and a composite of the two is what
    // these numbers are. The assertion was on a different axis from the
    // risk: what a person can lose here is not a constant, it is the
    // ability to tell the open item from the others while dragging the
    // pointer down the rail.
    //
    // So it is the margin that has to survive, not the value.
    const openItem = openable[openable.length - 1][1];
    await openItem.hover();
    await wait(300);
    const openHovered = measure(
      await page.evaluate(READ('[data-rail-item][data-on="true"] svg [data-body]')),
    );
    if (openHovered.line === null) {
      fail(`${scheme}: the open glyph could not be read while hovered`);
    } else if (!(openHovered.line > restLine.line + GLYPH_MARGIN)) {
      fail(
        `${scheme}: hovered, the open glyph measures ${say(openHovered.line)} against a resting ` +
          `${say(restLine.line)} — under the ${GLYPH_MARGIN} margin, so pointing at it costs ` +
          `the one thing that says it is open`,
      );
    } else {
      console.log(
        `${scheme}: open glyph hovered ${say(openHovered.line)}, still clear of resting ${say(restLine.line)}`,
      );
    }
    await away();
  }
  await flipTheme(page, "light");

  if (thrown.length) fail(`the page threw while being measured:\n${thrown.join("\n")}`);
  if (!process.exitCode) console.log("rail gate: legible, grouped, and one way of being open");
} catch (e) {
  fail(String(e));
} finally {
  await browser?.close();
  vite?.kill();
}
