// ⌘F inside the file (CM-BRIEF step 1): the editor's own search, in the
// terminal's find bar (item 70, F4.8) — a glass capsule in a row of its
// own above the text, right-aligned, pushing the text down rather than
// lying over what it found: the field, the count, case, whole words
// (the file's alone: a terminal's search has no word edge) and regex, the
// two steps, the way out. Same words, same marks, same keys (⏎ / ⇧⏎,
// ⌘G / ⇧⌘G, esc), so a person who found something in a terminal finds
// it in a file the same way.
//
// **CodeMirror's search state, khor's bar.** The query, the highlighted
// matches and the steps are `@codemirror/search`; only the panel is
// drawn here (`createPanel`), because its own is a web form — and its
// keymap is not taken whole: ⌘D and ⇧⌘L there add selections, and this
// editor has one caret (「不提供任何自动补全什么的」, CM-BRIEF).
//
// ⚠️ Replace is not here yet: the terminal's capsule has no room for a
// second field, and where it goes is a lab question (cm-a.md).
import {
  SearchQuery,
  closeSearchPanel,
  findNext,
  findPrevious,
  getSearchQuery,
  openSearchPanel,
  search,
  searchPanelOpen,
  setSearchQuery,
} from "@codemirror/search";
import type { EditorState, Extension } from "@codemirror/state";
import { EditorSelection } from "@codemirror/state";
import { EditorView, keymap, type Panel, type ViewUpdate } from "@codemirror/view";
import { createRoot, type Root } from "react-dom/client";

import { GlassItem } from "@/components/Glass";
import { IconChevronDown, IconChevronUp, IconClose } from "@/components/icons";
import { TooltipProvider } from "@/components/ui/tooltip";
import { gui } from "@/gen/catalog";

/** Past this many the count stops and says only where the caret is. */
const COUNT_MAX = 2000;

/** Which match the caret is on and how many there are, or null before a
    word is typed. `of` is -1 past `COUNT_MAX`. */
function hitsOf(state: EditorState, q: SearchQuery): { at: number; of: number } | null {
  if (!q.valid || q.search === "") return null;
  const sel = state.selection.main;
  let at = -1;
  let of = 0;
  const c = q.getCursor(state);
  for (let r = c.next(); !r.done; r = c.next()) {
    if (r.value.from === sel.from && r.value.to === sel.to) at = of;
    of += 1;
    if (of >= COUNT_MAX) return { at, of: -1 };
  }
  return { at, of };
}

/** Put the caret on the first match at or after `from`, wrapping — what
    typing into the field does, so the match follows the word as it
    grows (the terminal's `incremental`). */
function seek(view: EditorView, q: SearchQuery, from: number) {
  if (!q.valid || q.search === "") return;
  const first = (start: number) => {
    const c = q.getCursor(view.state, start);
    const r = c.next();
    return r.done ? null : r.value;
  };
  const hit = first(from) ?? first(0);
  if (hit === null) return;
  view.dispatch({
    selection: EditorSelection.single(hit.from, hit.to),
    effects: EditorView.scrollIntoView(hit.from, { y: "center" }),
    userEvent: "select.search",
  });
}

