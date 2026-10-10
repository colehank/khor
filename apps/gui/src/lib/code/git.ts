// The file against HEAD, drawn in the editor (CM-BRIEF step 1): the
// change gutter, the peek a bar opens, and the overview ruler over the
// scrollbar. Each is what `FileView` drew beside its painted `<pre>` until
// this lane (`Gutter`, `Peek`, `Ruler`, now tombstones there), carried
// over with its rulings and none of its measuring: the editor knows where
// every line is, so nothing here reads a top off the DOM to place a bar.
//
// **The hunks go when the text moves.** A bar says 「this differs from
// HEAD here」 about the text git diffed, and the first keystroke makes
// that a claim about a different text — so a change to the document
// clears them in the same transaction, and the view asks again after the
// save lands (`FileView`'s `diffAt`).
import { RangeSet, RangeSetBuilder, StateEffect, StateField, type EditorState, type Extension, type Range } from "@codemirror/state";
import {
  Decoration,
  EditorView,
  GutterMarker,
  ViewPlugin,
  WidgetType,
  gutter,
  keymap,
  type DecorationSet,
  type ViewUpdate,
} from "@codemirror/view";

import type { GitHunk } from "@/gen/bindings/GitHunk";
import { gui } from "@/gen/catalog";

type Kind = "added" | "deleted" | "modified";

/** Which of git's inks a change wears, from what the change **is**
    (2026-09-16: 「和现在的 git 状态颜色一致」— the user). A deletion is
    asked first because it also has `was`; the order of the two tests is
    the whole difference between 删 and 改. */
export function hunkKind(h: GitHunk): Kind {
  if (h.lines === 0) return "deleted";
  if (h.was.length === 0) return "added";
  return "modified";
}

/** The hunks to draw, replacing whatever was drawn. */
export const setHunks = StateEffect.define<GitHunk[]>();
/** Open the peek on one hunk (its index), or close it with null. */
export const showHunk = StateEffect.define<number | null>();

type Git = { hunks: GitHunk[]; shown: number | null; peek: DecorationSet; bars: RangeSet<GutterMarker> };

/** The line a hunk's bar and peek belong to, or null when the hunk names
    a line this text does not have (a diff of a text that has moved on). */
function lineOf(state: EditorState, h: GitHunk): number | null {
  const n = state.doc.lines;
  if (h.lines === 0) return h.at === 0 ? 1 : h.at <= n ? h.at : null;
  return h.at >= 1 && h.at + h.lines - 1 <= n ? h.at : null;
}

class Bar extends GutterMarker {
  constructor(
    readonly kind: Kind,
    /** The hunk's `at` on its first line only — the handle a gate or a
        reader finds the hunk by, once. */
    readonly first: number | null,
    readonly edge: "top" | "bottom" | null,
    readonly on: boolean,
  ) {
    super();
  }
  eq(o: Bar) {
    return o.kind === this.kind && o.first === this.first && o.edge === this.edge && o.on === this.on;
  }
  toDOM() {
    const s = document.createElement("span");
    s.className = "cm-git-bar";
    s.dataset.kind = this.kind;
    if (this.first !== null) {
      s.dataset.hunk = String(this.first);
      // **Every bar presses, including the ones with nothing removed**
      // (`gui.git_added` is what an insertion's peek says).
      s.setAttribute("role", "button");
      s.setAttribute("aria-label", gui.git_hunk);
    }
    if (this.edge !== null) s.dataset.edge = this.edge;
    if (this.on) s.dataset.on = "";
    return s;
  }
}

function barsOf(state: EditorState, hunks: GitHunk[], shown: number | null) {
  const marks: { from: number; bar: Bar }[] = [];
  hunks.forEach((h, i) => {
    const at = lineOf(state, h);
    if (at === null) return;
    const kind = hunkKind(h);
    const on = shown === i;
    if (h.lines === 0) {
      marks.push({ from: state.doc.line(at).from, bar: new Bar(kind, h.at, h.at === 0 ? "top" : "bottom", on) });
      return;
    }
    for (let l = at; l < at + h.lines; l += 1) {
      marks.push({ from: state.doc.line(l).from, bar: new Bar(kind, l === at ? h.at : null, null, on) });
    }
  });
  marks.sort((a, b) => a.from - b.from);
  const b = new RangeSetBuilder<GutterMarker>();
  for (const m of marks) b.add(m.from, m.from, m.bar);
  return b.finish();
}

