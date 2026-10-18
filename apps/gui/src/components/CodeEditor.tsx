// One file's text in CodeMirror 6 (CM-BRIEF step 1, the owner 2026-10-10:
// the two-layer editor's caret never lined up with its paint, OWNER-NOTES-
// 10-09 已知问题 1). One `EditorView` per open file; the host owns what is
// saved and passes the text in, the editor hands every change back out.
//
// **What CodeMirror is here for is correct text editing, not an IDE**
// (「不提供任何自动补全什么的」 stands): line numbers, folding, ⌘F, undo,
// IME composition, tab and newline indentation, the caret's line, the
// selected word's other places, room past the last line — and nothing
// that guesses what you meant to type: no completion, no auto-closed
// brackets or tags, no bracket-matching highlight, no second caret.
//
// **The caret is CodeMirror's** (`drawSelection`, the owner 2026-10-13:
// on the Mac app and the iPhone the caret stood the line's 20 over 13 of
// letters, CJK the worst). The engine's own caret is whatever height the
// platform paints, and no stylesheet sets it — headless WebKit and
// Chrome happen to paint it at the glyph's 15, which is why no gate saw
// the app's. CodeMirror's is laid at the glyph's own box (`coordsAtPos`)
// on every engine, and drawn the row's height from it, as VS Code's is
// (`lib/code/theme`, 2026-10-16). The IME's marked text and the loupe
// stay the platform's. On iOS the main selection's ground stays native
// too (the platform's cannot be hidden, so CodeMirror leaves it) and
// CodeMirror draws the two handles; elsewhere the selection is drawn as
// well, in the theme's grey (`lib/code/theme`).
import { history, defaultKeymap, historyKeymap, indentWithTab } from "@codemirror/commands";
import { getChunks, unifiedMergeView } from "@codemirror/merge";
import { codeFolding, foldGutter, foldKeymap, indentUnit } from "@codemirror/language";
import { Annotation, Compartment, EditorSelection, EditorState, StateEffect, StateField, Transaction, type Extension } from "@codemirror/state";
import { highlightSelectionMatches } from "@codemirror/search";
import {
  Decoration,
  drawSelection,
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  scrollPastEnd,
  type DecorationSet,
} from "@codemirror/view";
import { useEffect, useLayoutEffect, useRef } from "react";

import { IconChevronDown, IconChevronRight } from "@/components/icons";
import { gui } from "@/gen/catalog";
import type { GitCommit } from "@/gen/bindings/GitCommit";
import type { GitHunk } from "@/gen/bindings/GitHunk";
import { inlineBlame, setBlamed } from "@/lib/code/blame";
import { diffPhrases, diffTheme } from "@/lib/code/diff";
import { codeFind, openFind } from "@/lib/code/find";
import { withFolding } from "@/lib/code/fold";
import { gitChanges, setHunks } from "@/lib/code/git";
import { indentOf } from "@/lib/code/indent";
import { keyboardCaret, observeScroller } from "@/lib/code/keyboard";
import { languageOfName, loadLanguage } from "@/lib/code/lang";
import { hasSymbols, symbolsOfView } from "@/lib/code/symbols";
import { codeHighlight, codeTheme } from "@/lib/code/theme";
import { BLINK_MS } from "@/lib/tempo";
import { cn } from "@/lib/utils";

/** A change that came from the host (a merge, a newer read), which is
    not handed back out: the host already has it, and handing it back as
    if typed would read as a keystroke (it clears a merge's notice). */
const fromHost = Annotation.define<boolean>();

/** The line a link or the terminal's path asked for (`goLine`), on the
    brand's wash until the reader moves the caret or types. */
const setAnchor = StateEffect.define<number | null>();
const anchored = StateField.define<DecorationSet>({
  create: () => Decoration.none,
  update(v, tr) {
    let next = tr.docChanged ? v.map(tr.changes) : v;
    if (tr.isUserEvent("select.pointer") || tr.isUserEvent("input") || tr.isUserEvent("delete")) next = Decoration.none;
    for (const e of tr.effects) {
      if (!e.is(setAnchor)) continue;
      next =
        e.value === null || e.value > tr.state.doc.lines
          ? Decoration.none
          : Decoration.set([Decoration.line({ class: "cm-anchored", attributes: { "data-anchored": String(e.value) } }).range(tr.state.doc.line(e.value).from)]);
    }
    return next;
  },
  provide: (f) => EditorView.decorations.from(f),
});

