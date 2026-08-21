// A live terminal for a session khor hosts here (docs/handoff 终端画屏).
// The screen is the node's — gui-core emulates it with vt100 and answers
// a cell grid; this pane paints the grid, sizes the PTY to the space it
// has, and turns key events into the bytes a terminal sends. It judges
// nothing about the contents (docs/UX.md 状态呈现).
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

import {
  inApp,
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
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";

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
    // The function keys, in the table xterm actually uses rather than
    // the one it looks like it should. F1–F4 are SS3 (`ESC O`) and the
    // rest are CSI with a number — **and the numbers skip 16 and 22**,
    // a historical gap that is not a typo here. A terminal that sends
    // its own tidier numbering is a terminal whose F6 arrives as
    // somebody else's key.
    F1: "\x1bOP",
    F2: "\x1bOQ",
    F3: "\x1bOR",
    F4: "\x1bOS",
    F5: "\x1b[15~",
    F6: "\x1b[17~",
    F7: "\x1b[18~",
    F8: "\x1b[19~",
    F9: "\x1b[20~",
    F10: "\x1b[21~",
    F11: "\x1b[23~",
    F12: "\x1b[24~",
  };
  if (k in named) return named[k];
  if (k.length === 1) return k;
  return null;
}

/** How often a visible terminal asks for its screen. Fast: this is the
    one surface where a person is watching characters appear. */
const LIVE_MS = 50;

/** …and how often while somebody is reading back through history, where
    nothing moves unless output arrives. */
