// **A block's output, read as data** (roadmap 3.4): when what a command
// printed is JSON or a table, its block offers 「结构化查看」, which shows
// it as a tree or as a sortable table instead of as text.
//
// What counts:
//   - JSON, the whole output (`curl …/api`, `jq .`, `kubectl get -o json`),
//     and JSON Lines, one value a line (`docker inspect`-ish logs, `jq -c`);
//   - delimited rows with the same count of cells on every line: tabs
//     (`cut`, `mlr --tsv`), or commas where no cell starts with a space
//     (CSV — prose's ", " is not a separator);
//   - columns aligned with spaces under a header (`ps`, `docker ps`,
//     `kubectl get`, `df`, `lsof`): split at the character columns that
//     are blank on every line, nushell's `detect columns` and jc's
//     universal table parser. A header names each column; a column the
//     header leaves blank (a command line's inner spaces lining up) joins
//     the one before it.
// A grid of names (`ls`) is not a table: the header must read as one —
// capitals, or a column below it that is figures.

/** What a block's output is, as data. */
export type Peek =
  | { kind: "json"; value: unknown }
  | { kind: "table"; head: string[]; rows: string[][] };

/** Past this many lines a block is not offered: the view is for looking. */
const MAX_LINES = 5000;

/** A size, a count, a time, a percentage: `12`, `3.5G`, `-`, `98%`, `10:42`. */
const FIGURE = /^[-+]?[\d.,:]+[%A-Za-z]{0,3}$/;

export function peekOf(text: string): Peek | null {
  const all = text.replace(/\s+$/, "");
  const t = all.trim();
  if (!t) return null;
  if (/^[[{]/.test(t)) {
    try {
      const value: unknown = JSON.parse(t);
      if (value !== null && typeof value === "object") return { kind: "json", value };
    } catch {
      /* JSON Lines, or not JSON */
    }
  }
  const lines = all.split("\n").filter((l) => l.trim() !== "");
  if (lines.length < 2 || lines.length > MAX_LINES) return null;
  if (lines.every((l) => /^\s*[[{]/.test(l))) {
    try {
      return { kind: "json", value: lines.map((l) => JSON.parse(l) as unknown) };
    } catch {
      /* not JSON Lines */
    }
  }
  return delimited(lines, "\t") ?? delimited(lines, ",") ?? aligned(lines);
}

function delimited(lines: string[], sep: string): Peek | null {
  if (sep === "," && lines.length < 3) return null;
  const cells = lines.map((l) => splitCells(l, sep));
  const n = cells[0].length;
  if (n < 2 || !cells.every((c) => c.length === n)) return null;
  if (sep === "," && cells.some((c) => c.some((v) => /^\s/.test(v)))) return null;
  return { kind: "table", head: cells[0], rows: cells.slice(1) };
}

/** One line's cells, with CSV's double quotes. */
function splitCells(line: string, sep: string): string[] {
  const out: string[] = [];
  let cell = "";
  let quoted = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (quoted) {
      if (c === '"' && line[i + 1] === '"') {
        cell += '"';
        i++;
      } else if (c === '"') quoted = false;
      else cell += c;
    } else if (c === '"' && cell === "") quoted = true;
    else if (c === sep) {
      out.push(cell);
      cell = "";
    } else cell += c;
  }
  out.push(cell);
  return out;
}

/** Whether a character takes two cells on a terminal (East Asian Wide and
    Fullwidth, and the emoji planes): a column lines up in cells, not in
    characters — `ps` on a Chinese system prints 「3:16上午」. */
function wide(c: string): boolean {
  const n = c.codePointAt(0) ?? 0;
  return (
    (n >= 0x1100 && n <= 0x115f) ||
    (n >= 0x2e80 && n <= 0xa4cf && n !== 0x303f) ||
    (n >= 0xac00 && n <= 0xd7a3) ||
    (n >= 0xf900 && n <= 0xfaff) ||
    (n >= 0xfe30 && n <= 0xfe4f) ||
    (n >= 0xff00 && n <= 0xff60) ||
    (n >= 0xffe0 && n <= 0xffe6) ||
    (n >= 0x1f300 && n <= 0x1faff) ||
    (n >= 0x20000 && n <= 0x3fffd)
  );
}

/** A line as it sits in cells: a wide character's second cell is "". */
function inCells(line: string): string[] {
  const out: string[] = [];
  for (const c of line) {
    out.push(c);
    if (wide(c)) out.push("");
  }
  return out;
}

function aligned(text: string[]): Peek | null {
  if (text.length < 3 || text.some((l) => l.includes("\t"))) return null;
  const lines = text.map(inCells);
  const width = Math.max(...lines.map((l) => l.length));
  const blank = new Array<boolean>(width).fill(true);
  for (const l of lines) for (let i = 0; i < l.length; i++) if (l[i] !== " ") blank[i] = false;
  // Each run of text between blank columns is a column; the last runs to
  // the line's end, whatever spaces it holds.
  const starts: number[] = [];
  for (let i = 0; i < width; i++) if (!blank[i] && (i === 0 || blank[i - 1])) starts.push(i);
  const header = lines[0];
  const cut = (l: string[], k: number) =>
    l
      .slice(starts[k], k + 1 < starts.length ? starts[k + 1] : undefined)
      .join("")
      .trim();
  // A column with nothing in the header belongs to the one before it.
  const kept = starts.filter((_, k) => k === 0 || cut(header, k) !== "");
  if (kept.length < 2) return null;
  starts.splice(0, starts.length, ...kept);
  const head = starts.map((_, k) => cut(header, k));
  const rows = lines.slice(1).map((l) => starts.map((_, k) => cut(l, k)));
  const capitals = head.every((h) => /^[A-Z%#][A-Z0-9%#_ ()/-]*$/.test(h));
  const figures = head.some((_, k) => {
    const filled = rows.map((r) => r[k]).filter((v) => v !== "");
    return filled.length >= rows.length / 2 && filled.every((v) => FIGURE.test(v));
  });
  const named = head.every((h) => /^[A-Za-z%#][\w %#()/.-]*$/.test(h) && !/\.\w+$/.test(h));
  if (!(capitals || (named && figures))) return null;
  return { kind: "table", head, rows };
}

/** Compares two cells: figures by value, the rest as words. */
export function cellOrder(a: string, b: string): number {
  const x = parseFloat(a);
  const y = parseFloat(b);
  if (FIGURE.test(a) && FIGURE.test(b) && !Number.isNaN(x) && !Number.isNaN(y) && x !== y) return x - y;
  return a.localeCompare(b, undefined, { numeric: true });
}