/** The fold marker: the app's own chevrons (`icons.tsx`), cloned from a
    pair the component renders hidden — one glyph set, not a second one
    drawn here (README 图标). */
let glyphs: { open: SVGElement; folded: SVGElement } | null = null;
function foldMarker(open: boolean): HTMLElement {
  const s = document.createElement("span");
  s.dataset.fold = open ? "open" : "folded";
  s.setAttribute("aria-label", open ? gui.code_fold : gui.code_unfold);
  const g = open ? glyphs?.open : glyphs?.folded;
  if (g) s.append(g.cloneNode(true));
  return s;
}

/** To `line`: the caret at its start, the line on the brand's wash, a
    third of the way down, as the painted sheet put it (KB-CARET rule 6).
    A link, the terminal's path and a press in 「符号」 all come here. */
function go(v: EditorView, line: number) {
  if (line < 1 || line > v.state.doc.lines) return;
  const at = v.state.doc.line(line).from;
  v.dispatch({
    selection: EditorSelection.cursor(at),
    effects: [setAnchor.of(line), EditorView.scrollIntoView(at, { y: "start", yMargin: v.scrollDOM.clientHeight / 3 })],
  });
}

/** Everything every file gets, readable or not. */
function base(): Extension {
  return [
    // Gutters in this order: numbers, git's bar beside them (VS Code's
    // place for it), the fold chevrons next to the text.
    lineNumbers(),
    gitChanges(),
    codeFolding({ placeholderText: "…" }),
    foldGutter({ markerDOM: foldMarker }),
    history(),
    // The caret and the selection, drawn (the file's head comment); the
    // caret blinks as VS Code's does (`lib/tempo`).
    drawSelection({ cursorBlinkRate: BLINK_MS }),
    // The line the caret is on, reading or writing (cm-d item 3).
    highlightActiveLine(),
    highlightActiveLineGutter(),
    // A selected word's other places in what is shown: whole words of
    // two characters or more, at most `maxMatches` (past it, none).
    highlightSelectionMatches({ minSelectionLength: 2, wholeWords: true, maxMatches: 100 }),
    // The last line can be scrolled to the top: with the keyboard up,
    // the foot of a file is somewhere to type, not under the keys
    // (KB-CARET rule 1).
    scrollPastEnd(),
    EditorState.allowMultipleSelections.of(false),
    codeHighlight,
    codeTheme,
    codeFind(),
    inlineBlame(),
    anchored,
    keymap.of([...defaultKeymap, ...historyKeymap, ...foldKeymap, indentWithTab]),
  ];
}

/** **The indent the file already uses** (cm-d item 2, `lib/code/indent`):
    what Tab and a new line insert, and how wide a tab is drawn. Read
    when the text arrives — at mount, and when the host hands in a new
    one — and never from what is typed. */
const indenting = (text: string, path: string): Extension => {
  const { unit, tabSize } = indentOf(text, languageOfName(path)?.name ?? null);
  return [indentUnit.of(unit), EditorState.tabSize.of(tabSize)];
};

const writable = (on: boolean): Extension => [
  EditorState.readOnly.of(!on),
  EditorView.editable.of(on),
  // `data-edit` marks the surface a person types into, the name the
  // textarea wore (`FilePage`'s hold, the gates).
  EditorView.contentAttributes.of(
    on ? { "data-edit": "", spellcheck: "false", autocorrect: "off", autocapitalize: "off" } : { spellcheck: "false" },
  ),
];

/** One of a chunk's two buttons, khor's and not the package's green and
    red: a neutral push button, the words from the catalog. */
function chunkButton(type: "accept" | "reject", act: (e: MouseEvent) => void): HTMLElement {
  const b = document.createElement("button");
  b.type = "button";
  b.className = "cm-merge-btn";
  // Not `data-merge`: that is the notice's 合并, which the gates press.
  b.dataset.mergeChunk = type;
  b.textContent = type === "accept" ? gui.merge_keep : gui.merge_drop;
  b.addEventListener("mousedown", act);
  return b;
}

/** **A merge waiting on the person** (CM-BRIEF step 2): the text the
    disk holds now is the original, the box is the merge, and every chunk
    where they differ carries 保留 (keep the box's) and 放弃 (take the
    disk's) — one decision per place, where the markers alone asked the
    person to edit them out by hand. */
const merging = (against: string | null): Extension =>
  against === null
    ? []
    : [
        unifiedMergeView({ original: against, mergeControls: chunkButton, gutter: true, highlightChanges: true, syntaxHighlightDeletions: true }),
        diffTheme,
        diffPhrases(),
      ];

