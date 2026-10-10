// Which language a file is painted in, loaded only when a file of it is
// opened (CM-BRIEF step 1). `@codemirror/language-data` is a table of
// descriptions — names, extensions, file names — and a dynamic import per
// language, so the base bundle carries the table and no grammar.
//
// **The language, never its support** (「不提供任何自动补全什么的」): a
// `LanguageSupport` is the language plus whatever its package thinks an
// editor wants — `html()` closes tags as you type, the JavaScript and CSS
// packages hand over completion sources. All of that is in `support`, and
// none of it is taken: what a file gets from here is the parser, which is
// what paints it and what folding reads.
import type { Language, LanguageDescription } from "@codemirror/language";
import { LanguageDescription as Desc } from "@codemirror/language";
import { languages } from "@codemirror/language-data";

/** The description that names this file, by its name: the extension, or a
    whole name a format owns (`Dockerfile`, `Makefile`). Never a guess
    from the contents. */
export function languageOfName(path: string): LanguageDescription | null {
  const name = path.split("/").pop() ?? "";
  return Desc.matchFilename(languages, name);
}

/** The parser for this file, or null for one with no language here. */
export async function loadLanguage(path: string): Promise<Language | null> {
  const d = languageOfName(path);
  if (d === null) return null;
  const s = await d.load();
  return s.language;
}
