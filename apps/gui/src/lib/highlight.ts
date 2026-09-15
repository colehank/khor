// Syntax colouring for the conversation's code blocks and the file
// sheet.
//
// **Nine roles, in colour (2026-09-15).** This file used to argue for
// three — comments, strings, and body ink for everything else — on the
// ground that the palette was spoken for. The user opened a .py file on
// the sheet and ruled 「我说的语法高亮，是有颜色的」; the palette grew
// seven `--code-*` tokens (`styles/tokens.css`, GitHub's values in both
// themes) and the mapping is in `styles/app.css`, because these are
// class names a stylesheet has to name; this file only produces them.
//
// **The tool is highlight.js, not Shiki.** Shiki paints VS Code's exact
// colours but ships TextMate grammars plus an oniguruma wasm, over a
// megabyte and async; highlight.js is already in the tree, synchronous,
// and a grammar here is 5–30 KB. Per-language modules rather than the
// all-languages bundle (~1 MB) for the same reason.
//
// **The whitelist is counted, not guessed.** 1089 real agent transcripts
// on this machine (2026-08-30), 7066 fenced blocks that named a
// language:
//
//   bash 1073 · rust 985 · json 984 · css 691 · swift 631 · ts 625 ·
//   js 456 · python 278 · tsx 267 · sh 260 · jsx 100 · yaml 94 ·
//   javascript 87 · toml 73 · mermaid 73 · text 65 · typescript 62 ·
//   …then a long tail of ≤38
//
// The ten modules below, with their aliases folded in, cover **6754 of
// 7066 — 95.6%**. What is deliberately left out: `mermaid` (73) is a
// diagram language, not code, and colouring its source would be
// colouring the wrong thing; `text`/`plaintext` (65) is a request *not*
// to be highlighted; everything else in the tail is under 40 and each
// module costs bytes that ship inside khor's one binary.
//
// **A second consumer arrived on 2026-09-11, and it counts differently**
// (工作区批2). The numbers above are *fenced blocks in conversations*;
// the file view asks the same question about *files on a disk*, and the
// two distributions disagree where it matters. Counted the same way,
// over every tracked file in the checkouts beside this one:
//
//   rs 306 · py 262 · md 195 · json 129 · tsx 101 · png 94 · ts 78 ·
//   swift 75 · sh 34 · toml 29 · mjs 28 · svg 24 · jsonl 22 · css 18 ·
//   yaml 14
//
// **`md` is third here and absent up there**, because nobody fences a
// markdown block inside a conversation — so the list below gained one
// module and not a rewrite. `png`/`svg` are not omissions: an image is
// not painted by colouring its bytes. The rest of the tail is under 30
// and each module costs bytes in khor's one binary, same as before.
//
// **A language not on the list is not an error and not a guess.** It
// draws exactly as it drew before this file existed. `highlightAuto` is
// never called: it is the expensive path and it is wrong often enough
// that a Rust snippet labelled nothing would come back coloured as
// something else, which is worse than plain.
import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import css from "highlight.js/lib/languages/css";
import ini from "highlight.js/lib/languages/ini";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import markdown from "highlight.js/lib/languages/markdown";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import swift from "highlight.js/lib/languages/swift";
import typescript from "highlight.js/lib/languages/typescript";
import yaml from "highlight.js/lib/languages/yaml";
// **The long tail, added 2026-09-15 with the colours**: a file sheet is
// opened on whatever is on the disk, and a .c or .sql that opened in
// body ink beside a coloured .py would read as「this one khor does not
// understand」. Each is one small module; the whole set is measured on
// the built face below, not guessed.
import c from "highlight.js/lib/languages/c";
import cpp from "highlight.js/lib/languages/cpp";
import csharp from "highlight.js/lib/languages/csharp";
import java from "highlight.js/lib/languages/java";
import kotlin from "highlight.js/lib/languages/kotlin";
import go from "highlight.js/lib/languages/go";
import ruby from "highlight.js/lib/languages/ruby";
import php from "highlight.js/lib/languages/php";
import sql from "highlight.js/lib/languages/sql";
import xml from "highlight.js/lib/languages/xml";
import makefile from "highlight.js/lib/languages/makefile";
import dockerfile from "highlight.js/lib/languages/dockerfile";
import r from "highlight.js/lib/languages/r";
import matlab from "highlight.js/lib/languages/matlab";
import latex from "highlight.js/lib/languages/latex";
import lua from "highlight.js/lib/languages/lua";
import perl from "highlight.js/lib/languages/perl";
import scss from "highlight.js/lib/languages/scss";
import less from "highlight.js/lib/languages/less";
import diff from "highlight.js/lib/languages/diff";
import scala from "highlight.js/lib/languages/scala";
import julia from "highlight.js/lib/languages/julia";
import dart from "highlight.js/lib/languages/dart";
import objectivec from "highlight.js/lib/languages/objectivec";
import cmake from "highlight.js/lib/languages/cmake";
import protobuf from "highlight.js/lib/languages/protobuf";
import graphql from "highlight.js/lib/languages/graphql";
import powershell from "highlight.js/lib/languages/powershell";
import nginx from "highlight.js/lib/languages/nginx";
import haskell from "highlight.js/lib/languages/haskell";
import elixir from "highlight.js/lib/languages/elixir";
import erlang from "highlight.js/lib/languages/erlang";
import clojure from "highlight.js/lib/languages/clojure";
import vim from "highlight.js/lib/languages/vim";
import http from "highlight.js/lib/languages/http";