/** **The host's four acts on the editor are not exported here** (CM-BRIEF
    §2): `editorIn`, `topLineOf`, `placeLine` and `focusAtTop` live in
    `lib/code/place`, and a host importing this module is a host that
    wants the editor itself. Keeping them here re-exported would let one
    stray import of a helper pull this whole module — and CodeMirror's
    runtime with it — back into the entry chunk. */
export function CodeEditor({
  path,
  text,
  editable,
  wrap = true,
  hunks,
  blamed = null,
  goLine = null,
  against = null,
  onText,
  onSave,
  onChunks,
  className,
}: {
  /** Names the language (`lib/code/lang`). */
  path: string;
  /** What the editor holds. Typing hands it out through `onText` and the
      host hands it back; a text that differs from what was handed out —
      a merge, a read of a newer file — replaces the document. */
  text: string;
  /** **A file that cannot be written back takes no caret**, rather than a
      caret that refuses every key: the whole of it is not here, or a
      phone is reading it first. */
  editable: boolean;
  /** Long lines fold into the width (「文件显示,要随屏幕宽度自动换行」, the
      user 2026-09-15). */
  wrap?: boolean;
  /** The change gutter's hunks, against this text; empty for none. */
  hunks?: GitHunk[];
  /** Who wrote each line (`lib/blame`), or null to say nothing. */
  blamed?: (GitCommit | null)[] | null;
  /** A line asked for from outside, stamped so the same line asked twice
      moves the view twice. */
  goLine?: { line: number; at: number } | null;
  /** The disk's text while a merge waits on the person, else null. */
  against?: string | null;
  onText?: (text: string) => void;
  /** ⌘S. */
  onSave?: () => void;
  /** How many places of the merge are still undecided, as they change. */
  onChunks?: (left: number) => void;
  className?: string;
}) {
  const box = useRef<HTMLDivElement>(null);
  const icons = useRef<HTMLSpanElement>(null);
  const view = useRef<EditorView | null>(null);
  /** The last text handed out, so a host echoing it back is not a
      replacement. */
  const out = useRef(text);
  const cb = useRef({ onText, onSave, onChunks });
  cb.current = { onText, onSave, onChunks };
  const lang = useRef(new Compartment());
  const edit = useRef(new Compartment());
  const wrapping = useRef(new Compartment());
  const merge = useRef(new Compartment());
  const indent = useRef(new Compartment());

  useLayoutEffect(() => {
    const svgs = icons.current?.querySelectorAll("svg");
    if (svgs && svgs.length === 2) glyphs = { open: svgs[0], folded: svgs[1] };
    const v = new EditorView({
      parent: box.current!,
      state: EditorState.create({
        doc: text,
        extensions: [
          base(),
          lang.current.of([]),
          indent.current.of(indenting(text, path)),
          edit.current.of(writable(editable)),
          wrapping.current.of(wrap ? EditorView.lineWrapping : []),
          merge.current.of(merging(against)),
          EditorView.contentAttributes.of({ "data-file-text": "", "data-sheet-scroll": "" }),
          keyboardCaret(),
          // The one shortcut of the textarea, `metaKey` and `ctrlKey`
          // both: a Mac app, a phone's browser and Windows, one build
          // (`Mod` alone is ⌘ on a Mac, and ⌃S there saved nothing).
          keymap.of(
            ["Mod-s", "Ctrl-s"].map((key) => ({
              key,
              preventDefault: true,
              run: () => {
                cb.current.onSave?.();
                return true;
              },
            })),
          ),
          EditorView.updateListener.of((u) => {
            const left = getChunks(u.state)?.chunks.length;
            if (left !== undefined && left !== getChunks(u.startState)?.chunks.length) cb.current.onChunks?.(left);
            if (!u.docChanged) return;
            const t = u.state.doc.toString();
            out.current = t;
            if (u.transactions.every((tr) => !tr.docChanged || tr.annotation(fromHost))) return;
            cb.current.onText?.(t);
          }),
        ],
      }),
    });
    // The scroller the rest of the app knows by name (the deck's fold on
    // a phone, the thinned thumb under the ruler in app.css).
    v.scrollDOM.dataset.fileScroll = "";
    v.scrollDOM.dataset.selectable = "";
    // ⌘F from the menu (`lib/shortcuts`' find-list), the terminal's
    // `__find` spelled for a file.
    // The view itself as `__view`, for whoever holds the box and not the
    // module (`FilePage` once the editor is a lazy chunk, the gates).
    // 「符号」 (`lib/code/place` `symbolsIn` / `goLineIn`): the file's
    // definitions, and the line a press goes to, through the box — the
    // walk is this chunk's, not the host's.
    const host = box.current as (HTMLElement & { __find?: () => boolean; __view?: EditorView; __symbols?: () => ReturnType<typeof symbolsOfView>; __hasSymbols?: () => boolean; __goLine?: (line: number) => void }) | null;
    if (host) {
      host.__find = () => openFind(v);
      host.__view = v;
      host.__symbols = () => symbolsOfView(v);
      host.__hasSymbols = () => hasSymbols(v.state);
      host.__goLine = (line) => go(v, line);
    }
    view.current = v;
    // Rule 2 and 4 (KB-CARET): watch the scroller for resize — reveal the
    // caret when it shrinks (keyboard rising), pin scrollTop when it grows
    // (keyboard going down).
    const teardownObserver = observeScroller(v.scrollDOM, () => view.current);
    return () => {
      teardownObserver();
      v.destroy();
      view.current = null;
    };
    // The view is made once per mount; the props below reach it through
    // their own effects.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    let live = true;
    const v = view.current;
    if (v) v.dispatch({ effects: indent.current.reconfigure(indenting(v.state.doc.toString(), path)) });
    // The language, and folding by indent where its tree folds nothing
    // (`lib/code/fold`) — a file with no language too.
    void loadLanguage(path).then((l) => {
      if (live) view.current?.dispatch({ effects: lang.current.reconfigure(withFolding(l)) });
    });
    return () => {
      live = false;
    };
  }, [path]);

  // Layout, not passive: 「编辑」 on a phone renders the editor writable
  // and focuses it in one press (`flushSync`, `use-read-first`).
  useLayoutEffect(() => {
    view.current?.dispatch({ effects: edit.current.reconfigure(writable(editable)) });
  }, [editable]);

  useLayoutEffect(() => {
    view.current?.dispatch({ effects: wrapping.current.reconfigure(wrap ? EditorView.lineWrapping : []) });
  }, [wrap]);

  useLayoutEffect(() => {
    const v = view.current;
    if (!v) return;
    v.dispatch({ effects: merge.current.reconfigure(merging(against)) });
    // Said once at the start too: a merge that left nothing to decide is
    // answered before anybody presses anything.
    const left = getChunks(v.state)?.chunks.length;
    if (left !== undefined) cb.current.onChunks?.(left);
  }, [against]);

  // A text from outside. Only the part that differs is replaced, so the
  // caret, the folds and the scroll outside it stay where they were.
  useLayoutEffect(() => {
    const v = view.current;
    if (!v || text === out.current) return;
    const now = v.state.doc.toString();
    out.current = text;
    if (now === text) return;
    let a = 0;
    while (a < now.length && a < text.length && now.charCodeAt(a) === text.charCodeAt(a)) a += 1;
    let b = 0;
    while (b < now.length - a && b < text.length - a && now.charCodeAt(now.length - 1 - b) === text.charCodeAt(text.length - 1 - b)) b += 1;
    v.dispatch({
      changes: { from: a, to: now.length - b, insert: text.slice(a, text.length - b) },
      effects: indent.current.reconfigure(indenting(text, path)),
      annotations: [fromHost.of(true), Transaction.addToHistory.of(false)],
    });
    // The path is read where it is now, not as a dependency: a new path
    // is the effect above's.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [text]);

  useEffect(() => {
    view.current?.dispatch({ effects: setHunks.of(hunks ?? []) });
  }, [hunks]);

  useEffect(() => {
    view.current?.dispatch({ effects: setBlamed.of(blamed) });
  }, [blamed]);

  const wentTo = useRef(0);
  useEffect(() => {
    const v = view.current;
    if (!v || goLine === null || goLine.at === wentTo.current) return;
    if (goLine.line > v.state.doc.lines) return;
    wentTo.current = goLine.at;
    go(v, goLine.line);
  }, [goLine, text]);

  return (
    <div
      ref={box}
      data-code-view
      data-code-path={path}
      className={cn("relative min-h-0 flex-1 font-mono text-mono leading-mono", className)}
    >
      <span ref={icons} hidden>
        <IconChevronDown />
        <IconChevronRight />
      </span>
    </div>
  );
}
