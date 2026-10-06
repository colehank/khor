// **Phone selection is iOS's own** (item 54; the audit's A2, option A).
//
// xterm draws its cells on a canvas, so a long press on the terminal found
// no text and iOS offered nothing: khor drew its own selection in xterm
// (a 450 ms press took a word, a drag extended it, the lift copied
// silently) with no handles, no loupe and no menu. term-8 measured the
// road (term-audit-2.md, 「53 addendum」): a transparent text layer over
// the grid, one box per cell (two for a wide one), lines up with xterm's
// cells, and a long press on it gets iOS's own selection — handles,
// loupe, word bounds by language — and the system's edit menu (拷贝 · 查询
// · 翻译 · 搜索网页 · 共享). khor's own items (打开, 粘贴) are added by the
// native side (`src-tauri/src/edit_menu.rs`, `lib/edit-menu`).
//
// The rules:
// - **The layer is the screen.** Redrawn a frame after the screen
//   changes and at a touch's start, row by row (an unchanged row keeps its
//   element, or a press on it is lost), never while a selection is up. It is inside the scroller, so a drag still scrolls natively,
//   and its taps and clicks are handed to xterm's screen underneath, so a
//   program that holds the mouse still gets them.
// - **While a selection exists the terminal yields the focus** (item 11's
//   rule changes: the field holding the focus took every selection back),
//   and, measured on the simulator, it must already have yielded when the
//   finger lands: so with the keyboard down the terminal rests without
//   the focus, and a hardware keyboard's keys are handed to it from the
//   page. A tap outside the selection, Esc, or typing ends it, with the
//   keyboard as it was.
// - **The lift copies nothing** (⚠️ 墓碑: the silent copy on lift, T-20);
//   the menu's 拷贝 does, and takes the cells' text — wrapped rows joined,
//   trailing blanks trimmed — not the layer's rows.
// - **Double tap takes a word, triple tap the line** (the grid's wrapped
//   rows are one line), as xterm's double and triple click do on the Mac.
// - A long press on a block's prompt keeps opening the block's menu (2.3):
//   those rows are left out of the selection.
import type { Terminal } from "@xterm/xterm";

/** xterm's own default `wordSeparator` (a double-click's word). */
const WORD_SEPARATOR = " ()[]{}',\"`";

/** A finger held this long without moving lets the field go: iOS's own
    text press is recognised at about half a second. */
const YIELD_MS = 400;
/** …and this much travel before then makes it a drag instead. */
const PRESS_SLOP = 8;
/** A focus without the keyboard is let go after this (`nativeSelects`). */
const REST_MS = 400;
/** Two taps this close in time and place are a double tap. */
const MULTI_TAP_MS = 350;
const MULTI_TAP_SLOP = 24;

/** A cell of the layer: its absolute buffer row and column. */
export type Cell = { row: number; col: number };

/** Where the selection is, in cells, first to last (inclusive). */
export type Picked = { from: Cell; to: Cell; text: string };

/** Whether a selection is up (or about to be): the pane's focus rules ask
    before they take the focus back (`tapsAskFirst`'s `left`). */
let holding = false;
export const selectionHolds = () => holding;

/**
 * **The layer over the grid**: one row element per screen row, one box per
 * cell (a wide cell's box is two cells wide, its spacer has none), each
 * exactly where xterm draws that cell. Rows a long press must not select
 * (`claimable`) are left out of the selection. Trailing empty cells are
 * not drawn, so a selection does not run into the blank right of a line.
 */
