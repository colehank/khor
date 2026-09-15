// Syntax colouring for the conversation's code blocks and the file
// sheet — **Shiki, GitHub's own themes** (2026-09-15).
//
// # Why this and not highlight.js
//
// highlight.js was here first, with a theme this app wrote by hand: two
// roles at first, nine after the user opened a .py file and said 「我说
// 的语法高亮，是有颜色的」, and then 「应该用已有成熟工具啊。做到 gh web
// 预览代码的那种高亮」. What GitHub paints is a TextMate grammar under
// a theme, and that is exactly what Shiki runs: the grammars VS Code
// ships, and `github-light-default` / `github-dark-default` — the
// colours on github.com itself, as published, not approximated. Nothing
// in this file or in `styles/app.css` chooses a colour any more; the
// stylesheet only says which of the two the page is currently in.
//
// # What it costs, measured on the built face
//
// - The engine is the **JavaScript regex engine**, not oniguruma: no
//   wasm, no async init, and the grammars in the list below all run on
//   it (Shiki's own compatibility table; a grammar that did not would
//   throw at load and the block would go plain, never wrong).
// - **Grammars load on demand**, one dynamic import each, so a .py costs
//   the Python grammar and nothing else; the two themes load with the
//   first block. `crates/web/build.rs` compiles every file under `dist`
//   into the binary, chunks included, so the lazy road works on the web
//   face too.
// - package-lock diff on install: **16 packages, all MIT**.
//
// # The whitelist is still counted
//
// The conversation half (1089 transcripts, 7066 fenced blocks,
// 2026-08-30): bash 1073 · rust 985 · json 984 · css 691 · swift 631 ·
// ts 625 · js 456 · python 278 · tsx 267 · sh 260 · jsx 100 · yaml 94 ·
// toml 73 — and the disk half (every tracked file beside this checkout,
// 2026-09-11): rs 306 · py 262 · md 195 · json 129 · tsx 101 · ts 78 ·
// swift 75 · sh 34 · toml 29. Both halves are in the map; the long tail
// after them is the set a file sheet opens on whatever is on a disk —
// a .c or a .sql beside a coloured .py must not read as 「这个 khor 不
// 认识」. `mermaid` stays out (a diagram's source is the wrong thing to
// colour) and `text` is a request not to be coloured at all.

