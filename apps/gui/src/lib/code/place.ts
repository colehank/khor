// **Reaching the editor through the box** (CM-BRIEF §2, the bundle
// split). A host that holds a file pane and not the editor needs four
// small things from it — which view is inside, where it is scrolled,
// putting it somewhere, and focusing it. `editorIn` used to answer the
// first with `EditorView.findFromDOM`, and that one import dragged the
// whole of CodeMirror's runtime into the entry chunk, because both
// `FileView` and `FilePage` import this module for the other three.
//
// `CodeEditor` already puts the view itself on its box as `__view` (the
// name the gates and the find bar use), and the box wears `data-code-view`
// (which `scripts/cm-probe.mjs` finds it by). So the view is reachable
// from the DOM the host already holds, through `__view`'s own methods —
// no runtime import at all. **`EditorView` is only a *type* here**, and a
// type is erased before the bundle is drawn.
import type { EditorView } from "@codemirror/view";

/** The box `CodeEditor` mounts into, as it carries the view. */
type Host = HTMLElement & { __view?: EditorView };

/** The view inside a box (a file pane, a page), for a host that holds the
    box and not the editor. `CodeEditor` sets `__view` on its own box when
    it mounts, and the box is `[data-code-view]`. */
export function editorIn(box: HTMLElement | null): EditorView | null {
  const el = box?.querySelector<Host>("[data-code-view]");
  return el?.__view ?? null;
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
  // `scrollIntoView` is `EditorView`'s own static; reached through the
  // instance so that the class itself stays a type here.
  const View = view.constructor as typeof EditorView;
  view.dispatch({ effects: View.scrollIntoView(view.state.doc.line(n).from, { y: "start", yMargin: 0 }) });
}

/** Into the text, where the reader was (P1, 2026-10-03): the caret at the
    start of the first line shown, without moving anything. Called inside
    a press — WebKit raises the soft keyboard only for a focus a touch
    asked for. */
export function focusAtTop(view: EditorView): void {
  const at = view.state.doc.line(topLineOf(view)).from;
  view.dispatch({ selection: { anchor: at } });
  view.focus();
}
