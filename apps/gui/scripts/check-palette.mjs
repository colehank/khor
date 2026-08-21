// The palette's readability, measured in a real browser, both themes.
//
// **What this is really guarding is a rule, not four numbers.** The brand
// green used to hold two jobs — "this is the main action" and "this
// session is done" — and it could not do the first one at all:
// `#93b04d` has a relative luminance of .378, which caps its contrast
// against *pure white* at 2.45. White-on-green measured 2.45 and no
// choice of backdrop could have saved it, because there is no lighter
// backdrop than white. The same green was `--ring`, at 2.39 against the
// card, under the 3.0 a non-text graphic needs.
//
// So green gave up carrying text and gave up meaning "press me"
// (`tokens.css`, the doctrine at the top). This asks the browser whether
// that is still true, rather than whether anyone remembered it.
//
// Measured before the change, on the real page:
//
//                            light    dark
//   label on the button       2.45    10.15
//   ring against the card     2.39    10.22
//   ring against the page     2.22    11.14
//   ring against the button   1.00     1.00
//
// That last row is the one nobody had reported, and it is the reason
// this gate looks at the focus indicator's *geometry* and not only its
// colour. `--ring` and `--primary` were the same value in both themes,
// so a ring drawn flush against the primary button contrasted 1.00 with
// it — a keyboard user focusing the app's main action saw nothing, on
// the dark theme too, where every other number here was fine. Making the
// button ink did not create that problem and would not have fixed it;
// the offset on the outline is what fixes it (`ui/button.tsx`).
//
// What it blocks:
//   - the main action, or the focus ring, going back to a colour that
//     cannot carry what is drawn on it — asserted as ratios, and also
//     as "not the identity green", since that is the specific colour
//     this rule was written about;
//   - the focus indicator losing its gap, which is invisible in a
//     screenshot and invisible in a diff, and only shows up when a
//     keyboard user tabs onto the one button whose colour matches it;
//   - the tokens being right while nothing wears them: a real button on
//     a real screen is read back and compared against the tokens.
//
// Run: `npm run palette` (boots its own vite, needs system Chrome). No
// backend: the join dialog reaches a primary button with no mesh behind
// it, and this measures paint, not data.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright-core";

const gui = fileURLToPath(new URL("..", import.meta.url));
const PORT = Number(process.env.PALETTE_PORT ?? 1450);

/** A button's label is text, so it answers to the text threshold. */
const TEXT_FLOOR = 4.5;
/** A focus ring is a graphic. */
const GRAPHIC_FLOOR = 3.0;

/**
 * The six words, in the order the doctrine gives them.
 *
 * **Each of these is a text colour**, not a decoration: a row paints the
 * word itself with `var(--state-<word>)`, so they answer to the text
 * threshold like any other prose. 空闲 measured 2.88 until 批⑨b — the
 * quietest word in the app was also the one nobody could read, and these
 * six are the most valuable signal khor has.
 */
const WORDS = ["busy", "blocked", "done", "errored", "failed", "idle"];

/**
 * The two that sit still, and the four that want you.
 *
 * The split is the doctrine, not a grouping invented here: 空闲 is
 * nothing happening and 忙碌 is the machine working for you — neither
 * asks for anything. The other four each want a person to do something.
 */
const QUIET = ["idle", "busy"];

/**
 * How far apart the two groups must read, in L*.
 *
 * **In L\* and not in contrast ratio**, because contrast ratio is not
 * perceptually uniform — the same 0.5 means one thing near 4.5 and
 * another near 9 — and what is being asserted here is whether a person
 * can *see* a difference.
 *
 * 8 rather than 1: 1 is what two samples need when held side by side,
 * and these never are. They appear on separate rows with other content
 * between them, so the comparison is from memory, which needs several
 * times the threshold. 8 is about one step of a ten-step tonal ramp —
 * the smallest step that can be named without something to compare to.
 *
 * They spanned 3.0 in total before 批⑨b/⑨c: the whole palette on one
 * lightness, telling six things apart by hue alone.
 */
const GROUP_GAP = 8.0;