const READING_MS = 250;

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
  // How often to ask for the screen. **A ref and a self-scheduling wait
  // rather than an interval**, because the pace has to change without
  // this pane re-attaching: the attachment lives in the same effect, and
  // re-running it to change a number would drop the terminal and open it
  // again on every switch between windows.
  const hidden = useHidden();
  const pace = useRef(LIVE_MS);
  const pollNow = useRef<() => void>(() => {});
  // How many lines above the live screen this pane is looking, and how
  // much history there is to look at. Refs as well as state: the poll
  // loop reads them at every tick and must not be rebuilt to see a new
  // value (that would re-attach the terminal).
  const [back, setBackAt] = useState(0);
  const [freshBelow, setFreshBelow] = useState(false);
  const backRef = useRef(0);
  const depthRef = useRef(0);
  const setBack = useCallback((n: number) => {
    const at = Math.max(0, Math.min(n, depthRef.current));
    backRef.current = at;
    setBackAt(at);
    if (at === 0) setFreshBelow(false);
  }, []);

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
      termPoll(id, seqRef.current, backRef.current)
        .then((b) => {
          if (stopped) return;
          // **Holding the reader's place while output keeps arriving.**
          // The view is an offset from the bottom, so every line that
          // scrolls off moves the content under a fixed offset by one.
          // Growing history by the same amount puts it back.
          //
          // It holds until the buffer is full. After that `depth` stops
          // growing — lines fall off the top as fast as they arrive —
          // and a program still printing will slide the view under
          // somebody reading it. Anchoring properly needs the emulator
          // to name a line, and `vt100` has no such name.
          const grew = b.depth - depthRef.current;
          depthRef.current = b.depth;
          if (backRef.current > 0) {
            if (grew > 0) {
              setBack(b.back + grew);
              setFreshBelow(true);
            } else if (b.back !== backRef.current) {
              // Clamped: there was less history than was asked for, and
              // the answer is where it really landed.
              setBack(b.back);
            }
          }
          if (b.screen) {
            setScreen(b.screen);
            seqRef.current = b.seq;
          }
          if (b.gone) setGone(true);
        })
        .catch(() => {});
    };
    let timer = 0;
    const loop = () => {
      tick();
      if (!stopped) timer = window.setTimeout(loop, pace.current);
    };
    pollNow.current = () => {
      if (stopped) return;
      window.clearTimeout(timer);
      loop();
    };
    loop();

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
      window.clearTimeout(timer);
      pollNow.current = () => {};
      if (raf) window.cancelAnimationFrame(raf);
      observer.disconnect();
      termLeave(id).catch(() => {});
    };
  }, [id, fit]);

  // Twenty polls a second is what a terminal costs to look at; nobody
  // looking is nobody paying. The same rule the conversation follows: a
  // shorter pace takes effect at once (a screen ten seconds stale on
  // return is the wrong thing to come back to), a longer one waits out
  // the beat already in flight.
  //
  // **Nothing is lost by not asking.** What is polled here is a screen,
  // not a stream — the host keeps the terminal's whole state and a
  // single poll answers with the current one, so skipping a hundred
  // polls costs nothing but freshness.
  useEffect(() => {
    // Reading history is not watching output: the view up there only
    // changes when something arrives, and while scrolled back every
    // poll ships a whole grid (the answer depends on where the reader
    // is standing, so it cannot be skipped by sequence alone). Coming
    // back to the bottom is a *shorter* pace and takes effect at once,
    // by the same rule the conversation uses.
    const next = hidden ? HIDDEN_MS : back > 0 ? READING_MS : LIVE_MS;
    const shorter = next < pace.current;
    pace.current = next;
    if (shorter) pollNow.current();
  }, [hidden, back]);

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
    if (!inApp) return;
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

  // A wheel moves through history, in lines rather than pixels — the
  // grid has no pixels to scroll, it is repainted from wherever the
  // reader is standing.
  const onWheel = (e: React.WheelEvent) => {
    const lines = Math.round(e.deltaY / (cellRef.current.h || 16));
    if (lines === 0) return;
    setBack(backRef.current - lines);
  };

  // **A drag is the wheel, on a screen that has no wheel.** Without
  // this the scrollback exists on a phone and nothing can reach it:
  // `onWheel` is the only way back through history and a touch screen
  // never sends one. Same units and the same direction of travel —
  // pulling the screen down walks backwards, which is where the older
  // lines are.
  const dragFrom = useRef<number | null>(null);
  const onTouchStart = (e: React.TouchEvent) => {
    dragFrom.current = e.touches[0]?.clientY ?? null;
  };
  const onTouchMove = (e: React.TouchEvent) => {
    const y = e.touches[0]?.clientY;
    if (y === undefined || dragFrom.current === null) return;
    const lines = Math.round((y - dragFrom.current) / (cellRef.current.h || 16));
    if (lines === 0) return;
    // Carried forward rather than reset, so a slow drag accumulates
    // instead of rounding to nothing on every frame.
    dragFrom.current = y;
    setBack(backRef.current + lines);
  };

  /**
   * Where a phone's keys come from.
   *
   * **A `div` cannot raise a soft keyboard.** iOS and Android open one
   * for an input, a textarea or a contenteditable, and for nothing
   * else — so this pane, which listens for `keydown` on a focusable
   * `div`, was *read-only on every phone* while looking completely
   * normal in every desktop screenshot. A field is the only way in.
   *
   * It stays out of the way: no pointer events, one pixel, invisible.
   * Focused only on a touch, so a mouse still selects text on the grid
   * the way it always has and the desktop path is unchanged.
   */
  const keys = useRef<HTMLTextAreaElement>(null);
  const onPointerDown = (e: React.PointerEvent) => {
    if (e.pointerType !== "touch") return;
    // **The default action is what takes it back.** Focusing the field
    // here is not enough: this `div` carries `tabIndex={0}`, so the
    // browser's own handling of the press moves focus onto it a moment
    // later and the keyboard never opens. Measured that way — the
    // handler ran, `pointerType` really was `touch`, and the focused
    // element afterwards was the grid.
    //
    // Only on the touch branch, so a mouse still presses, focuses and
    // selects exactly as before.
    e.preventDefault();
    keys.current?.focus();
  };

  /**
   * What a soft keyboard actually sends.
   *
   * On a phone most keys arrive as `keydown` with `key: "Unidentified"`
   * — `keyBytes` answers `null`, nothing is preventDefault'd, and the
   * character lands in the field instead. That is the split this relies
   * on: anything `keyBytes` recognised never reaches here, and anything
   * it did not shows up as text. Autocorrect and word suggestions come
   * through the same door, which is why the whole value is sent rather
   * than one character.
   */
  const onInput = () => {
    const field = keys.current;
    if (!field?.value) return;
    const typed = field.value;
    field.value = "";
    setBack(0);
    termKey(id, typed).catch(() => {});
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    // **Shift+Page is this app's, plain Page is the program's.** Full
    // screen programs use PageUp for their own paging, and a terminal
    // that ate it would break `less` to add a feature. Shift is the
    // shared convention for "I mean the terminal, not what is in it".
    if (e.shiftKey && (e.key === "PageUp" || e.key === "PageDown")) {
      e.preventDefault();
      const page = Math.max(1, sentRef.current.rows - 1);
      setBack(backRef.current + (e.key === "PageUp" ? page : -page));
      return;
    }
    const bytes = keyBytes(e);
    if (bytes === null) return;
    e.preventDefault();
    // Typing puts the reader back at the live screen: the answer to
    // what they type appears there, and a terminal that left them in
    // the past would look like it had ignored them.
    setBack(0);
    termKey(id, bytes).catch(() => {});
  };

  // Paste goes to the PTY, not the page. Selection is the other half and
  // has its own note above `onCopy` — it needs less care than this, not
  // none.
  //
  // **Sent as a paste, not as keystrokes.** A program that asked for
  // bracketed paste gets the text marked as one lump; one that did not
  // gets exactly what it got before. Which of those it is belongs to the
  // screen the registry already keeps, so it is decided there — this
  // only says what kind of event this was (`term_paste`).
  // What a selection copies. **The grid is DOM text, so selecting and
  // copying already work** — the reason mandala had to intercept Cmd+C
  // was a canvas with nothing in it to select. What does not come free
  // is the shape of what lands on the clipboard: every cell is painted,
  // including the empty ones, so a line with five characters on it
  // copies as five characters and seventy-five spaces. Paste that into
  // anything and the padding comes too.
  //
  // Trailing blanks only, and only at the end of a line: what a terminal
  // pads with is on the right, and a space inside a line is somebody's
  // text. This is what every terminal does on copy, and doing it here
  // rather than at paste time keeps it out of the way of the program on
  // the other side.
  const onCopy = (e: React.ClipboardEvent) => {
    const picked = window.getSelection()?.toString() ?? "";
    if (!picked) return;
    const tidy = picked
      .split("\n")
      .map((line) => line.replace(/[ \t]+$/, ""))
      .join("\n");
    if (tidy === picked) return;
    e.preventDefault();
    e.clipboardData.setData("text/plain", tidy);
  };

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
      onWheel={onWheel}
      onCopy={onCopy}
      onPaste={onPaste}
      onPointerDown={onPointerDown}
      onTouchStart={onTouchStart}
      onTouchMove={onTouchMove}
      ref={boxRef}
      // `touch-none`: the browser's own idea of a drag here is to scroll
      // the page, and there is no page to scroll — the grid is repainted
      // from wherever the reader is standing, so the gesture has to
      // reach `onTouchMove` instead of being spent on the viewport.
      className="relative min-h-0 flex-1 touch-none overflow-hidden bg-[var(--term-bg)] p-1 font-mono text-sm leading-tight text-[var(--term-fg)] outline-none"
      style={
        {
          // The terminal owns its own two colours; they follow the theme
          // so a plain shell is the app's paper, not a black box.
          "--term-bg": "var(--background)",
          "--term-fg": "var(--foreground)",
        } as React.CSSProperties
      }
    >
      {/* The only thing on this screen a phone will open a keyboard for.
          `aria-hidden` because it is a mechanism, not a control: the
          grid beside it is what a reader is reading. */}
      <textarea
        ref={keys}
        data-term-keys
        aria-hidden
        tabIndex={-1}
        onInput={onInput}
        autoCapitalize="off"
        autoCorrect="off"
        autoComplete="off"
        spellCheck={false}
        className="pointer-events-none absolute h-px w-px resize-none border-0 p-0 opacity-0"
      />
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
      {/* The way back down, and the only sign that anything arrived
          while the reader was up here. Two sentences on one control:
          "something came in" and "you are simply not at the bottom" are
          different reasons to press it, and one word for both would
          make the arrival silent. It is the same pair the conversation
          wears, from the same two words. Absent at the bottom, which is
          what lets it go to zero. */}
      {back > 0 && (
        <button
          type="button"
          data-term-bottom
          data-fresh={freshBelow}
          onClick={() => setBack(0)}
          className="absolute bottom-2 left-1/2 z-10 -translate-x-1/2 rounded-full border bg-popover px-3 py-1 text-xs shadow-md"
        >
          {freshBelow ? gui.new_below : gui.to_bottom}
        </button>
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