export function drawLayer(term: Terminal, layer: HTMLElement, host: HTMLElement, claimable: (row: number) => boolean = () => false) {
  const screen = term.element?.querySelector(".xterm-screen");
  if (!screen) return;
  const s = screen.getBoundingClientRect();
  const h = host.getBoundingClientRect();
  const cw = s.width / term.cols;
  const ch = s.height / term.rows;
  layer.style.left = `${s.left - h.left}px`;
  layer.style.top = `${s.top - h.top}px`;
  layer.style.width = `${s.width}px`;
  layer.style.height = `${s.height}px`;
  layer.style.fontFamily = term.options.fontFamily ?? "monospace";
  layer.style.fontSize = `${term.options.fontSize ?? 13}px`;
  layer.style.lineHeight = `${ch}px`;
  const buffer = term.buffer.active;
  // **A row that did not change keeps its element**: a touch that lands
  // on a box the next draw replaced loses its long press (measured on the
  // simulator), and the cursor's blink alone renders its row twice a
  // second.
  const kept = new Map<string, HTMLElement>();
  for (const old of Array.from(layer.children) as HTMLElement[]) kept.set(old.dataset.sig ?? "", old);
  const rows: HTMLElement[] = [];
  for (let y = 0; y < term.rows; y++) {
    const at = buffer.viewportY + y;
    const line = buffer.getLine(at);
    const claimed = claimable(at);
    const sig = `${at}|${line?.isWrapped ? 1 : 0}|${claimed ? 1 : 0}|${cw}|${line?.translateToString(false) ?? ""}`;
    const same = kept.get(sig);
    if (same) {
      kept.delete(sig);
      rows.push(same);
      continue;
    }
    const row = document.createElement("div");
    row.dataset.sig = sig;
    row.dataset.r = String(at);
    if (line?.isWrapped) row.dataset.wrapped = "";
    row.style.height = `${ch}px`;
    row.style.overflow = "hidden";
    if (claimed) {
      row.style.setProperty("user-select", "none");
      row.style.setProperty("-webkit-user-select", "none");
    }
    let last = -1;
    for (let x = term.cols - 1; x >= 0 && line; x--) {
      if (line.getCell(x)?.getChars()) {
        last = x;
        break;
      }
    }
    for (let x = 0; x <= last; x++) {
      const c = line!.getCell(x);
      const w = c?.getWidth() ?? 1;
      if (w === 0) continue;
      const box = document.createElement("span");
      box.dataset.c = String(x);
      box.textContent = c?.getChars() || " ";
      box.style.display = "inline-block";
      // Each box ends where its cell ends, in the layout's own 1/64 px,
      // so the widths' rounding never adds up along a row.
      box.style.width = `${(Math.round((x + w) * cw * 64) - Math.round(x * cw * 64)) / 64}px`;
      row.appendChild(box);
    }
    rows.push(row);
  }
  if (rows.length !== layer.children.length || rows.some((r, i) => layer.children[i] !== r)) layer.replaceChildren(...rows);
}

/** The cell a node of the layer stands for, or the nearest one. */
function cellOf(node: Node | null, offset: number, end: boolean): Cell | null {
  let el: Element | null = node instanceof Element ? node : (node?.parentElement ?? null);
  if (el?.hasAttribute("data-r")) {
    // A boundary between a row's boxes: the box at the offset.
    const boxes = el.children;
    const i = Math.min(Math.max(end ? offset - 1 : offset, 0), boxes.length - 1);
    el = boxes[i] ?? null;
    if (!el) {
      const row = Number((node as Element).getAttribute("data-r"));
      return { row, col: 0 };
    }
  }
  const box = el?.closest?.("[data-c]");
  const row = box?.parentElement?.getAttribute("data-r");
  if (!box || row == null) return null;
  return { row: Number(row), col: Number(box.getAttribute("data-c")) };
}

/**
 * **The text a copy takes: the cells, not the layer's rows** — rows the
 * grid wrapped are one line, each line's trailing blanks trimmed (item 22),
 * read from the layer, which is the screen the person selected on.
 */
export function textOf(layer: HTMLElement, from: Cell, to: Cell): string {
  const lines: string[] = [];
  let line = "";
  for (const row of Array.from(layer.children) as HTMLElement[]) {
    const r = Number(row.dataset.r);
    if (r < from.row || r > to.row) continue;
    if (r > from.row && !("wrapped" in row.dataset)) {
      lines.push(line);
      line = "";
    }
    for (const box of Array.from(row.children) as HTMLElement[]) {
      const c = Number(box.dataset.c);
      if ((r === from.row && c < from.col) || (r === to.row && c > to.col)) continue;
      line += box.textContent ?? "";
    }
  }
  lines.push(line);
  return lines.map((l) => l.replace(/[ \t]+$/, "")).join("\n");
}

/** The selection, if it lies in the layer. */
export function pickedIn(layer: HTMLElement): Picked | null {
  const sel = window.getSelection();
  if (!sel || sel.isCollapsed || sel.rangeCount === 0) return null;
  const range = sel.getRangeAt(0);
  if (!layer.contains(range.startContainer) || !layer.contains(range.endContainer)) return null;
  const from = cellOf(range.startContainer, range.startOffset, false);
  const to = cellOf(range.endContainer, range.endOffset, true);
  if (!from || !to) return null;
  return { from, to, text: textOf(layer, from, to) };
}

