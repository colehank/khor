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

/**
 * **Whether a cell is on the line being typed** (roadmap 2.2, `auto`): from
 * where the shell's input starts on, while the prompt is up; the cursor's
 * row when a program runs or when the shell marks nothing.
 */
export function inInput(term: Terminal, marks: Marks, line: number, col: number): boolean {
  const buf = term.buffer.active;
  const cursorLine = buf.baseY + buf.cursorY;
  const b = marks.current();
  if (buf.type === "normal" && marks.marked() && b?.input && !b.output) {
    // Everything after the input's start is input while the prompt is
    // up: a long or multi-line command runs on below the cursor too.
    const at = b.input.marker.line;
    return line > at || (line === at && col >= b.input.col);
  }
  return line === cursorLine;
}
