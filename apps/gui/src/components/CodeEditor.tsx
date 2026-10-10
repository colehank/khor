// One file's text in CodeMirror 6 (CM-BRIEF step 1, the owner 2026-10-10:
// the two-layer editor's caret never lined up with its paint, OWNER-NOTES-
// 10-09 已知问题 1). One `EditorView` per open file; the host owns what is
// saved and passes the text in, the editor hands every change back out.
//
// **What CodeMirror is here for is correct text editing, not an IDE**
// (「不提供任何自动补全什么的」 stands): line numbers, folding, ⌘F, undo,
// IME composition, tab and newline indentation — and nothing that guesses
// what you meant to type: no completion, no auto-closed brackets or tags,
// no bracket-matching highlight, no second caret.
//
// **The caret and the selection are the system's** (no `drawSelection`):
// the IME's marked text, the iOS handles and loupe and the selection
// colour are the platform's own, which is what a person typing Chinese on
// a phone needs most (memory khor-native-walk-gotchas).
import { history, defaultKeymap, historyKeymap, indentWithTab } from "@codemirror/commands";
import { getChunks, unifiedMergeView } from "@codemirror/merge";
import { codeFolding, foldGutter, foldKeymap, indentUnit } from "@codemirror/language";
import { Annotation, Compartment, EditorSelection, EditorState, StateEffect, StateField, Transaction, type Extension } from "@codemirror/state";
import { Decoration, EditorView, keymap, lineNumbers, type DecorationSet } from "@codemirror/view";
import { useEffect, useLayoutEffect, useRef } from "react";

import { IconChevronDown, IconChevronRight } from "@/components/icons";
import { gui } from "@/gen/catalog";
import type { GitCommit } from "@/gen/bindings/GitCommit";
import type { GitHunk } from "@/gen/bindings/GitHunk";
import { inlineBlame, setBlamed } from "@/lib/code/blame";
import { diffPhrases, diffTheme } from "@/lib/code/diff";
import { codeFind, openFind } from "@/lib/code/find";
import { gitChanges, setHunks } from "@/lib/code/git";
import { loadLanguage } from "@/lib/code/lang";
import { codeHighlight, codeTheme } from "@/lib/code/theme";
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
    indentUnit.of("  "),
    EditorState.allowMultipleSelections.of(false),
    codeHighlight,
    codeTheme,
    codeFind(),
    inlineBlame(),
    anchored,
    keymap.of([...defaultKeymap, ...historyKeymap, ...foldKeymap, indentWithTab]),
  ];
}

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
  b.dataset.merge = type;
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

/** The view inside a box (a file pane, a page), for a host that holds the
    box and not the editor. */
export function editorIn(box: HTMLElement | null): EditorView | null {
  const el = box?.querySelector<HTMLElement>(".cm-editor");
  return el ? EditorView.findFromDOM(el) : null;
}

/** The source line at the top of what is shown. */
export function topLineOf(view: EditorView): number {
  const s = view.scrollDOM;
  const h = s.getBoundingClientRect().top - view.documentTop + 1;
  const b = view.lineBlockAtHeight(Math.max(0, h));
  return view.state.doc.lineAt(b.from).number;
}

/** Scroll so `line` stands at the top of what is shown. */
export function placeLine(view: EditorView, line: number): void {
  const n = Math.max(1, Math.min(line, view.state.doc.lines));
  view.dispatch({ effects: EditorView.scrollIntoView(view.state.doc.line(n).from, { y: "start", yMargin: 0 }) });
}

/** Into the text, where the reader was (P1, 2026-10-03): the caret at the
    start of the first line shown, without moving anything. Called inside
    a press — WebKit raises the soft keyboard only for a focus a touch
    asked for. */
export function focusAtTop(view: EditorView): void {
  const at = view.state.doc.line(topLineOf(view)).from;
  view.dispatch({ selection: EditorSelection.cursor(at) });
  view.focus();
}

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
          edit.current.of(writable(editable)),
          wrapping.current.of(wrap ? EditorView.lineWrapping : []),
          merge.current.of(merging(against)),
          EditorView.contentAttributes.of({ "data-file-text": "" }),
          keymap.of([
            {
              // The one shortcut of the textarea, `metaKey` and `ctrlKey`
              // both: a Mac app, a phone's browser and Windows, one build.
              key: "Mod-s",
              preventDefault: true,
              run: () => {
                cb.current.onSave?.();
                return true;
              },
            },
          ]),
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
    const host = box.current as (HTMLElement & { __find?: () => boolean }) | null;
    if (host) host.__find = () => openFind(v);
    view.current = v;
    return () => {
      v.destroy();
      view.current = null;
    };
    // The view is made once per mount; the props below reach it through
    // their own effects.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    let live = true;
    void loadLanguage(path).then((l) => {
      if (live && l) view.current?.dispatch({ effects: lang.current.reconfigure(l) });
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
    v.dispatch({ changes: { from: a, to: now.length - b, insert: text.slice(a, text.length - b) }, annotations: [fromHost.of(true), Transaction.addToHistory.of(false)] });
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
    const at = v.state.doc.line(goLine.line).from;
    // A third of the way down, as the painted sheet put it.
    v.dispatch({
      selection: EditorSelection.cursor(at),
      effects: [setAnchor.of(goLine.line), EditorView.scrollIntoView(at, { y: "start", yMargin: v.scrollDOM.clientHeight / 3 })],
    });
  }, [goLine, text]);

  return (
    <div
      ref={box}
      data-code
      className={cn("relative min-h-0 flex-1 font-mono text-mono leading-mono", className)}
    >
      <span ref={icons} hidden>
        <IconChevronDown />
        <IconChevronRight />
      </span>
    </div>
  );
}