/** Selects cells `from`…`to` of the layer as a person's selection would. */
export function selectCells(layer: HTMLElement, from: Cell, to: Cell): boolean {
  const box = (c: Cell) =>
    layer.querySelector<HTMLElement>(`[data-r="${c.row}"] > [data-c="${c.col}"]`) ??
    (layer.querySelector<HTMLElement>(`[data-r="${c.row}"]`)?.lastElementChild as HTMLElement | null);
  const a = box(from);
  const b = box(to);
  if (!a || !b) return false;
  const range = document.createRange();
  range.setStart(a.firstChild ?? a, 0);
  range.setEnd(b.firstChild ?? b, (b.firstChild?.textContent ?? "").length);
  const sel = window.getSelection();
  sel?.removeAllRanges();
  sel?.addRange(range);
  return true;
}

/** The word around a cell, by xterm's separators, over wrapped edges. */
export function wordAround(term: Terminal, at: Cell): { from: Cell; to: Cell } {
  const cols = term.cols;
  const buffer = term.buffer.active;
  const sep = term.options.wordSeparator ?? WORD_SEPARATOR;
  const word = (i: number) => {
    const ch = buffer.getLine(Math.floor(i / cols))?.getCell(i % cols)?.getChars() ?? "";
    return ch !== "" && !sep.includes(ch);
  };
  const wraps = (row: number) => !!buffer.getLine(row)?.isWrapped;
  let from = at.row * cols + at.col;
  let to = from;
  if (word(from)) {
    while (from % cols > 0 || wraps(from / cols)) {
      if (!word(from - 1)) break;
      from -= 1;
    }
    while ((to + 1) % cols > 0 || wraps((to + 1) / cols)) {
      if (!word(to + 1)) break;
      to += 1;
    }
  }
  return { from: { row: Math.floor(from / cols), col: from % cols }, to: { row: Math.floor(to / cols), col: to % cols } };
}

/** The line around a row: the rows the grid wrapped it over. */
export function lineAround(term: Terminal, row: number): { from: Cell; to: Cell } {
  const buffer = term.buffer.active;
  let first = row;
  while (first > 0 && buffer.getLine(first)?.isWrapped) first -= 1;
  let last = row;
  while (buffer.getLine(last + 1)?.isWrapped) last += 1;
  return { from: { row: first, col: 0 }, to: { row: last, col: term.cols - 1 } };
}

export type Selects = {
  /** A tap the selection takes (it ended one, or made one by a double
      or triple tap): the pane's tap rules leave it be. */
  tapEnds: (x: number, y: number) => boolean;
  /** The layer, for the gates. */
  layer: HTMLElement;
  dispose: () => void;
};

/**
 * Wires the layer into the pane: built at a touch's start, yields the
 * focus for a press, keeps the selection's state and gives the focus back.
 */
