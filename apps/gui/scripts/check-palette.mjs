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
//     a real screen is read back and compared against the tokens;
//   - (2026-10-02, 一张底加灰线) a second ground coming back — the list
//     column, the content or the terminal on a colour other than the one
//     ground; what floats losing its step off the ground; the glass not
//     reading as glass on it, or its chosen piece not going the way the
//     owner set (lighter in light, the ground's black in dark); the
//     hairline that now does every separating vanishing into the ground;
//     the selected row's grey block not standing off it; any ink falling
//     under its floor on the ground, a float, the glass or the chosen
//     glass; a picked row in a floating list keeping a second ink.
//
// Run: `npm run palette` (boots its own vite, needs system Chrome). No
// backend: 添加机器 reaches a primary button with no mesh behind
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

/*
 * ⚠️ 墓碑 (UX-5): FRAME_STEP (6 L* between 「the paper」 and 「the frame」)
 * and SIDE_STEP (3 L* between the left block and each) measured the
 * three grounds of 2026-09-30 「板是纸底是框」. The redesign has one ground
 * (2026-10-02, memory `khor-one-ground-redesign`), so there is no step
 * between grounds left to measure; what separates things now is the
 * hairline, the glass and the selected row's grey, measured below.
 */
/**
 * **The hairline has to be seen on the ground** (contrast, not L*: it is
 * a line against what it lies on). It carries no meaning, so not 3:1;
 * 1.2 is under both themes' 1.27 / 1.31 and over a line nobody sees.
 */
const HAIR_FLOOR = 1.2;
/**
 * **The selected row's block and the glass stand off the ground** (L*).
 * The row's grey measures 7.7 light, 14.5 dark; the glass over the
 * ground 6.1 / 10.6. 4 is the float's own step: a surface a person sees
 * as a surface.
 */
const SURFACE_STEP = 4;
/**
 * What floats lands on the paper and on the frame, and has to be told
 * from both. On the frame by lightness in both themes; on the paper by
 * lightness in the dark only — a light popover is the paper's own white
 * and `--shadow-float` lifts it, where on black paper a black shadow is
 * a darkening nobody sees (2026-09-28, D14).
 */
