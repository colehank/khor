// **Blame at the end of the caret's line** (UX-23, the owner 2026-10-05:
// the 22ch column squeezed the code to half the pane — GitLens' default
// instead). One annotation in the tertiary ink, hung off the line's last
// character so it takes no room and moves no line: `7a921aa · 作者 · 3 天前`,
// or the short hash alone where the line leaves no room for that, or —
// where not even the hash fits — at the row's end over the text on the
// file's ground. The whole commit and its subject on hover, or a long
// press under a finger. Not committed: the catalog's word.
//
// Carried over from `FileView`'s `InlineBlame`, which measured a marker
// span in a painted `<pre>` to find the line's end; here the widget *is*
// at the line's end, and only the room after it is measured.
import { EditorSelection, StateEffect, StateField, type Extension } from "@codemirror/state";
import { Decoration, EditorView, WidgetType, type DecorationSet } from "@codemirror/view";

import type { GitCommit } from "@/gen/bindings/GitCommit";
import { gui } from "@/gen/catalog";
import { ago } from "@/words";

/** Who wrote each line of this text, or null while it differs from the
    disk (`lib/blame`'s `blameOfLines`, the host's to compute). */
export const setBlamed = StateEffect.define<(GitCommit | null)[] | null>();

class Blame extends WidgetType {
  /** `line` too, not the commit alone (2026-10-16): the room after the
      line is measured once, in `toDOM`, and a widget equal to the last
      one is kept — so the caret moving to another line of the same
      commit kept the first line's room, and the annotation ran past the
      column's edge (`npm run caret` 4). */
  constructor(
    readonly commit: GitCommit | null,
    readonly line: number,
  ) {
    super();
  }
  eq(o: Blame) {
    return o.commit === this.commit && o.line === this.line;
  }
  toDOM(view: EditorView) {
    const c = this.commit;
    const long = c === null ? gui.blame_uncommitted : `${c.id.slice(0, 7)} · ${c.author} · ${ago(c.when_ms)}`;
    const brief = c === null ? gui.blame_uncommitted : c.id.slice(0, 7);
    const anchor = document.createElement("span");
    anchor.className = "cm-blame";
    anchor.setAttribute("aria-hidden", "true");
    const label = document.createElement("span");
    label.dataset.blameInline = "";
    label.textContent = long;
    if (c !== null) label.title = `${long}\n${c.subject}`;
    anchor.append(label);
    view.requestMeasure({
      read: () => {
        const a = anchor.getBoundingClientRect();
        const content = view.contentDOM.getBoundingClientRect();
        const end = Number.parseFloat(getComputedStyle(view.contentDOM).paddingRight) || 0;
        const line = anchor.closest(".cm-line");
        const pad = line ? Number.parseFloat(getComputedStyle(line).paddingRight) || 0 : 0;
        const room = Math.max(0, content.right - end - pad - a.left);
        const longW = label.scrollWidth;
        label.textContent = brief;
        const briefW = label.scrollWidth;
        label.textContent = long;
        return { room, longW, briefW, x: a.left - content.left };
      },
      write: ({ room, longW, briefW }) => {
        const short = longW > room;
        const over = briefW > room;
        label.textContent = short ? brief : long;
        if (short) label.dataset.blameShort = "";
        if (over) {
          label.dataset.blameOver = "";
          // At the row's end, over the text, on the file's ground.
          label.style.left = "auto";
          label.style.right = `${-room}px`;
          label.style.paddingInlineStart = "0";
          label.style.background = "var(--card)";
          label.style.borderRadius = "var(--radius-xs)";
        } else label.style.maxWidth = `${room}px`;
      },
    });
    // **A press on the annotation is a press on the blank after the
    // line** (the owner 2026-10-16): the caret to the line's end, as
    // CodeMirror puts it for a click past the last glyph. The widget
    // keeps its events from CodeMirror (`ignoreEvent`) and the label
    // takes no selection, so before this the press did nothing — and in
    // WebKit it put the DOM selection inside the label's text, where the
    // next key was lost (`npm run caret` 3). A finger's tap arrives here
    // too, as the mouse event that follows it; the long press is below.
    label.addEventListener("mousedown", (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      const end = view.posAtDOM(anchor);
      view.focus();
      view.dispatch({
        selection: e.shiftKey ? EditorSelection.range(view.state.selection.main.anchor, end) : EditorSelection.cursor(end, -1),
        userEvent: "select.pointer",
      });
    });
    if (c !== null) {
      let press: ReturnType<typeof setTimeout> | null = null;
      label.addEventListener("pointerdown", (e) => {
        if (e.pointerType === "mouse") return;
        press = setTimeout(() => card(anchor, long, c.subject), 450);
      });
      const stop = () => press !== null && clearTimeout(press);
      label.addEventListener("pointerup", stop);
      label.addEventListener("pointercancel", stop);
      label.addEventListener("contextmenu", (e) => e.preventDefault());
    }
    return anchor;
  }
  ignoreEvent() {
    return true;
  }
}

/** The long press's card: the commit and its subject, until the next
    press anywhere. */
function card(anchor: HTMLElement, long: string, subject: string) {
  const c = document.createElement("div");
  c.dataset.blameCard = "";
  c.className =
    "absolute left-0 top-full z-20 w-max max-w-[min(32rem,80vw)] rounded-lg border bg-popover p-in text-caption shadow-(--shadow-float) whitespace-normal";
  const a = document.createElement("div");
  a.className = "font-mono text-muted-foreground";
  a.textContent = long;
  const b = document.createElement("div");
  b.className = "text-foreground";
  b.textContent = subject;
  c.append(a, b);
  anchor.append(c);
  const away = () => {
    c.remove();
    document.removeEventListener("pointerdown", away);
  };
  setTimeout(() => document.addEventListener("pointerdown", away), 0);
}

type State = { blamed: (GitCommit | null)[] | null; line: number | null; deco: DecorationSet };

function decoOf(s: { blamed: (GitCommit | null)[] | null; line: number | null }, doc: { lines: number; line: (n: number) => { to: number } }): DecorationSet {
  if (s.blamed === null || s.line === null || s.line > s.blamed.length || s.line > doc.lines) return Decoration.none;
  const commit = s.blamed[s.line - 1] ?? null;
  return Decoration.set([Decoration.widget({ widget: new Blame(commit, s.line), side: 1 }).range(doc.line(s.line).to)]);
}

/** **Said only once a person has put the caret somewhere** — a file that
    opens wears no annotation on line 1 that nobody asked about. */
const field = StateField.define<State>({
  create: () => ({ blamed: null, line: null, deco: Decoration.none }),
  update(v, tr) {
    let { blamed, line } = v;
    for (const e of tr.effects) if (e.is(setBlamed)) blamed = e.value;
    if (tr.selection && (tr.isUserEvent("select") || tr.isUserEvent("input") || tr.isUserEvent("delete"))) {
      line = tr.state.doc.lineAt(tr.state.selection.main.head).number;
    }
    if (blamed === v.blamed && line === v.line && !tr.docChanged) return v;
    return { blamed, line, deco: decoOf({ blamed, line }, tr.state.doc) };
  },
  provide: (f) => EditorView.decorations.from(f, (s) => s.deco),
});

export function inlineBlame(): Extension {
  return field;
}
