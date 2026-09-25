// **tokens.css against the design system's tokens.json, both themes**
// (docs/SHELL.md 第十五轮 第 1 条; docs/design-system/guidelines/
// 30-behavior.md 「`tokens.css` 与这份系统的值一致 → check-boot」).
//
// The design system (`docs/design-system/tokens.json`) is where a value
// is decided; `src/styles/tokens.css` is where the app reads it. Two
// homes for one number rot the way the boot frame's colours did — no
// error, one of them just stops being true — so this reads both and
// compares every token the design system names: colours in the light
// and the dark theme, the fourteen type styles (size, weight, leading),
// spacing, radii, shadows, sizes, opacities, durations, easings.
//
// **How a CSS value is read.** The file is split into its top-level
// blocks; `:root`, `@theme static` and `@theme inline` are the light
// theme and everything theme-free, `:root[data-theme="dark"]` is laid
// over that for the dark one. A `var(--x)` is followed to its value in
// the same theme, so `--ring: var(--brand-strong)` compares as the
// colour it is. Colours are compared as numbers (a hex and an `rgba()`
// of the same colour agree); other values by their numbers and words.
//
// **Exceptions are named here, each with its reason, and printed on
// every run** — a gate that skips quietly is the 假绿 this repo keeps
// writing tombstones for. Imported by `check-boot.mjs` (`npm run boot`,
// and `npm test`'s first leg).
import { readFileSync } from "node:fs";

/** The CSS name a design-system name is spelled as, where they differ. */
const CSS_NAME = {
  canvas: "background",
  ink: "foreground",
  secondary: "muted-foreground",
  hairline: "border",
};

/**
 * Tokens whose CSS value the design system knows it does not carry, and
 * why. Each is printed, never silently passed.
 */
export const EXCEPTIONS = {
  "color:git-mark":
    "tokens.css writes it as color-mix(--foreground 50%, transparent); tokens.json's format cannot hold a function and carries an rgba() stand-in (40-migration, the section on what was not carried over)",
  "color:day-unwatched":
    "color-mix in tokens.css, an rgba() stand-in in tokens.json — the same note as git-mark",
  "type:read:line":
    "read's leading is 24 (1.5), the owner's 2026-08-30 choice to match the reference product whole, measured by check-read; tokens.json says 1.6 and SHELL.md round 15 says read does not move — the owner decides which one moves",
};

function stripComments(css) {
  return css.replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, " "));
}

/** The declarations of each top-level block whose header matches. */
function blocks(css) {
  const out = [];
  const text = stripComments(css);
  const head = /(^|\n)([^\n{}]+)\{/g;
  let m;
  while ((m = head.exec(text))) {
    const start = m.index + m[0].length;
    let depth = 1;
    let i = start;
    for (; i < text.length && depth > 0; i++) {
      if (text[i] === "{") depth++;
      else if (text[i] === "}") depth--;
    }
    out.push({ header: m[2].trim(), body: text.slice(start, i - 1) });
    head.lastIndex = i;
  }
  return out;
}

function decls(body) {
  const map = {};
  for (const d of body.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/gi)) map[d[1]] = d[2].trim();
  return map;
}

/** Both themes' declarations, the dark one laid over the light. */
export function readCss(css) {
  const light = {};
  const dark = {};
  for (const b of blocks(css)) {
    if (b.header === ":root" || b.header === "@theme static" || b.header === "@theme inline") {
      Object.assign(light, decls(b.body));
    } else if (b.header === ':root[data-theme="dark"]') {
      Object.assign(dark, decls(b.body));
    }
  }
  return { light, dark: { ...light, ...dark } };
}

function resolve(theme, name, seen = new Set()) {
  if (seen.has(name)) return null;
  seen.add(name);
  const v = theme[name];
  if (v === undefined) return null;
  const only = v.match(/^var\((--[a-z0-9-]+)\)$/i);
  return only ? resolve(theme, only[1], seen) : v;
}

function colour(v) {
  const t = String(v).trim().toLowerCase();
  if (t.startsWith("#")) {
    const h = t.slice(1);
    const parts = h.length === 3 ? h.split("").map((c) => c + c) : [h.slice(0, 2), h.slice(2, 4), h.slice(4, 6)];
    const a = h.length === 8 ? parseInt(h.slice(6, 8), 16) / 255 : 1;
    return [...parts.map((c) => parseInt(c, 16)), a];
  }
  const m = t.match(/^rgba?\(([^)]+)\)$/);
  if (!m) return null;
  const n = m[1].split(/[\s,/]+/).filter(Boolean).map(Number);
  return [n[0], n[1], n[2], n[3] === undefined ? 1 : n[3]];
}

