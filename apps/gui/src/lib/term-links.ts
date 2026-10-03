// **Paths in a terminal are links** (roadmap 3.1): absolute and relative
// paths, `~/x`, a bare name in the directory the command ran in (what `ls`
// prints), and the line marks compilers and tracebacks put after them —
// `file.py:12`, `file.ts:12:3`, `file.ts(12,3)`, Python's
// `File "x.py", line 12`. Only a path the machine actually has becomes a
// link: each candidate's directory is listed once on that machine
// (`fetchLs`) and remembered, the conversation's rule (`views/linkHome`:
// an underline that points at nothing is a lie). iTerm2's Semantic History
// and VS Code's terminal link detection work the same way: match broadly,
// resolve against the shell's cwd (OSC 7), keep only what exists.
//
// Relative paths resolve against the directory the line's own command ran
// in (`lib/term-marks`' block cwd), else where the shell stands now.
import type { IBufferLine, ILink, ILinkProvider, Terminal } from "@xterm/xterm";

import { fetchLs } from "@/api";

/** What a path link opens. */
export type PathTarget = { path: string; dir: boolean; line: number | null; col: number | null };

/** A run of path characters, with an optional line mark after it. */
const CANDIDATE = /(?:~\/|\.{0,2}\/)?[\w.@+~-]+(?:\/[\w.@+~-]+)*\/?(?::(\d+)(?::(\d+))?|\((\d+),(\d+)\))?/g;
/** Python's traceback line: `File "…", line 12`. */
const PY_LINE = /^",\s+line\s+(\d+)/;

type Listing = Map<string, boolean>;

/** The directories asked of each machine: name → is a directory. */
const listed = new Map<string, Map<string, Promise<Listing | null>>>();
const homes = new Map<string, Promise<string | null>>();

function list(machine: string, dir: string): Promise<Listing | null> {
  let m = listed.get(machine);
  if (!m) {
    m = new Map();
    listed.set(machine, m);
  }
  let p = m.get(dir);
  if (!p) {
    p = fetchLs(machine, dir)
      .then((l) => new Map(l.entries.map((e) => [e.name, e.dir] as [string, boolean])))
      .catch(() => null);
    m.set(dir, p);
    // A listing goes stale: a file made a minute later is a link then.
    window.setTimeout(() => m?.delete(dir), 60_000);
  }
  return p;
}

function home(machine: string): Promise<string | null> {
  let p = homes.get(machine);
  if (!p) {
    p = fetchLs(machine, "")
      .then((l) => l.path)
      .catch(() => null);
    homes.set(machine, p);
  }
  return p;
}

/** `a/./b/../c` → `a/c`, for a path that starts at `/`. */
function tidy(abs: string): string {
  const out: string[] = [];
  for (const seg of abs.split("/")) {
    if (seg === "" || seg === ".") continue;
    if (seg === "..") out.pop();
    else out.push(seg);
  }
  return `/${out.join("/")}`;
}

/** Whether a path is there on that machine, and a directory. */
async function exists(machine: string, abs: string): Promise<{ dir: boolean } | null> {
  const clean = tidy(abs);
  if (clean === "/") return { dir: true };
  const cut = clean.lastIndexOf("/");
  const parent = cut <= 0 ? "/" : clean.slice(0, cut);
  const got = await list(machine, parent);
  const dir = got?.get(clean.slice(cut + 1));
  return dir === undefined ? null : { dir };
}

/** The string index of each cell's first character, and back. */
function cells(line: IBufferLine, cols: number): { text: string; at: number[] } {
  let text = "";
  const at: number[] = [];
  for (let x = 0; x < cols; x++) {
    const c = line.getCell(x);
    if (!c) break;
    const ch = c.getChars();
    if (c.getWidth() === 0) continue;
    at[text.length] = x;
    text += ch || " ";
  }
  return { text: text.replace(/\s+$/, ""), at };
}

/** One path found on a line: its cells (1-based, inclusive) and target. */
export type FoundPath = { x0: number; x1: number; text: string; target: PathTarget };

/** The paths on buffer line `y` (0-based) that the machine has. */
export async function pathsOn(
  term: Terminal,
  machine: string,
  cwdAt: (line: number) => string | undefined,
  y: number,
): Promise<FoundPath[]> {
  const line = term.buffer.active.getLine(y);
  if (!line) return [];
  const { text, at } = cells(line, term.cols);
  const cwd = cwdAt(y);
  const found: Promise<FoundPath | null>[] = [];
  CANDIDATE.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = CANDIDATE.exec(text)) !== null) {
    const whole = m[0];
    if (whole.length < 2) continue;
    const start = m.index;
    const mark = whole.search(/(:\d+(:\d+)?|\(\d+,\d+\))$/);
    const written = mark > 0 ? whole.slice(0, mark) : whole;
    let lineNo = m[1] ?? m[3];
    const colNo = m[2] ?? m[4];
    const py = PY_LINE.exec(text.slice(start + whole.length));
    if (py) lineNo = py[1];
    found.push(
      (async () => {
        // A bare word with no dot or slash is only a link when it is a
        // name where the command ran — `ls` prints exactly those.
        let abs: string | null = null;
        if (written.startsWith("/")) abs = written;
        else if (written.startsWith("~/")) {
          const h = await home(machine);
          abs = h ? `${h}/${written.slice(2)}` : null;
        } else if (cwd) abs = `${cwd}/${written}`;
        if (!abs) return null;
        const is = await exists(machine, abs);
        if (!is) return null;
        const last = start + whole.length - 1;
        return {
          x0: (at[start] ?? start) + 1,
          x1: (at[last] ?? last) + 1,
          text: whole,
          target: { path: tidy(abs), dir: is.dir, line: lineNo ? Number(lineNo) : null, col: colNo ? Number(colNo) : null },
        };
      })(),
    );
  }
  return (await Promise.all(found)).filter((f): f is FoundPath => f !== null);
}

/**
 * The provider, for a mouse: xterm asks for a line's links as the pointer
 * crosses it, underlines them, and a ⌘-click opens one (iTerm2's gesture,
 * as for URLs here). A finger has no hover to ask before a tap; the pane
 * asks `pathsOn` at the tap itself.
 */
export function pathLinks(
  term: Terminal,
  machine: string,
  cwdAt: (line: number) => string | undefined,
  open: (target: PathTarget) => void,
): ILinkProvider {
  return {
    provideLinks(y, done) {
      void pathsOn(term, machine, cwdAt, y - 1).then((found) => {
        const links: ILink[] = found.map((f) => ({
          range: { start: { x: f.x0, y }, end: { x: f.x1, y } },
          text: f.text,
          decorations: { underline: true, pointerCursor: true },
          activate: (ev: MouseEvent) => {
            if (ev.metaKey || ev.ctrlKey) open(f.target);
          },
        }));
        done(links.length ? links : undefined);
      });
    },
  };
}

/** A URL under a cell of buffer line `y`, for a finger's tap. */
export function urlAt(term: Terminal, y: number, x: number): string | null {
  const line = term.buffer.active.getLine(y);
  if (!line) return null;
  const { text, at } = cells(line, term.cols);
  for (const m of text.matchAll(/https?:\/\/[^\s"'<>`]+[^\s"'<>`.,;:!?)\]]/g)) {
    const a = at[m.index] ?? m.index;
    const last = m.index + m[0].length - 1;
    const b = at[last] ?? last;
    if (x >= a && x <= b) return m[0];
  }
  return null;
}
