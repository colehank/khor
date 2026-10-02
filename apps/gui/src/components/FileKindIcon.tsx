// A file's kind, as the icon VS Code shows for it.
//
// **Seti, the set VS Code ships as its default file icon theme**
// (2026-09-16:「我们有没有可能和 github 和 vscode 一样,对每类文件有一个
// 小 icon」, and, on the one-family rule: 「我们这里不考虑 one family,
// 因为对于开发者来说,one family 反而是认知负荷。他们需要更直观的看到文件
// 类型」). The user chose Seti over the two bigger sets by size and by
// being what VS Code draws out of the box: 135 shapes, 161 KB, MIT,
// against Material's 1,251 shapes and 1.67 MB.
//
// **Vendored, not depended on** (`src/vendor/seti`, its LICENSE says
// where from). The npm package that carries these two JSON files lists
// svgo as a runtime dependency; installing it put 134 packages in the
// lock file for 155 KB of data. The lookup below is the package's own,
// twenty lines: an exact file name first, then every suffix from the
// first dot on, then a few partial names, then the default.
//
// **Two palettes, both VS Code's own.** Seti's theme file in VS Code
// (`extensions/theme-seti/icons/vs-seti-icon-theme.json`) paints every
// icon in one of ten named colours and carries a darker twin of each
// for the light theme; the two tables below are those twenty values,
// read off that file, and khor picks none of them — the rule the
// highlighter set (`lib/highlight.ts`: the colours are GitHub's).
// Which twin shows is `app.css`'s business, on the same media query the
// code colours switch on.
//
// **One live area, the house's** (2026-10-02, icons.tsx `LIVE`). Seti
// draws its shapes in a font's em box, each with its own air — measured,
// from 0.38 of the box (the default document) to all of it (elm, twig).
// `FileKindIcon.live.json` names, per shape, the square of its canvas its
// ink is centred in at 0.90, measured by rasterising; the shape is
// untouched, only its `viewBox` is swapped. So a box paints one size
// whatever the file is, and a seat sizes the ink by sizing the box.
// `elm` and `twig` keep their own canvas: each carries a `<rect>` of 100%
// that paints the whole box, so they have no ink to fit.
//
// **Only files.** A directory's slot holds its chevron (`FileTree`),
// which already says what it is; Seti's folder glyph would be a second
// answer to a question the row has answered.
import definitions from "@/vendor/seti/definitions.json";
import icons from "@/vendor/seti/icons.json";
import live from "./FileKindIcon.live.json";

import { cn } from "@/lib/utils";

/** Seti's colour names → the values VS Code paints them in, dark theme. */
const DARK: Record<string, string> = {
  blue: "#519aba",
  red: "#cc3e44",
  orange: "#e37933",
  purple: "#a074c4",
  green: "#8dc149",
  yellow: "#cbcb41",
  white: "#d4d7d6",
  "grey-light": "#6d8086",
  pink: "#f55385",
  grey: "#4d5a5e",
  ignore: "#41535b",
};
/** …and the light theme's twins, from the same file's `light` block. */
const LIGHT: Record<string, string> = {
  blue: "#498ba7",
  red: "#b8383d",
  orange: "#cc6d2e",
  purple: "#9068b0",
  green: "#7fae42",
  yellow: "#b7b73b",
  white: "#bfc2c1",
  "grey-light": "#627379",
  pink: "#dd4b78",
  grey: "#455155",
  ignore: "#3b4b50",
};

/** What Seti says a file name is: its shape as an `<svg>` string and
    its colour name. Exported for the probes, which compare shapes
    between two extensions rather than trusting a class. */
type Pick = [icon: string, color: string];
const DEFS = definitions as unknown as {
  files: Record<string, Pick>;
  extensions: Record<string, Pick>;
  partials: [string, Pick][];
  default: Pick;
};
const SHAPES = Object.fromEntries(
  Object.entries(icons as unknown as Record<string, string>).map(([k, svg]) => [
    k,
    svg.replace(/viewBox="[^"]*"/, `viewBox="${(live as Record<string, string>)[k]}"`),
  ]),
);

function pick(name: string): Pick {
  if (Object.hasOwn(DEFS.files, name)) return DEFS.files[name];
  let ext = name.slice(name.indexOf("."));
  while (ext !== "" && name.includes(".")) {
    if (Object.hasOwn(DEFS.extensions, ext)) return DEFS.extensions[ext];
    ext = ext.slice(1);
    ext = ext.slice(ext.indexOf("."));
    if (!ext.startsWith(".")) break;
  }
  for (const [part, p] of DEFS.partials) if (name.includes(part)) return p;
  return DEFS.default;
}

export function kindOf(name: string): { svg: string; color: string } {
  const [icon, color] = pick(name);
  return { svg: SHAPES[icon] ?? SHAPES[DEFS.default[0]], color };
}

/** The icon, sized by the caller (`size-3` in the tree's type slot,
    `size-4` on a tab). **The SVG string is the package's own data**,
    MIT, shipped as text — it is set as markup for that reason and no
    other; nothing from a user or a machine reaches this. */
export function FileKindIcon({ name, className }: { name: string; className?: string }) {
  const { svg, color } = kindOf(name);
  return (
    <span
      data-file-kind={color}
      aria-hidden="true"
      className={cn("inline-flex size-4 flex-none [&>svg]:size-full [&>svg]:fill-current", className)}
      style={
        {
          "--kind-light": LIGHT[color] ?? LIGHT.white,
          "--kind-dark": DARK[color] ?? DARK.white,
        } as React.CSSProperties
      }
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}