hljs.registerLanguage("bash", bash);
hljs.registerLanguage("css", css);
hljs.registerLanguage("ini", ini);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("json", json);
hljs.registerLanguage("markdown", markdown);
hljs.registerLanguage("python", python);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("swift", swift);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("yaml", yaml);
hljs.registerLanguage("c", c);
hljs.registerLanguage("cpp", cpp);
hljs.registerLanguage("csharp", csharp);
hljs.registerLanguage("java", java);
hljs.registerLanguage("kotlin", kotlin);
hljs.registerLanguage("go", go);
hljs.registerLanguage("ruby", ruby);
hljs.registerLanguage("php", php);
hljs.registerLanguage("sql", sql);
hljs.registerLanguage("xml", xml);
hljs.registerLanguage("makefile", makefile);
hljs.registerLanguage("dockerfile", dockerfile);
hljs.registerLanguage("r", r);
hljs.registerLanguage("matlab", matlab);
hljs.registerLanguage("latex", latex);
hljs.registerLanguage("lua", lua);
hljs.registerLanguage("perl", perl);
hljs.registerLanguage("scss", scss);
hljs.registerLanguage("less", less);
hljs.registerLanguage("diff", diff);
hljs.registerLanguage("scala", scala);
hljs.registerLanguage("julia", julia);
hljs.registerLanguage("dart", dart);
hljs.registerLanguage("objectivec", objectivec);
hljs.registerLanguage("cmake", cmake);
hljs.registerLanguage("protobuf", protobuf);
hljs.registerLanguage("graphql", graphql);
hljs.registerLanguage("powershell", powershell);
hljs.registerLanguage("nginx", nginx);
hljs.registerLanguage("haskell", haskell);
hljs.registerLanguage("elixir", elixir);
hljs.registerLanguage("erlang", erlang);
hljs.registerLanguage("clojure", clojure);
hljs.registerLanguage("vim", vim);
hljs.registerLanguage("http", http);

/**
 * What an agent writes after the fence, to the module that reads it.
 *
 * Spelled out rather than handed to highlight.js's own alias table: that
 * table would also resolve names for languages this build does not
 * register, and the answer to those has to be "draw it plain", not a
 * throw at paint time.
 */
