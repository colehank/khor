// The one-family gate: no icon may come from anywhere but
// src/components/icons.tsx. Stroke width is a ratio to its canvas, and
// two families pick different ratios — side by side on one screen that
// reads the way two typefaces in one paragraph read.
//
// What this actually catches is not a person typing `lucide-react`: it is
// a future `shadcn add`, which vendors components carrying lucide marks
// and adds the dependency without being asked. That lands as a silent
// second family; here it lands as a build error, next to the porting work
// it implies.
// Matched against module specifiers, not against the word: a gate that
// fires on prose fires on the comment explaining the gate, and the fix
// people reach for then is to stop writing the comment.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join, relative } from "node:path";

const root = fileURLToPath(new URL("..", import.meta.url));
const FOREIGN = /["'][^"']*lucide[^"']*["']/;

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
  if (!/\.(tsx?|css|html)$/.test(rel)) return;
  readFileSync(path, "utf8")
    .split("\n")
    .forEach((line, i) => {
      if (FOREIGN.test(line)) offenders.push(`${rel}:${i + 1}: ${line.trim()}`);
    });
}

if (offenders.length) {
  console.error(
    "a second icon family — port the mark into src/components/icons.tsx:\n" + offenders.join("\n"),
  );
  process.exit(1);
}
console.log("icon gate: one family");
