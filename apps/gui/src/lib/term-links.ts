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
import {linkIntent,canActivateTerminalLink,type LinkIntent} from "@/lib/content-link-intent";

/** What a path link opens. */
export type PathTarget = { path: string; dir: boolean; line: number | null; col: number | null };

/** A run of path characters, with an optional line mark after it. Any
    letter or digit, not only ASCII's (the terminal audit, item 53: a
    `报告/图.png` was no link). */
const CANDIDATE = /(?:~\/|\.{0,2}\/)?[\p{L}\p{N}\p{M}_.@+~-]+(?:\/[\p{L}\p{N}\p{M}_.@+~-]+)*\/?(?::(\d+)(?::(\d+))?|\((\d+),(\d+)\))?/gu;
/** A quoted path, spaces and all: `ls` quotes such names (`'my file.txt'`),
    and so do most tools' messages. Only taken when it holds a slash or a
    dot, and only kept when the machine has it, like every candidate. */
const QUOTED = /(["'])([^"'\n]*[ ][^"'\n]*)\1/g;
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

/** A cell of the buffer: column and buffer line, both 0-based. */
type Cell = { x: number; y: number };

/** The most rows one wrapped line is read across. */
const WRAPPED_MOST = 16;

/**
 * **A line as the program wrote it, not as the grid wrapped it** (the
 * terminal audit, item 53): on a phone's 44 columns a compiler's path or
 * a URL runs on over the row's end, and read row by row it was two
 * halves that neither existed nor opened. The rows xterm marks as
 * wrapped (`isWrapped`) are joined, with the cell each string index came
 * from, so a link can span rows (VS Code's and the web-links addon's
 * rule).
 */
function logical(term: Terminal, y: number): { text: string; at: Cell[]; first: number } {
  const b = term.buffer.active;
  let first = y;
  while (first > 0 && y - first < WRAPPED_MOST && b.getLine(first)?.isWrapped) first--;
  let text = "";
  const at: Cell[] = [];
  for (let row = first; row < first + WRAPPED_MOST; row++) {
    const line: IBufferLine | undefined = b.getLine(row);
    if (!line || (row > first && !line.isWrapped)) break;
    for (let x = 0; x < term.cols; x++) {
      const c = line.getCell(x);
      if (!c) break;
      if (c.getWidth() === 0) continue;
      // xterm moves a wide character to the next soft-wrapped row and
      // clears the unused last cell to code 0. A typed space is code 32.
      const next = x === term.cols - 1 ? b.getLine(row + 1) : undefined;
      if (c.getCode() === 0 && next?.isWrapped && next.getCell(0)?.getWidth() === 2) continue;
      at[text.length] = { x, y: row };
      text += c.getChars() || " ";
    }
  }
  return { text: text.replace(/\s+$/, ""), at, first };
}

/** The cell string index `i` was read from (a character of two code
    units has its cell at the first). */
function cellOf(at: Cell[], i: number): Cell | undefined {
  while (i > 0 && !at[i]) i--;
  return at[i];
}

/** One path found on a line: its first and last cells (`x` 1-based,
    `y` the 0-based buffer line) and what it opens. */
export type FoundPath = { from: Cell; to: Cell; text: string; target: PathTarget };

/** Whether cell (`col` 0-based) of buffer line `row` is inside `f`. */
export function covers(f: { from: Cell; to: Cell }, row: number, col: number): boolean {
  const at = row * 1e5 + col + 1;
  return at >= f.from.y * 1e5 + f.from.x && at <= f.to.y * 1e5 + f.to.x;
}

