// The px gate: no pixel literal outside the whitelist — tokens.css owns
// the numbers, ui/ owns the controls, gen/ is machine output. Everything
// else must speak in tokens. mandala's styles grew 614 stray px this
// gate exists to prevent; discipline as a build error, not a review nag.
//
// **Comments are not code, and this gate used to disagree.** It read
// every line whole, so a comment recording a measurement — 「the pid sat
// 229px into the panel」, 「four rows were 128px」 — was a build error,
// and the way past it was to write 「229 像素」 instead. That is a gate
// making the prose around the code worse to satisfy a rule about the
// code. Twice in one night (2026-09-04), by the person who had just
// been bitten by it once.
//
// So comments are blanked before the scan. **Strings are not** — a
// pixel literal in this codebase almost always lives inside one
// (`className="h-[10px]"`, `style={{ top: "3px" }}`), and exempting
// strings would empty the gate out. Knowing where the strings are is
// required anyway, and not for tidiness: `"https://…"` contains `//`,
// and a scanner that took that for a comment would blank the rest of
// the line and stop seeing anything after it. **The failure of a
// careless stripper is silence, not noise**, which is the direction a
// gate must never fail in.
import { readdirSync, readFileSync, realpathSync, statSync } from "node:fs";
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

/**
 * The file with its comments blanked out, character for character, so
 * line and column numbers still line up with the original.
 *
 * `kind` is "ts" (line comments, block comments, and the three string
 * forms), "css" (block comments only — CSS has no line comment, and a
 * `//` in a url() is not one) or "html" (`<!-- -->`).
 *
 * Blanking rather than deleting: the caller reports the *original*
 * line, so what a reader sees in the error is the line they wrote.
 */
export function blankComments(text, kind) {
  const out = text.split("");
  const blank = (i) => {
    if (out[i] !== "\n") out[i] = " ";
  };
  let i = 0;
  while (i < text.length) {
    const two = text.slice(i, i + 2);
    if (kind === "html") {
      if (text.startsWith("<!--", i)) {
        const end = text.indexOf("-->", i + 4);
        const stop = end === -1 ? text.length : end + 3;
        for (let k = i; k < stop; k++) blank(k);
        i = stop;
        continue;
      }
      i += 1;
      continue;
    }
    if (two === "/*") {
      const end = text.indexOf("*/", i + 2);
      const stop = end === -1 ? text.length : end + 2;
      for (let k = i; k < stop; k++) blank(k);
      i = stop;
      continue;
    }
    if (kind === "ts") {
      if (two === "//") {
        let k = i;
        while (k < text.length && text[k] !== "\n") blank(k++);
        i = k;
        continue;
      }
      // A string: walked through, never blanked, so that a `//` or a
      // `/*` inside one cannot start a comment. Escapes are honoured
      // for the same reason — `"\\"` ends where it looks like it ends.
      if (two[0] === '"' || two[0] === "'" || two[0] === "`") {
        const quote = two[0];
        i += 1;
        while (i < text.length) {
          if (text[i] === "\\") {
            i += 2;
            continue;
          }
          if (text[i] === quote) {
            i += 1;
            break;
          }
          // An unterminated string would otherwise swallow the file;
          // a newline ends the two quote forms that cannot span one.
          if (text[i] === "\n" && quote !== "`") break;
          i += 1;
        }
        continue;
      }
    }
    i += 1;
  }
  return out.join("");
}

const kindOf = (rel) => (rel.endsWith(".css") ? "css" : rel.endsWith(".html") ? "html" : "ts");

export function offendersIn(rel, text) {
  const found = [];
  const lines = text.split("\n");
  const scanned = blankComments(text, kindOf(rel)).split("\n");
  scanned.forEach((line, i) => {
    if (PX.test(line) || (/className/.test(line) && ARBITRARY.test(line))) {
      found.push(`${rel}:${i + 1}: ${lines[i].trim()}`);
    }
  });
  return found;
}

function walk(dir, out) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p, out);
    else check(p, out);
  }
}

function check(path, out) {
  const rel = relative(root, path).replaceAll("\\", "/");
  if (ALLOW.some((rx) => rx.test(rel))) return;
  if (!/\.(tsx?|css|html)$/.test(rel)) return;
  out.push(...offendersIn(rel, readFileSync(path, "utf8")));
}

// Only when run as the gate. The two functions above are exported so
// the change to this file could be checked against a file full of real
// pixel literals (tokens.css) rather than against a hand-made sample.
const isMain = process.argv[1] && realpathSync(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const offenders = [];
  walk(join(root, "src"), offenders);
  if (offenders.length) {
    console.error(
      "px outside tokens/ui — use a token:\n" +
        offenders.join("\n") +
        "\n(comments are exempt — these are in code or in a string literal)",
    );
    process.exit(1);
  }
  console.log("px gate: clean");
}
