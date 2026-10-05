// **What a terminal set on attach, and what its first touch did** (item
// 33). The owner's phone, entering his own long-lived tmux, would not
// scroll until the keyboard came up once; no build reproduced it on the
// scratch desk, in Chrome or in the simulator. So each attach notes the
// modes the face is in after the replay, the grid and the cell size xterm
// measured (a wheel becomes a mouse report only with a cell height,
// `CoreMouseService.consumeWheelEvent`), and the first touch after it
// notes which scroller took it and what that did. Two lines per entry,
// into the app's notes (`face_note`, read by `khor doctor`'s `terminal`
// row); nothing per write, nothing per touch after the first.
import type { Terminal } from "@xterm/xterm";
import { programScrolls } from "@/lib/term-scroller";
import { keyboardShown } from "@/lib/viewport";

/** A mouse report as xterm encodes it (SGR or the older X10 form). */
const REPORT = /^\x1b\[(<|M)/;
/** Travel past which a touch is a drag. */
const DRAG_PX = 10;
/** Held this long without travel, a touch is a press. */
const PRESS_MS = 450;
/** A tap's keyboard is read once the keyboard has had time to rise. */
const KEYBOARD_MS = 700;
/** A replay that comes by the stream is taken as landed after this much
    quiet, or this long at most. */
const STREAM_QUIET_MS = 300;
const STREAM_MAX_MS = 3000;

/** xterm's measured cell, as the renderer holds it (`?` when it has none). */
function cell(term: Terminal): string {
  const d = (term as unknown as { _core?: { _renderService?: { dimensions?: { css?: { cell?: { width?: number; height?: number } }; device?: { cell?: { height?: number } } } } } })._core?._renderService?.dimensions;
  const r = (n: number | undefined) => (n ? String(Math.round(n * 10) / 10) : "?");
  return `cell ${r(d?.css?.cell?.width)}x${r(d?.css?.cell?.height)} device ${r(d?.device?.cell?.height)}`;
}

export function notesFirstTouch(o: {
  id: string;
  term: Terminal;
  box: HTMLElement;
  scroller: HTMLElement;
  /** A tmux row's history view holds the scroller (item 31). */
  history: () => boolean;
  /** Whether this row would hold a tmux history view on this device. */
  tmuxRow: boolean;
  /** The pane in front, as the history's newest read found it (item 36:
      whether its program holds the mouse). */
  pane?: () => { pane: string; mouse: boolean; alternate: boolean } | null;
  note: (line: string) => void;
}): { attached: (bytes: number) => void; stop: () => void } {
  const { id, term, box, scroller } = o;
  let armed = false;
  let attachedAt = 0;
  let touch: { at: number; x: number; y: number; top: number; moved: number; wheels: number; reports: number; scroller: string } | null = null;
  const say = (what: string) => o.note(`term ${id}: ${what}`);
  /** Says the attach early, before a touch is judged against it. */
  let unwait: (() => void) | null = null;
  /** Gives up waiting without a word (the pane went). */
  let drop: (() => void) | null = null;
  const start = (e: TouchEvent) => {
    if (!armed || touch || e.touches.length !== 1) return;
    unwait?.();
    const t = e.touches[0];
    touch = {
      at: performance.now(),
      x: t.clientX,
      y: t.clientY,
      top: scroller.scrollTop,
      moved: 0,
      wheels: 0,
      reports: 0,
      // Decided as the finger lands, the way the pane's own rules are.
      scroller: o.history() ? "history view" : programScrolls(term) ? "wheel to the program" : "native",
    };
  };
  const move = (e: TouchEvent) => {
    if (!touch || e.touches.length !== 1) return;
    const t = e.touches[0];
    touch.moved = Math.max(touch.moved, Math.hypot(t.clientX - touch.x, t.clientY - touch.y));
  };
  // The drag's wheel, as `fingerWheels` dispatches it (not a trackpad's).
  const wheel = (e: WheelEvent) => {
    if (touch && !e.isTrusted) touch.wheels += 1;
  };
  const data = term.onData((d) => {
    if (touch && REPORT.test(d)) touch.reports += 1;
  });
  const end = () => {
    const t = touch;
    if (!t) return;
    touch = null;
    armed = false;
    const after = Math.round(t.at - attachedAt);
    const head = `first touch after ${after} ms`;
    if (t.moved >= DRAG_PX) {
      const what =
        t.scroller === "wheel to the program"
          ? `${t.wheels} wheels, ${t.reports} reports, mouse ${term.modes.mouseTrackingMode}, ${cell(term)}`
          : `moved ${Math.round(scroller.scrollTop - t.top)} px of ${Math.round(scroller.scrollHeight - scroller.clientHeight)}`;
      const p = o.pane?.();
      say(`${head}: drag, ${t.scroller}: ${what}${p ? `; pane ${p.pane} mouse ${p.mouse ? "held" : "free"}` : ""}`);
    } else if (performance.now() - t.at >= PRESS_MS) {
      say(`${head}: press, selection ${term.hasSelection() ? "made" : "none"}`);
    } else {
      window.setTimeout(() => say(`${head}: tap, keyboard ${keyboardShown() ? "up" : "down"}`), KEYBOARD_MS);
    }
  };
  box.addEventListener("touchstart", start, { capture: true, passive: true });
  box.addEventListener("touchmove", move, { capture: true, passive: true });
  box.addEventListener("wheel", wheel, { capture: true, passive: true });
  window.addEventListener("touchend", end, { capture: true, passive: true });
  window.addEventListener("touchcancel", end, { capture: true, passive: true });
  return {
    attached: (bytes) => {
      drop?.();
      armed = true;
      attachedAt = performance.now();
      const paneSaid = () => {
        const p = o.pane?.();
        return p ? `pane ${p.pane} mouse ${p.mouse ? "held" : "free"}${p.alternate ? ", alternate screen" : ""}` : "pane not read yet";
      };
      const now = (how: string) => {
        const b = term.buffer.active;
        say(
          `attach ${how}; ${b.type} buffer, mouse ${term.modes.mouseTrackingMode}; ` +
            `${term.cols}x${term.rows}, ${cell(term)}; ` +
            (o.tmuxRow ? `tmux row, ${paneSaid()}, history view ${o.history() ? "on" : "off"}` : "row"),
        );
      };
      // A tmux row's line waits for the pane's first read (at most
      // `STREAM_MAX_MS`): whether its program holds the mouse is the point.
      const state = (how: string) => {
        if (!o.tmuxRow || o.pane?.()) return now(how);
        const from = performance.now();
        const poll = window.setInterval(() => {
          if (o.pane?.() || performance.now() - from > STREAM_MAX_MS) {
            window.clearInterval(poll);
            now(how);
          }
        }, 100);
      };
      if (bytes > 0) return state(`${bytes} B`);
      // The feed was empty at the attach (it had only just opened): the
      // host's replay comes by the stream, and the modes are its.
      let quiet = 0;
      let cap = 0;
      const waiting = term.onWriteParsed(() => {
        window.clearTimeout(quiet);
        quiet = window.setTimeout(said, STREAM_QUIET_MS);
      });
      const done = () => {
        window.clearTimeout(quiet);
        window.clearTimeout(cap);
        waiting.dispose();
        unwait = null;
        drop = null;
      };
      const said = () => {
        done();
        state("by the stream");
      };
      cap = window.setTimeout(said, STREAM_MAX_MS);
      unwait = said;
      drop = done;
    },
    stop: () => {
      drop?.();
      data.dispose();
      box.removeEventListener("touchstart", start, true);
      box.removeEventListener("touchmove", move, true);
      box.removeEventListener("wheel", wheel, true);
      window.removeEventListener("touchend", end, true);
      window.removeEventListener("touchcancel", end, true);
    },
  };
}
