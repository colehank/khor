// **The shell's marks, as the face's xterm sees them** (roadmap 2.2 and 2.3).
//
// The host keeps the command ledger (`khor_node::commands`); this is the
// face's half, the way VS Code's terminal does it (its
// `ShellIntegrationAddon` registers OSC 633/133 handlers and pins each mark
// to an xterm marker): every `OSC 133 ; A/B/C/D` and VS Code's `OSC 633 ; E`
// that reaches xterm — live, or replayed on attach from the host's ring —
// becomes a marker on the buffer line it was written on, so a block's
// lines move with the scrollback and fall away with it. Nothing here is
// drawn; the pane decides what to show.
import type { IDisposable, IMarker, Terminal } from "@xterm/xterm";

export type Block = {
  /** Where its prompt starts (A). */
  prompt: IMarker;
  /** Where the input starts (B), and the column. */
  input?: { marker: IMarker; col: number };
  /** Where the output starts (C). */
  output?: IMarker;
  /** Where it ended (D), and how. */
  end?: IMarker;
  exit?: number | null;
  command?: string;
  /** Where the shell stood when it ran (OSC 7). */
  cwd?: string;
};

export type Marks = {
  /** Every block still in the buffer, oldest first. */
  blocks: () => Block[];
  /** The block being typed or run now, if any. */
  current: () => Block | null;
  /** Whether a command has started and not ended. */
  running: () => boolean;
  /** Whether the shell has marked anything at all. */
  marked: () => boolean;
  /** Where the shell stands (OSC 7), if it said. */
  cwd: () => string | undefined;
  /** Called when a block starts or ends. */
  onChange: (f: () => void) => IDisposable;
  dispose: () => void;
};

/** VS Code's escaping, undone (`\\`, `\xNN`). */
function unescape(s: string): string {
  return s.replace(/\\(\\|x([0-9a-fA-F]{2}))/g, (_, all: string, hex?: string) =>
    hex ? String.fromCharCode(parseInt(hex, 16)) : all,
  );
}

export function trackMarks(term: Terminal): Marks {
  let list: Block[] = [];
  let seen = false;
  let named: string | undefined;
  let cwd: string | undefined;
  const listeners = new Set<() => void>();
  const changed = () => listeners.forEach((f) => f());
  const alive = () => {
    list = list.filter((b) => !b.prompt.isDisposed);
    return list;
  };
  const last = () => alive()[list.length - 1] ?? null;
  const here = () => term.registerMarker(0);

  const mark = (kind: string, params: string[]) => {
    // A full-screen program's screen is not a shell's scrollback: marks
    // that reach the alternate screen came through a multiplexer (tmux's
    // passthrough) and name places on its redraw, not on lines the face
    // keeps — VS Code's command detection stops there too.
    if (term.buffer.active.type === "alternate") return;
    seen = true;
    const b = last();
    switch (kind) {
      case "A": {
        // A second A on the line the last one began is the same prompt.
        if (b && !b.input && !b.output && b.prompt.line === term.buffer.active.baseY + term.buffer.active.cursorY) return;
        const m = here();
        if (!m) return;
        list.push({ prompt: m });
        named = undefined;
        changed();
        return;
      }
      case "B": {
        if (!b || b.input) return;
        const m = here();
        if (m) b.input = { marker: m, col: term.buffer.active.cursorX };
        return;
      }
      case "C": {
        if (!b || b.output) return;
        const m = here();
        if (!m) return;
        b.output = m;
        b.cwd = cwd;
        for (const p of params) {
          if (p.startsWith("cmdline_url=")) {
            try {
              b.command = decodeURIComponent(p.slice("cmdline_url=".length));
            } catch {
              /* not a URL encoding */
            }
          } else if (p.startsWith("cmdline=")) b.command = unescape(p.slice("cmdline=".length));
        }
        b.command ??= named;
        changed();
        return;
      }
      case "D": {
        if (!b || !b.output || b.end) return;
        const m = here();
        if (!m) return;
        b.end = m;
        const n = parseInt(params[0] ?? "", 10);
        b.exit = Number.isFinite(n) ? n : null;
        changed();
        return;
      }
    }
  };

  const fin = term.parser.registerOscHandler(133, (data) => {
    const [kind = "", ...params] = data.split(";");
    mark(kind, params);
    return true;
  });
  const vsc = term.parser.registerOscHandler(633, (data) => {
    const [kind = "", ...rest] = data.split(";");
    if (kind === "E") {
      const line = unescape(rest[0] ?? "").trim();
      const b = last();
      // fish 4 says C first, then E.
      if (b?.output && !b.end) b.command = line;
      else named = line;
      seen = true;
    } else if ("ABCD".includes(kind) && kind.length === 1) mark(kind, rest);
    return true;
  });

  const dir = term.parser.registerOscHandler(7, (data) => {
    const m = /^file:\/\/[^/]*(\/.*)$/.exec(data);
    if (m) {
      try {
        cwd = decodeURIComponent(m[1]);
      } catch {
        cwd = m[1];
      }
    }
    return true;
  });

  return {
    blocks: alive,
    cwd: () => cwd,
    current: () => {
      const b = last();
      return b && !b.end ? b : null;
    },
    running: () => {
      const b = last();
      return !!b?.output && !b.end;
    },
    marked: () => seen,
    onChange: (f) => {
      listeners.add(f);
      return { dispose: () => listeners.delete(f) };
    },
    dispose: () => {
      fin.dispose();
      vsc.dispose();
      dir.dispose();
      listeners.clear();
      list.forEach((b) => {
        b.prompt.dispose();
        b.input?.marker.dispose();
        b.output?.dispose();
        b.end?.dispose();
      });
      list = [];
    },
  };
}

