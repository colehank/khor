// **Folding for languages without a tree** (cm-d item 4). `codeFolding`
// folds what a syntax tree marks foldable (`foldNodeProp`), and only the
// 33 lezer grammars of `@codemirror/language-data` mark anything: the
// other 110 are legacy stream modes — TOML, INI, shell, C#, Ruby, Lua …
// — which paint but have no tree, so the fold gutter stayed empty there.
// They fold by indentation instead, as VS Code folds any language with no
// folding of its own: a line's fold runs to the last following line
// indented deeper.
//
// **Only where the tree has nothing to say**: a `foldService` answers
// before the tree is asked, so registered beside a lezer grammar it would
// fold a `.ts` by its indent rather than by its blocks. It is added with
// the language, and only for a language none of whose node types folds.
import { foldNodeProp, foldService, type Language } from "@codemirror/language";
import { countColumn, type EditorState, type Extension } from "@codemirror/state";

/** Whether `l`'s tree marks anything foldable: a lezer grammar's node
    types carry `foldNodeProp`; a stream mode's never do. */
export function foldsByTree(l: Language): boolean {
  const set = (l.parser as { nodeSet?: { types: readonly { prop: (p: typeof foldNodeProp) => unknown }[] } }).nodeSet;
  return set?.types.some((t) => t.prop(foldNodeProp) !== undefined) ?? false;
}

/** How far in a line's text starts, a tab as wide as the file's tabs; null
    for a blank line, which says nothing about depth. */
function depth(state: EditorState, text: string): number | null {
  const i = text.search(/\S/);
  return i < 0 ? null : countColumn(text.slice(0, i), state.tabSize);
}

/** A line's fold: from its end to the end of the last following line that
    stands deeper than it, blank lines inside carried along. */
const byIndent = foldService.of((state, from, to) => {
  const line = state.doc.lineAt(from);
  const own = depth(state, line.text);
  if (own === null) return null;
  let last = 0;
  for (let n = line.number + 1; n <= state.doc.lines; n += 1) {
    const d = depth(state, state.doc.line(n).text);
    if (d === null) continue;
    if (d <= own) break;
    last = n;
  }
  return last === 0 ? null : { from: to, to: state.doc.line(last).to };
});

/** What the editor's language compartment holds for `l` — the language,
    and the indent's folding when its tree folds nothing. A file with no
    language (plain text) folds by indent too. */
export function withFolding(l: Language | null): Extension {
  if (l === null) return byIndent;
  return foldsByTree(l) ? l : [l, byIndent];
}
