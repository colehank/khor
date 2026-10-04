// **A tmux pane's history, scrolled on the phone** (item 31;
// `audit/term-tmux-history.md`).
//
// khor's tmux client draws on the outer terminal's alternate screen, which
// keeps no history, and even off it tmux redraws rather than scrolls when
// the outer terminal drains slowly — so the face's own xterm never holds
// the pane's past. Under tmux's mouse a drag was the wheel, tmux's copy
// mode, a round trip a line (the owner: 「不跟手」). Here, on a phone, the
// history comes from tmux itself (`Tmux::history`: `capture-pane -e -J`)
// into a second xterm that only shows it, and the page's own scroller
// scrolls that one (`lib/term-scroller`) the way a plain shell's history
// scrolls. iTerm2's tmux integration keeps a pane's history locally the
// same way, from `capture-pane`. The live xterm is left exactly as it was:
// at the bottom it is the one on screen; the Mac is not touched.
import { Terminal } from "@xterm/xterm";
import type { PaneAnchor } from "@/gen/bindings/PaneAnchor";
import type { PaneHistory } from "@/gen/bindings/PaneHistory";
import { scrollsNatively } from "@/lib/term-scroller";

/** Output quiet this long, tmux has pushed what it will into the history. */
const QUIET_MS = 300;
/** While output streams, the history is read at least this often. */
const STREAMING_MS = 1000;
/** The history xterm keeps what one read sends (`Tmux::history`'s cap). */
const HISTORY_LINES = 10_000;

/**
 * The bytes that put a read into the history xterm: the new history lines
 * above, the pane's screen as the screen. Written from the screen's top
 * left after clearing it (not into the scrollback), so the history lines
 * plus a full screen push exactly the history lines up into the
 * scrollback, however many rows each one wraps to. The rows below a
 * pane shorter than the screen (tmux's status line) stay blank.
 */
export function historyBytes(h: PaneHistory, rows: number): string {
  const pad = Math.max(0, rows - h.screen.length);
  return (
    "\x1b[?25l\x1b[0m\x1b[H\x1b[J" +
    h.lead +
    h.lines.map((l) => `${l}\r\n`).join("") +
    "\x1b[0m" +
    h.screen.join("\r\n") +
    "\r\n".repeat(pad) +
    "\x1b[0m"
  );
}

/**
 * Holds a tmux row's history on a phone. `active` says whether it is in
 * charge of the scroller (the live xterm's own binding and its drag as
 * the wheel step aside while it is); `stop` puts everything back.
 */
export function holdsTmuxHistory(o: {
  live: Terminal;
  /** Where the history xterm is drawn, over the live screen. */
  seat: HTMLElement;
  scroller: HTMLElement;
  room: HTMLElement;
  read: (since: PaneAnchor | null) => Promise<PaneHistory>;
  /** The history took the scroller, or gave it back. */
  changed: () => void;
}): { active: () => boolean; stop: () => void } {
  const { live, seat } = o;
  let history: Terminal | null = null;
  let native: { sync: () => void; stop: () => void } | null = null;
  let anchor: PaneAnchor | null = null;
  let on = false;
  let stopped = false;
  let reading = false;
  let again = false;
  let quiet = 0;
  let streaming = 0;
  /** Whether the person is up in the history: then it is on screen. */
  const show = () => {
    const b = history?.buffer.active;
    const up = on && !!b && b.viewportY < b.baseY;
    seat.style.visibility = up ? "visible" : "hidden";
  };
  const make = () => {
    const t = new Terminal({
      cols: live.cols,
      rows: live.rows,
      scrollback: HISTORY_LINES + live.rows,
      fontFamily: live.options.fontFamily,
      fontSize: live.options.fontSize,
      fontWeight: live.options.fontWeight,
      fontWeightBold: live.options.fontWeightBold,
      lineHeight: live.options.lineHeight,
      letterSpacing: live.options.letterSpacing,
      theme: live.options.theme,
      allowProposedApi: true,
      disableStdin: true,
      cursorBlink: false,
    });
    // Opaque over the live screen, in the live theme's ground.
    seat.style.background = live.options.theme?.background ?? "";
    t.open(seat);
    // For the gates, as the live one is.
    (seat as HTMLElement & { __xterm?: Terminal }).__xterm = t;
    t.onScroll(show);
    native = scrollsNatively(t, o.scroller, o.room, seat, { yields: () => !on });
    return t;
  };
  const apply = (h: PaneHistory) => {
    const was = on;
    on = !h.alternate;
    if (on !== was) o.changed();
    if (!on) {
      // Nothing of this read was written: the anchor stays where the
      // copy ends, and the read after the program leaves catches up.
      show();
      return;
    }
    anchor = h.anchor;
    history ??= make();
    // The live xterm's look, as it is now (a theme or a size changed).
    for (const k of ["theme", "fontFamily", "fontSize", "lineHeight", "letterSpacing"] as const) {
      if (history.options[k] !== live.options[k]) (history.options as Record<string, unknown>)[k] = live.options[k];
    }
    seat.style.background = live.options.theme?.background ?? "";
    if (h.refill) history.reset();
    history.write(historyBytes(h, history.rows), () => {
      native?.sync();
      show();
    });
  };
  const read = () => {
    window.clearTimeout(quiet);
    window.clearTimeout(streaming);
    streaming = 0;
    if (stopped) return;
    if (reading) {
      again = true;
      return;
    }
    reading = true;
    o.read(anchor)
      .then((h) => {
        if (!stopped) apply(h);
      })
      // Not here (a row on another machine, older than this): the drag
      // stays tmux's wheel, as before.
      .catch(() => {
        if (on) {
          on = false;
          o.changed();
        }
        show();
      })
      .finally(() => {
        reading = false;
        if (again) {
          again = false;
          read();
        }
      });
  };
  const soon = () => {
    window.clearTimeout(quiet);
    quiet = window.setTimeout(read, QUIET_MS);
    if (!streaming) streaming = window.setTimeout(read, STREAMING_MS);
  };
  const wrote = live.onWriteParsed(soon);
  // Another pane or window in front: tmux retitles khor's client with the
  // pane's id (`Tmux::grouped_attach_argv`), and the next read is that
  // pane's, whole.
  const titled = live.onTitleChange(() => soon());
  const resized = live.onResize(({ cols, rows }) => {
    history?.resize(cols, rows);
    soon();
  });
  read();
  return {
    active: () => on,
    stop: () => {
      stopped = true;
      if (on) {
        on = false;
        o.changed();
      }
      window.clearTimeout(quiet);
      window.clearTimeout(streaming);
      wrote.dispose();
      titled.dispose();
      resized.dispose();
      native?.stop();
      history?.dispose();
      seat.style.visibility = "hidden";
    },
  };
}