/** Where the cursor sits on the screen (column, row within the screen). */
export type Cursor = { x: number; y: number };

/** Rows searched above and below the cursor for its input box's borders. */
const BOX_SCAN = 12;
/** Rows beyond a box's edges that still count as its input (item 43). */
const BOX_SLACK = 2;
/** Rows beyond a bare cursor row that still count (item 43). */
const ROW_SLACK = 1;

/** A row that is a box's edge: mostly box-drawing characters (U+2500–257F). */
function border(text: string): boolean {
  const t = [...text.trim()];
  if (t.length < 3) return false;
  const drawn = t.filter((c) => c >= "\u2500" && c <= "\u257f").length;
  return drawn / t.length >= 0.8;
}

const rowText = (term: Terminal, line: number) => term.buffer.active.getLine(line)?.translateToString(true) ?? "";

/** The box drawn around row `at` (Claude Code's input box): the lines of
    its top and bottom edges, within `BOX_SCAN` rows of it and the screen
    (`top`..`end`). */
function boxAround(term: Terminal, at: number, top: number, end: number): { over: number; under: number } | null {
  let over = -1;
  for (let i = at - 1; i >= Math.max(top, at - BOX_SCAN); i--) {
    if (border(rowText(term, i))) {
      over = i;
      break;
    }
  }
  if (over < 0) return null;
  for (let i = at + 1; i <= Math.min(end - 1, at + BOX_SCAN); i++) {
    if (border(rowText(term, i))) return { over, under: i };
  }
  return null;
}

/**
 * **Whether a tap on a cell asks for the keyboard** (roadmap 2.2, `auto`;
 * item 28): a tap on the input area raises, a tap elsewhere does not.
 * The input area, by what khor can know:
 *
 * - a marked shell at its prompt: from where its input starts on, the
 *   blank below included — a tap up in the history is a reader's;
 * - a program whose cursor sits in a box (Claude Code's input box, its
 *   `─` edges above and below the `>` line): the box, and two rows
 *   beyond each edge (item 43; the owner: 「要点得很准」 — the box is
 *   three rows, ~48 pt, and a tap on the output just above or the hint
 *   line just below missed). Never the whole screen, never down to the
 *   screen's bottom: the owner refused anything that amounts to
 *   tap-anywhere;
 * - otherwise the cursor's row and one row either side (item 43), and —
 *   a shell at its prompt with no marks, where the cursor is on the last
 *   row with anything on it — the blank below as well: a fresh shell's
 *   prompt sits at the top with the whole screen blank under it; a
 *   status line on the screen's last row (tmux's) does not count.
 *
 * `cursor` is where the last whole frame left the cursor
 * (`settledCursor`), not where a redraw in progress has it: Claude Code
 * moves it up through its output while it paints.
 */
export function inInput(term: Terminal, marks: Marks, line: number, col: number, cursor: Cursor): boolean {
  const buf = term.buffer.active;
  const b = marks.current();
  if (buf.type === "normal" && marks.marked() && b?.input && !b.output) {
    // Everything after the input's start is input while the prompt is
    // up: a long or multi-line command runs on below the cursor too.
    const at = b.input.marker.line;
    return line > at || (line === at && col >= b.input.col);
  }
  const top = buf.baseY;
  const end = top + term.rows;
  const at = top + Math.min(cursor.y, term.rows - 1);
  const box = boxAround(term, at, top, end);
  if (box) return line >= box.over - BOX_SLACK && line <= box.under + BOX_SLACK;
  if (Math.abs(line - at) <= ROW_SLACK) return true;
  if (line > at) {
    for (let i = at + 1; i < end; i++) {
      if (i === end - 1 && buf.type === "alternate") break;
      if (rowText(term, i).trim() !== "") return false;
    }
    return true;
  }
  return false;
}
