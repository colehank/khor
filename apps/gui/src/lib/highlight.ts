// Syntax colouring for the conversation's code blocks, at the one
// intensity this app has room for.
//
// **Three roles, not six.** Comments take the aux ink they already
// mean; string literals take one new colour; everything else — keywords,
// types, function names, numbers, operators — stays body ink. A
// six-role theme would have to either reuse the state words' colours or
// invent four or five more, and this app's palette is fully spoken for
// (the six words, `--strain`, the brand green). The mapping itself is in
// `styles/app.css`, because these are class names a stylesheet has to
// name; this file only produces them.
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
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import swift from "highlight.js/lib/languages/swift";
import typescript from "highlight.js/lib/languages/typescript";
import yaml from "highlight.js/lib/languages/yaml";

hljs.registerLanguage("bash", bash);
hljs.registerLanguage("css", css);
hljs.registerLanguage("ini", ini);
hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("json", json);
hljs.registerLanguage("python", python);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("swift", swift);
hljs.registerLanguage("typescript", typescript);
hljs.registerLanguage("yaml", yaml);

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