export function nativeSelects(o: {
  live: Terminal;
  /** The terminal on screen instead of the live one: a tmux pane's
      history while the person is up in it (item 31). */
  over: () => Terminal | null;
  /** The scroller the touches land on; the layer goes in `host`. */
  scroller: HTMLElement;
  host: HTMLElement;
  /** A block's prompt row, where a long press opens the block's menu. */
  claimable: (row: number) => boolean;
  claim: (row: number, x: number, y: number) => boolean;
  /** Whether the soft keyboard is up now, and the ways to put it back. */
  keyboardUp: () => boolean;
  /** The field keeps the focus with the keyboard down: khor's key panel
      stands in its place (UX-31), and the terminal is typed into. */
  keeps: () => boolean;
  /** Gives the terminal the focus back with the keyboard as it was:
      `up` raises it, which only a gesture can. */
  refocus: (up: boolean, inGesture: boolean) => void;
  /** Told the selection whenever it changes, `null` when it goes. */
  picked: (p: Picked | null) => void;
}): Selects {
  const layer = document.createElement("div");
  layer.setAttribute("data-term-select", "");
  layer.setAttribute("data-selectable", "");
  layer.setAttribute("aria-hidden", "true");
  // Its own look, whole, so the gate's bare page draws it as the pane
  // does: over the screen, the cells' text there but not seen.
  layer.style.cssText = "position:absolute;z-index:10;white-space:pre;color:transparent;user-select:text;-webkit-user-select:text;-webkit-touch-callout:default";
  o.host.appendChild(layer);
  // The screen the layer was drawn from, and whether it changed since.
  let shown: Terminal | null = null;
  let stale = true;
  // **Kept current while nothing is selected**, a frame after the screen
  // changes, so the layer a long press lands on is the screen it sees.
  let frame = 0;
  const dirty = () => {
    stale = true;
    if (frame || selecting) return;
    frame = window.requestAnimationFrame(() => {
      frame = 0;
      if (!selecting && !pressing) draw();
    });
  };
  const watch = (t: Terminal) => [t.onRender(dirty), t.onScroll(dirty), t.onResize(dirty)];
  const watching = watch(o.live);
  const draw = () => {
    const t = o.over() ?? o.live;
    if (!stale && t === shown && layer.childElementCount === t.rows) return;
    if (t !== o.live && t !== shown) watching.push(...watch(t));
    shown = t;
    drawLayer(t, layer, o.host, t === o.live ? o.claimable : () => false);
    stale = false;
  };
  // Drawn once the pane is up, so even a first touch lands on it.
  const first = window.requestAnimationFrame(() => draw());

  // A selection's life: up while the layer holds one.
  let selecting = false;
  // The keyboard when the press began: what giving the focus back restores.
  let wasUp = false;
  // When the system took a selection away (a tap it read as "deselect"):
  // the tap that did it is still the selection's.
  let lostAt = -Infinity;
  let pressing = false;

  // **At rest, with the keyboard down, the terminal does not hold the
  // focus** (item 11's rule changes, measured on the simulator: WebKit
  // decides at a touch's start whether a long press selects text or works
  // in the focused field — a field let go in the touchstart, or 400 ms
  // in, got nothing; one not focused before the touch got iOS's
  // selection, handles and menu). So a focus without the keyboard (the
  // page opening, a tab chosen, the keyboard put away) is let go after
  // `REST_MS`, and nothing shows: the cursor keeps its shape
  // (`cursorInactiveStyle`), the program is not told (the pane drops
  // focus reports while `selectionHolds`), and a hardware keyboard's keys
  // are taken from the page and handed to the terminal (`key` below).
  // With the keyboard up the field holds the focus as before.
  let resting: Terminal["options"]["cursorInactiveStyle"] | undefined;
  const field = () => o.live.textarea ?? null;
  const visible = () => o.host.getClientRects().length > 0;
  const away = () => {
    holding = true;
    if (resting === undefined) {
      resting = o.live.options.cursorInactiveStyle;
      o.live.options.cursorInactiveStyle = o.live.options.cursorStyle;
    }
    const f = field();
    if (f && document.activeElement === f) f.blur();
  };
  const home = () => {
    holding = false;
    if (resting !== undefined) {
      o.live.options.cursorInactiveStyle = resting;
      resting = undefined;
    }
  };
  let restTimer = 0;
  const settle = () => {
    window.clearTimeout(restTimer);
    restTimer = window.setTimeout(() => {
      const f = field();
      if (!f || selecting || document.activeElement !== f || f.getAttribute("inputmode") !== "none" || !visible() || o.keeps()) return;
      away();
    }, REST_MS);
  };
  const focused = () => {
    if (field()?.getAttribute("inputmode") === "none") {
      holding = true;
      settle();
    } else home();
  };
  const typed = () => {
    if (field()?.getAttribute("inputmode") === "none") settle();
  };
  // The key panel put away leaves the field focused with no keyboard:
  // at rest again (`lib/key-panel`'s change event).
  const panelMoved = () => {
    if (!o.keeps() && document.activeElement === field()) focused();
  };
  window.addEventListener("khor:key-panel", panelMoved);
  const watched = field();
  watched?.addEventListener("focus", focused);
  watched?.addEventListener("keydown", typed);
  watched?.addEventListener("input", typed);
  const resting0 = window.requestAnimationFrame(() => {
    if (document.activeElement === field()) focused();
    else if (field()?.getAttribute("inputmode") !== "text") away();
  });

  // The key panel was up when the press began: the field had the focus.
  let held = false;
  const giveBack = (inGesture: boolean) => {
    selecting = false;
    o.picked(null);
    // A keyboard that was up comes back up, a key panel's field gets its
    // focus back; otherwise the terminal stays at rest.
    if (wasUp || held) o.refocus(wasUp, inGesture);
    else holding = true;
  };
  const changed = () => {
    const p = pickedIn(layer);
    if (p) {
      selecting = true;
      holding = true;
      o.picked(p);
    } else if (selecting && !pressing) {
      // Gone by the system's own hand: the tap that did it raises a
      // keyboard that was up (`tapEnds`).
      lostAt = performance.now();
      giveBack(false);
    }
  };
  document.addEventListener("selectionchange", changed);

  // **The press.** With the keyboard down iOS does the rest. With it (or
  // the key panel) up the field holds the focus, so a press held 400 ms lets it go and
  // selects the word under the finger itself: iOS's handles, its menu on
  // a tap on the selection. A block's prompt row opens the block's menu
  // instead (2.3), and is left out of the selection.
  let timer = 0;
  let x0 = 0;
  let y0 = 0;
  let yielded = false;
  const start = (e: TouchEvent) => {
    window.clearTimeout(timer);
    pressing = e.touches.length === 1;
    if (!pressing) return;
    x0 = e.touches[0].clientX;
    y0 = e.touches[0].clientY;
    if (selecting) return;
    yielded = false;
    draw();
    wasUp = o.keyboardUp();
    held = !wasUp && o.keeps();
    timer = window.setTimeout(() => {
      if (selecting) return;
      const t = o.over() ?? o.live;
      const at = cellAtPoint(t, x0, y0);
      if (!at) return;
      if (t === o.live && o.claimable(at.row)) {
        o.claim(at.row, x0, y0);
        return;
      }
      if (!wasUp && !held) return;
      yielded = true;
      away();
      const word = wordAround(t, at);
      if (selectCells(layer, word.from, word.to)) changed();
    }, YIELD_MS);
  };
  const move = (e: TouchEvent) => {
    const t = e.touches[0];
    if (t && Math.hypot(t.clientX - x0, t.clientY - y0) > PRESS_SLOP) window.clearTimeout(timer);
  };
  const end = () => {
    window.clearTimeout(timer);
    pressing = false;
    if (selecting || !yielded) return;
    yielded = false;
    // The keyboard was up and nothing was selected: it comes back, inside
    // the lift.
    if (!pickedIn(layer)) giveBack(true);
  };
  // A press's lift that ended a handle's drag: the selection may have
  // changed under it.
  const lifted = () => {
    if (selecting) changed();
  };
  o.scroller.addEventListener("touchstart", start, { capture: true, passive: true });
  o.scroller.addEventListener("touchmove", move, { capture: true, passive: true });
  o.scroller.addEventListener("touchend", end, { capture: true });
  o.scroller.addEventListener("touchcancel", end, { capture: true });
  document.addEventListener("touchend", lifted, true);

  // Taps and clicks on the layer are xterm's: a program that holds the
  // mouse still gets them, and xterm's own clear of its selection runs.
  // The press's own default (the focus to the page, a selection begun) is
  // held back: the field keeps the keys.
  const forward = (e: MouseEvent) => {
    const screen = (o.over() ?? o.live).element?.querySelector(".xterm-screen");
    if (!screen || !e.isTrusted) return;
    e.stopPropagation();
    if (e.type === "mousedown") e.preventDefault();
    screen.dispatchEvent(new MouseEvent(e.type, e));
  };
  for (const type of ["mousedown", "mouseup", "click"]) layer.addEventListener(type, forward as EventListener);

  // **The menu's 拷贝 takes the cells** — wrapped rows joined, trailing
  // blanks trimmed — not the layer's rows broken at the screen's edge.
  const copy = (e: ClipboardEvent) => {
    const p = pickedIn(layer);
    if (!p || !e.clipboardData) return;
    e.clipboardData.setData("text/plain", p.text);
    e.preventDefault();
  };
  document.addEventListener("copy", copy, true);

  // **A key with the terminal at rest goes to the terminal** (a hardware
  // keyboard: item 11's "type at once" kept). The field takes the focus
  // and the key is handed to xterm: a character as typed, anything else
  // (Enter, ⌫, arrows, ⌃-keys) as the same keydown, which xterm reads
  // whole. ⌘V gives the field the focus first, so the paste lands there;
  // other ⌘ keys are the app's. Over a selection Esc ends it and is not
  // sent; any other key ends it and is.
  const MODIFIERS = ["Shift", "Control", "Alt", "Meta", "CapsLock"];
  const route = (e: KeyboardEvent) => {
    const f = field();
    if (!f) return;
    f.focus({ preventScroll: true });
    settle();
    if (e.key === "Unidentified" || e.key === "Process") return;
    e.preventDefault();
    e.stopImmediatePropagation();
    if (e.key.length === 1 && !e.ctrlKey && !e.altKey) {
      o.live.input(e.key, true);
      return;
    }
    const again = new KeyboardEvent("keydown", e);
    Object.defineProperty(again, "keyCode", { get: () => e.keyCode });
    Object.defineProperty(again, "which", { get: () => e.keyCode });
    f.dispatchEvent(again);
  };
  const key = (e: KeyboardEvent) => {
    if (!visible() || MODIFIERS.includes(e.key)) return;
    if (selecting) {
      if (e.metaKey) return;
      window.getSelection()?.removeAllRanges();
      selecting = false;
      o.picked(null);
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopImmediatePropagation();
        o.refocus(wasUp, false);
        return;
      }
      route(e);
      return;
    }
    const at = document.activeElement;
    if (!holding || (at && at !== document.body)) return;
    if (e.metaKey) {
      if (e.key.toLowerCase() === "v") {
        field()?.focus({ preventScroll: true });
        settle();
      }
      return;
    }
    route(e);
  };
  document.addEventListener("keydown", key, true);

  // Taps: one outside the selection ends it; two take a word, three the line.
  let taps = 0;
  let lastTap = 0;
  let lastX = 0;
  let lastY = 0;
  // The keyboard before a run of taps began: the first tap of a double
  // may have raised or lowered it.
  let upBefore = false;
  const tapEnds = (x: number, y: number): boolean => {
    const now = performance.now();
    if (!selecting && now - lostAt < 400) {
      lostAt = -Infinity;
      if (wasUp) o.refocus(true, true);
      return true;
    }
    taps = now - lastTap < MULTI_TAP_MS && Math.hypot(x - lastX, y - lastY) < MULTI_TAP_SLOP ? taps + 1 : 1;
    lastTap = now;
    lastX = x;
    lastY = y;
    if (taps === 1 && !selecting) upBefore = o.keyboardUp();
    if (taps >= 2) {
      const t = o.over() ?? o.live;
      const at = cellAtPoint(t, x, y);
      if (!at || (t === o.live && o.claimable(at.row))) return false;
      const span = taps === 2 ? wordAround(t, at) : lineAround(t, at.row);
      if (taps === 2) wasUp = upBefore;
      away();
      if (!selectCells(layer, span.from, span.to)) {
        giveBack(true);
        return false;
      }
      changed();
      return true;
    }
    if (!selecting) return false;
    const sel = window.getSelection();
    const inside =
      !!sel &&
      sel.rangeCount > 0 &&
      Array.from(sel.getRangeAt(0).getClientRects()).some((r) => x >= r.left && x <= r.right && y >= r.top && y <= r.bottom);
    // A tap on the selection is iOS's (its menu); elsewhere it ends it.
    if (inside) return true;
    sel?.removeAllRanges();
    giveBack(true);
    return true;
  };

  return {
    tapEnds,
    layer,
    dispose: () => {
      window.clearTimeout(timer);
      window.clearTimeout(restTimer);
      window.cancelAnimationFrame(resting0);
      window.removeEventListener("khor:key-panel", panelMoved);
      watched?.removeEventListener("focus", focused);
      watched?.removeEventListener("keydown", typed);
      watched?.removeEventListener("input", typed);
      home();
      window.cancelAnimationFrame(first);
      window.cancelAnimationFrame(frame);
      holding = false;
      for (const d of watching) d.dispose();
      document.removeEventListener("selectionchange", changed);
      document.removeEventListener("copy", copy, true);
      document.removeEventListener("keydown", key, true);
      o.scroller.removeEventListener("touchstart", start, true);
      o.scroller.removeEventListener("touchmove", move, true);
      o.scroller.removeEventListener("touchend", end, true);
      o.scroller.removeEventListener("touchcancel", end, true);
      document.removeEventListener("touchend", lifted, true);
      layer.remove();
    },
  };
}

/** The cell under a point of the screen. */
function cellAtPoint(term: Terminal, x: number, y: number): Cell | null {
  const screen = term.element?.querySelector(".xterm-screen");
  if (!screen) return null;
  const r = screen.getBoundingClientRect();
  if (x < r.left || x > r.right || y < r.top || y > r.bottom) return null;
  const col = Math.max(0, Math.min(term.cols - 1, Math.floor(((x - r.left) / r.width) * term.cols)));
  const inView = Math.max(0, Math.min(term.rows - 1, Math.floor(((y - r.top) / r.height) * term.rows)));
  return { col, row: term.buffer.active.viewportY + inView };
}
