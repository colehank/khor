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
  // `src/day/` is the 24-hour panel and its stylesheet, moved across
  // whole from the prototype that is its definition — vendored in the
  // same sense `ui/` is. **The numbers in there are the design.**
  // Re-expressing them as tokens is the re-implementation the
  // transplant exists to undo, and this repo has already paid for that
  // once (2026-09-04).
  /^src\/day\//,
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

// ── and the sizes survive being merged ───────────────────────────────
//
// **A size can be written correctly and still not reach the screen.**
// `cn()` is tailwind-merge, which knows Tailwind's own size names and
// files every other `text-*` as a colour — so this app's six own sizes
// were in the colour group, and any colour class in the same call
// superseded them. A paragraph came back at the shell's base with the
// browser's leading: no error, `tsc` clean, build clean. `lib/utils.ts`
// registers them; this holds that it worked.
//
// **Behaviour, not the presence of a line.** Grepping `utils.ts` for
// `extendTailwindMerge` would go red the day somebody writes it another
// way, and stay green the day the registration is right there but wrong.
//
// **And both directions.** That file changes the merge rules for the
// whole app, so「the new names work」is half a claim; the other half is
// that the names that already worked still do, and still *conflict*.
{
  const { cn } = await import("../src/lib/utils.ts");
  const css = readFileSync(join(root, "src/styles/tokens.css"), "utf8");
  // Every size the theme declares, the companions dropped: `--text-x`
  // is a size, `--text-x--line-height` is its leading.
  const sizes = [...css.matchAll(/^\s*--text-([a-z0-9-]+):/gm)]
    .map((m) => m[1])
    .filter((n) => !n.endsWith("--line-height"));

  // The probe. A parse that found nothing would let the loop below pass
  // by doing nothing, and「all sizes survive」would be a sentence about
  // an empty list.
  if (sizes.length < 6) {
    offenders.push(`merge: probe dead — only ${sizes.length} --text-* found in tokens.css`);
  } else {
    // 1) every size the theme has, next to a colour, in both orders.
    //    Read against a colour that is *not* a size, which is the shape
    //    the failure took: `text-read text-read-ink` lost the size.
    for (const n of sizes) {
      for (const [a, b] of [
        [`text-${n}`, "text-muted-foreground"],
        ["text-muted-foreground", `text-${n}`],
      ]) {
        const out = cn(a, b);
        if (!out.includes(`text-${n}`)) {
          offenders.push(
            `merge: cn("${a}", "${b}") = "${out}" — the size is gone. ` +
              `--text-${n} exists in tokens.css but nothing told tailwind-merge it is a size ` +
              `(lib/utils.ts), so it is being filed as a colour and superseded`,
          );
        }
      }
    }
    // 2) the sizes really joined the size group rather than being made
    //    inert: two of them in one call must still resolve to the last.
    const twoOwn = cn("text-read", "text-aux");
    if (twoOwn !== "text-aux") {
      offenders.push(`merge: cn("text-read", "text-aux") = "${twoOwn}", want "text-aux"`);
    }
    const ownAndStock = cn("text-read", "text-sm");
    if (ownAndStock !== "text-sm") {
      offenders.push(`merge: cn("text-read", "text-sm") = "${ownAndStock}", want "text-sm"`);
    }
    // 3) nothing that already worked was broken by the registration.
    for (const [call, want] of [
      [["text-sm", "text-lg"], "text-lg"],
      [["text-lg", "text-sm"], "text-sm"],
      [["text-sm", "text-muted-foreground"], "text-sm text-muted-foreground"],
      [["text-foreground", "text-muted-foreground"], "text-muted-foreground"],
      [["p-2", "p-4"], "p-4"],
      [["font-mono", "font-sans"], "font-sans"],
    ]) {
      const out = cn(...call);
      if (out !== want) {
        offenders.push(
          `merge: cn(${call.map((c) => `"${c}"`).join(", ")}) = "${out}", want "${want}" — ` +
            `registering this app's sizes changed a rule it was not meant to touch`,
        );
      }
    }
  }
}

if (offenders.length) {
  console.error("type outside tokens — use a token (docs/DESIGN.md):\n" + offenders.join("\n"));
  process.exit(1);
}
console.log("type gate: tokens own the letterforms, and cn() keeps them");