import type { HighlighterCore, LanguageRegistration, ThemedToken } from "shiki/core";
import { createHighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";

const LIGHT = "github-light-default";
const DARK = "github-dark-default";

type LangModule = { default: LanguageRegistration[] };

/** Every grammar this build can load, by Shiki's own id. Keyed so a name
    that is not here is refused at the map and never reaches a dynamic
    import that a bundler could not resolve. */
const GRAMMARS: Record<string, () => Promise<LangModule>> = {
  "bash": () => import("@shikijs/langs/bash"),
  "rust": () => import("@shikijs/langs/rust"),
  "json": () => import("@shikijs/langs/json"),
  "jsonc": () => import("@shikijs/langs/jsonc"),
  "css": () => import("@shikijs/langs/css"),
  "markdown": () => import("@shikijs/langs/markdown"),
  "swift": () => import("@shikijs/langs/swift"),
  "typescript": () => import("@shikijs/langs/typescript"),
  "tsx": () => import("@shikijs/langs/tsx"),
  "javascript": () => import("@shikijs/langs/javascript"),
  "jsx": () => import("@shikijs/langs/jsx"),
  "python": () => import("@shikijs/langs/python"),
  "yaml": () => import("@shikijs/langs/yaml"),
  "toml": () => import("@shikijs/langs/toml"),
  "ini": () => import("@shikijs/langs/ini"),
  "c": () => import("@shikijs/langs/c"),
  "cpp": () => import("@shikijs/langs/cpp"),
  "csharp": () => import("@shikijs/langs/csharp"),
  "java": () => import("@shikijs/langs/java"),
  "kotlin": () => import("@shikijs/langs/kotlin"),
  "go": () => import("@shikijs/langs/go"),
  "ruby": () => import("@shikijs/langs/ruby"),
  "php": () => import("@shikijs/langs/php"),
  "sql": () => import("@shikijs/langs/sql"),
  "xml": () => import("@shikijs/langs/xml"),
  "html": () => import("@shikijs/langs/html"),
  "makefile": () => import("@shikijs/langs/makefile"),
  "dockerfile": () => import("@shikijs/langs/dockerfile"),
  "r": () => import("@shikijs/langs/r"),
  "matlab": () => import("@shikijs/langs/matlab"),
  "latex": () => import("@shikijs/langs/latex"),
  "lua": () => import("@shikijs/langs/lua"),
  "perl": () => import("@shikijs/langs/perl"),
  "scss": () => import("@shikijs/langs/scss"),
  "less": () => import("@shikijs/langs/less"),
  "diff": () => import("@shikijs/langs/diff"),
  "scala": () => import("@shikijs/langs/scala"),
  "julia": () => import("@shikijs/langs/julia"),
  "dart": () => import("@shikijs/langs/dart"),
  "objective-c": () => import("@shikijs/langs/objective-c"),
  "cmake": () => import("@shikijs/langs/cmake"),
  "proto": () => import("@shikijs/langs/proto"),
  "graphql": () => import("@shikijs/langs/graphql"),
  "powershell": () => import("@shikijs/langs/powershell"),
  "haskell": () => import("@shikijs/langs/haskell"),
  "elixir": () => import("@shikijs/langs/elixir"),
  "erlang": () => import("@shikijs/langs/erlang"),
  "clojure": () => import("@shikijs/langs/clojure"),
  "viml": () => import("@shikijs/langs/viml"),
  "http": () => import("@shikijs/langs/http"),
  "nginx": () => import("@shikijs/langs/nginx"),
};

/** Extension or fence label → Shiki id. Lower-cased at the lookup. `m`
    is Objective-C and not MATLAB: counted on this disk, .m files beside
    a .xcodeproj outnumber the other kind. */
const LANGS: Record<string, string> = {
  bash: "bash",
  sh: "bash",
  shell: "bash",
  zsh: "bash",
  console: "bash",
  rust: "rust",
  rs: "rust",
  json: "json",
  jsonc: "jsonc",
  css: "css",
  markdown: "markdown",
  md: "markdown",
  swift: "swift",
  typescript: "typescript",
  ts: "typescript",
  tsx: "tsx",
  javascript: "javascript",
  js: "javascript",
  jsx: "jsx",
  mjs: "javascript",
  cjs: "javascript",
  python: "python",
  py: "python",
  yaml: "yaml",
  yml: "yaml",
  toml: "toml",
  ini: "ini",
  c: "c",
  h: "c",
  cpp: "cpp",
  cc: "cpp",
  cxx: "cpp",
  hpp: "cpp",
  hh: "cpp",
  cs: "csharp",
  java: "java",
  kt: "kotlin",
  kts: "kotlin",
  go: "go",
  rb: "ruby",
  php: "php",
  sql: "sql",
  xml: "xml",
  svg: "xml",
  plist: "xml",
  xib: "xml",
  storyboard: "xml",
  html: "html",
  htm: "html",
  makefile: "makefile",
  mk: "makefile",
  dockerfile: "dockerfile",
  r: "r",
  matlab: "matlab",
  tex: "latex",
  sty: "latex",
  bib: "latex",
  lua: "lua",
  pl: "perl",
  pm: "perl",
  scss: "scss",
  less: "less",
  diff: "diff",
  patch: "diff",
  scala: "scala",
  jl: "julia",
  dart: "dart",
  m: "objective-c",
  mm: "objective-c",
  cmake: "cmake",
  proto: "proto",
  graphql: "graphql",
  gql: "graphql",
  ps1: "powershell",
  hs: "haskell",
  ex: "elixir",
  exs: "elixir",
  erl: "erlang",
  clj: "clojure",
  cljs: "clojure",
  vim: "viml",
  http: "http",
  conf: "nginx",
};

export function languageOf(info: string): string | null {
  return LANGS[info.trim().toLowerCase()] ?? null;
}

let core: Promise<HighlighterCore> | null = null;

/** The one highlighter, made on first use with both themes and no
    grammar; grammars join as blocks ask for them. */
function highlighter(): Promise<HighlighterCore> {
  core ??= createHighlighterCore({
    themes: [import("@shikijs/themes/github-light-default"), import("@shikijs/themes/github-dark-default")],
    langs: [],
    engine: createJavaScriptRegexEngine(),
  });
  return core;
}

const LOADING = new Map<string, Promise<void>>();

async function ready(language: string): Promise<HighlighterCore | null> {
  const load = GRAMMARS[language];
  if (load === undefined) return null;
  const h = await highlighter();
  if (!h.getLoadedLanguages().includes(language)) {
    let p = LOADING.get(language);
    if (p === undefined) {
      p = h.loadLanguage(load()).finally(() => LOADING.delete(language));
      LOADING.set(language, p);
    }
    await p;
  }
  return h;
}

function escape(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

/** One line of tokens as markup. Only the spans and the text: the `pre`
    and the `code` belong to whoever is drawing (the sheet has its own,
    with a gutter measured against it), and lines are joined with the
    newline the text had, so the sheet's transparent editor lines up
    over the paint character for character. A token with no style —
    Shiki emits those for plain runs — is left bare text. */
function line(tokens: ThemedToken[]): string {
  return tokens
    .map((t) => {
      const style = t.htmlStyle;
      if (!style) return escape(t.content);
      const decl = Object.entries(style)
        .map(([k, v]) => `${k}:${v}`)
        .join(";");
      return `<span style="${decl}">${escape(t.content)}</span>`;
    })
    .join("");
}

/**
 * Already-painted blocks, so a closed one is painted once and not again.
 *
 * **Closing the fence is not the end of the turn.** The text a turn is
 * folded into keeps growing after a code block has closed — the prose
 * that follows it arrives chunk by chunk — and every one of those chunks
 * re-renders the whole turn. Without this, a block that closed early in
 * a long answer would be re-highlighted on every chunk for the rest of
 * it: the same string, the same output, tens or hundreds of times.
 *
 * Keyed on the text and the language because that pair *is* the output;
 * bounded because a conversation is unbounded, and the entries worth
 * keeping are the recent ones — an old turn is scrolled away and its
 * blocks are not being re-rendered.
 */
const PAINTED = new Map<string, string>();
const PAINTED_CAP = 64;

/** How many times a grammar was actually run. **For `check-read`**,
    which proves the cache by counting: highlight.js could be wrapped
    from outside at its entry point, and Shiki's highlighter is a
    closure made in here that nothing outside can reach. Read, never
    written, by anybody but `paint`. */
export const measured = { paints: 0 };

/** What a block already painted looks like, or null if it has not been
    painted yet — the synchronous half, for a render that cannot wait. */
export function painted(code: string, language: string): string | null {
  return PAINTED.get(`${language} ${code}`) ?? null;
}

/**
 * Paints, loading the grammar first if this is the first block in that
 * language. `null` for a language the map does not know and for a
 * grammar that refused — plain is the honest outcome either way, and a
 * block that came back wrong would be worse than a block that came back
 * bare.
 */
export async function paint(code: string, language: string): Promise<string | null> {
  const key = `${language} ${code}`;
  const seen = PAINTED.get(key);
  if (seen !== undefined) return seen;
  try {
    const h = await ready(language);
    if (h === null) return null;
    measured.paints += 1;
    const { tokens } = h.codeToTokens(code, {
      lang: language,
      themes: { light: LIGHT, dark: DARK },
      defaultColor: false,
    });
    const html = tokens.map(line).join("\n");
    // **Always remembered now.** The one caller that asked not to be
    // was the file sheet, and it went its own way in 工作区批⑨
    // (`paintFileLines` — a file needs its lines apart, and a 64-entry
    // cache keyed on a whole file is not for one). What is left here is
    // the conversation's blocks, which are exactly what the cache is
    // for, so the flag had no second reader and went with it.
    if (PAINTED.size >= PAINTED_CAP) PAINTED.delete(PAINTED.keys().next().value as string);
    PAINTED.set(key, html);
    return html;
  } catch {
    return null;
  }
}

/** A file's worth, **one string per source line** (工作区批⑨).
 *
 * Never remembered — a file is large, edited, and open once, none of
 * which a 64-entry cache keyed on the whole text is for.
 *
 * **Lines rather than one joined string, because the sheet wraps now.**
 * A wrapped file's line numbers and change gutter have to know how tall
 * each source line came out, and「how tall」 is only askable of an
 * element that is exactly one line. Joining with 「\n」 and letting
 * `pre-wrap` break it puts every line inside one box with nothing to
 * measure — which is the arrangement 工作区批⑨ replaced.
 *
 * Shiki's `codeToTokens` already hands lines back; this only refuses to
 * flatten them. `null` for a language this build does not carry and for
 * a grammar that refused — plain is the honest outcome either way.
 *
 * **The count is checked against the text's own.** A grammar that
 * returned a different number of lines than `split("\n")` would put
 * every number below it against the wrong code, so the whole painting
 * is dropped rather than half-trusted. */
export async function paintFileLines(code: string, language: string): Promise<string[] | null> {
  try {
    const h = await ready(language);
    if (h === null) return null;
    measured.paints += 1;
    const { tokens } = h.codeToTokens(code, {
      lang: language,
      themes: { light: LIGHT, dark: DARK },
      defaultColor: false,
    });
    const out = tokens.map(line);
    return out.length === code.split("\n").length ? out : null;
  } catch {
    return null;
  }
}

export function openFenceAt(text: string): number | null {
  let at = 0;
  let open: { marker: string; len: number; at: number } | null = null;
  for (const line of text.split("\n")) {
    const m = /^ {0,3}(`{3,}|~{3,})/.exec(line);
    if (m) {
      const marker = m[1][0];
      const len = m[1].length;
      const rest = line.slice(m[0].length);
      if (open === null) {
        if (marker !== "`" || !rest.includes("`")) open = { marker, len, at };
      } else if (marker === open.marker && len >= open.len && rest.trim() === "") {
        open = null;
      }
    }
    at += line.length + 1;
  }
  return open?.at ?? null;
}