/** What HEAD had, **in the file itself, directly over the lines that
    replaced it** (2026-09-16: 「就地显示。跟 vscode 一样」— the user).
 *
 * **In the flow now, where it floated for a year.** It floated because
 * the editor was a textarea — one box holding one string — and pushing
 * its lines apart in the middle tore the caret out of register with the
 * paint (工作区第五轮). An editor that lays out its own lines can put a
 * block between two of them, which is what VS Code's peek is: the lines
 * below move down, nothing is covered, and typing still works.
 *
 * **Plain, unpainted text.** Highlighted, the removed copy would look
 * exactly like the live one below it; the tint and the ink say which is
 * which. */
class Peek extends WidgetType {
  constructor(readonly hunk: GitHunk) {
    super();
  }
  eq(o: Peek) {
    return o.hunk === this.hunk;
  }
  toDOM() {
    const box = document.createElement("div");
    box.className = "cm-git-peek";
    box.dataset.peek = String(this.hunk.at);
    box.dataset.kind = hunkKind(this.hunk);
    const lines = this.hunk.was.length > 0 ? this.hunk.was : [gui.git_added];
    for (const l of lines) {
      const d = document.createElement("div");
      d.textContent = l;
      box.append(d);
    }
    return box;
  }
  ignoreEvent() {
    return false;
  }
}

function peekOf(state: EditorState, hunks: GitHunk[], shown: number | null): DecorationSet {
  const h = shown === null ? undefined : hunks[shown];
  if (h === undefined) return Decoration.none;
  const at = lineOf(state, h);
  if (at === null) return Decoration.none;
  const line = state.doc.line(at);
  // A deletion's old lines go after the line it sits after; everything
  // else over its first line.
  const below = h.lines === 0 && h.at !== 0;
  const d: Range<Decoration> = Decoration.widget({ widget: new Peek(h), block: true, side: below ? 1 : -1 }).range(
    below ? line.to : line.from,
  );
  return Decoration.set([d]);
}

const field = StateField.define<Git>({
  create: () => ({ hunks: [], shown: null, peek: Decoration.none, bars: RangeSet.empty }),
  update(v, tr) {
    let { hunks, shown } = v;
    if (tr.docChanged && hunks.length > 0) {
      hunks = [];
      shown = null;
    }
    for (const e of tr.effects) {
      if (e.is(setHunks)) {
        hunks = e.value;
        shown = null;
      } else if (e.is(showHunk)) shown = e.value !== null && e.value < hunks.length ? e.value : null;
    }
    if (hunks === v.hunks && shown === v.shown && !tr.docChanged) return v;
    return { hunks, shown, peek: peekOf(tr.state, hunks, shown), bars: barsOf(tr.state, hunks, shown) };
  },
  provide: (f) => EditorView.decorations.from(f, (g) => g.peek),
});

/** The hunk a line's bar belongs to. */
function hunkAt(state: EditorState, line: number): number | null {
  const g = state.field(field);
  for (let i = 0; i < g.hunks.length; i += 1) {
    const h = g.hunks[i];
    const at = lineOf(state, h);
    if (at === null) continue;
    if (h.lines === 0 ? at === line : line >= at && line < at + h.lines) return i;
  }
  return null;
}

const gitGutter = gutter({
  class: "cm-git-gutter",
  markers: (v) => v.state.field(field).bars,
  domEventHandlers: {
    mousedown(view, block) {
      const i = hunkAt(view.state, view.state.doc.lineAt(block.from).number);
      if (i === null) return false;
      const was = view.state.field(field).shown;
      view.dispatch({ effects: showHunk.of(was === i ? null : i) });
      return true;
    },
  },
});

/** An opened peek is brought into view, its top a row's gap under the
    scroller's top edge — the place the old one was put, by the same
    arithmetic, without `scrollIntoView` walking every scrolling
    ancestor up to the window (2026-09-12). */
const bringPeek = ViewPlugin.fromClass(
  class {
    update(u: ViewUpdate) {
      const was = u.startState.field(field).shown;
      const now = u.state.field(field).shown;
      if (now === null || now === was) return;
      u.view.requestMeasure({
        read: (view) => {
          const pk = view.dom.querySelector<HTMLElement>(".cm-git-peek");
          if (!pk) return null;
          const air = Number.parseFloat(getComputedStyle(view.dom).getPropertyValue("--gap-row")) || 0;
          return pk.getBoundingClientRect().top - view.scrollDOM.getBoundingClientRect().top - air;
        },
        write: (by, view) => {
          if (by !== null) view.scrollDOM.scrollTop += by;
        },
      });
    }
  },
);

