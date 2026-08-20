// A live terminal for a session khor hosts here (docs/handoff 终端画屏).
// The screen is the node's — gui-core emulates it with vt100 and answers
// a cell grid; this pane paints the grid, sizes the PTY to the space it
// has, and turns key events into the bytes a terminal sends. It judges
// nothing about the contents (docs/UX.md 状态呈现).
import { useCallback, useLayoutEffect, useRef, useState } from "react";

import {
  onBridge,
  termDrop,
  termKey,
  termLeave,
  termOpen,
  termPaste,
  termPoll,
  termResize,
  type TermColor,
  type TermScreen,
} from "@/api";
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
  // Why the terminal never opened, in khor's own words. **Not the same
  // fact as `gone`**: `gone` means the host this pane was watching
  // ended, and 这个终端结束了 is true of it; a failed open means there
  // never was a host to watch, and saying 结束了 about a tmux session
  // sitting right there sends a person looking for a session that
  // ended. Every refusal khor makes here is already a sentence written
  // for a person (宿主没了 / 要 serve 在跑 / the far machine's own
  // answer), and throwing it away is what flattened all of them into
  // one wrong word.
  const [why, setWhy] = useState<string | null>(null);
  // Why the last drop produced no paste, in khor's own words. Kept on
  // screen until a drop works, because a failure that cleared itself
  // was not reported (docs/UX.md 失败就地说) — and this one has a
  // reason a person can act on: the far machine's khor is older than
  // the field that says where files landed.
  const [dropWhy, setDropWhy] = useState<string | null>(null);
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
    termOpen(id, cols, rows).catch((e: unknown) => {
      setWhy(String((e as { message?: string })?.message ?? e));
    });

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

    // Coalesced to one fit per frame (mandala's TerminalView judgment):
    // dragging a window fires ResizeObserver tens of times a second, and
    // a resize is an IPC round plus a PTY resize plus a full repaint —
    // per callback that is the drag stutter itself. rAF folds a burst
    // into at most one.
    let raf = 0;
    const observer = new ResizeObserver(() => {
      if (raf) return;
      raf = window.requestAnimationFrame(() => {
        raf = 0;
        const next = fit();
        if (next.cols !== sentRef.current.cols || next.rows !== sentRef.current.rows) {
          sentRef.current = next;
          termResize(id, next.cols, next.rows).catch(() => {});
        }
      });
    });
    if (boxRef.current) observer.observe(boxRef.current);

    return () => {
      stopped = true;
      window.clearInterval(poll);
      if (raf) window.cancelAnimationFrame(raf);
      observer.disconnect();
      termLeave(id).catch(() => {});
    };
  }, [id, fit]);

  // Files dropped on this pane land as shell-quoted paths at the cursor
  // (iTerm2's behaviour, 批④).
  //
  // **Only in the app, and that is not a shortcut.** tauri intercepts OS
  // drops before the webview sees them, so the real paths arrive on a
  // tauri event and the HTML5 `drop` never fires with anything useful —
  // in the browser a dropped file is a `File` object with no path at
  // all. Setting a listener up there would be a listener that never
  // fires, which reads like a feature that is present and broken.
  //
  // The position is checked against this pane's own box: the event is
  // the window's, not this element's, so without it a drop anywhere in
  // the app would type into whatever terminal happened to be mounted.
  useLayoutEffect(() => {
    if (onBridge) return;
    let drop: (() => void) | undefined;
    let stopped = false;
    void (async () => {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        const un = await getCurrentWebview().onDragDropEvent((e) => {
          if (e.payload.type !== "drop") return;
          const box = boxRef.current?.getBoundingClientRect();
          const at = e.payload.position;
          if (!box || !at) return;
          const inside =
            at.x >= box.left && at.x <= box.right && at.y >= box.top && at.y <= box.bottom;
          if (!inside || e.payload.paths.length === 0) return;
          // **A cross-machine drop takes as long as the files take**:
          // the paste only appears once they are over there, and what
          // narrates the wait is the transfer's own row — the corner
          // strip picks it up because it is a process the node already
          // has a state for (`use-status-bar`), so there is nothing to
          // invent here.
          setDropWhy(null);
          termDrop(id, e.payload.paths).catch((err: unknown) => {
            setDropWhy(String((err as { message?: string })?.message ?? err));
          });
        });
        if (stopped) un();
        else drop = un;
      } catch {
        // No tauri here. Nothing to listen to and nothing to say.
      }
    })();
    return () => {
      stopped = true;
      drop?.();
    };
  }, [id]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    const bytes = keyBytes(e);
    if (bytes === null) return;
    e.preventDefault();
    termKey(id, bytes).catch(() => {});
  };

  // Paste goes to the PTY, not the page. Selection needs no such care —
  // this grid is DOM text, so select-and-copy just work (the reason
  // mandala had to intercept Cmd+C was a canvas with no text in it).
  //
  // **Sent as a paste, not as keystrokes.** A program that asked for
  // bracketed paste gets the text marked as one lump; one that did not
  // gets exactly what it got before. Which of those it is belongs to the
  // screen the registry already keeps, so it is decided there — this
  // only says what kind of event this was (`term_paste`).
  const onPaste = (e: React.ClipboardEvent) => {
    const text = e.clipboardData.getData("text");
    if (!text) return;
    e.preventDefault();
    termPaste(id, text).catch(() => {});
  };

  const { w: cw, h: ch } = cellRef.current;

  return (
    <div
      data-terminal
      tabIndex={0}
      onKeyDown={onKeyDown}
      onPaste={onPaste}
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
      {why ? (
        // Centred, not a corner label: there is no screen behind it to
        // annotate, and this sentence is the whole of what the pane has
        // to say.
        <div
          data-term-why
          className="absolute inset-0 grid place-items-center px-6 text-center text-sm text-muted-foreground"
        >
          {why}
        </div>
      ) : (
        <div className="pointer-events-none absolute right-2 top-1 flex flex-col items-end gap-1 text-xs">
          {gone && (
            <div data-term-over className="text-muted-foreground">
              {gui.term_over}
            </div>
          )}
          {/* Held to the pane's own width so a sentence wraps here
              instead of pushing the screen sideways. */}
          {dropWhy && (
            <div data-term-drop-why className="max-w-72 text-right text-destructive">
              {dropWhy}
            </div>
          )}
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
