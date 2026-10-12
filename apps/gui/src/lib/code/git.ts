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
import { language } from "@codemirror/language";
import { RangeSet, RangeSetBuilder, StateEffect, StateField, type EditorState, type Extension, type Range } from "@codemirror/state";
import {
  Decoration,
  EditorView,
  GutterMarker,
  ViewPlugin,
  WidgetType,
  gutter,
  type DecorationSet,
  type ViewUpdate,
} from "@codemirror/view";

import { highlightTree } from "@lezer/highlight";

import type { GitHunk } from "@/gen/bindings/GitHunk";
import { gui } from "@/gen/catalog";
import { mountHead, unmountHead } from "@/lib/code/peek-head";
import { revertChange } from "@/lib/code/revert";
import { highlight } from "@/lib/code/theme";

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
 * **A head bar over the old lines** (cm-e, 2026-10-12: 「显示的内容是否
 * 和 vscode 一致以符合用户习惯」): which change of how many, the two
 * steps, 「还原」 and × (`peek-head.tsx`).
 *
 * **In the file's own syntax ink** (cm-e, 2026-10-12, VS Code's peek is
 * painted). Plain text was argued for once — painted, the removed copy
 * would look exactly like the live one below it — but the wash and its
 * hued edges already say which is which, and unpainted old code is
 * harder to read against the new than painted. The old text is parsed
 * on its own, with the language the editor holds (no second state: a
 * parse and a tree walk, which is all the paint needs), so a hunk that
 * begins mid-construct — inside a block comment, say — paints as its
 * lines read alone. */
class Peek extends WidgetType {
  constructor(
    readonly hunk: GitHunk,
    /** Its place among the drawn hunks, from 1, and how many. */
    readonly at: number,
    readonly of: number,
  ) {
    super();
  }
  eq(o: Peek) {
    return o.hunk === this.hunk && o.at === this.at && o.of === this.of;
  }
  toDOM(view: EditorView) {
    const box = document.createElement("div");
    box.className = "cm-git-peek";
    box.dataset.peek = String(this.hunk.at);
    box.dataset.kind = hunkKind(this.hunk);
    const head = document.createElement("div");
    head.className = "cm-git-peek-head";
    head.dataset.peekHead = "";
    mountHead(head, {
      at: this.at,
      of: this.of,
      step: (by) => stepHunk(view, by),
      revert: view.state.readOnly ? null : () => revertHunk(view),
      close: () => view.dispatch({ effects: showHunk.of(null) }),
    });
    box.append(head);
    if (this.hunk.was.length === 0) {
      const d = document.createElement("div");
      d.className = "cm-git-was";
      d.dataset.said = "";
      d.textContent = gui.git_added;
      box.append(d);
    } else {
      box.append(...paintWas(view.state, this.hunk.was));
    }
    return box;
  }
  destroy(dom: HTMLElement) {
    const head = dom.querySelector<HTMLElement>("[data-peek-head]");
    if (head) unmountHead(head);
  }
  ignoreEvent() {
    return true;
  }
}

/** The hunks this text has, by index, in document order (`hunks`
    arrive in it; the ones naming lines the text lacks are not drawn). */
function drawn(state: EditorState, hunks: GitHunk[]): number[] {
  return hunks.flatMap((h, i) => (lineOf(state, h) === null ? [] : [i]));
}

/** HEAD's lines, one `div` each, in the highlighter's classes when the
    editor has a language. */
function paintWas(state: EditorState, was: string[]): HTMLElement[] {
  const rows = was.map(() => {
    const d = document.createElement("div");
    d.className = "cm-git-was";
    return d;
  });
  const lang = state.facet(language);
  const text = was.join("\n");
  // Each line's start in `text`, to cut the spans at the breaks.
  const starts: number[] = [];
  let at = 0;
  for (const l of was) {
    starts.push(at);
    at += l.length + 1;
  }
  let row = 0;
  let pos = 0;
  /** `text[pos, to)` into the rows, in `cls` (none: a bare text node). */
  const put = (to: number, cls: string) => {
    while (pos < to) {
      while (row + 1 < starts.length && starts[row + 1] <= pos) row += 1;
      const end = Math.min(to, starts[row] + was[row].length);
      if (end > pos) {
        const piece = text.slice(pos, end);
        if (cls) {
          const s = document.createElement("span");
          s.className = cls;
          s.textContent = piece;
          rows[row].append(s);
        } else rows[row].append(piece);
      }
      // Past the line's end, over its break, to the next.
      pos = end < to ? end + 1 : end;
    }
  };
  if (lang !== null) {
    highlightTree(lang.parser.parse(text), highlight, (from, to, cls) => {
      put(from, "");
      put(to, cls);
    });
  }
  put(text.length, "");
  return rows;
}

