// The px gate: no pixel literal outside the whitelist — tokens.css owns
// the numbers, ui/ owns the controls, gen/ is machine output. Everything
// else must speak in tokens. mandala's styles grew 614 stray px this
// gate exists to prevent; discipline as a build error, not a review nag.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join, relative } from "node:path";

const root = fileURLToPath(new URL("..", import.meta.url));
// tokens own the numbers; shadcn ui/ components are vendored; gen/ is
// machine output; the one breakpoint lives in use-narrow.
const ALLOW = [
  /^src\/styles\/tokens\.css$/,
  /^src\/components\/ui\//,
  /^src\/gen\//,
  /^src\/hooks\/use-narrow\.ts$/,
];
// px inside a value, or a tailwind arbitrary value carrying px
const PX = /\b\d+(\.\d+)?px\b/;
const ARBITRARY = /\[[^\]]*\d+(\.\d+)?px[^\]]*\]/;

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
  readFileSync(path, "utf8")
    .split("\n")
    .forEach((line, i) => {
      if (PX.test(line) || (/className/.test(line) && ARBITRARY.test(line))) {
        offenders.push(`${rel}:${i + 1}: ${line.trim()}`);
      }
    });
}

if (offenders.length) {
  console.error("px outside tokens/ui — use a token:\n" + offenders.join("\n"));
  process.exit(1);
}
console.log("px gate: clean");
