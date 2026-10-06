// **「新输出」 means lines, not writes** (item 52). The way-back-down float
// said 新输出 on any write parsed while the reader was up in the history,
// and an idle shell writes without printing anything new: a deck fold
// resizes the grid, zsh answers the SIGWINCH by redrawing its prompt in
// place (`\r\e[A…\e[J` and the prompt again), tmux redraws its whole
// client and refreshes its status line's clock each minute. Each of those
// lit the float in a shell where nothing had run (the owner, a zsh shell,
// khor's own and in tmux; measured in `audit/term-52.md`).
//
// What counts is the buffer's extent: the last row that holds anything,
// counted from the first line the buffer ever had (rows trimmed off the
// top included), so it grows exactly when lines arrive below what was
// there and never for a redraw in place, a cleared screen or a cursor
// that moves. iTerm2's and Terminal.app's "new output" marks count
// lines the same way, not bytes.
import type { Terminal } from "@xterm/xterm";

type Trims = { onTrim?: (f: (n: number) => void) => { dispose(): void } };

/** The buffer's last row with anything in it, from the buffer's first
    line; above the screen when the screen is blank. */
function lastFilled(term: Terminal): number {
  const b = term.buffer.active;
  for (let y = b.baseY + term.rows - 1; y >= b.baseY; y--) {
    if (b.getLine(y)?.translateToString(true)) return y;
  }
  return b.baseY - 1;
}

/**
 * Whether lines arrived below since `mark`. `mark` takes the extent now
 * (when the reader goes up, and after a resize: a reflow moves rows
 * between the screen and the history, and a resize's own redraw follows
 * it); `defer` takes it at the next `grew` instead (a refill that is
 * mostly the same lines again).
 */
export function countsGain(term: Terminal): {
  mark: () => void;
  defer: () => void;
  grew: () => boolean;
  stop: () => void;
} {
  let trimmed = 0;
  const lines = (term as unknown as { _core?: { buffers?: { normal?: { lines?: Trims } } } })._core?.buffers?.normal?.lines;
  const trims = lines?.onTrim?.((n) => {
    trimmed += n;
  });
  const extent = () => trimmed + lastFilled(term);
  let from: number | null = null;
  const resized = term.onResize(() => {
    from = extent();
  });
  return {
    mark: () => {
      from = extent();
    },
    defer: () => {
      from = null;
    },
    grew: () => {
      const now = extent();
      if (from === null) {
        from = now;
        return false;
      }
      return now > from;
    },
    stop: () => {
      trims?.dispose();
      resized.dispose();
    },
  };
}