/** The overview ruler: the whole file squeezed into one column laid over
    the scrollbar's strip, a mark where every hunk is (2026-09-16: 「右侧
    滑动条后方…跟 vscode 一样」; 「可以透过滑动条颜色浮上来」). Marks are
    fractions of the scroll; `--overview-mark` keeps a one-line hunk in a
    long file from rounding away. A press on a mark puts its hunk a third
    of the way down; a press on the bare strip centres that place. */
const ruler = ViewPlugin.fromClass(
  class {
    dom: HTMLElement;
    constructor(readonly view: EditorView) {
      this.dom = document.createElement("div");
      this.dom.className = "cm-git-ruler";
      this.dom.addEventListener("click", (e) => this.press(e));
      view.dom.append(this.dom);
      this.measure();
    }
    update(u: ViewUpdate) {
      if (u.docChanged || u.geometryChanged || u.heightChanged || u.startState.field(field) !== u.state.field(field)) this.measure();
    }
    measure() {
      this.view.requestMeasure({
        key: this,
        read: (view) => {
          const g = view.state.field(field);
          const s = view.scrollDOM;
          const total = s.scrollHeight;
          const pad = view.documentTop - s.getBoundingClientRect().top + s.scrollTop;
          const marks = g.hunks.flatMap((h) => {
            const at = lineOf(view.state, h);
            if (at === null) return [];
            const first = view.lineBlockAt(view.state.doc.line(at).from);
            const last = h.lines === 0 ? first : view.lineBlockAt(view.state.doc.line(at + h.lines - 1).from);
            const from = h.lines === 0 ? (h.at === 0 ? first.top : first.bottom) : first.top;
            const to = h.lines === 0 ? from : last.bottom;
            return [{ h, from, to }];
          });
          return { total, pad, marks, top: s.offsetTop, height: s.offsetHeight };
        },
        write: (m) => {
          this.dom.style.top = `${m.top}px`;
          this.dom.style.bottom = "auto";
          this.dom.style.height = `${m.height}px`;
          this.dom.dataset.overview = String(m.marks.length);
          this.dom.replaceChildren(
            ...(m.total <= 0
              ? []
              : m.marks.map(({ h, from, to }) => {
                  const s = document.createElement("span");
                  s.dataset.kind = hunkKind(h);
                  s.dataset.overviewMark = String(h.at);
                  s.setAttribute("role", "button");
                  s.setAttribute("aria-label", gui.git_hunk);
                  s.style.top = `${((m.pad + from) / m.total) * 100}%`;
                  s.style.height = `${((to - from) / m.total) * 100}%`;
                  s.addEventListener("click", (e) => {
                    e.stopPropagation();
                    this.go(h);
                  });
                  return s;
                })),
          );
        },
      });
    }
    go(h: GitHunk) {
      const at = lineOf(this.view.state, h);
      if (at === null) return;
      this.view.dispatch({
        effects: EditorView.scrollIntoView(this.view.state.doc.line(at).from, {
          y: "start",
          yMargin: this.view.scrollDOM.clientHeight / 3,
        }),
      });
    }
    press(e: MouseEvent) {
      const box = this.dom.getBoundingClientRect();
      if (box.height <= 0) return;
      const s = this.view.scrollDOM;
      const want = ((e.clientY - box.top) / box.height) * s.scrollHeight - s.clientHeight / 2;
      s.scrollTop = Math.max(0, Math.min(s.scrollHeight - s.clientHeight, want));
    }
    destroy() {
      this.dom.remove();
    }
  },
);

/** Escape closes an open peek (and does nothing else when none is). */
const closeKey = keymap.of([
  {
    key: "Escape",
    run: (view) => {
      if (view.state.field(field).shown === null) return false;
      view.dispatch({ effects: showHunk.of(null) });
      return true;
    },
  },
]);

/** Everything git draws in the editor. Hunks arrive by `setHunks`. */
export function gitChanges(): Extension {
  return [field, gitGutter, bringPeek, ruler, closeKey];
}

/** The open peek's hunk index, for a host that has to know. */
export const shownHunk = (state: EditorState) => state.field(field, false)?.shown ?? null;
