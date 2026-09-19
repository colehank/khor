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

const want = { background: token("background"), card: token("card") };

// What the boot frame uses, in source order: body (light), rail
// (light), column (light), then the dark trio in the media query.
const found = [...html.matchAll(/background:\s*(#[0-9a-fA-F]{3,8})/g)].map((m) => m[1]);
if (found.length !== 5) {
  fail(`expected 5 colours in the boot frame, found ${found.length}: ${found.join(", ")}`);
} else {
  const [bodyLight, columnLight, darkGround, darkColumn] = [
    found[0],
    found[2],
    found[3],
    found[4],
  ];
  const eq = (a, b) => a.toLowerCase() === b.toLowerCase();
  if (!eq(bodyLight, want.background.light)) {
    fail(`the light ground is ${bodyLight}, --background is ${want.background.light}`);
  }
  if (!eq(found[1], want.background.light)) {
    fail(`the light rail is ${found[1]}, --background is ${want.background.light}`);
  }
  if (!eq(columnLight, want.card.light)) {
    fail(`the light column is ${columnLight}, --card is ${want.card.light}`);
  }
  if (!eq(darkGround, want.background.dark)) {
    fail(`the dark ground is ${darkGround}, --background is ${want.background.dark}`);
  }
  if (!eq(darkColumn, want.card.dark)) {
    fail(`the dark column is ${darkColumn}, --card is ${want.card.dark}`);
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
  `boot gate: the first frame's 5 colours and the window's 2 grounds all match --background and ` +
    `--card, and the theme stamp (${STAMP}, mirror ${JSON.stringify(mirrorInTs)}) is spelled the ` +
    `same in the frame, the module and the palette`,
);
