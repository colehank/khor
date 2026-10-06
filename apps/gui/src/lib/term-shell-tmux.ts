// **A shell with the person's own tmux client in front** (item 58; the
// owner: the two cannot be told apart, so they behave the same). A khor
// shell in which the person typed `tmux` is treated as a tmux row while
// that client is there: the node finds it by the shell's terminal
// (`LiveKind::tmux_in_shell`), and nothing of the person's is written —
// their `mouse` stays theirs, so the Mac's wheel is carried to the node,
// which puts the pane in copy mode as tmux's own wheel would.
//
// The face sees the client come as the alternate screen and leave as the
// normal one: each switch asks the node again (`termScroll(id, 0)`), so a
// vim or an htop in the shell is asked about once and left alone.
import type { Terminal } from "@xterm/xterm";

/** The ~/.zshrc line that loads khor's zsh integration in shells tmux
    starts on its own (`crates/node/shell/khor.zsh`, item 21). */
export const TMUX_ZSHRC = "[[ -n $TMUX && -r ~/.khor/shell/khor.zsh ]] && source ~/.khor/shell/khor.zsh";

/** Whether a tmux client the person started is in front of this shell. */
export function followsShellTmux(
  term: Terminal,
  /** The node's wheel: `rows` up when positive, 0 only asks; true when a
      tmux client is in front (`termScroll`). */
  ask: (rows: number) => Promise<boolean>,
): { front: () => boolean; lost: () => void; stop: () => void } {
  let front = false;
  let asked = 0;
  const check = () => {
    const mine = ++asked;
    front = false;
    if (term.buffer.active.type !== "alternate") return;
    ask(0)
      .then((yes) => {
        if (mine === asked) front = yes;
      })
      .catch(() => {});
  };
  const changed = term.buffer.onBufferChange(check);
  check();
  return {
    front: () => front && term.buffer.active.type === "alternate",
    lost: () => {
      front = false;
    },
    stop: () => {
      asked++;
      changed.dispose();
    },
  };
}

/**
 * **The Mac's wheel over it** (item 58): to tmux's copy mode through the
 * node, rows at a time, one request in flight and the rest owed. A tmux
 * whose `mouse` the person turned on asks for mouse reports itself, and
 * then xterm's own wheel goes to it as on a tmux row: nothing is taken.
 */
export function wheelsToShellTmux(
  box: HTMLElement,
  term: Terminal,
  tmux: { front: () => boolean; lost: () => void },
  send: (rows: number) => Promise<boolean>,
): () => void {
  let owed = 0;
  let carry = 0;
  let sending = false;
  const flush = () => {
    if (sending || owed === 0) return;
    const rows = owed;
    owed = 0;
    sending = true;
    send(rows)
      .then((took) => {
        // The client left between the switch and this wheel.
        if (!took) tmux.lost();
      })
      .catch(() => tmux.lost())
      .finally(() => {
        sending = false;
        flush();
      });
  };
  const wheel = (e: WheelEvent) => {
    if (!tmux.front() || term.modes.mouseTrackingMode !== "none" || e.ctrlKey || e.metaKey) return;
    e.preventDefault();
    e.stopPropagation();
    const cell = (term.element?.clientHeight ?? 0) / Math.max(1, term.rows) || 16;
    const lines = e.deltaMode === WheelEvent.DOM_DELTA_LINE ? e.deltaY : e.deltaMode === WheelEvent.DOM_DELTA_PAGE ? e.deltaY * term.rows : e.deltaY / cell;
    // Up the page (a negative delta) is rows up.
    carry -= lines;
    const whole = Math.trunc(carry);
    if (whole === 0) return;
    carry -= whole;
    owed += whole;
    flush();
  };
  box.addEventListener("wheel", wheel, { capture: true, passive: false });
  return () => box.removeEventListener("wheel", wheel, true);
}
