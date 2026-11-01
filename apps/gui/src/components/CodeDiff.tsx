// A file against an earlier text of itself, read-only (CM-BRIEF step 2):
// the diff tab (`DiffView`) and the history's 「与当前比较」. Side by side
// where the window is wide — the two sides scroll as one and their
// unchanged lines stay level, which the two self-drawn columns kept by
// counting rows — and one column where it is narrow, the old lines laid
// in above the new ones (two 40-column halves of a phone are two columns
// of nothing, docs/UX.md 窄档). Runs nothing changed in fold behind a
// 「展开」 bar.
import { MergeView, unifiedMergeView } from "@codemirror/merge";
import { Compartment, EditorState, type Extension } from "@codemirror/state";
import { EditorView, lineNumbers } from "@codemirror/view";
import { useEffect, useLayoutEffect, useRef } from "react";

import { COLLAPSE, diffPhrases, diffTheme } from "@/lib/code/diff";
import { loadLanguage } from "@/lib/code/lang";
import { codeHighlight, codeTheme } from "@/lib/code/theme";
import { cn } from "@/lib/utils";
import { band } from "@/lib/scroll-edge";

const reading = (lang: Compartment): Extension => [
  lineNumbers(),
  EditorState.readOnly.of(true),
  EditorView.editable.of(false),
  EditorView.lineWrapping,
  codeHighlight,
  codeTheme,
  diffTheme,
  diffPhrases(),
  lang.of([]),
];

export function CodeDiff({
  path,
  before,
  after,
  beforeLabel,
  afterLabel,
  narrow,
}: {
  /** Names the language. */
  path: string;
  before: string;
  after: string;
  /** What each side is: 「HEAD」 or a commit, and 「工作副本」. */
  beforeLabel: string;
  afterLabel: string;
  narrow: boolean;
}) {
  const box = useRef<HTMLDivElement>(null);
  const views = useRef<EditorView[]>([]);
  const langs = useRef<Compartment[]>([]);

  useLayoutEffect(() => {
    const parent = box.current!;
    if (narrow) {
      const lang = new Compartment();
      const v = new EditorView({
        parent,
        state: EditorState.create({
          doc: after,
          extensions: [
            reading(lang),
            unifiedMergeView({ original: before, mergeControls: false, gutter: true, collapseUnchanged: COLLAPSE }),
          ],
        }),
      });
      v.scrollDOM.dataset.selectable = "";
      views.current = [v];
      langs.current = [lang];
    } else {
      const la = new Compartment();
      const lb = new Compartment();
      const m = new MergeView({
        parent,
        a: { doc: before, extensions: [reading(la)] },
        b: { doc: after, extensions: [reading(lb)] },
        gutter: true,
        highlightChanges: true,
        collapseUnchanged: COLLAPSE,
      });
      m.dom.classList.add("min-h-0", "flex-1");
      m.dom.dataset.selectable = "";
      views.current = [m.a, m.b];
      langs.current = [la, lb];
      return () => m.destroy();
    }
    return () => {
      for (const v of views.current) v.destroy();
    };
  }, [before, after, narrow]);

  useEffect(() => {
    let live = true;
    void loadLanguage(path).then((l) => {
      if (!live || !l) return;
      views.current.forEach((v, i) => v.dispatch({ effects: langs.current[i].reconfigure(l) }));
    });
    return () => {
      live = false;
    };
  }, [path, before, after, narrow]);

  return (
    <div data-diff-tab={path} data-code-view className="flex min-h-0 flex-1 flex-col font-mono text-mono leading-mono">
      {!narrow && (
        <div ref={band} data-band className="flex flex-none pb-(--hair) font-sans text-caption text-muted-foreground">
          <div data-diff-side="head" className="min-w-0 flex-1 px-3 py-1">
            {beforeLabel}
          </div>
          <div data-diff-side="work" className="min-w-0 flex-1 border-l px-3 py-1">
            {afterLabel}
          </div>
        </div>
      )}
      <div ref={box} className={cn("flex min-h-0 flex-1 flex-col", narrow && "[&>.cm-editor]:min-h-0 [&>.cm-editor]:flex-1")} />
    </div>
  );
}