/**
 * The order, quietest first — asserted whole, not just at the ends.
 *
 * "空闲 is the faintest" only guards one end; a word pushed past its
 * neighbour in the middle would leave that assertion green while the
 * palette silently stopped meaning what it says.
 *
 * **Light only.** The dark theme's order is interleaved today — a quiet
 * word sits brighter than two loud ones — and that is a known open
 * decision, not something this file should freeze in place by asserting
 * it. Its gap is printed instead, so nobody has to rediscover it.
 */
const LIGHT_ORDER = ["idle", "busy", "errored", "done", "blocked", "failed"];

/** CIE L* of an already-composited colour. */
function lstar(rgb) {
  const y = luminance(rgb);
  return y > 216 / 24389 ? 116 * Math.cbrt(y) - 16 : (y * 24389) / 27;
}

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

function flatten(over, under) {
  const a = over[3];
  return [0, 1, 2].map((i) => over[i] * a + under[i] * (1 - a));
}

/** `#rgb`, `#rrggbb`, `rgb()` and `rgba()` — whichever form a token is written in. */
function parse(css) {
  const t = String(css).trim();
  if (t.startsWith("#")) {
    const h = t.slice(1);
    const parts = h.length === 3 ? h.split("").map((c) => c + c) : [h.slice(0, 2), h.slice(2, 4), h.slice(4, 6)];
    return [...parts.map((c) => parseInt(c, 16)), 1];
  }
  const n = t.match(/-?[\d.]+/g);
  if (!n || n.length < 3) throw new Error(`unreadable colour: ${css}`);
  return [Number(n[0]), Number(n[1]), Number(n[2]), n[3] === undefined ? 1 : Number(n[3])];
}

const rgb = (c) => parse(c).slice(0, 3);
const same = (a, b) => rgb(a).every((v, i) => Math.abs(v - rgb(b)[i]) < 1);

