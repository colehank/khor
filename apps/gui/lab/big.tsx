// Lab only (lane cm-d): one generated file of `?lines=` lines (a function
// every ten) in the editor, for the brief's 50 000-line timings.
// `window.__big()` answers { openMs, foldAllMs, folded, foldAllParsedMs,
// foldedParsed, selectMs[], marks, coldHas/coldHasMs (「…」 asking),
// coldSymbols/coldSymbolsMs, symbols/symbolsMs (「符号」 cold and parsed) }
// — the driver is `scripts/cm-measure.mjs big`.
import { foldAll, unfoldAll, forceParsing, foldedRanges } from "@codemirror/language";
import { EditorSelection } from "@codemirror/state";
import type { EditorView } from "@codemirror/view";
import ReactDOM from "react-dom/client";

import "@/styles/tokens.css";
import "@/styles/app.css";

import { CodeEditor } from "@/components/CodeEditor";

const n = Number(new URLSearchParams(location.search).get("lines") ?? 50_000);
const rows: string[] = [];
for (let i = 0; rows.length < n; i += 1) {
  rows.push(
    `export function fn${i}(a: number, b: string): number {`,
    `  const abc = a + ${i};`,
    `  if (b.length > abc) {`,
    `    return abc * 2;`,
    `  }`,
    `  // abc is the sum, said once`,
    `  const out = [abc, a, ${i}];`,
    `  return out.reduce((s, x) => s + x, 0);`,
    `}`,
    ``,
  );
}
const text = rows.slice(0, n).join("\n");

const view = (): EditorView => (document.querySelector("[data-code-view]") as HTMLElement & { __view: EditorView }).__view;
const frame = () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
const t0 = performance.now();

(window as unknown as { __big: () => Promise<unknown> }).__big = async () => {
  // Open: until the first screen wears the language's ink.
  while (!document.querySelector('[data-code-view] .cm-line [class*="tok-"]')) await new Promise((r) => setTimeout(r, 5));
  const openMs = performance.now() - t0;
  const v = view();
  await frame();
  // 「符号」 before anything has parsed past the first screen: the walk
  // parses up to its budget (`lib/code/symbols` PARSE_MS) and lists what
  // is parsed.
  const host0 = document.querySelector("[data-code-view]") as HTMLElement & { __symbols?: () => unknown[]; __hasSymbols?: () => boolean };
  // What 「…」 asks as it opens: one definition in what is parsed.
  let s = performance.now();
  const coldHas = host0.__hasSymbols?.() ?? null;
  const coldHasMs = +(performance.now() - s).toFixed(2);
  s = performance.now();
  const coldSymbols = host0.__symbols?.().length ?? null;
  const coldSymbolsMs = Math.round(performance.now() - s);
  await frame();
  s = performance.now();
  foldAll(v);
  await frame();
  const foldAllMs = performance.now() - s;
  const folded = foldedRanges(v.state).size;
  unfoldAll(v);
  await frame();
  s = performance.now();
  forceParsing(v, v.state.doc.length, 20_000);
  foldAll(v);
  await frame();
  const foldAllParsedMs = performance.now() - s;
  const foldedParsed = foldedRanges(v.state).size;
  unfoldAll(v);
  await frame();
  // `abc` on line 2 of each of the first functions, selected in turn.
  const selectMs: number[] = [];
  for (let k = 0; k < 10; k += 1) {
    const line = v.state.doc.line(2 + 10 * k);
    const at = line.from + line.text.indexOf("abc");
    // The dispatch is synchronous — the view's update, every plugin's
    // decorations and the DOM it draws — so this is what a selection
    // change costs before the next frame.
    s = performance.now();
    v.dispatch({ selection: EditorSelection.single(at, at + 3), userEvent: "select" });
    selectMs.push(performance.now() - s);
    await frame();
  }
  const marks = document.querySelectorAll(".cm-selectionMatch").length;
  // 「符号」 on the whole file, as the dialog asks for it.
  const host = document.querySelector("[data-code-view]") as HTMLElement & { __symbols?: () => unknown[] };
  s = performance.now();
  const symbols = host.__symbols?.().length ?? null;
  const symbolsMs = Math.round(performance.now() - s);
  return { openMs: Math.round(openMs), foldAllMs: Math.round(foldAllMs), folded, foldAllParsedMs: Math.round(foldAllParsedMs), foldedParsed, selectMs: selectMs.map((x) => +x.toFixed(2)), marks, coldHas, coldHasMs, coldSymbols, coldSymbolsMs, symbols, symbolsMs };
};

ReactDOM.createRoot(document.getElementById("root")!).render(
  <div className="flex h-dvh flex-col bg-card text-foreground">
    <CodeEditor path="big.ts" text={text} editable />
  </div>,
);
