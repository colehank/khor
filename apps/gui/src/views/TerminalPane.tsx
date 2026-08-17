// A live terminal for a session khor hosts here (docs/handoff 终端画屏).
// The screen is the node's — gui-core emulates it with vt100 and answers
// a cell grid; this pane paints the grid, sizes the PTY to the space it
// has, and turns key events into the bytes a terminal sends. It judges
// nothing about the contents (docs/UX.md 状态呈现).
import { useCallback, useLayoutEffect, useRef, useState } from "react";

import { termKey, termLeave, termOpen, termPoll, termResize, type TermColor, type TermScreen } from "@/api";
import { gui } from "@/gen/catalog";

/** The 16 ANSI colours, the one palette a terminal owns regardless of the
    app's theme. 0–7 normal, 8–15 bright. */
const ANSI16 = [
  "#000000", "#cc0000", "#4e9a06", "#c4a000", "#3465a4", "#75507b", "#06989a", "#d3d7cf",
  "#555753", "#ef2929", "#8ae234", "#fce94f", "#729fcf", "#ad7fa8", "#34e2e2", "#eeeeec",
];

/** An indexed colour past 15 is the xterm cube (16–231) then the grey
    ramp (232–255) — the standard mapping, computed not tabled. */
function idxColor(i: number): string {
  if (i < 16) return ANSI16[i];
  if (i < 232) {
    const n = i - 16;
    const r = Math.floor(n / 36);
    const g = Math.floor((n % 36) / 6);
    const b = n % 6;
    const c = (v: number) => (v === 0 ? 0 : 55 + v * 40);
    return `rgb(${c(r)}, ${c(g)}, ${c(b)})`;
  }
  const v = 8 + (i - 232) * 10;
  return `rgb(${v}, ${v}, ${v})`;
}

/** A cell colour to CSS. `Default` becomes the pane's own fg/bg vars, so
    a plain shell wears the app's colours and only coloured output leaves
    them. `null` means "leave it to the caller's default side". */
function css(c: TermColor, side: "fg" | "bg"): string | undefined {
  switch (c.kind) {
    case "default":
      return side === "fg" ? "var(--term-fg)" : undefined;
    case "idx":
      return idxColor(c.n);
    case "rgb":
      return `rgb(${c.n[0]}, ${c.n[1]}, ${c.n[2]})`;
  }
}

/** A DOM key event to the bytes a terminal expects. Printable keys are
    themselves; the named keys and Ctrl-letters are their control
    sequences. Returns null for a key the terminal does not speak (so the
    browser keeps its own shortcuts). */
function keyBytes(e: React.KeyboardEvent): string | null {
  const k = e.key;
  if (e.metaKey) return null; // leave ⌘ shortcuts to the OS/app
  if (e.ctrlKey && k.length === 1 && /[a-z]/i.test(k)) {
    return String.fromCharCode(k.toLowerCase().charCodeAt(0) - 96);
  }
  const named: Record<string, string> = {
    Enter: "\r",
    Backspace: "\x7f",
    Tab: "\t",
    Escape: "\x1b",
    ArrowUp: "\x1b[A",
    ArrowDown: "\x1b[B",
    ArrowRight: "\x1b[C",
    ArrowLeft: "\x1b[D",
    Home: "\x1b[H",
    End: "\x1b[F",
    Delete: "\x1b[3~",
    PageUp: "\x1b[5~",
    PageDown: "\x1b[6~",
  };
  if (k in named) return named[k];
  if (k.length === 1) return k;
  return null;
}

