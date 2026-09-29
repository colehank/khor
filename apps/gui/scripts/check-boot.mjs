// The first frame's colours must be the palette's.
//
// `index.html` paints a ground and two columns before any stylesheet
// has arrived, so it necessarily repeats four values from
// `src/styles/tokens.css`. That repetition is the point — it is what
// makes the frame possible — and it is also the only thing here that
// can rot. When it does there is no error anywhere: the window simply
// flashes last month's colour for a quarter of a second and settles.
//
// So this gate reads both files and compares. It never judges what a good
// colour is; it only says the two places agree.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import { compareFiles } from "./design-tokens.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const html = readFileSync(join(here, "..", "index.html"), "utf8");
const css = readFileSync(join(here, "..", "src", "styles", "tokens.css"), "utf8");

/** The value a token has, in the light block and in the dark one. */
function token(name) {
  const all = [...css.matchAll(new RegExp(`--${name}:\\s*([^;]+);`, "g"))].map((m) =>
    m[1].trim(),
  );
  if (all.length < 2) {
    fail(`tokens.css should declare --${name} twice (light, then dark); found ${all.length}`);
  }
  return { light: all[0], dark: all[1] };
}

let bad = 0;
function fail(why) {
  console.error(`boot gate: ${why}`);
  bad++;
}

// The self-test arm, and it runs before the comparison below so that a
// blinded reader cannot print "clean" on its way past: a token that is
// not in the file must be reported as missing, and one that is must be
// found. Without this, a regex that matches nothing would agree with
// everything.
const probe = [...css.matchAll(/--background:\s*([^;]+);/g)];
if (probe.length < 2) {
  console.error("boot gate: cannot read tokens.css at all — every check below would pass emptily");
  process.exit(1);
}

const want = { background: token("background"), card: token("card"), side: token("side") };