/** The paths on buffer line `y` (0-based) that the machine has. */
export async function pathsOn(
  term: Terminal,
  machine: string,
  cwdAt: (line: number) => string | undefined,
  y: number,
): Promise<FoundPath[]> {
  if (!term.buffer.active.getLine(y)) return [];
  const { text, at, first } = logical(term, y);
  const cwd = cwdAt(first);
  const found: Promise<FoundPath | null>[] = [];
  const candidates: { start: number; whole: string; written: string; lineNo?: string; colNo?: string }[] = [];
  CANDIDATE.lastIndex = 0;
  let m: RegExpExecArray | null;
  while ((m = CANDIDATE.exec(text)) !== null) {
    const whole = m[0];
    if (whole.length < 2) continue;
    const mark = whole.search(/(:\d+(:\d+)?|\(\d+,\d+\))$/);
    candidates.push({ start: m.index, whole, written: mark > 0 ? whole.slice(0, mark) : whole, lineNo: m[1] ?? m[3], colNo: m[2] ?? m[4] });
  }
  QUOTED.lastIndex = 0;
  while ((m = QUOTED.exec(text)) !== null) {
    const inner = m[2];
    if (!/[/.]/.test(inner)) continue;
    candidates.push({ start: m.index + 1, whole: inner, written: inner });
  }
  for (const c of candidates) {
    const { start, whole, written, colNo } = c;
    let lineNo = c.lineNo;
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
        const from = cellOf(at, start);
        const to = cellOf(at, last);
        if (!from || !to) return null;
        return {
          from: { x: from.x + 1, y: from.y },
          to: { x: to.x + 1, y: to.y },
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
 * crosses it and underlines them. Activation carries the requested
 * internal or client-system destination, as for URLs. A finger has no hover to ask before a tap; the pane
 * asks `pathsOn` at the tap itself.
 */
export function pathLinks(
  term: Terminal,
  machine: string,
  cwdAt: (line: number) => string | undefined,
  open: (target: PathTarget, intent: LinkIntent) => void,
): ILinkProvider {
  return {
    provideLinks(y, done) {
      void pathsOn(term, machine, cwdAt, y - 1).then((found) => {
        const links: ILink[] = found.map((f) => ({
          range: { start: { x: f.from.x, y: f.from.y + 1 }, end: { x: f.to.x, y: f.to.y + 1 } },
          text: f.text,
          decorations: { underline: true, pointerCursor: true },
          activate: (ev: MouseEvent) => {
            if(canActivateTerminalLink(ev,term))open(f.target,linkIntent(ev));
          },
        }));
        done(links.length ? links : undefined);
      });
    },
  };
}

/** The internals an OSC 8 link is read through, as xterm's own link
    provider reads it: the cell's link id, and the service that holds the
    id's target. Not in the public API (xterm 6). */
type LinkedCell = { hasExtendedAttrs?: () => number | boolean; extended?: { urlId?: number } };
type LinkService = { getLinkData(id: number): { uri: string } | undefined };

/** **The OSC 8 link on a cell** (`ls --hyperlink`, a compiler's
    `file://`, a program's 「read the docs」; the terminal audit, item
    68): its target, or `null`. A link's text need not be a URL, so the
    regex below never found these. */
export function oscLinkAt(term: Terminal, y: number, x: number): string | null {
  const cell = term.buffer.active.getLine(y)?.getCell(x) as LinkedCell | undefined;
  const id = cell?.hasExtendedAttrs?.() ? cell.extended?.urlId : 0;
  if (!id) return null;
  const links = (term as unknown as { _core?: { _oscLinkService?: LinkService } })._core?._oscLinkService;
  return links?.getLinkData(id)?.uri ?? null;
}

/** A URL under a cell of buffer line `y`, for a finger's tap: the cell's
    OSC 8 link first (http(s) or `file://`; the caller sends a file where
    paths go), else a URL in the text; one the grid wrapped onto the next
    rows is read whole (`logical`). */
export function urlAt(term: Terminal, y: number, x: number): string | null {
  if (!term.buffer.active.getLine(y)) return null;
  const linked = oscLinkAt(term, y, x);
  if (linked && /^(https?|file):/i.test(linked)) return linked;
  const { text, at } = logical(term, y);
  for (const m of text.matchAll(/https?:\/\/[^\s"'<>`]+[^\s"'<>`.,;:!?)\]]/g)) {
    const from = cellOf(at, m.index);
    const to = cellOf(at, m.index + m[0].length - 1);
    if (from && to && covers({ from: { x: from.x + 1, y: from.y }, to: { x: to.x + 1, y: to.y } }, y, x)) return m[0];
  }
  return null;
}
