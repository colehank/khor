// **The rows a folded deck gives the terminal** (item 32; ux's UX-13 slot,
// audit/term-tmux-history.md 「Item 32 handoff」). Folding the deck grows
// the terminal's box by `--fold-band` (whole rows of it) and the grid moves
// down by that much instead of growing (80c72378), so no pty is resized.
// The band above the grid shows the history rows just above what the grid
// shows: lines `viewportY − N … viewportY − 1` of the buffer being
// scrolled, drawn as styled spans in xterm's own font and cell size — two
// rows of DOM, not a canvas and not a second xterm. That buffer is the
// tmux history view's on a phone tmux row (the live screen there is tmux's
// alternate screen, which has no history), else the live one; a program on
// its alternate screen with no such view has nothing above it, and the
// band stays empty (progressive disclosure: the fold still works).
//
// The mapping of the scroller is untouched: the band is a function of the
// source's `viewportY`, so item 30's anchor needs nothing new. Between
// rows the scroller moves the source's screen by a part of a row
// (`translateY`); the band takes the same shift, so its last row stays
// against the grid's first.
import type { IBufferCell, Terminal } from "@xterm/xterm";

/** xterm's 256-colour palette past the theme's 16: the 6×6×6 cube, then
    the 24 greys — the table every terminal agrees on. */
function color256(n: number): string {
  if (n >= 232) {
    const v = 8 + (n - 232) * 10;
    return `rgb(${v},${v},${v})`;
  }
  const i = n - 16;
  const level = (c: number) => (c === 0 ? 0 : 55 + c * 40);
  return `rgb(${level(Math.floor(i / 36))},${level(Math.floor(i / 6) % 6)},${level(i % 6)})`;
}

const ANSI_KEYS = [
  "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
  "brightBlack", "brightRed", "brightGreen", "brightYellow", "brightBlue", "brightMagenta", "brightCyan", "brightWhite",
] as const;

/** xterm's own 16, for a theme that leaves them out. */
const ANSI_DEFAULTS = [
  "#2e3436", "#cc0000", "#4e9a06", "#c4a000", "#3465a4", "#75507b", "#06989a", "#d3d7cf",
  "#555753", "#ef2929", "#8ae234", "#fce94f", "#729fcf", "#ad7fa8", "#34e2e2", "#eeeeec",
];

function paletteColor(term: Terminal, n: number): string {
  if (n < 16) return (term.options.theme?.[ANSI_KEYS[n]] as string | undefined) ?? ANSI_DEFAULTS[n];
  return color256(n);
}

function rgb(v: number): string {
  return `rgb(${(v >> 16) & 255},${(v >> 8) & 255},${v & 255})`;
}

/** A cell's colours and attributes as CSS, the way xterm draws it. */
function styleOf(term: Terminal, c: IBufferCell): string {
  const fgDefault = c.isFgDefault();
  const bgDefault = c.isBgDefault();
  let fg = fgDefault ? "" : c.isFgRGB() ? rgb(c.getFgColor()) : paletteColor(term, c.getFgColor());
  let bg = bgDefault ? "" : c.isBgRGB() ? rgb(c.getBgColor()) : paletteColor(term, c.getBgColor());
  if (c.isInverse()) {
    const theme = term.options.theme ?? {};
    const f = fg || (theme.foreground as string | undefined) || "#ffffff";
    const b = bg || (theme.background as string | undefined) || "#000000";
    fg = b;
    bg = f;
  }
  let s = "";
  if (fg) s += `color:${fg};`;
  if (bg) s += `background:${bg};`;
  if (c.isBold()) s += "font-weight:bold;";
  if (c.isItalic()) s += "font-style:italic;";
  if (c.isDim()) s += "opacity:0.5;";
  const lines = [c.isUnderline() && "underline", c.isStrikethrough() && "line-through", c.isOverline() && "overline"].filter(Boolean);
  if (lines.length) s += `text-decoration:${lines.join(" ")};`;
  if (c.isInvisible()) s += "visibility:hidden;";
  return s;
}