// What the boot frame uses, in source order: body (light), rail
// (light), column (light), then the dark trio under the stamp. **The
// rail and the column are one ground now** (docs/SHELL.md 第十三轮 第 1
// 条: 「竖栏和列表一种底色」) — `--side`, on both.
const found = [...html.matchAll(/background:\s*(#[0-9a-fA-F]{3,8})/g)].map((m) => m[1]);
if (found.length !== 6) {
  fail(`expected 6 colours in the boot frame (ground, rail, column — light then dark), found ${found.length}: ${found.join(", ")}`);
} else {
  const [bodyLight, railLight, columnLight, darkGround, darkRail, darkColumn] = found;
  const eq = (a, b) => a.toLowerCase() === b.toLowerCase();
  if (!eq(bodyLight, want.background.light)) {
    fail(`the light ground is ${bodyLight}, --background is ${want.background.light}`);
  }
  if (!eq(railLight, want.side.light)) {
    fail(`the light rail is ${railLight}, --side is ${want.side.light}`);
  }
  if (!eq(columnLight, want.side.light)) {
    fail(`the light column is ${columnLight}, --side is ${want.side.light}`);
  }
  if (!eq(darkGround, want.background.dark)) {
    fail(`the dark ground is ${darkGround}, --background is ${want.background.dark}`);
  }
  if (!eq(darkRail, want.side.dark)) {
    fail(`the dark rail is ${darkRail}, --side is ${want.side.dark}`);
  }
  if (!eq(darkColumn, want.side.dark)) {
    fail(`the dark column is ${darkColumn}, --side is ${want.side.dark}`);
  }
}

// **And the frame before that one.** The window itself is painted from
// Rust, because it has to be coloured before any HTML exists and the
// tauri config file takes one value while this app has two grounds
// (`paint_the_window_before_the_page`). That is a third copy of
// `--background`, on the other side of a language boundary, and it rots
// the same way — into a quarter-second flash of last month's colour with
// nothing anywhere to say so.
const rust = readFileSync(
  join(here, "..", "src-tauri", "src", "lib.rs"),
  "utf8",
);
// **The left block's first row, in the one place CSS cannot reach**
// (docs/SHELL.md 第十三轮 第 2 条): macOS's three buttons are placed by
// numbers worked out from this row's height, from where the rail's
// glyph box starts, and from the gap before the search box — handed to
// tauri when the window is built, in Rust, where no token is readable.
// So three numbers are written twice, each read here from both files,
// and a disagreement is refused: the same arrangement the two grounds
// above have. `--pane-head` is written as a number (38 since 2026-09-29),
// so it is matched as the literal it is.
const traffic = readFileSync(join(here, "..", "src-tauri", "src", "traffic.rs"), "utf8");
{
  const px = (name) => {
    const m = new RegExp(`--${name}:\\s*([0-9.]+)px;`).exec(css);
    return m ? parseFloat(m[1]) : null;
  };
  const rs = (name) => {
    const m = new RegExp(`pub const ${name}: f64 = ([0-9.]+);`).exec(traffic);
    return m ? parseFloat(m[1]) : null;
  };
  const lip = px("tab-lip");
  const railW = px("rail-w");
  const railItem = px("rail-item");
  const gapIn = px("gap-in");
  // `--pane-head` is written as a number — a decision about the window's
  // first rows, not the sum of the finger's floor and a tab's lip
  // (docs/design-system/guidelines/40-migration.md). 38 since 2026-09-29.
  const paneHead = px("pane-head");
  const pairs = [
    ["the first row's height", "PANE_HEAD", paneHead, "--pane-head"],
    ["the first button's left edge", "X", railW !== null && railItem !== null ? (railW - railItem) / 2 : null, "(--rail-w − --rail-item) / 2, the rail's glyph box"],
    ["the gap before the search box", "GAP", gapIn, "--gap-in"],
  ];
  for (const [what, name, inCss, from] of pairs) {
    const inRust = rs(name);
    if (inCss === null || inRust === null) {
      fail(
        `${what} is declared in tokens.css (${from}: ${inCss ?? "missing"}) and in src-tauri/src/traffic.rs ` +
          `(${name} = ${inRust ?? "missing"}) — one of them is not there, so nothing compares them`,
      );
    } else if (inCss !== inRust) {
      fail(`${what} is ${inCss} by tokens.css (${from}) and ${inRust} in src-tauri/src/traffic.rs ${name} — the window's three buttons would be placed on a row that is not the one drawn`);
    } else {
      console.log(`first row: ${name} ${inRust} in src-tauri and in tokens.css (${from})`);
    }
  }

  // **One centreline, in both postures** (2026-09-29, 「整行收紧到 38」).
  // A tab is a `--hit` tall label on the row's middle — where the search
  // box, the title and the window's buttons stand — plus a foot,
  // `--tab-lip`, that carries it down into the board it is joined to; so
  // the lip is also what stands above the label, and the row is the
  // label with a lip either side. Three numbers that only mean anything
  // together, written apart, and a drift between them is a tab a pixel
  // or two off the line everything else is on — the kind of thing no
  // one sees until the owner does. Read for the mouse (`:root`) and for
  // the finger (`@media (pointer: coarse)`, where `--hit` is `--touch`
  // and the row goes back to 48). `check-face` §12 measures it drawn.
  {
    const coarse = /@media \(pointer: coarse\) \{\s*:root \{([\s\S]*?)\n  \}\n\}/.exec(css)?.[1] ?? null;
    const decl = (text, name) => new RegExp(`--${name}:\\s*([^;]+);`).exec(text)?.[1].trim() ?? null;
    const asPx = (v) => {
      if (v === null) return null;
      const ref = /^var\(--([a-z0-9-]+)\)$/.exec(v);
      if (ref) return px(ref[1]);
      const n = /^([0-9.]+)px$/.exec(v);
      return n ? parseFloat(n[1]) : null;
    };
    if (coarse === null) {
      fail("tokens.css has no `@media (pointer: coarse) { :root { … } }` block to read the finger's row from — the second posture is not being held");
    } else {
      const postures = [
        ["a mouse", { row: paneHead, hit: asPx(decl(css, "hit")), lip }],
        ["a finger", { row: asPx(decl(coarse, "pane-head")), hit: asPx(decl(coarse, "hit")), lip: asPx(decl(coarse, "tab-lip")) }],
      ];
      for (const [who, p] of postures) {
        if (p.row === null || p.hit === null || p.lip === null) {
          fail(`the first row under ${who} does not read: --pane-head ${p.row}, --hit ${p.hit}, --tab-lip ${p.lip} — nothing holds a tab's label on its centreline`);
        } else if (p.hit + 2 * p.lip !== p.row) {
          fail(
            `under ${who} the row is ${p.row} and a tab is a ${p.hit} label with a ${p.lip} lip over and a ${p.lip} foot under it ` +
              `(${p.hit + 2 * p.lip}) — its label is ${(p.row - p.hit) / 2 - p.lip} off the centreline the search box and the window's buttons stand on`,
          );
        } else {
          console.log(`first row under ${who}: ${p.row} = a ${p.hit} label + ${p.lip} over + ${p.lip} under, one centreline`);
        }
      }
    }
  }
}

// ── and the design system's own values (第十五轮) ─────────────────
//
// `docs/design-system/tokens.json` decides a value; tokens.css is where
// the app reads it. Every token the design system names is compared,
// both themes (`design-tokens.mjs` says how, and names its exceptions).
{
  const json = join(here, "..", "..", "..", "docs", "design-system", "tokens.json");
  let r = null;
  try {
    r = compareFiles(json, join(here, "..", "src", "styles", "tokens.css"));
  } catch (e) {
    fail(`cannot compare with docs/design-system/tokens.json (${String(e).split("\n")[0]}) — the design system and the app are not being held together`);
  }
  if (r) {
    if (r.checked < 150) fail(`probe dead: only ${r.checked} design tokens were compared — tokens.json's shape moved and this reads past it`);
    for (const b of r.bad) fail(`design system: ${b}`);
    for (const m of r.met) console.log(`design system · named exception · ${m}`);
    if (!r.bad.length) console.log(`design system: ${r.checked} tokens of docs/design-system/tokens.json agree with tokens.css, both themes`);
  }
}

// ── the left block's first frame, by platform (第十五轮 第 7 条, D4) ──
//
// On the macOS app the left block stands on the system's sidebar
// material, so its ground is **clear** there and opaque everywhere else.
// That is four things in four files, and each is read here:
//
//   - the window is built transparent with the sidebar effect, inside
//     the macOS half of `build_the_window` (src-tauri/src/lib.rs);
//   - tauri's private API is switched on in both places tauri-build
//     holds together (tauri.conf.json, src-tauri/Cargo.toml);
//   - the webview is not painted an opaque ground on macOS, or the
//     material would never show (`paint_the_window_before_the_page`);
//   - the page clears `--side` only inside `[data-side]` and only under
//     the stamp `App` sets on the macOS app — **and `--side` itself
//     stays an opaque colour in both themes**: a translucent `--side`
//     would be the fake material this round forbids.
{
  const before = bad;
  const conf = JSON.parse(readFileSync(join(here, "..", "src-tauri", "tauri.conf.json"), "utf8"));
  const cargo = readFileSync(join(here, "..", "src-tauri", "Cargo.toml"), "utf8");
  const appCss = readFileSync(join(here, "..", "src", "styles", "app.css"), "utf8");
  const appTsx = readFileSync(join(here, "..", "src", "App.tsx"), "utf8");
  const build = rust.match(/fn build_the_window[\s\S]*?\n}\n/)?.[0] ?? "";
  const mac = build.match(/#\[cfg\(target_os = "macos"\)\]\s*\{[\s\S]*?\n {4}\}/)?.[0] ?? "";
  if (!/\.transparent\(true\)/.test(mac) || !/Effect::Sidebar/.test(mac)) {
    fail("the macOS window is not built transparent with the sidebar effect (src-tauri/src/lib.rs build_the_window) — D4's material has nothing to show through");
  }
  if (conf.app?.macOSPrivateApi !== true || !/features = \[[^\]]*"macos-private-api"/.test(cargo)) {
    fail("tauri's macos-private-api is not on in both tauri.conf.json (app.macOSPrivateApi) and src-tauri/Cargo.toml — a transparent window needs it");
  }
  const paint = rust.match(/fn paint_the_window_before_the_page[\s\S]*?\n}\n/)?.[0] ?? "";
  if (!/#\[cfg\(not\(target_os = "macos"\)\)\]\s*let _ = w\.set_background_color/.test(paint)) {
    fail("the window's ground is painted on macOS too (paint_the_window_before_the_page) — an opaque webview covers the material for good");
  }
  if (!/:root\[data-material="sidebar"\][^{]*\[data-side\]\s*\{\s*--side:\s*transparent;/.test(appCss)) {
    fail("app.css does not clear --side inside [data-side] under :root[data-material=\"sidebar\"] — the block paints over the material");
  }
  if (!/if \(trafficLights\) document\.documentElement\.setAttribute\("data-material", "sidebar"\)/.test(appTsx)) {
    fail("App.tsx does not stamp data-material=\"sidebar\" on the macOS app alone (trafficLights) — the clear ground would reach faces with no material");
  }
  for (const theme of ["light", "dark"]) {
    const v = want.side[theme];
    if (!/^#[0-9a-f]{6}$/i.test(v)) fail(`--side (${theme}) is ${v} — it must stay opaque; a translucent ground is not the material`);
  }
  if (bad === before) console.log("left block: macOS window transparent over the sidebar material, --side cleared in the block under the stamp only, opaque elsewhere");
}

const painted = [...rust.matchAll(/Color\(0x([0-9a-f]{2}), 0x([0-9a-f]{2}), 0x([0-9a-f]{2}), 0xff\)/g)].map(
  (m) => `#${m[1]}${m[2]}${m[3]}`,
);
if (painted.length !== 2) {
  fail(
    `expected the window's two grounds in src-tauri/src/lib.rs, found ${painted.length}` +
      ` — if that function was renamed or reshaped this check is now blind`,
  );
} else {
  const [dark, light] = painted;
  if (dark.toLowerCase() !== want.background.dark.toLowerCase()) {
    fail(`the window's dark ground is ${dark}, --background is ${want.background.dark}`);
  }
  if (light.toLowerCase() !== want.background.light.toLowerCase()) {
    fail(`the window's light ground is ${light}, --background is ${want.background.light}`);
  }
}

// ── and the stamp that decides which of the two the frame wears ────
//
// 批 D2 moved every dark rule off `prefers-color-scheme` and onto
// `:root[data-theme="dark"]`, so the frame above paints light unless
// something stamps first. `index.html` does it in a script that runs
// before the bundle exists — which means it cannot ask khor, and reads
// a mirror of the setting out of `localStorage` instead.
//
// **That mirror's key is written in two places by necessity**, exactly
// like the four colours: one copy in a script that must run before any
// module, one in the module that keeps it (`src/theme.ts`). Drift is
// silent in the worst direction — the frame quietly stops following the
// setting and flashes the other theme on every cold launch, with no
// error anywhere. So the two spellings are compared, and so is the
// attribute, since the stylesheets key off that name too.
const theme = readFileSync(join(here, "..", "src", "theme.ts"), "utf8");

const mirrorInTs = theme.match(/THEME_MIRROR = "([^"]+)"/)?.[1] ?? null;
const mirrorInHtml = html.match(/localStorage\.getItem\("([^"]+)"\)/)?.[1] ?? null;
if (mirrorInTs === null || mirrorInHtml === null) {
  fail(
    `cannot find the theme mirror key (theme.ts: ${JSON.stringify(mirrorInTs)}, ` +
      `index.html: ${JSON.stringify(mirrorInHtml)}) — if either moved, this check is now blind`,
  );
} else if (mirrorInTs !== mirrorInHtml) {
  fail(
    `the boot script reads ${JSON.stringify(mirrorInHtml)} and src/theme.ts writes ` +
      `${JSON.stringify(mirrorInTs)} — the first frame would stop following the setting, silently`,
  );
}

// The attribute itself, in all three places that spell it: the frame's
// own rules, the module that stamps it, and the palette it selects.
const STAMP = 'data-theme';
for (const [what, text] of [
  ["index.html", html],
  ["src/theme.ts", theme],
  ["src/styles/tokens.css", css],
]) {
  if (!text.includes(STAMP)) {
    fail(`${what} never mentions ${STAMP} — the theme stamp is not reaching it`);
  }
}
// …and that the frame is keyed on the stamp rather than back on the
// media query it came from. A frame that asked the desktop again would
// be right for everybody except the person who set the other one.
if (/@media\s*\(prefers-color-scheme/.test(html)) {
  fail(
    "the boot frame still asks prefers-color-scheme — 批 D2 keys it on data-theme so an " +
      "explicit 浅色 beats a dark desktop before the first paint",
  );
}

if (bad) {
  console.error(`boot gate: ${bad} thing(s) no longer line up`);
  process.exit(1);
}
console.log(
  `boot gate: the first frame's 6 colours match --background and --side, the window's 2 grounds match ` +
    `--background, the first row's three numbers agree with src-tauri, and the theme stamp (${STAMP}, mirror ${JSON.stringify(mirrorInTs)}) is spelled the ` +
    `same in the frame, the module and the palette`,
);