function FindBar({ view, state }: { view: EditorView; state: EditorState }) {
  const q = getSearchQuery(state);
  const hits = hitsOf(state, q);
  const set = (next: Partial<{ search: string; caseSensitive: boolean; regexp: boolean; wholeWord: boolean }>) => {
    const nq = new SearchQuery({
      search: next.search ?? q.search,
      caseSensitive: next.caseSensitive ?? q.caseSensitive,
      regexp: next.regexp ?? q.regexp,
      wholeWord: next.wholeWord ?? q.wholeWord,
    });
    view.dispatch({ effects: setSearchQuery.of(nq) });
    seek(view, nq, view.state.selection.main.from);
  };
  // The terminal's count: 「0」 before a word, the lists' 没有匹配项 for
  // none, "3 / 12" otherwise.
  const hitsText = !hits ? "0" : hits.of === 0 ? gui.no_matches : hits.of < 0 ? `${hits.at + 1}` : `${hits.at + 1} / ${hits.of}`;
  return (
    <div data-code-find-row className="flex flex-none justify-end px-page pt-xxs pb-xxs">
      <div data-code-find className="glass flex h-glass-btn min-w-0 items-center gap-1 rounded-full ps-3 pe-glass-pad">
        <input
          data-code-find-input
          // `openSearchPanel` focuses and selects the field that says this.
          {...{ "main-field": "true" }}
          value={q.search}
          placeholder={gui.search_file}
          aria-label={gui.search_file}
          onChange={(e) => set({ search: e.target.value })}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.preventDefault();
              closeSearchPanel(view);
            } else if (e.key === "Enter") {
              e.preventDefault();
              (e.shiftKey ? findPrevious : findNext)(view);
            } else if ((e.metaKey || e.ctrlKey) && !e.altKey && e.key.toLowerCase() === "g") {
              e.preventDefault();
              (e.shiftKey ? findPrevious : findNext)(view);
            }
          }}
          className="min-w-0 w-48 bg-transparent font-sans text-callout outline-none placeholder:text-tertiary"
        />
        <span data-code-find-hits className="min-w-12 text-right font-sans text-caption tabular-nums text-muted-foreground">
          {hitsText}
        </span>
        <GlassItem
          data-code-find-case
          onMouseDown={(e) => e.preventDefault()}
          aria-label={gui.match_case}
          aria-pressed={q.caseSensitive}
          on={q.caseSensitive}
          onClick={() => set({ caseSensitive: !q.caseSensitive })}
        >
          <span aria-hidden className="font-sans text-caption font-label">Aa</span>
        </GlassItem>
        {/* Whole words (cm-d item 3): VS Code's third toggle, between
            case and regex as there. */}
        <GlassItem
          data-code-find-word
          onMouseDown={(e) => e.preventDefault()}
          aria-label={gui.match_word}
          aria-pressed={q.wholeWord}
          on={q.wholeWord}
          onClick={() => set({ wholeWord: !q.wholeWord })}
        >
          <span aria-hidden className="font-sans text-caption font-label underline underline-offset-2">ab</span>
        </GlassItem>
        <GlassItem
          data-code-find-regex
          onMouseDown={(e) => e.preventDefault()}
          aria-label={gui.match_regex}
          aria-pressed={q.regexp}
          on={q.regexp}
          onClick={() => set({ regexp: !q.regexp })}
        >
          <span aria-hidden className="font-mono text-caption">.*</span>
        </GlassItem>
        <GlassItem data-code-find-prev onMouseDown={(e) => e.preventDefault()} aria-label={gui.match_prev} onClick={() => findPrevious(view)}>
          <IconChevronUp className="size-affordance" />
        </GlassItem>
        <GlassItem data-code-find-next onMouseDown={(e) => e.preventDefault()} aria-label={gui.match_next} onClick={() => findNext(view)}>
          <IconChevronDown className="size-affordance" />
        </GlassItem>
        <GlassItem data-code-find-close aria-label={gui.close} onClick={() => closeSearchPanel(view)}>
          <IconClose className="size-affordance" />
        </GlassItem>
      </div>
    </div>
  );
}

function findPanel(view: EditorView): Panel {
  const dom = document.createElement("div");
  dom.dataset.codeFindPanel = "";
  let root: Root | null = null;
  // A root of its own, so the app's providers are not above it: the
  // glass items' tooltips need theirs here.
  const paint = (state: EditorState) =>
    root?.render(
      <TooltipProvider>
        <FindBar view={view} state={state} />
      </TooltipProvider>,
    );
  return {
    dom,
    top: true,
    mount() {
      root = createRoot(dom);
      paint(view.state);
    },
    update(u: ViewUpdate) {
      if (u.docChanged || u.selectionSet || u.transactions.some((t) => t.effects.some((e) => e.is(setSearchQuery)))) paint(u.state);
    },
    destroy() {
      const r = root;
      root = null;
      // Not inside CodeMirror's own update: React refuses to unmount a
      // root synchronously while it may be rendering.
      queueMicrotask(() => r?.unmount());
    },
  };
}

/** Open the find bar on this editor — the menu's ⌘F, which reaches the
    Mac app's menu before the page (memory khor-native-walk-gotchas), and
    the editor's own ⌘F everywhere else. */
export function openFind(view: EditorView): boolean {
  return openSearchPanel(view);
}

export function codeFind(): Extension {
  return [
    search({ top: true, createPanel: findPanel }),
    keymap.of([
      { key: "Mod-f", run: openSearchPanel, scope: "editor search-panel", preventDefault: true },
      { key: "F3", run: findNext, shift: findPrevious, scope: "editor search-panel", preventDefault: true },
      { key: "Mod-g", run: findNext, shift: findPrevious, scope: "editor search-panel", preventDefault: true },
      { key: "Escape", run: (v) => searchPanelOpen(v.state) && closeSearchPanel(v), scope: "editor search-panel" },
    ]),
  ];
}