/** One buffer line as a row of spans, `cw` px a cell; empty past the end. */
function rowOf(term: Terminal, buffer: Terminal["buffer"]["active"], y: number, cw: number, rowPx: number): HTMLElement {
  const row = document.createElement("div");
  row.style.cssText = `height:${rowPx}px;line-height:${rowPx}px;white-space:pre;overflow:hidden;`;
  const line = y >= 0 ? buffer.getLine(y) : undefined;
  if (!line) return row;
  let cell: IBufferCell | undefined;
  let run = "";
  let runStyle = "";
  let runCells = 0;
  const flush = () => {
    if (!runCells) return;
    const span = document.createElement("span");
    span.style.cssText = `display:inline-block;width:${runCells * cw}px;${runStyle}`;
    span.textContent = run;
    row.appendChild(span);
    run = "";
    runCells = 0;
  };
  for (let x = 0; x < term.cols; x++) {
    cell = line.getCell(x, cell);
    if (!cell) break;
    const width = cell.getWidth();
    if (width === 0) continue; // the second half of a wide character
    const style = styleOf(term, cell);
    if (style !== runStyle) {
      flush();
      runStyle = style;
    }
    run += cell.getChars() || " ";
    runCells += width;
  }
  flush();
  return row;
}

export function drawsFoldBand(o: {
  /** The band: the top of the terminal's box, `--fold-band` high. */
  band: HTMLElement;
  /** The pane's root, where UX-13 sets `data-folded` and `--fold-band`. */
  root: HTMLElement;
  live: Terminal;
  /** The tmux history view's xterm while it holds the scroller, else null. */
  history: () => Terminal | null;
  /** The elements the scrollers move by a part of a row: the live grid's
      mount and the history view's seat. */
  screens: HTMLElement[];
}): { draw: () => void; stop: () => void } {
  const { band, root, live } = o;
  let frame = 0;
  const folded = () => root.dataset.folded === "true";
  const source = (): Terminal | null => o.history() ?? (live.buffer.active.type === "normal" ? live : null);
  /** The source's part-row shift, on the band too. */
  const align = (t: Terminal | null) => {
    const v = t?.element?.parentElement?.style.transform ?? "";
    if (band.style.transform !== v) band.style.transform = v;
  };
  const draw = () => {
    frame = 0;
    const t = folded() ? source() : null;
    align(t);
    const screen = live.element?.querySelector(".xterm-screen") as HTMLElement | null;
    const rowPx = screen ? screen.getBoundingClientRect().height / live.rows : 0;
    const n = rowPx > 0 ? Math.round(band.getBoundingClientRect().height / rowPx) : 0;
    if (!t || n <= 0) {
      band.replaceChildren();
      band.dataset.rows = "0";
      return;
    }
    const cw = screen ? screen.getBoundingClientRect().width / live.cols : 0;
    // xterm's own font, as it is now (a pinch, a setting).
    band.style.fontFamily = String(live.options.fontFamily ?? "");
    band.style.fontSize = `${live.options.fontSize ?? 14}px`;
    const buffer = t.buffer.active;
    const top = buffer.viewportY - n;
    const rows: HTMLElement[] = [];
    for (let i = 0; i < n; i++) rows.push(rowOf(t, buffer, top + i, cw, rowPx));
    band.replaceChildren(...rows);
    band.dataset.rows = String(n);
    band.dataset.from = String(top);
  };
  const soon = () => {
    if (!frame && folded()) frame = window.requestAnimationFrame(draw);
  };
  const watch = new MutationObserver(() => {
    window.cancelAnimationFrame(frame);
    frame = 0;
    // The box grows over the fold's one animation: read once it has.
    if (folded()) frame = window.requestAnimationFrame(() => window.requestAnimationFrame(draw));
    else draw();
  });
  watch.observe(root, { attributes: true, attributeFilter: ["data-folded", "style"] });
  const shifts = new MutationObserver(() => {
    if (folded()) align(source());
  });
  for (const s of o.screens) shifts.observe(s, { attributes: true, attributeFilter: ["style"] });
  const subs = [live.onScroll(soon), live.onWriteParsed(soon), live.buffer.onBufferChange(soon)];
  const scroller = band.closest("[data-terminal]")?.querySelector("[data-term-scroller]");
  scroller?.addEventListener("scroll", soon, { passive: true });
  draw();
  return {
    draw: soon,
    stop: () => {
      window.cancelAnimationFrame(frame);
      watch.disconnect();
      shifts.disconnect();
      for (const s of subs) s.dispose();
      scroller?.removeEventListener("scroll", soon);
    },
  };
}