const LANGS: Record<string, string> = {
  bash: "bash",
  sh: "bash",
  shell: "bash",
  zsh: "bash",
  console: "bash",
  rust: "rust",
  rs: "rust",
  json: "json",
  // A JSON file with comments in it: the comments are the reason it is
  // spelled differently, and they are the one thing this build would
  // otherwise paint as body ink.
  jsonc: "json",
  css: "css",
  // Third by file count and absent by fence count — the second
  // consumer's whole difference, in one entry.
  markdown: "markdown",
  md: "markdown",
  swift: "swift",
  typescript: "typescript",
  ts: "typescript",
  tsx: "typescript",
  javascript: "javascript",
  js: "javascript",
  jsx: "javascript",
  mjs: "javascript",
  python: "python",
  py: "python",
  yaml: "yaml",
  yml: "yaml",
  // highlight.js files TOML under `ini` — same module, and the two are
  // near enough that the comment and string roles land correctly.
  toml: "ini",
  ini: "ini",
  // The tail (2026-09-15). `m` is Objective-C and not MATLAB: counted
  // on this disk, .m files beside a .xcodeproj outnumber the other kind.
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
  html: "xml",
  htm: "xml",
  svg: "xml",
  plist: "xml",
  xib: "xml",
  storyboard: "xml",
  makefile: "makefile",
  mk: "makefile",
  dockerfile: "dockerfile",
  r: "r",
  m: "objectivec",
  mm: "objectivec",
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
  cmake: "cmake",
  proto: "protobuf",
  graphql: "graphql",
  gql: "graphql",
  ps1: "powershell",
  hs: "haskell",
  ex: "elixir",
  exs: "elixir",
  erl: "erlang",
  clj: "clojure",
  cljs: "clojure",
  vim: "vim",
  http: "http",
  conf: "nginx",
  matlab: "matlab",
};

/** The module for an info string, or `null` to leave the block plain. */
export function languageOf(info: string): string | null {
  return LANGS[info.trim().toLowerCase()] ?? null;
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

/**
 * The block's HTML, or `null` if it could not be painted.
 *
 * **The output is markup, and the safety of that is not taken on
 * trust.** highlight.js escapes every character of the input before it
 * wraps any of it in a span, so what comes back is the agent's text as
 * text plus spans this build generated — never the agent's own markup,
 * which `Markdown` refuses everywhere else and still refuses. The gate
 * feeds a block containing a script tag and an event handler and asserts
 * the rendered block contains no such element and that its text comes
 * back character for character (`scripts/check-read.mjs`).
 *
 * `ignoreIllegals` because a snippet in a conversation is a fragment far
 * more often than it is a whole file, and a fragment that trips a
 * language's illegal rule should come back plain-but-painted rather than
 * throwing.
 */
export function paint(code: string, language: string): string | null {
  const key = `${language} ${code}`;
  const seen = PAINTED.get(key);
  if (seen !== undefined) return seen;
  try {
    const html = hljs.highlight(code, { language, ignoreIllegals: true }).value;
    if (PAINTED.size >= PAINTED_CAP) PAINTED.delete(PAINTED.keys().next().value as string);
    PAINTED.set(key, html);
    return html;
  } catch {
    // Only reachable if a name in LANGS is not registered above — a
    // mistake in this file, not in anything an agent wrote. Plain is the
    // honest outcome either way.
    return null;
  }
}

/**
 * Where an unterminated fence begins, or `null` if every fence in `text`
 * is closed.
 *
 * **This is what keeps the highlighter out of the streaming path.** In
 * CommonMark a fence that was never closed runs to the end of the
 * document — so the block that is still arriving, chunk by chunk, is
 * exactly the block whose language is still half-typed and whose last
 * line is half a token. Painting it would mean re-highlighting a growing
 * string on every chunk, and painting a guess each time.
 *
 * Line-based rather than a regular expression over the whole text
 * because a fence is a line construct: a run of three or more backticks
 * or tildes, indented at most three, and a closing fence must be at
 * least as long as its opener, of the same character, with nothing after
 * it. An opening backtick fence may not carry a backtick in its info
 * string — that is what stops a lone `` `x` `` in prose from being read
 * as the start of a block.
 */
/**
 * The same painting, for a **whole file** instead of a fenced block.
 *
 * **Separate because of the cache, not the colours.** `paint` keys on
 * the code itself and keeps 64 of them, which is right for a
 * conversation: the same short block is re-rendered on every update and
 * the keys stay small. A file is up to one slice — 256 KB — so one
 * would evict the whole table and the key alone would be as large as
 * the thing it caches. A file is painted once when it opens; there is
 * nothing to cache.
 *
 * Everything else, including why the markup is safe to insert, is the
 * same and is argued on `paint`.
 */
export function paintFile(code: string, language: string): string | null {
  try {
    return hljs.highlight(code, { language, ignoreIllegals: true }).value;
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
