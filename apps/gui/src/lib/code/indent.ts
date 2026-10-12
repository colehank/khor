// **The indent unit follows the file** (cm-d item 2): Tab in a `.rs`
// inserted two spaces because `indentUnit` was `"  "` for every file. It is
// read from the file's own lines when the text arrives — VS Code's
// `detectIndentation`, not a guess at what you will type: tabs, or the
// step its space-indented lines most often move by (2, 4 or 8), ties and
// files with nothing indented falling to the language's own convention.
//
// No imports: `scripts/check-indent.mjs` runs this as it is.

export type Indent = { unit: string; tabSize: number };

/** What a language's own style guide indents by, for `LanguageDescription`
    names (`@codemirror/language-data`) — gofmt's tab, rustfmt's and
    PEP 8's four, Prettier's two. Anything not named takes two, what every
    file took before this. */
const CONVENTION: Record<string, number | "tab"> = {
  go: "tab",
  makefile: "tab",
  rust: 4,
  python: 4,
  java: 4,
  kotlin: 4,
  scala: 4,
  swift: 4,
  c: 4,
  "c++": 4,
  "c#": 4,
  "objective-c": 4,
  php: 4,
  perl: 4,
  powershell: 4,
  sql: 4,
  javascript: 2,
  typescript: 2,
  jsx: 2,
  tsx: 2,
  json: 2,
  css: 2,
  sass: 2,
  scss: 2,
  less: 2,
  yaml: 2,
  html: 2,
  vue: 2,
  markdown: 2,
  ruby: 2,
  lua: 2,
  toml: 2,
};

/** How many lines are read: past this a file has said what it does. */
const READ = 10_000;

const of = (step: number | "tab"): Indent => (step === "tab" ? { unit: "\t", tabSize: 4 } : { unit: " ".repeat(step), tabSize: step });

/** The indent `text` already uses, in a file of language `lang` (its
    `LanguageDescription` name, or null for none). */
export function indentOf(text: string, lang: string | null): Indent {
  const own = CONVENTION[lang?.toLowerCase() ?? ""] ?? 2;
  let tabs = 0;
  let spaced = 0;
  const steps = new Map<number, number>([
    [2, 0],
    [4, 0],
    [8, 0],
  ]);
  let prev = 0;
  let at = 0;
  for (let n = 0; n < READ && at <= text.length; n += 1) {
    const end = text.indexOf("\n", at);
    const stop = end < 0 ? text.length : end;
    let i = at;
    while (i < stop && (text.charCodeAt(i) === 32 || text.charCodeAt(i) === 9)) i += 1;
    // A blank line says nothing, and does not reset the step.
    if (i < stop && text.charCodeAt(i) !== 13) {
      if (text.charCodeAt(at) === 9) tabs += 1;
      else {
        const lead = i - at;
        if (lead > 0) spaced += 1;
        // The step between two lines, not the depth: a two-space file's
        // lines stand at 2, 4, 6 …, and every one of them moved by 2. A
        // step of 1 is a doc comment's ` * `, and says nothing.
        const step = Math.abs(lead - prev);
        if (steps.has(step)) steps.set(step, steps.get(step)! + 1);
        prev = lead;
      }
    }
    if (end < 0) break;
    at = end + 1;
  }
  if (tabs === 0 && spaced === 0) return of(own);
  if (tabs > spaced) return of("tab");
  const most = Math.max(...steps.values());
  if (most === 0) return of(own === "tab" ? 4 : own);
  const tied = [...steps].filter(([, c]) => c === most).map(([s]) => s);
  return of(typeof own === "number" && tied.includes(own) ? own : tied[0]);
}
