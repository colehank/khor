// The rail's legibility gate: what the glyphs actually measure against
// what they actually sit on, read out of a real browser.
//
// **Why this is not a static check.** Everything that decides whether a
// rail glyph can be seen is a product of things written in different
// files: an alpha on the ink (`tokens.css`), a `fill-opacity` on the
// solid piece (`app.css`), a utility class on the button (`App.tsx`),
// and whichever ancestor happens to be the last one that paints a
// background. Reading any one of them tells you nothing, and a script
// that multiplied them itself would be a second implementation of the
// cascade — green whenever *it* was right, rather than whenever the
// screen was. So this asks Chrome for the computed values and composites
// exactly what Chrome would put on the glass.
//
// The floor is 3.0, the ratio a non-text graphic needs against its own
// background to be made out at all. It was 2.30 light / 2.71 dark before
// this gate existed: the button wore an ink that already carried alpha
// and the body took a further .6 **of it**, so the two multiplied and
// the filled half of every glyph went to fog while its outline stayed
// crisp. Nothing was red. Nothing could be — no test in this repo, and
// no type, can see a colour.
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
//   - the whole rail failing to paint, which is why nothing here is
//     asserted before the probe has proved it can see a glyph at all.
//
// Run: `npm run contrast` (boots its own vite, needs system Chrome).
// No `pregen` hook on purpose: this measures paint, `gen` takes cargo's
// artifact lock, and a colour check that queues behind a build is a
// colour check nobody runs. A missing `src/gen` fails here as a dead
// probe rather than as a quiet pass — which is the only property that
// matters when the step is skipped.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));

// Its own port, not the dev server's 1430. Borrowing a running vite
// would be borrowing whatever source *that* one is serving, and a
// measurement of somebody else's tree reported as a measurement of this
// one is worse than no measurement. `--strictPort` makes the collision
// an error instead of a silent hop to the next port.
const PORT = Number(process.env.CONTRAST_PORT ?? 1449);

// The floor for the solid half of a glyph, against whatever it sits on.
const FLOOR = 3.0;
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

function parse(css) {
  const n = css.match(/-?[\d.]+/g);
  if (!n) throw new Error(`unreadable colour: ${css}`);
  return [Number(n[0]), Number(n[1]), Number(n[2]), n[3] === undefined ? 1 : Number(n[3])];
}

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
  // measures paint, and every poll in this app swallows its own errors,
  // so a bridge that answers nothing still renders the rail. Nothing
  // here reads data — a gate that needed a mesh to measure a colour
  // would be a gate nobody runs.
  await page.goto(`http://localhost:${PORT}/?bridge=${PORT}`, { waitUntil: "load" });

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
    const sel = '[data-rail-item][data-on="false"] svg [data-body]';
    const at = await page.evaluate(READ(sel));
    if (!at) throw new Error(`probe dead in ${scheme}: no unselected glyph to measure`);

    const m = measure(at);
    const say = (n) => n.toFixed(2);
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

  if (thrown.length) fail(`the page threw while being measured:\n${thrown.join("\n")}`);
  if (!process.exitCode) console.log("contrast gate: the rail is legible in both themes");
} catch (e) {
  fail(String(e));
} finally {
  await browser?.close();
  vite?.kill();
}
