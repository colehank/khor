// **The terminal's history on the page's own scroller** (item 15 B; items
// 29 and 30). Moved here from `views/TerminalPane` so a gate can drive it
// against a real xterm without the rest of the pane.
import type { Terminal } from "@xterm/xterm";

/**
 * Whether the program, not the history, gets the scroll: it took the
 * mouse (khor turns tmux's on for its own grouped session,
 * `Tmux::grouped_attach_argv`), or it is on the alternate screen, which
 * keeps no history. This only selects the program-owned travel range;
 * `term-scroll-target` separately decides mouse reports or standard page
 * keys from the inner program’s capabilities.
 */
export function programScrolls(term: Terminal): boolean {
  return term.modes.mouseTrackingMode !== "none" || term.buffer.active.type === "alternate";
}

/** Quiet this long after the last scroll event, a scroll has stopped. */
export const SETTLE_MS = 140;
/** Under a finger, while the program scrolls itself, the scroller's range:
    room for any fling, the place set back to the middle once it rests. */
export const WHEEL_ROOM = 40_000;

/**
 * **The history scrolls the way a page does** (2026-10-03, item 15 B; the
 * owner: 「滚动很卡顿，范围也很小，没有那种浏览网页的拖动感」).
 *
 * xterm 6 replaced its native scroller with VS Code's drawn one
 * (`SmoothScrollableElement`), which answers the wheel by itself and a
 * finger not at all, so the finger scroll this pane had was synthetic:
 * a line per row of travel and an imitated fling. Here the scrolling is
 * the engine's again, the way xterm 5's `.xterm-viewport` and hterm's
 * `ScrollPort` (Blink Shell's renderer) do it: `scroller` scrolls
 * natively (momentum, the edge's bounce, the trackpad's own curve), and
 * `room`, after the terminal in it, is sized to the history above the
 * screen — so its range is the buffer's. The terminal itself is
 * `position: sticky` at the scroller's top, so every touch and click
 * still lands on xterm (selection, taps, links), and a drag or a wheel
 * that xterm does not take scrolls the scroller around it.
 *
 * - **scroller → xterm**: the line under the scroller's top is
 *   `scrollTop / row`; xterm is moved there (`scrollToLine`), and the
 *   screen is shifted by the part of a row left over, so the content
 *   follows the finger by the pixel, not by the line. A scroll that
 *   stops between two rows is carried to the nearer one.
 * - **xterm → scroller**: output, a key that scrolls, a resize — any
 *   change of xterm's line — sets `scrollTop` once; the scroll event
 *   that answers it is recognised and ignored, so the two never chase.
 * - **The wheel**: a wheel the scroller takes never reaches xterm's own
 *   scroller (stopped on the way down), so a Mac's wheel and trackpad are
 *   the system's too. When the program scrolls (`programScrolls`) the
 *   scroller has no range and the wheel goes on to xterm, as before.
 *
 * VS Code's terminal keeps xterm's drawn scroller (it has no touch
 * case); Hyper keeps xterm's too. Termius and Blink draw natively.
 */