function peekOf(state: EditorState, hunks: GitHunk[], shown: number | null): DecorationSet {
  const h = shown === null ? undefined : hunks[shown];
  if (h === undefined) return Decoration.none;
  const at = lineOf(state, h);
  if (at === null) return Decoration.none;
  const order = drawn(state, hunks);
  const line = state.doc.line(at);
  // A deletion's old lines go after the line it sits after; everything
  // else over its first line.
  const below = h.lines === 0 && h.at !== 0;
  const d: Range<Decoration> = Decoration.widget({
    widget: new Peek(h, order.indexOf(shown!) + 1, order.length),
    block: true,
    side: below ? 1 : -1,
  }).range(
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

/** **An opened peek moves the view only as far as it has to** (2026-10-12:
    「点击后总是移动了我的视窗位置，这对吗」— the user; VS Code's
    `revealRange`, not a centre). The block that has to be seen is the
    peek with the line it is drawn against; if it is wholly on screen,
    with a line's air at either edge, nothing moves. Otherwise the least
    scroll that brings it on, and when it is taller than the screen its
    top wins — `scrollIntoView`'s `nearest` with one line of margin
    (KB-CARET rule 2), done here on the block's box because that effect
    takes text positions, which a block widget is not, and because it
    walks every scrolling ancestor up to the window (2026-09-12). The
    read is in the measure, the write after it. */
const bringPeek = ViewPlugin.fromClass(
  class {
    update(u: ViewUpdate) {
      const was = u.startState.field(field).shown;
      const now = u.state.field(field).shown;
      if (now === null || now === was) return;
      this.reveal(u.view, now, true);
    }
    reveal(view: EditorView, i: number, again: boolean) {
      view.requestMeasure({
        read: (view) => {
          const h = view.state.field(field).hunks[i];
          const at = h === undefined ? null : lineOf(view.state, h);
          if (at === null) return null;
          // The line's block in the height map holds the peek with it, so
          // a peek CodeMirror has not drawn yet (off screen) still has a
          // place — its estimate until it is drawn.
          const block = view.lineBlockAt(view.state.doc.line(at).from);
          const top = view.documentTop + block.top;
          const bottom = view.documentTop + block.bottom;
          const s = view.scrollDOM.getBoundingClientRect();
          const air = view.defaultLineHeight;
          const floor = s.top + view.scrollDOM.clientHeight;
          const drawn = view.dom.querySelector(".cm-git-peek") !== null;
          if (top < s.top + air) return { by: top - (s.top + air), drawn };
          if (bottom > floor - air) return { by: Math.min(bottom - floor + air, top - (s.top + air)), drawn };
          return { by: 0, drawn };
        },
        write: (m, view) => {
          if (m === null || m.by === 0) return;
          view.scrollDOM.scrollTop += m.by;
          // Brought on by its estimate: once more on its drawn height.
          if (!m.drawn && again) this.reveal(view, i, false);
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

/** Walk to the next (1) or previous (-1) hunk with the peek open,
    wrapping at either end as VS Code's does; the scroll is `bringPeek`'s. */
function stepHunk(view: EditorView, by: 1 | -1): boolean {
  const g = view.state.field(field);
  if (g.shown === null) return false;
  const order = drawn(view.state, g.hunks);
  if (order.length === 0) return false;
  const here = order.indexOf(g.shown);
  const next = order[(here + by + order.length) % order.length];
  if (next !== g.shown) view.dispatch({ effects: showHunk.of(next) });
  return true;
}

/** 「还原」: the open hunk's lines become HEAD's, one transaction, so one
    ⌘Z takes it back; the host's save road sees an edit like any other,
    and the hunks clear with it until the next diff (the header's rule). */
function revertHunk(view: EditorView): boolean {
  const g = view.state.field(field);
  const h = g.shown === null ? undefined : g.hunks[g.shown];
  if (h === undefined || view.state.readOnly) return false;
  const change = revertChange(view.state.doc, h);
  if (change === null) return false;
  view.dispatch({ changes: change, userEvent: "input.revert", scrollIntoView: false });
  view.focus();
  return true;
}

/** **Esc shuts the open peek, wherever the focus is** (2026-09-16, carried
    over from `FileView`'s window listener): the bar is a strip nobody
    focuses and a reader's file takes no caret, so a key handler on the
    editor would hear nothing. Listening only while a peek is open — and
    for the same reason VS Code's ⌥F5 / ⇧⌥F5 (next / previous change) are
    heard here and not in the editor's keymap: with no peek open the keys
    are nobody's. */
const closeKey = ViewPlugin.fromClass(
  class {
    off: (() => void) | null = null;
    constructor(readonly view: EditorView) {
      this.sync();
    }
    update(u: ViewUpdate) {
      if (u.state.field(field).shown !== u.startState.field(field).shown) this.sync();
    }
    sync() {
      const open = this.view.state.field(field).shown !== null;
      if (open && this.off === null) {
        const onKey = (e: KeyboardEvent) => {
          if (e.defaultPrevented) return;
          if (e.key === "Escape") this.view.dispatch({ effects: showHunk.of(null) });
          else if (e.key === "F5" && e.altKey && !e.metaKey && !e.ctrlKey && stepHunk(this.view, e.shiftKey ? -1 : 1)) e.preventDefault();
        };
        window.addEventListener("keydown", onKey);
        this.off = () => window.removeEventListener("keydown", onKey);
      } else if (!open && this.off !== null) {
        this.off();
        this.off = null;
      }
    }
    destroy() {
      this.off?.();
    }
  },
);

/** Everything git draws in the editor. Hunks arrive by `setHunks`. */
export function gitChanges(): Extension {
  return [field, gitGutter, bringPeek, ruler, closeKey];
}

/** The open peek's hunk index, for a host that has to know. */
export const shownHunk = (state: EditorState) => state.field(field, false)?.shown ?? null;