export function TerminalPane({ id }: { id: string }) {
  const boxRef = useRef<HTMLDivElement>(null);
  const measureRef = useRef<HTMLSpanElement>(null);
  const [screen, setScreen] = useState<TermScreen | null>(null);
  const [gone, setGone] = useState(false);
  // The last screen sequence seen — a poll asking for it gets nothing
  // back when the terminal has not moved.
  const seqRef = useRef(0);
  // The character box, measured once from the terminal font.
  const cellRef = useRef<{ w: number; h: number }>({ w: 8, h: 16 });
  // The size last sent, so a resize observer does not spam identical ones.
  const sentRef = useRef<{ cols: number; rows: number }>({ cols: 0, rows: 0 });

  const fit = useCallback((): { cols: number; rows: number } => {
    const box = boxRef.current;
    const { w, h } = cellRef.current;
    if (!box || w === 0 || h === 0) return { cols: 80, rows: 24 };
    return {
      cols: Math.max(1, Math.floor(box.clientWidth / w)),
      rows: Math.max(1, Math.floor(box.clientHeight / h)),
    };
  }, []);

  useLayoutEffect(() => {
    // Measure one character in the terminal font before opening, so the
    // PTY is born the right size and the first paint is not reflowed.
    if (measureRef.current) {
      const r = measureRef.current.getBoundingClientRect();
      cellRef.current = { w: r.width || 8, h: r.height || 16 };
    }
    const { cols, rows } = fit();
    sentRef.current = { cols, rows };
    let stopped = false;
    termOpen(id, cols, rows).catch(() => setGone(true));

    const tick = () => {
      if (stopped) return;
      termPoll(id, seqRef.current)
        .then((b) => {
          if (stopped) return;
          if (b.screen) {
            setScreen(b.screen);
            seqRef.current = b.seq;
          }
          if (b.gone) setGone(true);
        })
        .catch(() => {});
    };
    const poll = window.setInterval(tick, 50);

    const observer = new ResizeObserver(() => {
      const next = fit();
      if (next.cols !== sentRef.current.cols || next.rows !== sentRef.current.rows) {
        sentRef.current = next;
        termResize(id, next.cols, next.rows).catch(() => {});
      }
    });
    if (boxRef.current) observer.observe(boxRef.current);

    return () => {
      stopped = true;
      window.clearInterval(poll);
      observer.disconnect();
      termLeave(id).catch(() => {});
    };
  }, [id, fit]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    const bytes = keyBytes(e);
    if (bytes === null) return;
    e.preventDefault();
    termKey(id, bytes).catch(() => {});
  };

  const { w: cw, h: ch } = cellRef.current;

  return (
    <div
      data-terminal
      tabIndex={0}
      onKeyDown={onKeyDown}
      ref={boxRef}
      className="relative min-h-0 flex-1 overflow-hidden bg-[var(--term-bg)] p-1 font-mono text-sm leading-tight text-[var(--term-fg)] outline-none"
      style={
        {
          // The terminal owns its own two colours; they follow the theme
          // so a plain shell is the app's paper, not a black box.
          "--term-bg": "var(--background)",
          "--term-fg": "var(--foreground)",
        } as React.CSSProperties
      }
    >
      {/* An off-screen glyph in the exact font, measured for the cell box
          before anything is painted. */}
      <span ref={measureRef} className="invisible absolute font-mono text-sm leading-tight">
        M
      </span>
      {gone && (
        <div data-term-over className="absolute right-2 top-1 text-xs text-muted-foreground">
          {gui.term_over}
        </div>
      )}
      {screen?.lines.map((runs, row) => (
        <div key={row} className="whitespace-pre" style={{ height: ch }}>
          {runs.map((run, i) => {
            const fg = run.inverse ? css(run.bg, "bg") ?? "var(--term-bg)" : css(run.fg, "fg");
            const bg = run.inverse ? css(run.fg, "fg") : css(run.bg, "bg");
            return (
              <span
                key={i}
                style={{
                  color: fg,
                  background: bg,
                  fontWeight: run.bold ? 700 : undefined,
                  fontStyle: run.italic ? "italic" : undefined,
                  textDecoration: run.underline ? "underline" : undefined,
                }}
              >
                {run.text}
              </span>
            );
          })}
        </div>
      ))}
      {/* The cursor: a block where the screen says it is, unless the
          program hid it. Positioned by the measured cell, not by counting
          characters — a run can hold many. */}
      {screen && !screen.cursor_hidden && (
        <div
          data-term-cursor
          className="pointer-events-none absolute bg-[var(--term-fg)] opacity-70"
          style={{
            left: 4 + screen.cursor_col * cw,
            top: 4 + screen.cursor_row * ch,
            width: cw,
            height: ch,
          }}
        />
      )}
    </div>
  );
}
