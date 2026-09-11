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

if (bad) {
  console.error(`boot gate: ${bad} colour(s) in index.html no longer match tokens.css`);
  process.exit(1);
}
console.log(`boot gate: the first frame's 5 colours match --background and --card`);
