// **The file's editor is CodeMirror** (cm-a, 2026-10-10): the text is
// the view's document and not a textarea's value, and only the lines in
// the viewport are drawn — so a probe asks the view (`__view` on the
// `[data-code-view]` box). Run in the page before anything else
// (`page.addInitScript(cmProbe)`); everything is on `window.__cm`. A
// `scope` (an element or a selector) narrows to a pane or a page.
export function cmProbe() {
  const view = (scope = document) => {
    const root = typeof scope === "string" ? document.querySelector(scope) : scope;
    return root?.querySelector("[data-file-text]")?.closest("[data-code-view]")?.__view ?? null;
  };
  window.__cm = {
    view,
    text: (scope) => view(scope)?.state.doc.toString() ?? null,
    /** The caret at the start of `line` (1-based), focused. */
    caretTo(line, scope) {
      const v = view(scope);
      if (!v) return false;
      v.focus();
      v.dispatch({ selection: { anchor: v.state.doc.line(line).from }, scrollIntoView: true, userEvent: "select" });
      return true;
    },
    /** The caret at the end of the text, focused. */
    caretEnd(scope) {
      const v = view(scope);
      if (!v) return false;
      v.focus();
      v.dispatch({ selection: { anchor: v.state.doc.length }, scrollIntoView: true, userEvent: "select" });
      return true;
    },
    /** Line `n`'s drawn row, or null when it is outside the viewport. */
    line(n, scope) {
      const v = view(scope);
      if (!v || n > v.state.doc.lines) return null;
      const { node } = v.domAtPos(v.state.doc.line(n).from);
      const el = node.nodeType === 1 ? node : node.parentElement;
      return el?.closest(".cm-line") ?? null;
    },
    /** Line `n`'s number in the gutter, or null when not drawn. */
    num(n, scope) {
      const root = typeof scope === "string" ? document.querySelector(scope) : (scope ?? document);
      return [...(root?.querySelectorAll(".cm-lineNumbers .cm-gutterElement") ?? [])].find((el) => el.textContent === String(n)) ?? null;
    },
    /** A press on a gutter's marker: CodeMirror's gutters answer
        `mousedown` at a height, never a bare `click()`. */
    press(el) {
      const r = el.getBoundingClientRect();
      const at = { bubbles: true, cancelable: true, button: 0, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 };
      el.dispatchEvent(new MouseEvent("mousedown", at));
      el.dispatchEvent(new MouseEvent("mouseup", at));
    },
    /** Where line `n` is on the screen, drawn or not: the view's
        height map, so a line far outside the viewport still answers. */
    block(n, scope) {
      const v = view(scope);
      if (!v || n > v.state.doc.lines) return null;
      const b = v.lineBlockAt(v.state.doc.line(n).from);
      return { top: v.documentTop + b.top, bottom: v.documentTop + b.bottom, offset: b.top };
    },
    /** The line at the top of the scroller (1-based). */
    topLine(scope) {
      const v = view(scope);
      if (!v) return null;
      const h = v.scrollDOM.getBoundingClientRect().top - v.documentTop + 1;
      return v.state.doc.lineAt(v.lineBlockAtHeight(Math.max(0, h)).from).number;
    },
    /** Scroll so line `n` is drawn and at the top. */
    place(n, scope) {
      const v = view(scope);
      if (!v) return false;
      v.dispatch({ effects: v.constructor.scrollIntoView(v.state.doc.line(n).from, { y: "start", yMargin: 0 }) });
      return true;
    },
    /** The caret head's bounding rect on screen, or null when the view
        has no focus or the caret's line is outside the drawn viewport.
        `distanceFromBottom` is negative when the caret is below the
        scroller's visible bottom edge (hidden behind the keyboard). */
    caretRect(scope) {
      const v = view(scope);
      if (!v) return null;
      const pos = v.state.selection.main.head;
      const coords = v.coordsAtPos(pos);
      if (!coords) return null;
      const scroller = v.scrollDOM;
      const sb = scroller.getBoundingClientRect();
      return {
        top: coords.top,
        bottom: coords.bottom,
        scrollerBottom: sb.bottom,
        distanceFromBottom: sb.bottom - coords.bottom,
      };
    },
  };
}