export function scrollsNatively(
  term: Terminal,
  scroller: HTMLElement,
  room: HTMLElement,
  screen: HTMLElement,
  opts: {
    /** Another binding has the scroller for now (a tmux pane's history,
        `lib/term-tmux-history`): this one does nothing until it is back. */
    yields?: () => boolean;
    /** **Under a finger, the program's wheel by the scroller's own
        travel** (item 36). Where the program scrolls itself (it holds the
        mouse, or is on its alternate screen — Claude Code in tmux), the
        scroller keeps a range of its own and scrolls natively, momentum
        and all, and every row it travels is one notch of the wheel,
        handed here (positive: later lines). Without it the scroller is
        inert there, as on a Mac, whose wheel reaches xterm itself. */
    wheel?: (rows: number) => void;
  } = {},
): { sync: () => void; stop: () => void } {
  const yields = opts.yields ?? (() => false);
  let row = 0;
  let kicked = false;
  // The `scrollTop` this code set last, until its scroll event comes back.
  let echo: number | null = null;
  let touching = 0;
  let settle = 0;
  let frame = 0;
  /** When the person last moved the scroller — a finger, its momentum, a
      wheel (ms). */
  let personAt = -Infinity;
  /** A placing put off while a gesture held the scroller (item 30). */
  let held = false;
  /** The scroller's size and range as the last placing left them. */
  let geom = "";
  /** Lines xterm trimmed off the history's top since the range was last
      set: a finger's place in the scroller counts from before them. */
  let trimmed = 0;
  /** Wheel mode: the `scrollTop` the last notch was counted from. */
  let notched: number | null = null;
  const wheeling = () => !!opts.wheel && programScrolls(term);
  const shape = () => `${scroller.clientHeight}:${scroller.scrollHeight}`;
  /** **A gesture holds the scroller** (item 30): a finger is down, or its
      momentum (or a wheel) is still moving it. */
  const gesture = () => touching > 0 || performance.now() - personAt < SETTLE_MS;
  const trims = (term as unknown as { _core?: { buffers?: { normal?: { lines?: { onTrim?: (f: (n: number) => void) => { dispose(): void } } } } } })._core?.buffers?.normal?.lines?.onTrim?.((n) => {
    trimmed += n;
  });
  const measure = () => {
    const el = term.element?.querySelector(".xterm-screen");
    const h = el ? el.getBoundingClientRect().height / term.rows : 0;
    if (h > 0) row = h;
    return row;
  };
  const shift = (px: number) => {
    const v = px === 0 ? "" : `translateY(${px}px)`;
    if (screen.style.transform !== v) screen.style.transform = v;
  };
  /** xterm's own line → the scroller's range and place. */
  const sync = () => {
    frame = 0;
    if (yields() || !measure()) return;
    const b = term.buffer.active;
    const inert = programScrolls(term);
    if (inert && opts.wheel) {
      // The range is the wheel's own; the place goes back to the middle
      // only while nothing moves it, so a fling never meets an end.
      const height = `${WHEEL_ROOM}px`;
      if (room.style.height !== height) {
        room.style.height = height;
        // WebKit's stale range on a layer made with nothing to scroll
        // (item 18 A): read it now, as below.
        if (!kicked) {
          kicked = true;
          scroller.style.overflowY = "hidden";
          void scroller.offsetHeight;
          scroller.style.overflowY = "";
        }
      }
      const mid = WHEEL_ROOM / 2;
      if (!gesture() && (notched === null || Math.abs(scroller.scrollTop - mid) > WHEEL_ROOM / 4)) {
        echo = mid;
        scroller.scrollTop = mid;
        notched = mid;
      }
      shift(0);
      geom = shape();
      return;
    }
    notched = null;
    // **Output under a finger goes into the buffer only** (item 30; the
    // owner: Claude Code's screen shook while he scrolled up). Ink pushes
    // whole frames into the history on every redraw when its frame is
    // taller than the screen, so the range grew by tens of lines a tick
    // and each placing answered the finger's last move: two scrollers
    // chasing. hterm's ScrollPort and iTerm2 hold the view under a
    // gesture the same way. The placing waits for the settle (`rest`).
    if (gesture() && !inert) {
      held = true;
      return;
    }
    held = false;
    const height = `${(inert ? 0 : b.baseY) * row}px`;
    if (room.style.height !== height) {
      const first = !kicked && !inert && b.baseY > 0;
      room.style.height = height;
      // **WebKit on iOS, the first time there is anything to scroll**
      // (item 18 A, hypothesis 3): a scroller that had nothing to scroll
      // when its layer was made can keep that answer until something
      // relays it out — the keyboard's resize was that something. Drop
      // and restore its overflow once, so the range is read now.
      if (first) {
        kicked = true;
        scroller.style.overflowY = "hidden";
        void scroller.offsetHeight;
        scroller.style.overflowY = "";
      }
    }
    trimmed = 0;
    const line = inert ? 0 : b.viewportY;
    if (Math.round(scroller.scrollTop / row) !== line || inert) {
      const top = line * row;
      if (Math.abs(scroller.scrollTop - top) >= 0.5) {
        echo = top;
        scroller.scrollTop = top;
      }
      shift(0);
    }
    geom = shape();
  };
  const syncSoon = () => {
    if (!frame) frame = window.requestAnimationFrame(sync);
  };
  /** The scroller's place → xterm's line. */
  const scrolled = () => {
    if (yields()) return;
    const top = scroller.scrollTop;
    if (echo !== null && Math.abs(top - echo) < 1) {
      echo = null;
      geom = shape();
      if (wheeling()) notched = top;
      return;
    }
    echo = null;
    if (wheeling() && row) {
      // A resize moved it, not a finger: count from here. Under a
      // finger too — the deck folding mid-drag (item 50) grows the box
      // while the finger moves, and the engine's shift was handed to the
      // program as a notch the other way.
      if (shape() !== geom) {
        geom = shape();
        notched = top;
        return;
      }
      personAt = performance.now();
      if (notched === null) notched = top;
      const rows = Math.trunc((top - notched) / row);
      if (rows !== 0) {
        notched += rows * row;
        opts.wheel?.(rows);
      }
      window.clearTimeout(settle);
      settle = window.setTimeout(rest, SETTLE_MS);
      return;
    }
    if (programScrolls(term) || !row) return;
    // **A resize is not a scroll** (item 29; the owner: the keyboard came
    // up and the line being typed was not in view). When the box shrinks
    // the engine clamps `scrollTop` and says so with a scroll event; taken
    // as the person's, it moved xterm up. With no gesture going and the
    // scroller's size or range changed since the last placing, the scroll
    // is the engine's: xterm keeps its line — at the bottom stays at the
    // bottom, as iTerm2 and VS Code keep it — and the scroller is placed
    // from it again.
    if (!gesture() && shape() !== geom) {
      geom = shape();
      syncSoon();
      return;
    }
    personAt = performance.now();
    const b = term.buffer.active;
    const raw = Math.round(top / row);
    const line = Math.max(0, Math.min(b.baseY, raw - trimmed));
    if (line !== b.viewportY) {
      term.scrollToLine(line);
      // xterm's own scroller can hold a stale range — it does not
      // re-measure when its renderer's size changes without the buffer's
      // (xterm.js #6117, #6172) — and then refuses the line; bring it up
      // to date and ask again.
      if (term.buffer.active.viewportY !== line) {
        (term as unknown as { _core?: { _viewport?: { _sync?: () => void } } })._core?._viewport?._sync?.();
        term.scrollToLine(line);
      }
    }
    // Between two rows: the screen follows by the pixel. Not past either
    // end, where the engine's own bounce moves everything already.
    const max = (b.baseY + trimmed) * row;
    shift(top < 0 || top > max ? 0 : Math.max(-row, Math.min(row, raw * row - top)));
    window.clearTimeout(settle);
    settle = window.setTimeout(rest, SETTLE_MS);
  };
  /** A scroll that stopped between two rows goes to the nearer one. */
  const rest = () => {
    if (touching > 0 || yields()) return;
    if (wheeling()) {
      personAt = -Infinity;
      sync();
      return;
    }
    // The gesture is over: the range and the place set once, from xterm's
    // line (which followed the finger, trims and all).
    if (held) {
      personAt = -Infinity;
      sync();
      return;
    }
    const top = scroller.scrollTop;
    const to = Math.round(top / row) * row;
    if (Math.abs(top - to) < 0.5) return shift(0);
    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    scroller.scrollTo({ top: to, behavior: still ? "auto" : "smooth" });
  };
  const down = (e: TouchEvent) => {
    touching = e.touches.length;
    window.clearTimeout(settle);
  };
  const up = (e: TouchEvent) => {
    touching = e.touches.length;
    window.clearTimeout(settle);
    settle = window.setTimeout(rest, SETTLE_MS);
  };
  // Capture, on the way down: a wheel the scroller will take must not
  // reach xterm's scroller, which would take it first and cancel it.
  const wheel = (e: WheelEvent) => {
    if (yields() || programScrolls(term)) return;
    e.stopPropagation();
    personAt = performance.now();
  };
  scroller.addEventListener("scroll", scrolled, { passive: true });
  scroller.addEventListener("wheel", wheel, { capture: true, passive: true });
  scroller.addEventListener("touchstart", down, { passive: true });
  // The lift is heard on the window, first: a long press that became a
  // selection (`pressSelects`) stops its touchend on the way down, and a
  // finger the scroller never heard leave held the mapping frozen.
  window.addEventListener("touchend", up, { capture: true, passive: true });
  window.addEventListener("touchcancel", up, { capture: true, passive: true });
  const moved = term.onScroll(syncSoon);
  const wrote = term.onWriteParsed(syncSoon);
  const sized = term.onResize(syncSoon);
  const swapped = term.buffer.onBufferChange(syncSoon);
  const rendered = term.onRender(() => {
    // The first frame drawn knows the row's height.
    if (!row) syncSoon();
  });
  return {
    sync,
    stop: () => {
      window.cancelAnimationFrame(frame);
      window.clearTimeout(settle);
      scroller.removeEventListener("scroll", scrolled);
      scroller.removeEventListener("wheel", wheel, true);
      scroller.removeEventListener("touchstart", down);
      window.removeEventListener("touchend", up, true);
      window.removeEventListener("touchcancel", up, true);
      moved.dispose();
      wrote.dispose();
      sized.dispose();
      swapped.dispose();
      rendered.dispose();
      trims?.dispose();
    },
  };
}