const fail = (m) => {
  console.error(m);
  process.exitCode = 1;
};
const say = (n) => n.toFixed(2);
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
  await page.goto(`http://localhost:${PORT}/?bridge=${PORT}`, { waitUntil: "load" });
  await wait(1200);

  // A real primary button, on a real screen, with nothing behind the
  // app. The join dialog is the shortest way to one that does not need a
  // machine to exist first.
  //
  // The field has to be filled: that button is disabled until a ticket
  // is typed, and **a disabled button cannot be focused**, so a keyboard
  // probe against the empty dialog silently measures nothing. The value
  // is never submitted — nothing here presses it.
  const join = page.locator("[data-join]");
  const openJoin = async () => {
    await page.locator('[data-rail-item][data-landing="devices"]').click();
    await wait(400);
    await page.locator("[data-pane-new]").click();
    await wait(400);
    const items = page.locator('[role="menuitem"]');
    if ((await items.count()) < 2) throw new Error("probe dead: the devices pane's + menu did not open");
    await items.last().click();
    await wait(600);
    if ((await join.count()) !== 1) throw new Error("probe dead: no primary button in the join dialog");
    await page.locator("[data-ticket-input]").fill("not-a-real-ticket");
    await wait(200);
    if (await join.isDisabled()) throw new Error("probe dead: the primary button stayed disabled");
  };

  for (const scheme of ["light", "dark"]) {
    await page.emulateMedia({ colorScheme: scheme });
    await wait(400);
    await openJoin();

    const t = await page.evaluate(() => {
      const s = getComputedStyle(document.documentElement);
      const g = (n) => s.getPropertyValue(n).trim();
      return {
        primary: g("--primary"),
        fg: g("--primary-foreground"),
        ring: g("--ring"),
        card: g("--card"),
        bg: g("--background"),
        lq3: g("--lq3"),
        accent: g("--accent"),
        words: Object.fromEntries(
          ["busy", "blocked", "done", "errored", "failed", "idle"].map((w) => [
            w,
            g(`--state-${w}`),
          ]),
        ),
        done: g("--state-done"),
      };
    });
    if (!t.primary || !t.ring || !t.lq3) throw new Error(`probe dead in ${scheme}: tokens did not resolve`);

    const label = contrast(rgb(t.fg), rgb(t.primary));
    const onCard = contrast(flatten(parse(t.ring), rgb(t.card)), rgb(t.card));
    const onPage = contrast(flatten(parse(t.ring), rgb(t.bg)), rgb(t.bg));
    const onButton = contrast(rgb(t.ring), rgb(t.primary));
    console.log(
      `${scheme}: label ${say(label)} | ring vs card ${say(onCard)}, page ${say(onPage)}, button ${say(onButton)}`,
    );

    if (label < TEXT_FLOOR) fail(`${scheme}: the main action's label reads ${say(label)}, under ${TEXT_FLOOR}`);
    if (onCard < GRAPHIC_FLOOR) fail(`${scheme}: the focus ring is ${say(onCard)} on a card, under ${GRAPHIC_FLOOR}`);
    if (onPage < GRAPHIC_FLOOR) fail(`${scheme}: the focus ring is ${say(onPage)} on the page, under ${GRAPHIC_FLOOR}`);

    // The rule itself, not just its arithmetic. A ratio can be satisfied
    // by any colour; this says *which* colour is out of a job, and it is
    // the one the whole decision was about.
    if (same(t.primary, t.lq3)) fail(`${scheme}: the main action is the identity green again (${t.primary})`);
    if (same(t.ring, t.lq3)) fail(`${scheme}: the focus ring is the identity green again (${t.ring})`);

    // ── the six words, every one of them, on both themes ──────────
    //
    // **Measured against the worst ground a word can land on**, not
    // against one chosen ground: the same colour is painted on a card in
    // the session list, on a selected row where the card is tinted, and
    // on the app's own paper. A word that clears 4.5 on the lightest of
    // those and fails on another is a word that is unreadable exactly
    // where it happens to be sitting.
    const grounds = [rgb(t.card), rgb(t.bg), flatten(parse(t.accent), rgb(t.card))];
    const readings = WORDS.map((w) => {
      const colour = t.words[w];
      if (!colour) throw new Error(`probe dead in ${scheme}: --state-${w} did not resolve`);
      return {
        word: w,
        worst: Math.min(...grounds.map((g) => contrast(flatten(parse(colour), g), g))),
      };
    });
    console.log(
      `${scheme}: ${readings.map((r) => `${r.word} ${say(r.worst)}`).join(", ")}`,
    );
    for (const r of readings) {
      if (r.worst < TEXT_FLOOR) {
        fail(`${scheme}: the word ${r.word} reads ${say(r.worst)} at worst, under ${TEXT_FLOOR}`);
      }
    }
    // **Readable and still the quietest are two requirements.** Raising
    // 空闲 until it passed would have been half the job; a fix that also
    // made it louder than 忙碌 would have traded the doctrine away to
    // satisfy a number.
    const faintest = readings.reduce((a, b) => (a.worst <= b.worst ? a : b));
    if (faintest.word !== "idle") {
      fail(`${scheme}: ${faintest.word} is now fainter than 空闲 — the six words changed order`);
    }

    // How far the quiet pair sits from the four that want you, measured
    // on the ground where they sit closest.
    const worstGround = grounds.reduce((a, b) =>
      contrast(flatten(parse(t.words.idle), a), a) <= contrast(flatten(parse(t.words.idle), b), b) ? a : b,
    );
    const lightnessOf = (w) => lstar(flatten(parse(t.words[w]), worstGround));
    const quiet = QUIET.map(lightnessOf);
    const loud = WORDS.filter((w) => !QUIET.includes(w)).map(lightnessOf);
    // Light darkens the loud ones, dark brightens them, so the gap is
    // signed by theme; take it in the direction that theme moves.
    const gap =
      scheme === "light"
        ? Math.min(...quiet) - Math.max(...loud)
        : Math.min(...loud) - Math.max(...quiet);
    console.log(`${scheme}: group gap ΔL* ${gap.toFixed(1)}`);

    if (scheme === "light") {
      if (gap < GROUP_GAP) {
        fail(
          `light: the quiet pair is only ΔL* ${gap.toFixed(1)} from the four that want you, under ` +
            `${GROUP_GAP} — a difference that has to be compared side by side to be seen`,
        );
      }
      const order = [...readings].sort((a, b) => a.worst - b.worst).map((r) => r.word);
      if (order.join() !== LIGHT_ORDER.join()) {
        fail(`light: the six words read ${order.join(" < ")}, not ${LIGHT_ORDER.join(" < ")}`);
      }
    } else if (gap < GROUP_GAP) {
      // Printed, never failed: the dark theme was deliberately left
      // alone (its own decision, not this batch's), and a red gate for a
      // state nobody was asked to change is a gate people learn to skip.
      console.log(`dark: (interleaved — a quiet word outshines a loud one; open decision, untouched)`);
    }

    // The one post green kept that still touches text: the word 完成,
    // painted straight onto the row through `var(--state-done)`. The
    // doctrine allows that and allows nothing else, so this is the
    // boundary of the rule rather than a breach of it — and a boundary
    // is only worth writing down if crossing it goes red. A word answers
    // to the text threshold like any other.
    const done = Math.min(
      contrast(flatten(parse(t.done), rgb(t.card)), rgb(t.card)),
      contrast(flatten(parse(t.done), rgb(t.bg)), rgb(t.bg)),
    );
    console.log(`${scheme}: 完成 reads ${say(done)} at worst`);
    if (done < TEXT_FLOOR) {
      fail(`${scheme}: 完成 reads ${say(done)}, under ${TEXT_FLOOR} — the one green that may still be read cannot be`);
    }

    // Tokens are worth nothing if nothing wears them.
    const worn = await join.evaluate((el) => {
      const s = getComputedStyle(el);
      return { bg: s.backgroundColor, color: s.color };
    });
    if (!same(worn.bg, t.primary)) fail(`${scheme}: the primary button paints ${worn.bg}, not ${t.primary}`);
    if (!same(worn.color, t.fg)) fail(`${scheme}: its label paints ${worn.color}, not ${t.fg}`);

    // The focus indicator, by keyboard — `.focus()` from a script does
    // not arm `:focus-visible`, so a ring measured that way would be a
    // ring nobody can get to.
    let landed = false;
    for (let i = 0; i < 30 && !landed; i++) {
      await page.keyboard.press("Tab");
      landed = await page.evaluate(() => document.activeElement?.matches("[data-join]") ?? false);
    }
    if (!landed) throw new Error(`probe dead in ${scheme}: Tab never reached the primary button`);

    const ind = await join.evaluate((el) => {
      const s = getComputedStyle(el);
      return {
        color: s.outlineColor,
        style: s.outlineStyle,
        width: Number.parseFloat(s.outlineWidth),
        offset: Number.parseFloat(s.outlineOffset),
      };
    });
    console.log(`${scheme}: focus ${ind.style} ${ind.width}/${ind.offset} in ${ind.color}`);

    if (ind.style === "none" || !(ind.width >= 2)) {
      fail(`${scheme}: the focused main action draws ${ind.style} ${ind.width} — there is no indicator`);
    }
    if (!same(ind.color, t.ring)) fail(`${scheme}: the focus indicator is ${ind.color}, not --ring ${t.ring}`);
    // The gap. Without it the indicator sits flush against a button of
    // its own colour and disappears — which is what `ring vs button`
    // above measures, and why that number is allowed to be 1.00 only
    // while this offset is real.
    if (!(ind.offset > 0) && onButton < GRAPHIC_FLOOR) {
      fail(
        `${scheme}: the focus indicator has no gap and contrasts ${say(onButton)} with the button it surrounds ` +
          `— nothing separates them, so focus is invisible on the app's main action`,
      );
    }

    // Shut, so the next theme starts from the same place rather than
    // from a page still holding focus.
    await page.keyboard.press("Escape");
    await wait(300);
  }

  if (thrown.length) fail(`the page threw while being measured:\n${thrown.join("\n")}`);
  if (!process.exitCode) console.log("palette gate: the main action and the focus ring can both be seen");
} catch (e) {
  fail(String(e));
} finally {
  await browser?.close();
  vite?.kill();
}
