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
 * the wheel step aside while it is); `shown` follows the pane on and off
 * the screen; `stop` puts everything back.
 *
 * **What it costs, and when** (the lead's bounds, item 31):
 * - the history xterm exists only while the pane is on screen — off it,
 *   it is disposed with its 10 000 lines, and read whole again on return;
 * - it draws with xterm's DOM renderer (no second WebGL context) in the
 *   live xterm's font, size and theme (`lib/term-font` is the one source;
 *   the live xterm carries it);
 * - it is read when the pane comes on screen, when another pane comes to
 *   the front, when a finger lands on a history that is behind, and —
 *   only while the person is up in it — after output has been quiet
 *   `QUIET_MS`. Never on every write: a pane streaming at the bottom
 *   costs one flag.
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
  /** A finger landed and the history cannot be read: say so. */
  outOfReach: () => void;
}): { active: () => boolean; shown: (on: boolean) => void; stop: () => void } {
  const { live, seat } = o;
  let history: Terminal | null = null;
  let native: { sync: () => void; stop: () => void } | null = null;
  let anchor: PaneAnchor | null = null;
  let on = false;
  let visible = false;
  let stopped = false;
  let reading = false;
  let again = false;
  /** Output since the last read: the history may be behind. */
  let behind = false;
  /** The last read failed (an older khor there, out of reach). */
  let failed = false;
  let quiet = 0;
  /** Whether the person is up in the history. */
  const up = () => {
    const b = history?.buffer.active;
    return on && !!b && b.viewportY < b.baseY;
  };
  const show = () => {
    seat.style.visibility = up() ? "visible" : "hidden";
  };
  const give = (to: boolean) => {
    if (on === to) return;
    on = to;
    o.changed();
  };
  const make = () => {
    const t = new Terminal({
      cols: live.cols,
      rows: live.rows,
      scrollback: HISTORY_LINES,
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
    t.open(seat);
    // For the gates, as the live one is.
    (seat as HTMLElement & { __xterm?: Terminal }).__xterm = t;
    t.onScroll(show);
    native = scrollsNatively(t, o.scroller, o.room, seat, { yields: () => !on });
    return t;
  };
  /** Off the screen: the history and its lines go. */
  const drop = () => {
    window.clearTimeout(quiet);
    give(false);
    native?.stop();
    native = null;
    history?.dispose();
    history = null;
    delete (seat as HTMLElement & { __xterm?: Terminal }).__xterm;
    anchor = null;
    seat.style.visibility = "hidden";
  };
  const apply = (h: PaneHistory) => {
    failed = false;
    give(!h.alternate);
    if (!on) {
      // Nothing of this read was written: the anchor stays where the
      // copy ends, and the read after the program leaves catches up.
      show();
      return;
    }
    anchor = h.anchor;
    history ??= make();
    // The live xterm's look, as it is now (a theme, a font or a size
    // changed since).
    for (const k of ["theme", "fontFamily", "fontSize", "lineHeight", "letterSpacing"] as const) {
      if (history.options[k] !== live.options[k]) (history.options as Record<string, unknown>)[k] = live.options[k];
    }
    // Opaque over the live screen, in the live theme's ground.
    seat.style.background = live.options.theme?.background ?? "";
    if (h.refill) history.reset();
    history.write(historyBytes(h, history.rows), () => {
      native?.sync();
      show();
    });
  };
  /** Resolves false when the history could not be read. */
  const read = (): Promise<boolean> => {
    window.clearTimeout(quiet);
    if (stopped || !visible) return Promise.resolve(true);
    if (reading) {
      again = true;
      return Promise.resolve(true);
    }
    reading = true;
    behind = false;
    return o
      .read(anchor)
      .then((h) => {
        if (!stopped && visible) apply(h);
        return true;
      })
      // An older khor there, or out of reach: the drag stays tmux's
      // wheel, as before, and a finger is told why.
      .catch(() => {
        failed = true;
        give(false);
        show();
        return false;
      })
      .finally(() => {
        reading = false;
        if (again) {
          again = false;
          void read();
        }
      });
  };
  const quietThen = () => {
    window.clearTimeout(quiet);
    quiet = window.setTimeout(() => void read(), QUIET_MS);
  };
  const wrote = live.onWriteParsed(() => {
    behind = true;
    if (up()) quietThen();
  });
  // Another pane or window in front: tmux retitles khor's client with the
  // pane's id (`Tmux::grouped_attach_argv`), and the next read is that
  // pane's, whole.
  const titled = live.onTitleChange(quietThen);
  const resized = live.onResize(({ cols, rows }) => {
    history?.resize(cols, rows);
    behind = true;
  });
  // A finger landing: a history that is behind is brought up to date
  // before the drag reaches it (the scroller holds its place meanwhile,
  // `lib/term-scroller`'s gesture rule).
  const landed = () => {
    const already = failed;
    if (already) o.outOfReach();
    if (already || behind) void read().then((ok) => ok || already || o.outOfReach());
  };
  o.scroller.addEventListener("touchstart", landed, { passive: true, capture: true });
  return {
    active: () => on,
    shown: (to) => {
      if (to === visible) return;
      visible = to;
      if (to) void read();
      else drop();
    },
    stop: () => {
      stopped = true;
      drop();
      wrote.dispose();
      titled.dispose();
      resized.dispose();
      o.scroller.removeEventListener("touchstart", landed, true);
    },
  };
}