const FLOAT_STEP = 4;

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
 * **Both themes, one order.** It was light-only while the dark theme was
 * still interleaved — a quiet word brighter than two loud ones — and
 * this file printed that rather than asserting it, because a gate that
 * goes red over something nobody was asked to change is a gate people
 * learn to skip. That was fixed (#96b), so the exception is gone with
 * the state that justified it.
 */
const ORDER = ["idle", "busy", "errored", "done", "blocked", "failed"];

/**
 * The words a person finds by *colour*. 空闲 is the one that is not one
 * of these: nothing is happening, and a neutral is the honest picture of
 * that.
 *
 * 忙碌 joined this list on 2026-08-30. Before that it was the app's
 * neutral and this rule had a written exception — which meant the most
 * frequently shown word of the six was the one you had to identify by
 * ruling the others out.
 */
const HUED = ["busy", "blocked", "done", "errored", "failed"];
/**
 * Where a colour stops being a colour. The five above measure .077–.158
 * (light) and .085–.120 (dark); the app's neutrals measure .007. There
 * is nothing in between to argue about, so the floor sits where the two
 * populations are furthest apart rather than at a threshold anybody
 * would have to defend.
 */
const CHROMA_FLOOR = 0.05;
/**
 * How far apart two of them have to be. The tight pair is 完成/中断 at
 * 30° on the dark theme and predates this claim — the floor is under
 * that on purpose, so this gate reports the rule it was written for
 * (忙碌's slot, 57° from its nearest neighbour) rather than an old pair
 * nobody was asked to move.
 */
const HUE_APART = 25;

/** OKLCH of an already-composited colour — the space this palette's own
    decisions are made in (`tokens.css`, the six words). */
function oklch(rgb) {
  const f = (c) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  const [R, G, B] = rgb.map(f);
  const l = Math.cbrt(0.4122214708 * R + 0.5363325363 * G + 0.0514459929 * B);
  const m = Math.cbrt(0.2119034982 * R + 0.6806995451 * G + 0.1073969566 * B);
  const s = Math.cbrt(0.0883024619 * R + 0.2817188376 * G + 0.6299787005 * B);
  const a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const b = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  return { c: Math.hypot(a, b), h: (Math.atan2(b, a) * 180) / Math.PI + (b < 0 ? 360 : 0) };
}

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
  // app. 添加机器's SSH 主机 page is the shortest way to one that does not need a
  // machine to exist first.
  //
  // The field has to be filled: that button is disabled until a ticket
  // is typed, and **a disabled button cannot be focused**, so a keyboard
  // probe against the empty dialog silently measures nothing. The value
  // is never submitted — nothing here presses it.
  const join = page.locator("[data-add-go]");
  const openJoin = async () => {
    // Reach devices as a *switch*, never a re-click: pressing the open
    // landing's own glyph now collapses its list (2026-09-02「再点收起」,
    // App.tsx), and this helper runs once per scheme — the second time
    // devices is already open. A hop through sessions makes devices a
    // fresh switch every time, which always leaves the list expanded.
    await page.locator('[data-rail-item][data-landing="sessions"]:visible').click();
    await wait(200);
    await page.locator('[data-rail-item][data-landing="devices"]:visible').click();
    await wait(400);
    // 添加机器… is the machines landing's one act, so ＋ is it (round
    // join-d); its SSH 主机 page holds the dialog's primary 添加.
    await page.locator("[data-side-head] [data-pane-new]").click();
    await wait(400);
    const ssh = page.locator('[data-add-machine] [data-add-row="ssh"]');
    if ((await ssh.count()) !== 1) throw new Error("probe dead: 添加机器 did not open on the machines landing's ＋");
    await ssh.click();
    await wait(600);
    if ((await join.count()) !== 1) throw new Error("probe dead: no primary button on 添加机器's SSH 主机 page");
    if (await join.isDisabled()) throw new Error("probe dead: the primary button is disabled");
  };

  for (const scheme of ["light", "dark"]) {
    await flipTheme(page, scheme);
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
        // The left block's own ground (docs/SHELL.md 第十三轮): the rail
        // and every list sit on it, so the row's title, its second line
        // and the selected row's tint are all read against it.
        side: g("--side"),
        // What floats (添加机器 open right now is one), and the
        // red a refusal inside one is written in.
        popover: g("--popover"),
        // A segmented control's track and its chosen face
        // (components/SegmentedControl, 2026-09-28).
        fill: g("--fill"),
        segment: g("--segment"),
        destructive: g("--destructive"),
        dialog: getComputedStyle(document.querySelector('[data-slot="dialog-content"]') ?? document.body).backgroundColor,
        ink: g("--foreground"),
        inkMuted: g("--muted-foreground"),
        tertiary: g("--tertiary"),
        lq3: g("--lq3"),
        accent: g("--accent"),
        fillStrong: g("--fill-strong"),
        // The one ground's separating line, and the glass (the
        // redesign): the glass controls' fill and the chosen piece.
        border: g("--border"),
        glass: g("--glass"),
        glassSel: g("--glass-sel"),
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

    // ── one ground, and what stands on it (the redesign, 2026-10-02) ──
    //
    // ⚠️ 墓碑: this held 「板是纸底是框」 (2026-09-30) — the paper at the end
    // of the range, the frame a step in, the left block between, each a
    // step off the others — and before that the left block's own third
    // ground (第十三轮). The redesign decided one ground: the list column,
    // the content and the paper are one colour, and what is held now is
    // that they stay one, and that the things that separate them on it
    // can be seen: the hairline, the glass, the chosen glass, the
    // selected row's grey, and what floats.
    {
      const ground = rgb(t.bg);
      const L = {
        ground: lstar(ground),
        float: lstar(rgb(t.popover)),
        glass: lstar(flatten(parse(t.glass), ground)),
        chosen: lstar(flatten(parse(t.glassSel), ground)),
        row: lstar(flatten(parse(t.fillStrong), ground)),
      };
      const hair = contrast(flatten(parse(t.border), ground), ground);
      console.log(
        `${scheme}: L* ground ${say(L.ground)}, float ${say(L.float)}, glass ${say(L.glass)}, chosen glass ${say(L.chosen)}, ` +
          `selected row ${say(L.row)}; hairline ${say(hair)} on the ground`,
      );
      if (!same(t.card, t.bg) || !same(t.side, t.bg)) {
        fail(`${scheme}: --card ${t.card}, --side ${t.side} and --background ${t.bg} are not one ground`);
      }
      if (scheme === "dark" && L.float - L.ground < FLOAT_STEP) {
        fail(`dark: --popover ${t.popover} is ${say(L.float - L.ground)} L* over the ground ${t.bg} — the lightest thing in a dark window is what floats, ${FLOAT_STEP} over it`);
      }
      if (scheme === "light" && L.float < L.ground) {
        fail(`light: --popover ${t.popover} is darker than the ground ${t.bg} — what floats is lifted, never sunk`);
      }
      if (hair < HAIR_FLOOR) fail(`${scheme}: the hairline --border reads ${say(hair)} on the ground, under ${HAIR_FLOOR} — the one thing that separates`);
      if (Math.abs(L.row - L.ground) < SURFACE_STEP) {
        fail(`${scheme}: the selected row's grey (--fill-strong) is ${say(Math.abs(L.row - L.ground))} L* off the ground, under ${SURFACE_STEP}`);
      }
      if (Math.abs(L.glass - L.ground) < SURFACE_STEP) {
        fail(`${scheme}: the glass (--glass) is ${say(Math.abs(L.glass - L.ground))} L* off the ground, under ${SURFACE_STEP}`);
      }
      // **The chosen piece of glass** (the owner, 2026-10-02): in light it
      // turns white and lifts, brighter than the glass; in dark it is the
      // content's black showing through, darker than the glass.
      if (scheme === "light" ? L.chosen <= L.glass : L.chosen >= L.glass) {
        fail(`${scheme}: the chosen glass (--glass-sel ${t.glassSel}) is not ${scheme === "light" ? "lighter" : "darker"} than the glass — the owner's rule for which way a choice goes`);
      }
    }

    // ── every ink on every ground ─────────────────────────────────
    //
    // The design system's pairs (README 颜色「可访问性例外」), measured on
    // what a word can stand on since the redesign: the ground, a float,
    // the glass (the nav's words, the pill's title) and the chosen glass.
    // Text answers to 4.5, the ring to 3.0; `tertiary` is printed and not
    // held — it is the one written exception, and its lowest reading is
    // what the README says. A selected row's words are its own section.
    {
      const ground = rgb(t.bg);
      const grounds = {
        ground,
        float: rgb(t.popover),
        glass: flatten(parse(t.glass), ground),
        "chosen glass": flatten(parse(t.glassSel), ground),
      };
      const inks = { ink: t.ink, secondary: t.inkMuted, destructive: t.destructive, tertiary: t.tertiary, ring: t.ring };
      for (const [name, ink] of Object.entries(inks)) {
        const got = Object.entries(grounds).map(([g, under]) => [g, contrast(flatten(parse(ink), under), under)]);
        console.log(`${scheme}: ${name} on ${got.map(([g, v]) => `${g} ${say(v)}`).join(", ")}`);
        if (name === "tertiary") continue;
        const floor = name === "ring" ? GRAPHIC_FLOOR : TEXT_FLOOR;
        for (const [g, v] of got) {
          if (v < floor) fail(`${scheme}: ${name} reads ${say(v)} on the ${g} (${name === "ring" ? "a graphic" : "text"} needs ${floor})`);
        }
      }
    }

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

    // ── what floats: a step above the board on the dark theme ──────
    //
    // A dialog, a sheet, a menu and the search results wear `--popover`.
    // On the light theme that is the paper's value and `--shadow-float`
    // lifts it; on the dark one a shadow is black on black paper, and a
    // popover at the board's value sat flush on it — the same colour,
    // nothing to say it was in front (2026-09-28). So there it must be
    // lighter than the paper by a step a person sees: 4 L*
    // (`FLOAT_STEP`; its step off the frame is asserted above). And
    // being lighter must not cost the words on it — body, second line and
    // a refusal in `--destructive` — the text floor.
    if (!t.popover || !t.destructive) {
      fail(`${scheme}: --popover or --destructive does not resolve (src/styles/tokens.css)`);
    } else {
      const pop = rgb(t.popover);
      const lift = lstar(pop) - lstar(rgb(t.card));
      const onPop = {
        ink: contrast(flatten(parse(t.ink), pop), pop),
        secondary: contrast(flatten(parse(t.inkMuted), pop), pop),
        destructive: contrast(flatten(parse(t.destructive), pop), pop),
      };
      console.log(
        `${scheme}: on --popover ${t.popover} (${say(lift)} L* over --card): ` +
          Object.entries(onPop).map(([k, v]) => `${k} ${say(v)}`).join(", ") +
          ` | the open dialog wears ${t.dialog}`,
      );
      for (const [what, ratio] of Object.entries(onPop)) {
        if (ratio < TEXT_FLOOR) fail(`${scheme}: ${what} reads ${say(ratio)} on --popover, under ${TEXT_FLOOR}`);
      }
      if (scheme === "dark" && lift < FLOAT_STEP) {
        fail(
          `dark: --popover ${t.popover} is ${say(lift)} L* over --card ${t.card} — a shadow cannot lift ` +
            "anything on a ground this dark, so what floats is lighter than the board by 4 or more",
        );
      }
      if (!same(t.dialog, t.popover)) {
        fail(`${scheme}: the open dialog is painted ${t.dialog}, not --popover ${t.popover} (components/ui/dialog.tsx)`);
      }
    }

    // ── the chosen segment is a face above its track ──────────────
    //
    // A segmented control is a `--fill` track with the chosen segment a
    // `--segment` face in it (components/SegmentedControl). **The face
    // has to be lighter than the track on both themes**, or the chosen
    // one reads as a hole — which is what `--card` does on the dark
    // theme, where the card is darker than a track laid over a sheet,
    // and why the face has a token of its own. Measured on the two
    // grounds the control stands on: a sheet (`--popover`) and the
    // board (`--card`). 4 L* is the step that reads as lifted
    // (`FLOAT_STEP`), and the words on the face are text.
    if (!t.fill || !t.segment) {
      fail(`${scheme}: --fill or --segment does not resolve (src/styles/tokens.css)`);
    } else {
      for (const [where, ground] of [["a sheet", t.popover], ["the board", t.card]]) {
        const track = flatten(parse(t.fill), rgb(ground));
        const face = flatten(parse(t.segment), track);
        const up = lstar(face) - lstar(track);
        const ink = contrast(flatten(parse(t.ink), face), face);
        console.log(`${scheme}: segmented control on ${where}: face ${say(up)} L* over its track, ink ${say(ink)} on it`);
        if (!(up >= 4)) {
          fail(
            `${scheme}: on ${where} the chosen segment (--segment ${t.segment}) is ${say(up)} L* over its ` +
              "track — under 4 it does not read as a face lifted out of the track (components/SegmentedControl)",
          );
        }
        if (ink < TEXT_FLOOR) fail(`${scheme}: the chosen segment's words read ${say(ink)} on ${where}, under ${TEXT_FLOOR}`);
      }
    }

    // ── the six words, every one of them, on both themes ──────────
    //
    // **Measured against the worst ground a word can land on**, not
    // against one chosen ground: the same colour is painted on a session
    // row in the left block, on the paper (the overview, a conversation),
    // on the frame (the window's foot), in a floating list, and behind
    // the brand wash on the paper. A word that clears 4.5 on the
    // lightest of those and fails on another is a word that is
    // unreadable exactly where it happens to be sitting. (The left block
    // and the popover joined on 2026-09-30; the light block's own 4.52
    // was on the floor and nothing here read it.)
    const grounds = [
      rgb(t.card),
      rgb(t.bg),
      flatten(parse(t.side), rgb(t.bg)),
      rgb(t.popover),
      // ⚠️ The brand wash over the paper stood here while a selected row
      // wore it; a selected row is a grey block since 2026-09-27 and its
      // words take `--foreground` (app.css), so the wash carries no state
      // word any more.
    ];
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

    if (gap < GROUP_GAP) {
      fail(
        `${scheme}: the quiet pair is only ΔL* ${gap.toFixed(1)} from the four that want you, under ` +
          `${GROUP_GAP} — a difference that has to be compared side by side to be seen` +
          (gap < 0 ? " (negative: the two groups are interleaved)" : ""),
      );
    }
    const order = [...readings].sort((a, b) => a.worst - b.worst).map((r) => r.word);
    if (order.join() !== ORDER.join()) {
      fail(`${scheme}: the six words read ${order.join(" < ")}, not ${ORDER.join(" < ")}`);
    }

    // ── hue is what says *which* word ─────────────────────────────
    //
    // The lightness claims above hold the two groups apart — whether a
    // word wants you. Nothing above holds the other axis, and until
    // 2026-08-30 nothing had to: 忙碌 was the app's neutral, so「five
    // words are found by hue」was five-sixths of a rule with a written
    // exception. It has a hue now, and the exception is gone.
    //
    // **What this catches that nothing else does: 忙碌 going back to
    // grey.** Every claim above stays green through that — a neutral
    // passes the floor, keeps its place in the order, and sits in the
    // quiet band. The word simply stops being findable except by
    // elimination, and no number moves.
    const chromas = Object.fromEntries(
      [...HUED, ...QUIET.filter((w) => w !== "busy")].map((w) => [
        w,
        oklch(flatten(parse(t.words[w]), rgb(t.card))),
      ]),
    );
    console.log(
      `${scheme}: hues ${HUED.map((w) => `${w} ${chromas[w].h.toFixed(0)}°/C${chromas[w].c.toFixed(3)}`).join(", ")}`,
    );
    // The probe, and it is the one this claim cannot do without: if the
    // chroma reading cannot tell the app's neutral apart from its
    // colours, then「all five carry a hue」is a sentence about a
    // function that answers the same thing to everything.
    if (!(chromas.idle.c < CHROMA_FLOOR)) {
      fail(
        `${scheme}: probe dead — 空闲 measures chroma ${chromas.idle.c.toFixed(3)}, at or above the ` +
          `${CHROMA_FLOOR} floor meant to separate a neutral from a colour, so the hue claim below ` +
          `cannot tell them apart either`,
      );
    } else {
      for (const w of HUED) {
        if (chromas[w].c < CHROMA_FLOOR) {
          fail(
            `${scheme}: ${w} carries chroma ${chromas[w].c.toFixed(3)}, under ${CHROMA_FLOOR} — it is a ` +
              `neutral, and a word without a hue is a word found by elimination`,
          );
        }
      }
      // And no two of them close enough to be mistaken. The tight pair
      // is 完成/中断 at 30° on the dark theme, which predates this and is
      // why the floor is 25 rather than the 48 the light theme happens
      // to have.
      //
      // **Only among the ones that actually carry a colour.** A
      // near-neutral has a hue angle the way noise has a direction, and
      // reporting「忙碌 sits 19° from 完成」about a grey would send
      // somebody to move 完成. The chroma failure above already says the
      // true thing about that word; this stays quiet rather than adding
      // a red that points at the wrong token (the false-red rule: a gate
      // that misreports is a gate people turn off).
      const coloured = HUED.filter((w) => chromas[w].c >= CHROMA_FLOOR);
      for (let i = 0; i < coloured.length; i++) {
        for (let j = i + 1; j < coloured.length; j++) {
          const d = Math.abs(chromas[coloured[i]].h - chromas[coloured[j]].h);
          const apart = Math.min(d, 360 - d);
          if (apart < HUE_APART) {
            fail(
              `${scheme}: ${coloured[i]} and ${coloured[j]} sit ${apart.toFixed(0)}° apart in hue, under ` +
                `${HUE_APART}° — hue is the only thing saying which word this is`,
            );
          }
        }
      }
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
      landed = await page.evaluate(() => document.activeElement?.matches("[data-add-go]") ?? false);
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


  // ── the selected row (docs/SHELL.md 第十五轮 第 2 条, D6) ──────────
  //
  // 「选中的列表行是实心品牌深绿、白字；窗口不在前台时变灰」. Every piece
  // of text in a real selected row — its title, its second line, whatever
  // stands at its right end — is read against the row's own ground, both
  // themes, with the window in front and behind. **Full value, no
  // opacity**: white on `#5d7b06` at 0.92 is 4.42, so any fade on the way
  // down to a letter is itself a red (docs/design-system ListRow).
  //
  // Two session rows are routed in for it: this gate has no backend, and
  // a list with nothing in it has no selected row to measure. The second
  // one's word is 忙碌, so a state word stands in the row too.
  {
    const now = Date.now();
    const sRow = (n, word) => ({
      id: `tui/palette-gate-${n}`,
      kind: "tui",
      title: n === 4 ? "a long session title that must truncate before its unread mark" : `palette-gate-${n}`,
      word,
      at_ms: now,
      unread: n === 0 ? 0 : n === 2 ? 3 : 1,
      source: null,
      pinned: false,
      stale: false,
      category: "shell",
      group: "",
      face: null,
      hosted: false,
      attachable: false,
      last: "the second line of a selected row",
      via: null,
      via_face: null,
      home: "gate",
      place: "/home/gate/work",
      mux: false,
    });
    await page.route("**/sessions", (r) =>
      r.fulfill({ contentType: "application/json", body: JSON.stringify([...WORDS.map((w, n) => sRow(n, w)), { ...sRow(6, ""), category: "codex", unread: 0, last: null, doing: { thread_unbound: true }, at_ms: 0 }]) }),
    );
    // **Motion off**: 忙碌 breathes (its opacity walks while the machine
    // works), and a breath is motion, not a fade that ranks one line
    // under another — which is what 「no opacity」 is about. With the
    // desktop asking for less motion it holds still at full value.
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.reload({ waitUntil: "load" });
    await wait(1200);
    await page.locator('[data-rail-item][data-landing="devices"]:visible').click();
    await wait(200);
    await page.locator('[data-rail-item][data-landing="sessions"]:visible').click();
    await wait(400);
    const opener = page.locator("[data-list] [data-row-open]").first();
    if ((await opener.count()) === 0) throw new Error("probe dead: the routed session rows drew no [data-row-open] — no selected row to measure");
    await opener.click();
    await wait(500);
    const read = () =>
      page.evaluate(() => {
        const row = document.querySelector('[data-side-row][data-selected="true"]');
        if (!row) return null;
        const bgOf = (el) => {
          const out = [];
          for (let e = el; e; e = e.parentElement) {
            const c = getComputedStyle(e).backgroundColor;
            if (c && c !== "rgba(0, 0, 0, 0)" && c !== "transparent") out.push(c);
          }
          return out;
        };
        const texts = [...row.querySelectorAll("*")]
          .filter((el) => [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim() !== ""))
          .filter((el) => el.getBoundingClientRect().width > 0)
          .map((el) => {
            let fade = 1;
            for (let e = el; e && e !== row.parentElement; e = e.parentElement) fade *= Number(getComputedStyle(e).opacity);
            return { text: el.textContent.trim().slice(0, 24), color: getComputedStyle(el).color, grounds: bgOf(el), fade };
          });
        const status = row.querySelector('[data-status-word]');
        const word = status?.getAttribute('data-status-word');
        const root = getComputedStyle(document.documentElement);
        return {
          grounds: bgOf(row), texts, key: document.documentElement.getAttribute("data-window-key"),
          word, status: status && getComputedStyle(status).color,
          stateInk: root.getPropertyValue(`--state-${word}`).trim(),
        };
      });

    /** One reading of the selected row, judged. */
    const judgeRow = (scheme, state, how, got, base) => {
      if (!got) {
        fail(`${scheme} · ${state}: no [data-side-row][data-selected=true] on screen — nothing to measure`);
        return;
      }
      // The row's ground as painted: every layer from the outside in,
      // over the page's own ground.
      let ground = rgb(base);
      for (const c of [...got.grounds].reverse()) ground = flatten(parse(c), ground);
      const worst = got.texts.map((t) => {
        // Measure each text against its own painted ground.
        let under = rgb(base);
        for (const c of [...t.grounds].reverse()) under = flatten(parse(c), under);
        return { ...t, ratio: contrast(flatten(parse(t.color), under), under) };
      });
      const low = worst.filter((t) => t.ratio < TEXT_FLOOR);
      const faded = worst.filter((t) => t.fade < 1);
      console.log(
        `${scheme} · selected row · ${state} (${how}): ground ${ground.map((v) => Math.round(v)).join(",")}, ` +
          worst.map((t) => `${JSON.stringify(t.text)} ${say(t.ratio)}`).join(", "),
      );
      if (worst.length < 2) fail(`${scheme} · ${state}: the selected row shows ${worst.length} piece(s) of text — a title and a second line were routed in`);
      for (const t of low) fail(`${scheme} · selected row · ${state}: ${JSON.stringify(t.text)} reads ${say(t.ratio)} on its row, under ${TEXT_FLOOR}`);
      for (const t of faded) fail(`${scheme} · selected row · ${state}: ${JSON.stringify(t.text)} is faded to ${t.fade} — every line on a selected row is full-value, no opacity`);
      // The selected word keeps its state's hue, including its dot.
      // Contrast alone let the old white-word override pass this gate.
      if (got.word !== "idle") {
        const ink = oklch(rgb(got.status));
        const source = oklch(rgb(got.stateInk));
        const distance = Math.abs(ink.h - source.h);
        if (ink.c < 0.04 || Math.min(distance, 360 - distance) > 5)
          fail(`${scheme} · selected ${got.word}: state hue lost (${got.status})`);
      }
      // Never the brand, in front or behind (2026-09-27: the owner found
      // the solid 「太亮」, and 额度's rings vanished on it). A grey block,
      // as the Finder draws a sidebar's selection.
      if (got.grounds.length && strongOf[scheme] && same(got.grounds[0], strongOf[scheme])) {
        fail(`${scheme} · ${state}: the selected row is painted --brand-strong — a selection is a grey block now (app.css, 2026-09-27)`);
      }
    };
    const strongOf = {};
    for (const scheme of ["light", "dark"]) {
      await flipTheme(page, scheme);
      await wait(200);
      strongOf[scheme] = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--brand-strong").trim());
      if (!strongOf[scheme]) fail(`${scheme}: --brand-strong does not resolve — the selected row has no solid to be (src/styles/tokens.css)`);
    }
    for (const scheme of ["light", "dark"]) {
      await flipTheme(page, scheme);
      await wait(400);
      const base = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--background").trim());
      const limited = page.locator('[data-side-row="tui/palette-gate-6"]');
      if (!(await limited.locator('[data-status-unavailable]').count())) fail(`${scheme}: unbound Codex has no explicit unavailable state`);
      if (await limited.locator('[data-halo], [data-status-word], [data-unread]').count()) fail(`${scheme}: unbound Codex paints an invented state or unread`);
      const limitedInk = await limited.locator('[data-status-unavailable]').evaluate(el => getComputedStyle(el).color);
      const limitedContrast = contrast(flatten(parse(limitedInk), rgb(base)), rgb(base));
      if (limitedContrast < TEXT_FLOOR) fail(`${scheme}: unavailable state contrast ${say(limitedContrast)} is under ${TEXT_FLOOR}`);
      let dotSize;
      for (let n = 0; n < WORDS.length; n++) {
        const row = page.locator(`[data-side-row="tui/palette-gate-${n}"]`);
        await row.locator('[data-row-open]').click();
        await wait(100);
        judgeRow(scheme, WORDS[n], "each state", await read(), base);
        const badge = row.locator('[data-unread]');
        if (n === 0) {
          if (await badge.count()) fail(`${scheme}: read session still has an unread mark`);
        } else {
          const text = (await badge.textContent())?.trim();
          if (text !== (n === 2 ? "3" : "")) fail(`${scheme}: unread ${n === 2 ? 3 : 1} rendered ${JSON.stringify(text)}`);
          if (!(await badge.getAttribute('aria-label'))) fail(`${scheme}: unread mark lost its accessible count`);
          if (!(await row.locator('[data-side-title] [data-unread]').count())) fail(`${scheme}: unread mark is not beside the title`);
          const mark = await badge.boundingBox();
          const title = await row.locator('[data-side-title]').boundingBox();
          if (!mark || !title || mark.x < title.x || mark.x + mark.width > title.x + title.width + 1)
            fail(`${scheme}: unread mark clipped by its title (${JSON.stringify({ mark, title })})`);
          const dot = badge.locator('[data-unread-dot]');
          if (!(await dot.count())) fail(`${scheme}: unread mark has no separate dot`);
          else {
            const box = await dot.boundingBox();
            if (!box || (dotSize && (box.width !== dotSize.width || box.height !== dotSize.height)))
              fail(`${scheme}: unread dot changes size with the count`);
            dotSize ??= box;
          }
          const number = badge.locator('[data-unread-count]');
          if (n === 2 && await number.count()) {
            const paint = await number.evaluate((el) => ({ color: getComputedStyle(el).color, bg: getComputedStyle(el).backgroundColor }));
            if (oklch(rgb(paint.color)).c > 0.02 || parse(paint.bg)[3] !== 0)
              fail(`${scheme}: unread count needs neutral text with no background`);
          } else if (n === 2 || await number.count()) fail(`${scheme}: unread numeric label does not match its count`);
        }
      }
      await opener.click();
      if (process.env.PALETTE_SHOTS) {
        const { mkdirSync } = await import('node:fs');
        mkdirSync(process.env.PALETTE_SHOTS, { recursive: true });
        await page.locator('[data-list]').screenshot({ path: `${process.env.PALETTE_SHOTS}/session-indicators-${scheme}.png` });
      }
      for (const state of ["front", "behind"]) {
        // **Behind, by the road the app itself takes**: the page loses
        // focus to another tab of the same browser, and `lib/window-key.ts`
        // hears the `blur`. If this Chrome does not deliver it, the stamp
        // is set by hand and the line says so.
        let how = "blur";
        if (state === "behind") {
          const other = await browser.newPage();
          await other.bringToFront();
          await wait(300);
          const key = await page.evaluate(() => document.documentElement.getAttribute("data-window-key"));
          if (key !== "false") {
            how = "stamped (this Chrome delivered no blur)";
            await page.evaluate(() => document.documentElement.setAttribute("data-window-key", "false"));
          }
          await wait(300);
          const got = await read();
          await other.close();
          await page.bringToFront();
          await page.evaluate(() => document.documentElement.setAttribute("data-window-key", "true"));
          await wait(300);
          judgeRow(scheme, state, how, got, base);
        } else {
          await page.evaluate(() => document.documentElement.setAttribute("data-window-key", "true"));
          await wait(300);
          judgeRow(scheme, state, "in front", await read(), base);
        }
      }
    }
  }


  // Focused row checks keep terminal and avatar pixel sweeps for a full run.
  if (process.env.PALETTE_ONLY !== "rows") {
  // ── the terminal is the paper, and so is its tab (2026-09-30) ──────
  //
  // 「用灰色背景，黑色内容怎么样」: the owner opened a terminal session
  // and saw the open tab in the board's grey and the screen under it in
  // the ground's black — `--term-bg` was `var(--background)`, the frame
  // round the board, so the tab joined a board the terminal then covered
  // in another colour. The terminal is content, and content is on the
  // paper. Read four ways, both themes: the screen's box, what xterm was
  // actually handed (it paints its own canvas from `themeOf`, and only
  // re-reads on a theme flip), the board, and the open tab — one colour,
  // and it is `--card`. **Red on the code before the fix**: the box and
  // xterm read `--background` in both themes.
  //
  // A routed row with a terminal to be had (`attachable`); nothing here
  // answers the terminal's own requests, which is fine — the box and
  // xterm's theme are drawn before the pty is asked for anything.
  {
    const now = Date.now();
    const term = {
      id: "tui/palette-term",
      kind: "tui",
      title: "palette-term",
      word: "idle",
      at_ms: now,
      unread: 0,
      source: null,
      pinned: false,
      stale: false,
      category: "shell",
      group: "",
      face: null,
      hosted: true,
      attachable: true,
      last: null,
      via: null,
      via_face: null,
      home: "gate",
      place: "/home/gate/work",
      mux: false,
    };
    await page.unroute("**/sessions");
    await page.route("**/sessions", (r) => r.fulfill({ contentType: "application/json", body: JSON.stringify([term]) }));
    await page.evaluate(() => window.localStorage.removeItem("khor.sessions.tabs"));
    await page.reload({ waitUntil: "load" });
    await wait(1200);
    await page.locator('[data-rail-item][data-landing="devices"]:visible').click();
    await wait(200);
    await page.locator('[data-rail-item][data-landing="sessions"]:visible').click();
    await wait(400);
    const opener = page.locator("[data-list] [data-row-open]").first();
    if ((await opener.count()) === 0) throw new Error("probe dead: the routed terminal row drew no [data-row-open]");
    await opener.click();
    const deadline = Date.now() + 5_000;
    while ((await page.locator("[data-terminal]").count()) === 0 && Date.now() < deadline) await wait(100);
    if ((await page.locator("[data-terminal]").count()) !== 1) {
      fail("probe dead: opening an attachable session drew no [data-terminal] — nothing to measure the terminal's ground on");
    } else {
      for (const scheme of ["light", "dark"]) {
        await flipTheme(page, scheme);
        // xterm re-reads its colours on the stamp's own mutation
        // (`TerminalPane`), one frame behind the page.
        await wait(400);
        const got = await page.evaluate(() => {
          const probe = document.createElement("span");
          probe.style.backgroundColor = "var(--card)";
          document.body.append(probe);
          const card = getComputedStyle(probe).backgroundColor;
          probe.style.backgroundColor = "var(--background)";
          const frame = getComputedStyle(probe).backgroundColor;
          probe.remove();
          const box = document.querySelector("[data-terminal]");
          const board = document.querySelector("[data-main] [data-board]");
          const tab = document.querySelector("[data-tab][data-on='true']");
          return {
            card,
            frame,
            box: box ? getComputedStyle(box).backgroundColor : null,
            xterm: box?.__xterm?.options?.theme?.background ?? null,
            board: board ? getComputedStyle(board).backgroundColor : null,
            tab: tab ? getComputedStyle(tab).backgroundColor : null,
          };
        });
        console.log(`${scheme}: the terminal's ground ${JSON.stringify(got)}`);
        // **The open tab is the chosen piece of glass** (the redesign,
        // 2026-10-02, the owner: 「深色选中是底色的黑、浅色玻璃为灰、选中为
        // 白」): white in light, the ground's black in dark. It was the
        // paper itself while a tab was a fold of the board (板是纸).
        const chosen = scheme === "light" ? "rgb(255, 255, 255)" : got.card;
        if (got.tab && !same(got.tab, chosen)) {
          fail(`${scheme}: the open tab is ${got.tab}, not the chosen glass ${chosen}`);
        }
        for (const where of ["box", "xterm", "board"]) {
          if (!got[where]) {
            fail(`${scheme}: probe dead — the terminal's ${where} could not be read`);
          } else if (!same(got[where], got.card)) {
            fail(
              `${scheme}: the terminal's ${where} is ${got[where]}, not the paper --card ${got.card}` +
                (same(got[where], got.frame) ? " — it is the frame, --background" : "") +
                " — the open tab and the screen under it are one piece of paper (板是纸, TerminalPane `--term-bg`)",
            );
          }
        }
      }
    }
  }

  // ── a picked row in a floating list wears one ink (2026-09-30) ─────
  //
  // The ⌘K hits mark the picked row with `--fill-strong` — the block a
  // selected list row wears — and kept its second line and its kind word
  // in `--muted-foreground` on it. A selected row does not
  // (README 颜色「选中的行上没有…第二、三层墨」), because on that block
  // the second ink falls under 4.5; the popover went a step lighter on
  // the dark theme to float over the grey frame, and there it read 4.11.
  // Every line of the picked hit is read against the block it stands on,
  // composited down to the popover, both themes. **Red before the fix**
  // (the dark note and kind word, 4.11).
  {
    await page.route("**/find", (r) =>
      r.fulfill({
        contentType: "application/json",
        body: JSON.stringify([
          { kind: "session", id: "tui/palette-term", label: "palette-term", note: "gate · /home/gate/work" },
          { kind: "machine", id: "palette-machine", label: "palette-machine", note: "a second line" },
        ]),
      }),
    );
    // Off the terminal first: its screen holds the keyboard, and the
    // question here is the hits, not who hears ⌘K inside a pty.
    await page.locator('[data-rail-item][data-landing="devices"]:visible').click();
    await wait(400);
    for (const scheme of ["light", "dark"]) {
      await flipTheme(page, scheme);
      await wait(300);
      await page.keyboard.press("Meta+k");
      const deadline = Date.now() + 3_000;
      while ((await page.locator("[data-search-box]").count()) === 0 && Date.now() < deadline) await wait(100);
      if ((await page.locator("[data-search-box]").count()) === 0) {
        fail(`${scheme}: probe dead — ⌘K opened no [data-search-box]`);
        continue;
      }
      await page.locator("[data-search-box]").fill("palette");
      const rowsBy = Date.now() + 3_000;
      while ((await page.locator('[data-search-row][data-on="true"]').count()) === 0 && Date.now() < rowsBy) await wait(100);
      const got = await page.evaluate(() => {
        const row = document.querySelector('[data-search-row][data-on="true"]');
        if (!row) return null;
        const layers = [];
        for (let e = row; e; e = e.parentElement) {
          const c = getComputedStyle(e).backgroundColor;
          if (c && c !== "rgba(0, 0, 0, 0)" && c !== "transparent") layers.push(c);
          if (e.matches("[data-search-rows]")) break;
        }
        const texts = [...row.querySelectorAll("*"), row]
          .filter((el) => [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim() !== ""))
          .filter((el) => el.getBoundingClientRect().width > 0)
          .map((el) => ({ text: el.textContent.trim().slice(0, 24), color: getComputedStyle(el).color }));
        return { layers, texts };
      });
      await page.keyboard.press("Escape");
      await wait(200);
      if (!got || got.texts.length < 2) {
        fail(`${scheme}: probe dead — the picked ⌘K hit showed ${got ? got.texts.length : 0} piece(s) of text; two hits with a note were routed in`);
        continue;
      }
      let ground = rgb(got.layers.at(-1));
      for (const c of [...got.layers].reverse().slice(1)) ground = flatten(parse(c), ground);
      const read = got.texts.map((x) => ({ ...x, ratio: contrast(flatten(parse(x.color), ground), ground) }));
      console.log(
        `${scheme} · picked ⌘K hit: ground ${ground.map((v) => Math.round(v)).join(",")}, ` +
          read.map((x) => `${JSON.stringify(x.text)} ${say(x.ratio)}`).join(", "),
      );
      for (const x of read) {
        if (x.ratio < TEXT_FLOOR) {
          fail(
            `${scheme} · picked ⌘K hit: ${JSON.stringify(x.text)} reads ${say(x.ratio)} on its block, under ${TEXT_FLOOR} — ` +
              "a picked row wears one ink, like a selected one (app.css)",
          );
        }
      }
    }
  }

  // ── a mark on a face (类型标记「浮现」, 2026-10-02) ───────────────────
  //
  // The owner:「画在脸上感觉很好看」. The kind mark is painted on the
  // machine's face — and a face is any colour, light and dark in one
  // gradient. **The face says which ink and how much veil** (the owner,
  // the same day:「按脸换墨，不够才压」): `khor_core::avatar` derives, per
  // face, the ink most of the centre wants (`--mark-ink-dark` on a bright
  // face, `--mark-ink-light` on a dark one) and the least veil — black
  // under the light ink, white under the dark — at which the worst pixel
  // under a mark reads 3.2 (`Avatar.mark`, `avatar/mark.rs`). This
  // measures that, as the app paints it, on **real faces**: the core's
  // own output (the `face_samples` bin), every factory palette × every
  // way of drawing, sixteen machines each, in the machines column.
  //
  // How: the mark is made transparent (the veil stays — it is the
  // face's), the box the mark's glyph occupies is cut out of a
  // screenshot, and the pixel in it that reads worst against **the ink
  // that face was given** is the worst ground the ink meets there — the
  // brightest under the light ink, the darkest under the dark. That is
  // conservative (the box holds pixels the glyph never touches) and it
  // does not depend on which glyph a row happens to wear. Floor: a
  // graphic's 3:1, both themes, the blank too.
  //
  // Also held, so the drawing is the record's: each mark is painted in
  // its face's ink (and the GUI's inks are the core's hex), each face
  // wears a veil exactly when its record has one, in the record's colour
  // and alpha — and **the point of the change**: most sample faces wear
  // no veil at all (the lab counted 269 of 336; the floor is 240).
  //
  // **The probe proves it can see a failure every run**: the same faces
  // forced to the light ink with no veil — the first version's ink
  // without its veil — must fall under the floor somewhere.
  //
  // ⚠️ 墓碑 (2026-10-02, the same day): this measured one white
  // `--mark-ink` over one `--mark-veil` (black 0.42) on every face, and
  // its teeth were the veil taken off. The owner:「浮现这，给原始头像上了
  // 蒙板？感觉没那么好看了」.
  {
    const { execFileSync } = await import("node:child_process");
    const out = JSON.parse(
      execFileSync("cargo", ["run", "-q", "-p", "khor-gui-core", "--bin", "face_samples"], {
        cwd: gui,
        maxBuffer: 64 << 20,
        stdio: ["ignore", "pipe", "inherit"],
      }).toString(),
    );
    const samples = out.samples ?? [];
    const grounds = new Map();
    for (const s of samples) {
      const k = s.palette;
      grounds.set(k, (grounds.get(k) ?? new Set()).add(s.face.background));
    }
    const thin = [...grounds].filter(([, g]) => g.size < 5).map(([p, g]) => `${p} ${g.size}`);
    if (samples.length === 0 || thin.length) {
      fail(`a mark on a face: probe dead — the samples tried fewer than five grounds on ${thin.join(", ") || "every palette"} (apps/gui/core/src/bin/face_samples.rs SEEDS)`);
    }
    if (samples.some((s) => !s.face.mark || !["light", "dark"].includes(s.face.mark.ink))) {
      fail("a mark on a face: probe dead — a sample face carries no mark record (khor_core::avatar Avatar.mark)");
    }
    // **The point of the change**, on the record: most faces bare.
    const BARE_FLOOR = 240;
    const bareRecords = samples.filter((s) => s.face.mark?.veil === 0).length;
    const darkRecords = samples.filter((s) => s.face.mark?.ink === "dark").length;
    const mostVeil = Math.max(...samples.map((s) => s.face.mark?.veil ?? 0));
    console.log(
      `a mark on a face: ${samples.length} sample faces — ${bareRecords} with no veil, ${darkRecords} dark ink / ${samples.length - darkRecords} light, the most veil ${mostVeil}`,
    );
    if (bareRecords < BARE_FLOOR) {
      fail(`a mark on a face: only ${bareRecords} of ${samples.length} sample faces go bare, under ${BARE_FLOOR} — the per-face ink is there so most faces wear no veil (khor_core::avatar mark.rs)`);
    }
    // The GUI's colours are the core's: the rule was derived with these.
    const tokenOf = (name) =>
      page.evaluate((name) => {
        const el = document.createElement("span");
        el.style.color = `var(${name})`;
        document.body.appendChild(el);
        const c = getComputedStyle(el).color;
        el.remove();
        return c;
      }, name);
    for (const [token, hex] of [
      ["--mark-ink-light", out.inks?.light],
      ["--mark-ink-dark", out.inks?.dark],
      ["--mark-veil-dark", out.veil_under?.light],
      ["--mark-veil-light", out.veil_under?.dark],
    ]) {
      const got = await tokenOf(token);
      if (!hex || !same(got, hex)) {
        fail(`a mark on a face: ${token} is ${got}, the core derives with ${hex} — two homes for one colour (src/styles/tokens.css, khor_core::avatar mark.rs)`);
      }
    }
    const idOf = (i) => i.toString(16).padStart(64, "0");
    // One more machine with no face at all: the blank is the theme's
    // grey, and the one ground here that reads the theme.
    const devices = [...samples.map((s, i) => ({ s, i })), { s: null, i: samples.length }].map(({ s, i }) => ({
      id: idOf(i + 1),
      name: s ? `${s.palette}-${s.variant}-${s.seed}` : "blank",
      shown: s ? `${s.palette}-${s.variant}-${s.seed}` : "blank",
      label: null,
      me: false,
      paused: false,
      face: s ? s.face : null,
      pinned: false,
      vitals: null,
      hop: "direct",
      sponsor: null,
      watch: [],
      watched: null,
      seat: { left: 50, top: 50 },
      hosts: true,
    }));
    await page.route("**/devices", (r) => r.fulfill({ contentType: "application/json", body: JSON.stringify(devices) }));
    await page.locator('[data-rail-item][data-landing="sessions"]:visible').click();
    await wait(200);
    await page.locator('[data-rail-item][data-landing="devices"]:visible').click();
    const drawnBy = Date.now() + 8_000;
    const markSel = '[data-side-row] [data-mark-at="face"]';
    while ((await page.locator(markSel).count()) < devices.length && Date.now() < drawnBy) await wait(200);
    const marks = await page.locator(markSel).count();
    if (marks < devices.length) {
      fail(`a mark on a face: probe dead — ${marks} of ${devices.length} routed machines drew a mark on their face (src/components/FaceMark.tsx)`);
    } else {
      /** What each row drew: the mark's ink, and the veil its face wears. */
      const drawn = () =>
        page.evaluate((ids) => {
          return ids.map((id) => {
            const row = document.querySelector(`[data-side-row="${id}"]`);
            const mark = row?.querySelector('[data-mark-at="face"]');
            const veils = [...(row?.querySelectorAll("[data-face-veil]") ?? [])];
            const v = veils[0];
            return {
              ink: mark?.getAttribute("data-mark-ink") ?? null,
              color: mark ? getComputedStyle(mark).color : null,
              veils: veils.length,
              veil: v ? { color: getComputedStyle(v).backgroundColor, alpha: Number(getComputedStyle(v).opacity) } : null,
            };
          });
        }, devices.map((d) => d.id));
      /** The pixels under each row's mark, as the screenshot has them. */
      const boxes = async () => {
        const shots = [];
        for (let n = 0; n < devices.length; n++) {
          const row = page.locator(`[data-side-row="${devices[n].id}"]`);
          const glyph = row.locator('[data-mark-at="face"] > *').first();
          await glyph.scrollIntoViewIfNeeded();
          shots.push((await glyph.screenshot({ animations: "disabled" })).toString("base64"));
        }
        return page.evaluate(async (pngs) => {
          const out = [];
          for (const b64 of pngs) {
            const img = new Image();
            img.src = `data:image/png;base64,${b64}`;
            await img.decode();
            const c = document.createElement("canvas");
            c.width = img.width;
            c.height = img.height;
            const g = c.getContext("2d");
            g.drawImage(img, 0, 0);
            const px = g.getImageData(0, 0, c.width, c.height).data;
            const all = [];
            for (let i = 0; i < px.length; i += 4) all.push([px[i], px[i + 1], px[i + 2]]);
            out.push({ w: c.width, px: all });
          }
          return out;
        }, shots);
      };
      /** The pixel that reads worst against `ink`, and how it reads. */
      const worstOf = (ink, px) => {
        let best = { ratio: Infinity, rgb: null };
        for (const p of px) {
          const r = contrast(rgb(ink), p);
          if (r < best.ratio) best = { ratio: r, rgb: p };
        }
        return best;
      };
      const hide = await page.addStyleTag({ content: '[data-mark-at="face"] > * { opacity: 0 !important; }' });
      for (const scheme of ["light", "dark"]) {
        await flipTheme(page, scheme);
        await wait(300);
        const blankInk = await tokenOf("--mark-ink-blank");
        const seen = await drawn();
        const shots = await boxes();
        const worst = new Map();
        let bareDrawn = 0;
        seen.forEach((d, n) => {
          const s = n < samples.length ? samples[n] : null;
          const key = s ? `${s.palette} × ${s.variant}` : "no face (the blank)";
          const where = `${scheme} · a mark on a face: ${s ? `${s.palette}-${s.variant}-${s.seed}` : "the blank"}`;
          const want = s ? s.face.mark : null;
          // The drawing is the record's.
          const wantInk = want ? want.ink : "blank";
          const wantColor = want ? (want.ink === "light" ? out.inks.light : out.inks.dark) : blankInk;
          if (d.ink !== wantInk || !d.color || !same(d.color, wantColor)) {
            fail(`${where}: the mark is ${d.ink} ${d.color}, the face was given ${wantInk} ${wantColor} (src/components/FaceMark.tsx)`);
          }
          const wantVeils = want && want.veil > 0 ? 1 : 0;
          if (d.veils !== wantVeils) {
            fail(`${where}: the face wears ${d.veils} veil(s), its record says ${want ? want.veil : "none (the blank)"} (src/components/Avatar.tsx Veil)`);
          } else if (wantVeils) {
            const veilColor = want.ink === "light" ? out.veil_under.light : out.veil_under.dark;
            if (!same(d.veil.color, veilColor) || Math.abs(d.veil.alpha - want.veil) > 0.005) {
              fail(`${where}: the veil is ${d.veil.color} at ${d.veil.alpha}, the record says ${veilColor} at ${want.veil}`);
            }
          }
          if (s && d.veils === 0) bareDrawn++;
          const w = worstOf(wantColor, shots[n].px);
          const was = worst.get(key);
          if (!was || w.ratio < was.ratio) worst.set(key, { ...w, w: shots[n].w, at: s ? s.seed : null, ink: wantInk, veil: want?.veil ?? 0 });
        });
        console.log(
          `${scheme} · a mark on a face: ${seen.length} faces, ${bareDrawn} of ${samples.length} drawn bare, glyph box ${shots[0]?.w}px — worst per palette × drawing:\n` +
            [...worst]
              .map(([k, v]) => `    ${k}: ${say(v.ratio)} (${v.ink} ink, veil ${v.veil}, worst pixel ${v.rgb?.join(",")}, machine ${v.at})`)
              .join("\n"),
        );
        if (bareDrawn < BARE_FLOOR) {
          fail(`${scheme} · a mark on a face: only ${bareDrawn} of ${samples.length} sample faces are drawn with no veil, under ${BARE_FLOOR}`);
        }
        for (const [k, v] of worst) {
          if (v.ratio < GRAPHIC_FLOOR) {
            fail(
              `${scheme} · a mark on a face: on ${k} the mark reads ${say(v.ratio)} against the face behind it, under ${GRAPHIC_FLOOR} — ` +
                "the ink or veil the face was given does not hold it (khor_core::avatar mark.rs; src/styles/tokens.css)",
            );
          }
        }
      }
      // The teeth: every face forced to the light ink with no veil — the
      // first version's ink without its veil — must fail somewhere.
      const unveil = await page.addStyleTag({ content: "[data-face-veil] { display: none !important; }" });
      await flipTheme(page, "light");
      await wait(300);
      const bare = (await boxes()).slice(0, samples.length).map((b) => worstOf(out.inks.light, b.px).ratio);
      const bareWorst = Math.min(...bare);
      const under = bare.filter((r) => r < GRAPHIC_FLOOR).length;
      console.log(`a mark on a face, every face forced to the light ink with no veil: worst ${say(bareWorst)}, ${under} under ${GRAPHIC_FLOOR} — the probe sees a failing face`);
      if (!(bareWorst < GRAPHIC_FLOOR)) {
        fail(`a mark on a face: probe dead — with every face on the light ink and no veil the worst still reads ${say(bareWorst)}; this section cannot see a failure`);
      }
      await unveil.evaluate((n) => n.remove());
      await hide.evaluate((n) => n.remove());
    }
    await page.unroute("**/devices");
  }

  } // full palette sweep

  if (thrown.length) fail(`the page threw while being measured:\n${thrown.join("\n")}`);
  if (!process.exitCode && process.env.PALETTE_ONLY === "rows") console.log("palette rows: both themes, selected state hues and contrast, fixed unread dot with neutral count");
  if (!process.exitCode && process.env.PALETTE_ONLY !== "rows") console.log("palette gate: the main action and the focus ring can both be seen, one ground under the list, the content and the terminal, the hairline, the glass and the selected row's grey stand off it, every ink reads on the ground, a float, the glass and the chosen glass, a selected row reads in front and behind, a picked hit reads in one ink, and the terminal is the paper");
} catch (e) {
  fail(String(e));
} finally {
  await browser?.close();
  vite?.kill();
}
