// The type gate, static half: tokens.css owns every font size, weight,
// radius and tempo — a literal outside it is a second source of truth
// that drifts silently (the px gate's judgment, extended to type).
//
// What it blocks:
//   - a tailwind weight class by number-name (font-medium, font-bold…):
//     the ink weights are semantic (font-body/label/title) so a rank is
//     stated as a role, not as a number someone liked;
//   - an arbitrary value carrying a length for type or radius
//     (text-[13px], rounded-[7px], font-[550]);
//   - a bare font-size / font-weight / border-radius declaration in a
//     stylesheet other than tokens.css;
//   - the TS tempo twin drifting from the CSS one: `--time-yield` and
//     `lib/tempo.ts` carry one number under one name, and this is the
//     only gate that can see both files at once.
//
// What it cannot block: a *wrong* role — a label dressed as a title
// compiles and passes; the face gate (`check-face.mjs`) holds the
// rendered distribution, and neither gate can read intent.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join, relative } from "node:path";

const root = fileURLToPath(new URL("..", import.meta.url));
// tokens own the numbers; shadcn ui/ is vendored; gen/ is machine output.
const ALLOW = [
  /^src\/styles\/tokens\.css$/,
  /^src\/components\/ui\//,
  /^src\/gen\//,
];

const NUMBER_WEIGHT =
  /\bfont-(thin|extralight|light|normal|medium|semibold|bold|extrabold|black)\b/;
const ARBITRARY_TYPE = /\b(text|font|rounded)-\[[^\]]*\d[^\]]*\]/;
const CSS_LITERAL = /\b(font-size|font-weight|border-radius)\s*:\s*[\d.]/;

const offenders = [];
walk(join(root, "src"));

function walk(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p);
    else check(p);
  }
}

function check(path) {
  const rel = relative(root, path).replaceAll("\\", "/");
  if (ALLOW.some((rx) => rx.test(rel))) return;
  if (!/\.(tsx?|css|html)$/.test(rel)) return;
  const css = rel.endsWith(".css") || rel.endsWith(".html");
  readFileSync(path, "utf8")
    .split("\n")
    .forEach((line, i) => {
      if (
        NUMBER_WEIGHT.test(line) ||
        ARBITRARY_TYPE.test(line) ||
        (css && CSS_LITERAL.test(line))
      ) {
        offenders.push(`${rel}:${i + 1}: ${line.trim()}`);
      }
    });
}

// The tempo twins. Parsed, not assumed: a gate that greps for the
// numbers it remembers keeps passing after somebody changes one.
const cssYield = readFileSync(join(root, "src/styles/tokens.css"), "utf8").match(
  /--time-yield:\s*([\d.]+)s/,
);
const tsYield = readFileSync(join(root, "src/lib/tempo.ts"), "utf8").match(
  /YIELD_MS\s*=\s*([\d_]+)/,
);
if (!cssYield || !tsYield) {
  offenders.push("tempo: probe dead — --time-yield or YIELD_MS not found");
} else if (Number(cssYield[1]) * 1000 !== Number(tsYield[1].replaceAll("_", ""))) {
  offenders.push(
    `tempo: --time-yield ${cssYield[1]}s != YIELD_MS ${tsYield[1]} — one name, two numbers`,
  );
}

if (offenders.length) {
  console.error("type outside tokens — use a token (docs/DESIGN.md):\n" + offenders.join("\n"));
  process.exit(1);
}
console.log("type gate: tokens own the letterforms");
