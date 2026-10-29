// **One file of a commit, on `@codemirror/merge`** (cm-d item 1): the
// commit page's diff for one file, read-only — the parent's text laid in
// above the commit's where they differ (`unifiedMergeView`), syntax ink on
// both sides, the unchanged stretches folded behind 「展开未更改的 N 行」
// with three lines kept around each change, git's own shape.
//
// **The page scrolls, not the file.** The view is as tall as its content
// (`height: auto`), so a commit of fifty files is one scroll on a phone;
// only a long line scrolls sideways, inside its own file, as the drawn
// hunks did. CodeMirror still draws only what the page shows of it.
//
// **Both sides' numbers, as the hunks had them.** `lineNumbers()` counts
// the commit's lines; a second column counts the parent's: an unchanged
// line has both, a line the commit wrote has only the new, and the
// parent's lines laid in above (the deletion's block) have only the old —
// one number per drawn row, the lines not wrapping.
import { getChunks, getOriginalDoc, unifiedMergeView } from "@codemirror/merge";
import { Compartment, EditorState, type Extension } from "@codemirror/state";
import { EditorView, GutterMarker, gutter, lineNumbers } from "@codemirror/view";
import { useEffect, useLayoutEffect, useRef } from "react";

import { diffPhrases, diffTheme } from "@/lib/code/diff";
import { loadLanguage } from "@/lib/code/lang";
import { codeHighlight, codeTheme } from "@/lib/code/theme";

class OldNumber extends GutterMarker {
  constructor(readonly from: number, readonly to: number) {
    super();
  }
  eq(o: OldNumber) {
    return o.from === this.from && o.to === this.to;
  }
  toDOM() {
    const box = document.createElement("div");
    for (let n = this.from; n <= this.to; n += 1) box.appendChild(document.createElement("div")).textContent = String(n);
    return box;
  }
}

/** The parent's line numbers: beside an unchanged line, the number it had
    there; beside the parent's lines laid in above a change, theirs. */
const oldNumbers = gutter({
  class: "cm-lineNumbers cm-oldNumbers",
  lineMarker(view, line) {
    let at = line.from;
    let was = 0;
    for (const c of getChunks(view.state)?.chunks ?? []) {
      if (c.fromB > at) break;
      if (at < c.toB) return null;
      was = c.toA - c.toB;
    }
    at += was;
    const old = getOriginalDoc(view.state);
    if (at > old.length) return null;
    const n = old.lineAt(at).number;
    return new OldNumber(n, n);
  },
  widgetMarker(view, _widget, block) {
    if (block.from !== block.to) return null;
    const c = getChunks(view.state)?.chunks.find((c) => c.fromB === block.from && c.fromA < c.toA);
    if (!c) return null;
    const old = getOriginalDoc(view.state);
    return new OldNumber(old.lineAt(c.fromA).number, old.lineAt(Math.min(c.toA - 1, old.length)).number);
  },
  initialSpacer: (view) => {
    const n = getOriginalDoc(view.state).lines;
    return new OldNumber(n, n);
  },
});

/** On the page, not in a pane: as tall as its lines, the page scrolling
    it; the two number columns parted only from the text. */
const onPage = EditorView.theme({
  "&": { height: "auto" },
  ".cm-scroller": { overflowY: "hidden" },
  ".cm-gutter.cm-oldNumbers": { borderRight: "none" },
  ".cm-oldNumbers .cm-gutterElement > div": { height: "var(--text-mono--line-height)" },
});

const reading = (lang: Compartment, before: string, full: boolean): Extension => [
  oldNumbers,
  lineNumbers(),
  EditorState.readOnly.of(true),
  EditorView.editable.of(false),
  unifiedMergeView({ original: before, mergeControls: false, gutter: true, collapseUnchanged: full ? undefined : { margin: 3 } }),
  codeHighlight,
  codeTheme,
  diffTheme,
  diffPhrases(),
  onPage,
  lang.of([]),
];

export function CommitDiff({ path, before, after, full = false }: { path: string; before: string; after: string; full?: boolean }) {
  const box = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const lang = useRef(new Compartment());

  useLayoutEffect(() => {
    const v = new EditorView({ parent: box.current!, state: EditorState.create({ doc: after, extensions: reading(lang.current, before, full) }) });
    view.current = v;
    return () => {
      v.destroy();
      view.current = null;
    };
  }, [before, after, full]);

  useEffect(() => {
    let live = true;
    void loadLanguage(path).then((l) => {
      if (live && l) view.current?.dispatch({ effects: lang.current.reconfigure(l) });
    });
    return () => {
      live = false;
    };
  }, [path, before, after]);

  // `data-code-view` is what app.css paints the syntax ink under.
  return <div ref={box} data-commit-diff data-code-view data-selectable className="font-mono text-mono leading-mono" />;
}