const sameColour = (a, b) =>
  a !== null && b !== null && [0, 1, 2].every((i) => Math.abs(a[i] - b[i]) < 0.6) && Math.abs(a[3] - b[3]) < 0.006;

/** A value as its words and numbers: `0 12px 32px rgba(0,0,0,0.40)` and
    `0 12px 32px rgba(0, 0, 0, 0.4)` read the same; `55%` is `0.55`. */
function norm(v) {
  let t = String(v).trim().toLowerCase().replace(/\s*,\s*/g, ",").replace(/\s+/g, " ");
  if (/^-?[\d.]+%$/.test(t)) t = String(parseFloat(t) / 100);
  return t.replace(/-?\d*\.?\d+/g, (n) => String(parseFloat(n)));
}

const px = (v) => (v === null || v === undefined ? null : /px$/.test(String(v).trim()) ? parseFloat(v) : null);

/** Every disagreement between tokens.json and tokens.css, and the
    exceptions that were met. */
export function compare(json, css) {
  const { light, dark } = readCss(css);
  const bad = [];
  const met = [];
  let checked = 0;
  const except = (key, why) => {
    if (EXCEPTIONS[key]) {
      met.push(`${key}: ${EXCEPTIONS[key]}`);
      return true;
    }
    bad.push(why);
    return false;
  };

  for (const t of json.color.tokens) {
    const name = `--${CSS_NAME[t.name] ?? t.name}`;
    for (const [themeName, theme] of [
      ["light", light],
      ["dark", dark],
    ]) {
      checked++;
      const want = t.value[themeName];
      const got = resolve(theme, name);
      if (got === null) {
        except(`color:${t.name}`, `${themeName} ${name} (${t.name}) is not in tokens.css; tokens.json has ${want}`);
      } else if (!sameColour(colour(got), colour(want))) {
        except(`color:${t.name}`, `${themeName} ${name} (${t.name}) is ${got} in tokens.css, ${want} in tokens.json`);
      }
    }
  }

  for (const group of json.type.groups) {
    for (const s of group.styles) {
      checked++;
      const base = `--text-${s.name}`;
      const size = px(resolve(light, base));
      const want = parseFloat(s.fontSize);
      if (size === null) {
        bad.push(`type ${s.name}: ${base} is not in tokens.css (tokens.json ${s.fontSize})`);
        continue;
      }
      if (Math.abs(size - want) > 0.01) bad.push(`type ${s.name}: ${base} is ${size}px, tokens.json ${s.fontSize}`);
      const weight = resolve(light, `${base}--font-weight`);
      if (weight === null || Number(weight) !== Number(s.fontWeight)) {
        bad.push(`type ${s.name}: ${base}--font-weight is ${weight}, tokens.json ${s.fontWeight}`);
      }
      const lh = px(resolve(light, `${base}--line-height`));
      const wantLh = typeof s.lineHeight === "string" ? parseFloat(s.lineHeight) : s.lineHeight * want;
      // Leadings are whole pixels in tokens.css (WebKit snaps rows —
      // `check-wk`), so a ratio's product may be rounded, never moved.
      if (lh === null || Math.abs(lh - wantLh) > 0.5) {
        except(`type:${s.name}:line`, `type ${s.name}: ${base}--line-height is ${lh}px, tokens.json ${s.lineHeight} → ${+wantLh.toFixed(2)}px`);
      }
    }
  }

  const plain = [
    ["spacing", (n) => n],
    ["radius", (n) => n],
    ["size", (n) => n],
    ["opacity", (n) => n],
    ["duration", (n) => n],
    ["easing", (n) => n],
  ];
  for (const [family, cssName] of plain) {
    for (const t of json[family].tokens) {
      checked++;
      const name = `--${cssName(t.name)}`;
      const got = resolve(light, name);
      if (got === null) bad.push(`${family} ${t.name}: ${name} is not in tokens.css (tokens.json ${t.value})`);
      else if (norm(got) !== norm(t.value)) bad.push(`${family} ${t.name}: ${name} is ${got} in tokens.css, ${t.value} in tokens.json`);
    }
  }
  for (const t of json.shadow.tokens) {
    for (const [themeName, theme] of [
      ["light", light],
      ["dark", dark],
    ]) {
      checked++;
      const name = `--shadow-${t.name}`;
      const got = resolve(theme, name);
      if (got === null) bad.push(`${themeName} shadow ${t.name}: ${name} is not in tokens.css`);
      else if (norm(got) !== norm(t.value[themeName])) bad.push(`${themeName} shadow ${t.name}: ${name} is ${got}, tokens.json ${t.value[themeName]}`);
    }
  }
  return { bad, met: [...new Set(met)], checked };
}

export function compareFiles(jsonPath, cssPath) {
  return compare(JSON.parse(readFileSync(jsonPath, "utf8")), readFileSync(cssPath, "utf8"));
}
