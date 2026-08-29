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
// And a floor for the open item's block against the same ground. Not a
// legibility threshold — nothing has to be *read* off a block — but the
// complaint that started this was "选中态几乎没有实体表达", and an
// assertion that the block merely exists is satisfied by an alpha of
// .002. This is the level below which there is nothing to see; the
// block measures 1.30 light / 1.61 dark.
const BLOCK_FLOOR = 1.15;
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

const wait = (ms) => new Promise((r) => setTimeout(r, ms));
const say = (n) => n.toFixed(2);

const LANDINGS = ["sessions", "devices", "files", "browser"];

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
    await page.emulateMedia({ colorScheme: scheme });
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
  await page.emulateMedia({ colorScheme: "light" });

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

    const boxes = {};
    for (const tab of LANDINGS) boxes[tab] = await boxOf(item(tab));

    // Inside the group first: four landings at one distance. Without
    // this, "the mark is further away" would also be satisfied by a rail
    // with four random gaps in it.
    const inside = [];
    for (let i = 1; i < LANDINGS.length; i++) {
      inside.push(near(boxes[LANDINGS[i]]) - far(boxes[LANDINGS[i - 1]]));
    }
    const spread = Math.max(...inside) - Math.min(...inside);
    if (spread > 1) {
      fail(`${face}: the four landings are not evenly spaced — gaps ${inside.map((g) => g.toFixed(1))}`);
    }
    const within = Math.min(...inside);

    const settings = page.locator("[data-rail-item]:not([data-landing])", { hasNot: page.locator("[data-rail-mark]") });
    if ((await settings.count()) !== 1) throw new Error("probe dead: cannot find the settings glyph");
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
    await page.emulateMedia({ colorScheme: scheme });
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
      lit.push([name, await bgOf(loc)]);
    }
    const [, first] = lit[0];
    console.log(`${scheme}: open ${first}   resting: ${resting}`);
    for (const [name, bg] of lit) {
      if (bg !== first) {
        fail(`${scheme}: ${name} says it is open with ${bg}, while the others say it with ${first}`);
      }
    }

    // The block is actually there to see. Measured against the ground
    // the rail sits on rather than trusted from its alpha, because the
    // alpha is only one of the two numbers that decide it.
    const ground = parse(await page.locator("body").evaluate((el) => getComputedStyle(el).backgroundColor));
    const block = contrast(flatten(parse(first), ground), ground);
    console.log(`${scheme}: open block ${say(block)} against the rail's ground`);
    if (block < BLOCK_FLOOR) {
      fail(`${scheme}: the open item's block measures ${say(block)} — under ${BLOCK_FLOOR}, there is nothing to see`);
    }

    // And *which* colour it is. Every number in this section is blind to
    // that: the neutral this replaced cleared them all, and so would any
    // grey of the same weight. The green is the thing the user asked for
    // and the thing a later "let's calm the rail down" would take away
    // without any of the floors noticing, so it is asserted as a colour
    // — read off `--lq3` rather than written here, because a literal
    // copied into this file would keep passing after the brand moved.
    // Alpha is not compared: how much of the green is painted is what
    // the floors above and below are for.
    const lq3 = await page.evaluate(() =>
      getComputedStyle(document.documentElement).getPropertyValue("--lq3").trim(),
    );
    if (!lq3) throw new Error(`probe dead in ${scheme}: --lq3 did not resolve`);
    if (!same(first, lq3)) {
      fail(`${scheme}: the open item's block paints ${first}, which is not the brand green ${lq3}`);
    }

    // And the other half of the expression: the open glyph is heavier
    // than a resting one. Without this the block could be carrying the
    // whole statement while the ink silently went to the brand green —
    // which is where this started, and which measures *worse* than a
    // resting glyph on the light theme. On the dark theme it is also
    // what caps the block: an almost-white glyph standing on a green
    // plate loses contrast as the plate gets heavier.
    const openLine = measure(await page.evaluate(READ('[data-rail-item][data-on="true"] svg [data-body]')));
    const restLine = measure(await page.evaluate(READ('[data-rail-item][data-on="false"] svg [data-body]')));
    console.log(`${scheme}: open glyph line ${say(openLine.line)} against resting ${say(restLine.line)}`);
    if (!(openLine.line > restLine.line)) {
      fail(
        `${scheme}: the open glyph measures ${say(openLine.line)}, no heavier than a resting ${say(restLine.line)}`,
      );
    }

    // Selection outranks the pointer, and the probe that proves the
    // reading is alive: an unselected glyph must answer a hover, or
    // "nothing changed" on the selected one would mean nothing.
    const [, open] = lit[lit.length - 1];
    await away();
    await wait(300);
    const cold = await bgOf(item("devices"));
    await item("devices").hover();
    await wait(300);
    if ((await bgOf(item("devices"))) === cold) {
      throw new Error(
        `probe dead in ${scheme}: a resting glyph shows no hover, so nothing measured here means anything`,
      );
    }
    const hovered = await bgOf(item("devices"));
    // Pointed at and open must not look alike. They are different facts,
    // and a rail where the mouse makes an item look open has two open
    // items in it for as long as the pointer is inside.
    const hoverBlock = contrast(flatten(parse(hovered), ground), ground);
    console.log(`${scheme}: hovered block ${say(hoverBlock)}, against the open ${say(block)}`);
    if (hovered === first) {
      fail(`${scheme}: hover paints ${hovered}, the very thing that means open`);
    }
    if (!(block > hoverBlock)) {
      fail(`${scheme}: the open block (${say(block)}) is no heavier than a hovered one (${say(hoverBlock)})`);
    }

    const openItem = openable[openable.length - 1][1];
    await openItem.hover();
    await wait(300);
    if ((await bgOf(openItem)) !== open) {
      fail(`${scheme}: hovering the open item changed its block — being open must outrank being pointed at`);
    }
    await away();
  }
  await page.emulateMedia({ colorScheme: "light" });

  if (thrown.length) fail(`the page threw while being measured:\n${thrown.join("\n")}`);
  if (!process.exitCode) console.log("rail gate: legible, grouped, and one way of being open");
} catch (e) {
  fail(String(e));
} finally {
  await browser?.close();
  vite?.kill();
}
