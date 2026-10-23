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
// **One deliberate exception, and it is not a second family of the
// same kind**: the file-kind icons (`components/FileKindIcon`, Seti via
// `seti-icons`). The user set the one-family rule aside for those on
// 2026-09-16 —「对于开发者来说,one family 反而是认知负荷。他们需要更直观
// 的看到文件类型」— and they answer a different question from every
// mark in icons.tsx: not「what does this control do」but「what kind of
// file is this」, the way a language's colour in the sheet does. They
// are drawn in one component, sized by its callers, and coloured by
// VS Code's own palette. This gate does not match them because it
// matches lucide; it is written here so the next reader does not port
// them.
// Matched against module specifiers, not against the word: a gate that
// fires on prose fires on the comment explaining the gate, and the fix
// people reach for then is to stop writing the comment.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join, relative } from "node:path";

const root = fileURLToPath(new URL("..", import.meta.url));
// 2026-10-23: user requested the upstream AI Elements icon semantics.
// Only this adapter imports Lucide; app navigation keeps its own family.
const AI_ICONS = "src/components/ai-elements/icons.ts";
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
      if (rel !== AI_ICONS && FOREIGN.test(line)) offenders.push(`${rel}:${i + 1}: ${line.trim()}`);
    });
}

// ── The one named exception: brand marks (docs/SHELL.md 第五轮第 7 条改画法)
//
// Claude's, Codex's, Cursor's marks and the four system families' (apple,
// linux, windows, android; macOS's Finder is ours, drawn in icons.tsx —
// UX-27) are **somebody else's marks**, filled, drawn on a 1024 canvas: redrawn in this family's strokes they stop being
// recognisable, which is the only thing they are for. So they are let in
// — **by name, in one file**, `src/components/BrandIcons.tsx` — and held
// to the two things that keep them from becoming a second family by the
// back door:
//   · nowhere else draws on their canvas (`viewBox="0 0 1024 1024"`
//     outside that file is a brand mark that escaped, or a new family);
//   · nothing the exporter wrote survives in that file: no hard-coded
//     fill (`fill="#…"`: a mark that is `#040000` by fiat vanishes on the
//     dark theme), no `p-id` / `t="…"` / `data-darkreader-*`.
// Source of the set: iconfont.cn, provided by the user 2026-09-22, whose
// licence allows sharing (written beside the marks too).
const BRANDS = "src/components/BrandIcons.tsx";
const CANVAS = /viewBox=["']0 0 1024 1024["']/;
const LITTER = /fill=\\?["']#|\bp-id=|\bt=\\?["']\d|data-darkreader/;
walkBrands(join(root, "src"));

function walkBrands(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) {
      walkBrands(p);
      continue;
    }
    const rel = relative(root, p).replaceAll("\\", "/");
    if (!/\.(tsx?|html)$/.test(rel) || rel.startsWith("src/gen/")) continue;
    readFileSync(p, "utf8")
      .split("\n")
      .forEach((line, i) => {
        if (rel !== BRANDS && CANVAS.test(line)) {
          offenders.push(`${rel}:${i + 1}: a brand-mark canvas outside ${BRANDS}`);
        }
        // Prose is not markup: a comment naming the litter it removed is
        // this file's own lesson about gates that fire on the comment.
        if (rel === BRANDS && LITTER.test(line) && !/^\s*(\/\/|\*|\/\*)/.test(line)) {
          offenders.push(`${rel}:${i + 1}: exporter litter or a hard-coded fill in a brand mark`);
        }
      });
  }
}

if (offenders.length) {
  console.error(
    "a second icon family — port the mark into src/components/icons.tsx " +
      `(brand marks go in ${BRANDS}, cleaned):\n` +
      offenders.join("\n"),
  );
  process.exit(1);
}
console.log("icon gate: app family, scoped upstream AI Elements adapter, and cleaned brand marks");
