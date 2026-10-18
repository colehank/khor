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
import { countsGain } from "@/lib/term-gain";
import { scrollsNatively, SETTLE_MS } from "@/lib/term-scroller";
import { terminalAppearance, onTermTheme } from "@/lib/term-theme";

/** Output quiet this long, tmux has pushed what it will into the history. */
const QUIET_MS = 300;
/** A read waits for quiet at most this long: a program that never goes
    quiet (Claude Code's spinner) would otherwise put it off for good. */
const QUIET_MAX_MS = 1000;
/** The history xterm keeps what one read sends (`Tmux::history`'s cap). */
const HISTORY_LINES = 10_000;

/**
 * The bytes that put a read into the history xterm: the new history lines
 * above, the pane's screen as the screen, at the rows and columns it holds
 * on khor's client (under tmux's status line when that is at the top,
 * right of a split), so the seam at the bottom does not move. Written from
 * the screen's top left after clearing it (not into the scrollback), so
 * the history lines plus a full screen push exactly the history lines up
 * into the scrollback, however many rows each one wraps to.
 */
export function historyBytes(h: PaneHistory, rows: number): string {
  const indent = h.left > 0 ? `\x1b[${h.left}C` : "";
  const above = Math.min(h.top, rows);
  const below = Math.max(0, rows - above - h.screen.length);
  return (
    "\x1b[?25l\x1b[0m\x1b[H\x1b[J" +
    h.lead +
    h.lines.map((l) => `${indent}${l}\r\n`).join("") +
    "\x1b[0m" +
    "\r\n".repeat(above) +
    h.screen.map((l) => indent + l).join("\r\n") +
    "\r\n".repeat(below) +
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
 *   `QUIET_MS` (`QUIET_MAX_MS` at most). Never on every write: a pane
 *   streaming at the bottom costs one flag.
 *
 * **The copy is tmux's history, row for row** (item 33). tmux moves rows
 * between the history and the screen when the pane is resized; the read
 * after a resize says which (`Tmux::history` places the copy by count).
 * So the history xterm must not move any itself: its screen is cleared
 * before it is resized, and xterm neither pushes rows into its scrollback
 * nor pulls them back. The first read waits for the attach (`joined`),
 * which sizes the pane for this face: read before it, at another face's
 * size, the pane's newest rows went back onto the screen at once, and
 * the copy had to be redone whole.
 */
export function holdsTmuxHistory(o: {
  /** The row is a tmux session's. An agent's row that sits in a tmux pane
      (`SessionRow.mux`) may be shown through khor's own host instead (a
      conversation khor started, resumed in tmux too): until a read
      succeeds there, a failed one only means there is no tmux history
      here, and nothing is said. */
  known: boolean;
  /** **A shell row** (item 58): tmux's only while a tmux client the person
      typed is in front, which the face sees as the alternate screen. Read
      when the screen switches to it; given back the moment it switches
      away; a failed read (vim, htop: not tmux) says nothing and waits for
      the next switch. */
  probe?: boolean;
  /** Desktop reads inner-pane capabilities, while tmux keeps its history. */
  observeOnly?: boolean;
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
  /** The person went up into the history, or came back to the bottom. */
  moved: (up: boolean) => void;
  /** Lines arrived below the person up in the history (item 52): a read
      brought the pane's history or screen further, not a redraw. */
  gained: () => void;
}): {
  active: () => boolean;
  /** The history xterm while the person is up in it: what a long press
      selects in. */
  over: () => Terminal | null;
  /** Back to the bottom (the way-back-down pill). */
  toBottom: () => void;
  shown: (on: boolean) => void;
  /** The attach has sized the pane for this face: reads may begin. */
  joined: () => void;
  /** Resolves once the first read since the attach has answered, or
      failed: whose the scroller is — the history's or the program's
      wheel — is decided from then on (item 51). */
  decided: () => Promise<void>;
  /** The pane in front, as the newest read found it (`null` before one),
      with tmux's window size then (item 45). */
  pane: () => { pane: string; mouse: boolean; alternate: boolean; window: string } | null;
  /** Reads now, whether or not the history is behind: for the notes'
      look at the window size after a touch (item 45). */
  reread: () => Promise<void>;
  /** A gesture is beginning: refresh stale inner-pane capabilities. */
  approached: () => void;
  stop: () => void;
} {
  const { live, seat } = o;
  let history: Terminal | null = null;
  const recolor = () => {
    if (!history) return;
    history.options = terminalAppearance(seat);
    seat.style.background = history.options.theme?.background ?? "";
  };
  const stopTheme = onTermTheme(recolor);
  let native: { sync: () => void; stop: () => void } | null = null;
  let anchor: PaneAnchor | null = null;
  let on = false;
  let visible = false;
  let stopped = false;
  let reading = false;
  let again = false;
  /** The attach has sized the pane (`joined`). */
  let attached = false;
  /** Output since the last read: the history may be behind. */
  let behind = false;
  /** The last read failed (an older khor there, out of reach). */
  let failed = false;
  /** What the newest read said of the pane in front. */
  let held: { pane: string; mouse: boolean; alternate: boolean; window: string } | null = null;
  /** A read has succeeded: this terminal is tmux's. */
  let proven = o.known;
  /** Not a tmux terminal after all: no more reads. */
  let absent = false;
  let quiet = 0;
  /** **The scroller's owner is decided before the screen is shown** (item
      51; the owner, a cold start: a drag did nothing). The first read
      lands a round trip after the attach's quiet, and gave the history
      the scroller then — its range and place rewritten under a finger
      that had begun a drag on the program's wheel, which stops iOS's
      pan dead. The pane keeps its opening cover until this resolves. */
  let decide = () => {};
  const firstAnswer = new Promise<void>((done) => (decide = done));
  /** When the read now waiting for quiet was first asked for. */
  let waitingSince = 0;
  /** Whether the person is up in the history. */
  const up = () => {
    const b = history?.buffer.active;
    return on && !!b && b.viewportY < b.baseY;
  };
  let wasUp = false;
  /** The history xterm's extent since the person went up (item 52). */
  let gain: ReturnType<typeof countsGain> | null = null;
  const show = () => {
    const now = up();
    seat.style.visibility = now ? "visible" : "hidden";
    if (now !== wasUp) {
      wasUp = now;
      // A copy still catching up (a read under way, or output since the
      // last) is counted from that read: what it brings was there before
      // the person went up.
      if (now && (behind || reading)) gain?.defer();
      else if (now) gain?.mark();
      o.moved(now);
    }
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
      ...terminalAppearance(seat),
      allowProposedApi: true,
      disableStdin: true,
      cursorBlink: false,
    });
    t.open(seat);
    // For the gates, as the live one is.
    (seat as HTMLElement & { __xterm?: Terminal }).__xterm = t;
    t.onScroll(show);
    gain = countsGain(t);
    native = scrollsNatively(t, o.scroller, o.room, seat, { yields: () => !on });
    return t;
  };
  /** Off the screen: the history and its lines go. */
  const drop = () => {
    // Off the screen there is nothing to wait for.
    decide();
    window.clearTimeout(quiet);
    waitingSince = 0;
    give(false);
    native?.stop();
    native = null;
    gain?.stop();
    gain = null;
    history?.dispose();
    history = null;
    delete (seat as HTMLElement & { __xterm?: Terminal }).__xterm;
    anchor = null;
    seat.style.visibility = "hidden";
  };
  const apply = (h: PaneHistory) => {
    failed = false;
    held = { pane: h.pane, mouse: h.mouse, alternate: h.alternate, window: h.window };
    if (o.observeOnly) {
      anchor = h.anchor;
      give(false);
      return;
    }
    // An alternate buffer has no ordinary history. Its scroll target is
    // resolved separately: mouse when requested, standard page keys when
    // not. Alternate alone never promises mouse support (term-scroll-target).
    give(!(h.alternate || h.mouse));
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
    for (const k of ["fontFamily", "fontSize", "lineHeight", "letterSpacing"] as const) {
      if (history.options[k] !== live.options[k]) (history.options as Record<string, unknown>)[k] = live.options[k];
    }
    // Opaque over the live screen, in the live theme's ground.
    recolor();
    // A refill under a person up in the history keeps them as far from
    // the bottom as they were: it is mostly the same lines again.
    const b = history.buffer.active;
    const fromBottom = h.refill && up() ? b.baseY - b.viewportY : 0;
    // A refill is the copy again, mostly the same lines: counted from
    // after it, not as arrivals.
    if (h.refill) gain?.defer();
    if (h.refill) history.reset();
    const t = history;
    t.write(historyBytes(h, t.rows), () => {
      if (fromBottom > 0) t.scrollToLine(Math.max(0, t.buffer.active.baseY - fromBottom));
      native?.sync();
      show();
      if (up() && gain?.grew()) o.gained();
    });
  };
  /** Resolves false when the history could not be read. */
  const read = (): Promise<boolean> => {
    window.clearTimeout(quiet);
    waitingSince = 0;
    if (stopped || !visible || absent) return Promise.resolve(true);
    // A shell on its own screen has no tmux in front: nothing to read,
    // and nothing for the cover to wait for.
    if (o.probe && live.buffer.active.type !== "alternate") {
      decide();
      return Promise.resolve(true);
    }
    if (!attached) {
      behind = true;
      return Promise.resolve(true);
    }
    if (reading) {
      again = true;
      return Promise.resolve(true);
    }
    reading = true;
    behind = false;
    return o
      .read(anchor)
      .then((h) => {
        proven = true;
        if (!stopped && visible) apply(h);
        return true;
      })
      // An older khor there, or out of reach: the drag stays tmux's
      // wheel, as before, and a finger is told why. An agent's row never
      // read yet is not shown through tmux at all: nothing to tell.
      .catch(() => {
        give(false);
        show();
        if (!proven) {
          absent = true;
          return true;
        }
        failed = true;
        return false;
      })
      .finally(() => {
        reading = false;
        decide();
        if (again) {
          again = false;
          void read();
        }
      });
  };
  const quietThen = () => {
    window.clearTimeout(quiet);
    const now = performance.now();
    waitingSince ||= now;
    quiet = window.setTimeout(() => void read(), Math.min(QUIET_MS, Math.max(0, waitingSince + QUIET_MAX_MS - now)));
  };
  const wrote = live.onWriteParsed(() => {
    behind = true;
    if (up()) quietThen();
  });
  // **A shell's tmux comes and goes with its screen** (item 58): in, a
  // read asks whether it is tmux; out, the scroller is the shell's again
  // at once, and the copy of a history that is no longer there goes.
  const switched = o.probe
    ? live.buffer.onBufferChange((b) => {
        absent = false;
        proven = false;
        anchor = null;
        if (b.type === "alternate") {
          behind = true;
          if (visible && attached) quietThen();
        } else {
          drop();
          show();
        }
      })
    : null;
  // Another pane or window in front: tmux retitles khor's client with the
  // pane's id (`Tmux::grouped_attach_argv`), and the next read is that
  // pane's, whole.
  const titled = live.onTitleChange(quietThen);
  // The copy moves no rows itself (above): its screen goes first, so the
  // resize neither pushes rows into its scrollback nor pulls them back,
  // and the read after it brings tmux's own move.
  const resized = live.onResize(() => {
    behind = true;
    const t = history;
    t?.write("\x1b[H\x1b[J", () => {
      if (history !== t) return;
      t.resize(live.cols, live.rows);
      // The screen was cleared for the resize: the next read puts it back.
      gain?.defer();
      if (up()) quietThen();
    });
  });
  // A finger landing: a history that is behind is brought up to date
  // before the drag reaches it (the scroller holds its place meanwhile,
  // `lib/term-scroller`'s gesture rule).
  let approachedAt = -Infinity;
  const landed = () => {
    approachedAt = performance.now();
    const already = failed;
    if (already) o.outOfReach();
    if (already || behind) void read().then((ok) => ok || already || o.outOfReach());
  };
  // A native fling emits many wheels, including synthetic notches on a
  // phone. Output marks us behind between them, but capabilities need one
  // refresh at gesture entry, not capture-pane after every rendered frame.
  const approached = () => {
    const now = performance.now();
    const fresh = now - approachedAt > SETTLE_MS;
    approachedAt = now;
    if (fresh) landed();
  };
  o.scroller.addEventListener("touchstart", landed, { passive: true, capture: true });
  o.scroller.addEventListener("wheel", approached, { passive: true, capture: true });
  return {
    active: () => on,
    over: () => (up() ? history : null),
    toBottom: () => history?.scrollToBottom(),
    shown: (to) => {
      if (to === visible) return;
      visible = to;
      if (to) void read();
      else drop();
    },
    joined: () => {
      attached = true;
      // A shell joined at its prompt: the scroller is its own, decided.
      if (o.probe && live.buffer.active.type !== "alternate") decide();
      if (visible) quietThen();
    },
    decided: () => firstAnswer,
    pane: () => held,
    reread: () => read().then(() => {}),
    approached,
    stop: () => {
      stopped = true;
      stopTheme();
      drop();
      wrote.dispose();
      switched?.dispose();
      titled.dispose();
      resized.dispose();
      o.scroller.removeEventListener("touchstart", landed, true);
      o.scroller.removeEventListener("wheel", approached, true);
    },
  };
}
