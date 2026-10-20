// 语言 (docs/MORE.md 语言): 跟随系统 / 中文 / English.
//
// # Two, and the fallback is English
//
// 「自动跟随系统语言，我们支持英语与中文。失败会退英文」 (the user,
// 2026-09-19). A browser that says `zh*` gets Chinese and **everything
// else gets English**, including a browser that says nothing. There is
// no third answer and no partial one: `build.rs` refuses to compile a
// key that exists in only one of the two wording files, so every
// sentence khor can say exists in both.
//
// # The store keeps the choice, the face resolves it
//
// `khor config`'s `lang` is `auto`, `zh` or `en` — never the language it
// resolves to. A store holding 「en」 because somebody spent a week on an
// English machine is a laptop that stops speaking Chinese when it goes
// home. `auto` can only be resolved here, because `navigator.language`
// is the only place the question can be asked.
//
// # It has to be settled before the first render, not after
//
// Every word on screen is a property read off the generated catalog
// (`speak` swaps which table answers), and a read is not reactive. So
// the language is set **during** App's render rather than in an effect:
// an effect runs after the render that would have used the new word,
// and nothing would render a second time. `main.tsx` does the same
// before `createRoot`, from the mirror, so the first paint is already
// right.
import { speak, speaking } from "@/gen/catalog";

/** Mirrors the choice for the next launch, the same shape 外观 keeps
    (`src/theme.ts`). A cache, never the preference: `khor config` is
    where it lives, and a browser that refuses storage simply follows
    itself — which is what khor did before the key existed. */
export const LANG_MIRROR = "khor.lang";

/** What a face can be told to speak. `auto` is a choice, not a language. */
export type LangPref = "auto" | "zh" | "en";

/** The last choice khor handed over, so a re-render with nothing new to
    say resolves the same way. */
let pref: LangPref = "auto";

/** **`zh` and anything under it**, and nothing else — the ruling is
    「非 zh* 一律英文」, so this is a prefix test rather than a list of
    regions khor would have to keep up with. The same rule the Rust side
    applies to a locale (`khor_catalog::lang::is_zh`). */
function browserIsZh(): boolean {
  try {
    const tag = (navigator.language || "").trim().toLowerCase();
    return tag === "zh" || tag.startsWith("zh-") || tag.startsWith("zh_");
  } catch {
    // No `navigator` at all is a face that cannot ask, and English is
    // what khor answers when it cannot ask.
    return false;
  }
}

/** The choice as it was last mirrored. Anything that is not a choice
    khor takes reads as `auto`, including a mirror written by a newer
    build. */
export function mirroredLang(): LangPref {
  try {
    const was = window.localStorage.getItem(LANG_MIRROR);
    if (was === "zh" || was === "en" || was === "auto") return was;
  } catch {
    // Private window, blocked site data.
  }
  return "auto";
}

/**
 * Speaks a choice, and answers which language that turned out to be.
 *
 * Handed `null` — khor has not answered, or has no such key — it keeps
 * the choice it already had rather than inventing one: a face that
 * snapped back to `auto` because a read was late would change language
 * on its own, mid-sentence.
 *
 * Also stamps `<html lang>`, which is not decoration: it is what picks
 * the font a browser falls back to for CJK, how a screen reader
 * pronounces the page, and what `:lang()` would key off.
 */
export function speakLang(next: LangPref | null): "zh" | "en" {
  if (next !== null) pref = next;
  const lang = pref === "auto" ? (browserIsZh() ? "zh" : "en") : pref;
  speak(lang);
  // **Only when it changes** (2026-10-21): `App` calls this on every
  // render, and Chromium restyles the whole document for an attribute
  // set on the root even to the value it already had — a formula-heavy
  // preview spent ~20 ms per render on it, 21 s of a 30 s scroll.
  try {
    if (document.documentElement.lang !== lang) document.documentElement.lang = lang;
  } catch {
    // Nothing to stamp; the words are already right.
  }
  try {
    if (window.localStorage.getItem(LANG_MIRROR) !== pref) window.localStorage.setItem(LANG_MIRROR, pref);
  } catch {
    // Only the head start on the next launch is lost.
  }
  return lang;
}

/** Which language the last render spoke. Re-exported so callers do not
    have to know that the switch itself lives in generated code. */
export { speaking };
