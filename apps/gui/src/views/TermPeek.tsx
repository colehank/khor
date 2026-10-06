// **A block's output as data** (roadmap 3.4): what `lib/term-peek` read
// as a table or as JSON, drawn over the terminal pane. A table has its
// header held at the top and sorts on a click of a column's name (again
// for the other way); JSON is a tree whose objects and arrays fold, the
// first two levels open — Firefox's JSON viewer. Values are in the
// terminal's own type, so they read as what the program printed.
import { useMemo, useState } from "react";

import { IconChevronDown, IconChevronRight, IconChevronUp } from "@/components/icons";
import { cellOrder, type Peek } from "@/lib/term-peek";

/** Levels open when the tree is first drawn. */
const OPEN_LEVELS = 2;

export function TermPeek({ peek }: { peek: Peek }) {
  return (
    <div data-term-peek-body className="min-h-0 flex-1 overflow-auto text-callout" style={{ fontFamily: "var(--font-term)" }}>
      {peek.kind === "table" ? <Table head={peek.head} rows={peek.rows} /> : <Tree name={null} value={peek.value} depth={0} />}
    </div>
  );
}

function Table({ head, rows }: { head: string[]; rows: string[][] }) {
  const [by, setBy] = useState<{ col: number; up: boolean } | null>(null);
  const shown = useMemo(() => {
    if (!by) return rows;
    const out = [...rows];
    out.sort((a, b) => cellOrder(a[by.col] ?? "", b[by.col] ?? "") * (by.up ? 1 : -1));
    return out;
  }, [rows, by]);
  return (
    <table className="min-w-full border-collapse">
      <thead className="sticky top-0 bg-background">
        <tr>
          {head.map((h, k) => (
            <th key={k} className="border-b border-border p-0 text-left font-title whitespace-nowrap">
              <button
                type="button"
                data-term-peek-col={k}
                aria-sort={by?.col === k ? (by.up ? "ascending" : "descending") : undefined}
                className="flex min-h-touch w-full items-center gap-1 px-2 text-left hover:bg-fill"
                onClick={() => setBy(by?.col === k ? { col: k, up: !by.up } : { col: k, up: true })}
              >
                {h}
                {by?.col === k && (by.up ? <IconChevronUp className="size-3" /> : <IconChevronDown className="size-3" />)}
              </button>
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {shown.map((r, i) => (
          <tr key={i} className="border-b border-border">
            {r.map((v, k) => (
              <td key={k} className="px-2 py-1 align-top whitespace-pre">
                {v}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function Tree({ name, value, depth }: { name: string | null; value: unknown; depth: number }) {
  const [open, setOpen] = useState(depth < OPEN_LEVELS);
  const label = name === null ? null : <span className="text-muted-foreground">{name}: </span>;
  if (value === null || typeof value !== "object") {
    return (
      <div className="py-0.5 whitespace-pre-wrap break-all" style={{ paddingLeft: `${depth * 1.25}em` }}>
        {label}
        <Scalar value={value} />
      </div>
    );
  }
  const entries: [string, unknown][] = Array.isArray(value) ? value.map((v, i) => [String(i), v]) : Object.entries(value);
  const brackets = Array.isArray(value) ? ["[", "]"] : ["{", "}"];
  return (
    <div>
      <button
        type="button"
        data-term-peek-fold
        aria-expanded={open}
        className="flex min-h-touch w-full items-center gap-1 text-left hover:bg-fill"
        style={{ paddingLeft: `${depth * 1.25}em` }}
        onClick={() => setOpen(!open)}
      >
        {open ? <IconChevronDown className="size-3" /> : <IconChevronRight className="size-3" />}
        {label}
        <span className="text-muted-foreground">
          {open ? brackets[0] : `${brackets[0]} ${entries.length} ${brackets[1]}`}
        </span>
      </button>
      {open && entries.map(([k, v]) => <Tree key={k} name={k} value={v} depth={depth + 1} />)}
    </div>
  );
}

function Scalar({ value }: { value: unknown }) {
  if (typeof value === "string") return <span>{JSON.stringify(value)}</span>;
  return <span className="text-muted-foreground">{String(value)}</span>;
}
