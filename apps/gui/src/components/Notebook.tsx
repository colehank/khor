// A Jupyter notebook, read (lane nb-view): the `.ipynb` file's second
// face, behind the same 源码 / 预览 switch Markdown and HTML have
// (`FileView`'s `rendererOf`). The source face stays the JSON in the
// editor, editable; this one only reads.
//
// **Nothing here runs.** No kernel, no widget, no script from the file:
// what a cell printed is drawn from what was saved (`lib/nb/read`), and
// an output only a script can draw says so in one sentence.
//
// **No second renderer for anything khor already renders.** A markdown
// cell, a `text/markdown` output and a `text/latex` output are the file
// face of `Markdown` (GitHub's rules, KaTeX, the clean); a `text/html`
// output is its `HtmlFragment` (the same clean and parts); a code cell
// is coloured by Shiki through `lib/highlight`, GitHub's two themes; an
// ANSI colour is the terminal's own palette for the ground it is read
// on. The libraries that render notebooks whole (`@jupyter-kit/*`,
// `notebookjs`, `react-ipynb-renderer`) each bring their own markdown,
// highlighter, math or sanitizer and their own stylesheet, which is the
// second set this file exists not to have.
//
// **Large notebooks draw what is near the screen.** A cell far below is
// a block of about its height until the scroller comes within a couple
// of screens of it, then it is drawn once and stays; the first dozen are
// drawn at once. A notebook of hundreds of cells and dozens of plots
// opens on its first screen, not on the whole file.
import { useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";

import "./Notebook.css";

import { attach, ATTACHED, HtmlFragment, Markdown } from "@/components/Markdown";
import { gui } from "@/gen/catalog";
import { languageOf, paint, painted } from "@/lib/highlight";
import { parseAnsi, xterm256, type Color, type Run } from "@/lib/nb/ansi";
import { readNotebook, type Cell, type Out } from "@/lib/nb/read";
import presets from "@/lib/term-palettes.json";
import { ANSI_KEYS } from "@/lib/term-theme";

/** Cells drawn on the first frame, before anything is measured. */
const EAGER = 12;
/** How far beyond the scroller's edge a cell is drawn ahead of time. */
const AHEAD = "200% 0%";

/** The source line each cell starts on, so switching between the two
    faces keeps the place (`FilePage`'s `topLine` / `placeAt`). A cell is
    the only object with a `"cell_type"` key; when the count disagrees
    with the cells read, nothing is marked rather than something wrong. */
function cellLines(text: string, cells: number): number[] | null {
  const lines: number[] = [];
  const key = /"cell_type"\s*:/g;
  let line = 1;
  let at = 0;
  for (const m of text.matchAll(key)) {
    for (let i = at; i < (m.index ?? 0); i++) if (text.charCodeAt(i) === 10) line += 1;
    at = m.index ?? 0;
    lines.push(line);
  }
  return lines.length === cells ? lines : null;
}

/** About how tall a cell will be, in lines of text, for the block that
    stands in for it until it is drawn. */
function guess(cell: Cell): number {
  const lines = (s: string) => Math.min(400, s.split("\n").length);
  let n = lines(cell.source) + 1;
  if (cell.kind === "code") {
    for (const o of cell.outputs) {
      if (o.kind === "image") n += Math.min(30, (o.height ?? 300) / 20);
      else if (o.kind === "html") n += 12;
      else if ("text" in o) n += lines(o.text);
      else n += 2;
    }
  }
  return n;
}

/** The scroller a node sits in, so a cell is drawn as that box comes
    near it rather than as the window does. */
function scrollerOf(el: HTMLElement): HTMLElement | null {
  for (let p = el.parentElement; p; p = p.parentElement) {
    const y = getComputedStyle(p).overflowY;
    if (y === "auto" || y === "scroll") return p;
  }
  return null;
}

function Deferred({ eager, lines, line, kind, children }: { eager: boolean; lines: number; line?: number; kind: Cell["kind"]; children: () => ReactNode }) {
  const [on, setOn] = useState(eager);
  const at = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = at.current;
    if (on || !el) return;
    const io = new IntersectionObserver(
      (seen) => {
        if (seen.some((e) => e.isIntersecting)) {
          setOn(true);
          io.disconnect();
        }
      },
      { root: scrollerOf(el), rootMargin: AHEAD },
    );
    io.observe(el);
    return () => io.disconnect();
  }, [on]);
  return (
    <div
      ref={at}
      data-nb-cell={kind}
      data-nb-drawn={on || undefined}
      {...(line !== undefined ? { "data-src-line": String(line) } : {})}
      style={on ? undefined : { height: `calc(${lines} * var(--text-mono--line-height))` }}
    >
      {on && children()}
    </div>
  );
}

// ── ANSI ──────────────────────────────────────────────────────────

type Palette = Record<(typeof ANSI_KEYS)[number], string>;
const LIGHT = presets.latte.theme as Palette;
const DARK = presets.mocha.theme as Palette;

/** One colour for each ground, the way the terminal's 跟随 resolves
    them (`term-theme`'s `terminalTheme`): the first sixteen from the
    light and dark pair it uses, 16–255 from xterm's table, an RGB as
    written. `"fg"` is the text's own ink, which an inverse needs. */
function tones(c: Color): [string, string] {
  if (c === "fg") return ["var(--foreground)", "var(--foreground)"];
  if (typeof c === "string") return [c, c];
  if (c < 16) return [LIGHT[ANSI_KEYS[c]], DARK[ANSI_KEYS[c]]];
  const x = xterm256(c);
  return [x, x];
}

function Ansi({ text }: { text: string }) {
  const runs = useMemo(() => parseAnsi(text), [text]);
  if (runs.length === 1 && Object.keys(runs[0]).length === 1) return <>{runs[0].text}</>;
  return (
    <>
      {runs.map((r: Run, i) => {
        const style: Record<string, string> = {};
        if (r.fg !== undefined) [style["--ansi-l"], style["--ansi-d"]] = tones(r.fg);
        if (r.bg !== undefined) [style["--ansi-bl"], style["--ansi-bd"]] = tones(r.bg);
        const plain = r.fg === undefined && r.bg === undefined && !r.bold && !r.dim && !r.italic && !r.underline;
        if (plain) return <span key={i}>{r.text}</span>;
        return (
          <span
            key={i}
            data-ansi-fg={r.fg !== undefined ? "" : undefined}
            data-ansi-bg={r.bg !== undefined ? "" : undefined}
            data-ansi-dim={r.dim ? "" : undefined}
            className={[r.bold && "font-title", r.italic && "italic", r.underline && "underline"].filter(Boolean).join(" ") || undefined}
            style={style as CSSProperties}
          >
            {r.text}
          </span>
        );
      })}
    </>
  );
}

// ── cells ─────────────────────────────────────────────────────────

type Where = { machine: string; path: string };

/** `text/latex` as the math the file face reads: as written when it
    already says where its math is, else as one display block. */
function asMath(tex: string): string {
  const t = tex.trim();
  return /\$|\\\(|\\\[/.test(t) ? t : `$$\n${t}\n$$`;
}

/** The `data:` pictures in an HTML output, lifted out before the clean
    (which keeps only `https` addresses) and put back after, the way a
    cell's attachments are (`Markdown`'s `attach`). Only pictures, only
    in a quoted `src`. */
function lift(html: string): { html: string; pictures: string[] | null } {
  const pictures: string[] = [];
  const out = html.replace(/(\ssrc\s*=\s*)(["'])(data:image\/(?:png|jpe?g|gif|webp|svg\+xml)[;,][^"']*)\2/gi, (_m, pre: string, q: string, url: string) => {
    pictures.push(url);
    return `${pre}${q}${ATTACHED}${pictures.length - 1}${q}`;
  });
  return { html: out, pictures: pictures.length ? pictures : null };
}

function Output({ out, at }: { out: Out; at: Where }) {
  switch (out.kind) {
    case "image":
      return (
        // A plot saved with a clear ground (matplotlib's inline default)
        // is drawn on the light paper whatever the ground: its ink is
        // black, and on the dark ground it would vanish.
        <div data-nb-out="image" data-mime={out.mime} data-ground="light" className="w-fit max-w-full overflow-x-auto rounded-sm bg-background">
          <img src={out.src} alt="" loading="lazy" decoding="async" width={out.width} height={out.height} className="block h-auto max-w-full" />
        </div>
      );
    case "html":
      return <HtmlOut html={out.html} at={at} />;
    case "markdown":
      return (
        <div data-nb-out="markdown">
          <Markdown text={out.text} className="prose-khor" machine={at.machine} path={at.path} />
        </div>
      );
    case "latex":
      return (
        <div data-nb-out="latex">
          <Markdown text={asMath(out.text)} className="prose-khor" machine={at.machine} path={at.path} />
        </div>
      );
    case "json":
      return <pre data-nb-out="json">{out.text}</pre>;
    case "text":
      return (
        <>
          <pre data-nb-out="text">
            <Ansi text={out.text} />
          </pre>
          {out.script && <p data-nb-script className="text-callout text-muted-foreground">{gui.nb_needs_script}</p>}
        </>
      );
    case "script":
      return <p data-nb-out="script" data-nb-script className="text-callout text-muted-foreground">{gui.nb_needs_script}</p>;
    case "stream":
      return (
        <pre data-nb-out={out.name}>
          <Ansi text={out.text} />
        </pre>
      );
    case "error":
      return (
        <pre data-nb-out="error">
          <Ansi text={out.text || `${out.ename}: ${out.evalue}`} />
        </pre>
      );
  }
}

function HtmlOut({ html, at }: { html: string; at: Where }) {
  const lifted = useMemo(() => lift(html), [html]);
  return (
    <div data-nb-out="html" className="min-w-0 overflow-x-auto">
      <HtmlFragment html={lifted.html} pictures={lifted.pictures} machine={at.machine} path={at.path} />
    </div>
  );
}

function Code({ source, language }: { source: string; language: string | null }) {
  const [html, setHtml] = useState<string | null>(() => (language ? painted(source, language) : null));
  useEffect(() => {
    if (html !== null || !language) return;
    let live = true;
    void paint(source, language).then((h) => live && h !== null && setHtml(h));
    return () => {
      live = false;
    };
  }, [source, language, html]);
  // Shiki's markup, generated after escaping every character of the
  // cell (`lib/highlight`'s `paint`); the cell's own text never reaches
  // the DOM as markup.
  return html === null ? <code>{source}</code> : <code dangerouslySetInnerHTML={{ __html: html }} />;
}

function CellBody({ cell, language, at }: { cell: Cell; language: string | null; at: Where }) {
  if (cell.kind === "markdown") return <MarkdownCell cell={cell} at={at} />;
  if (cell.kind === "raw") return <pre data-nb-raw>{cell.source}</pre>;
  return (
    <>
      <span data-nb-prompt aria-hidden>
        [{cell.count ?? " "}]
      </span>
      <pre data-nb-input>
        <Code source={cell.source} language={language} />
      </pre>
      {cell.outputs.length > 0 && (
        <div data-nb-outputs>
          {cell.outputs.map((o, i) => (
            <Output key={i} out={o} at={at} />
          ))}
        </div>
      )}
    </>
  );
}

function MarkdownCell({ cell, at }: { cell: Extract<Cell, { kind: "markdown" }>; at: Where }) {
  const { text, pictures } = useMemo(
    () => (cell.attachments ? attach(cell.source, cell.attachments) : { text: cell.source, pictures: null }),
    [cell],
  );
  return <Markdown text={text} pictures={pictures} className="prose-khor" machine={at.machine} path={at.path} />;
}

export default function Notebook({ text, machine, path }: { text: string; machine: string; path: string }) {
  const read = useMemo(() => readNotebook(text), [text]);
  const lines = useMemo(() => (read.ok ? cellLines(text, read.nb.cells.length) : null), [text, read]);
  const at = useMemo(() => ({ machine, path }), [machine, path]);
  if (!read.ok) {
    return (
      <p data-nb-unread={read.why} className="px-6 py-4 text-callout text-muted-foreground">
        {read.why === "version" ? gui.nb_version(String(read.version)) : gui.nb_unreadable}
      </p>
    );
  }
  const language = read.nb.language ? languageOf(read.nb.language) : null;
  return (
    <div data-nb data-selectable className="px-6 py-4">
      {read.nb.cells.map((cell, i) => (
        <Deferred key={i} eager={i < EAGER} lines={guess(cell)} line={lines?.[i]} kind={cell.kind}>
          {() => <CellBody cell={cell} language={language} at={at} />}
        </Deferred>
      ))}
    </div>
  );
}
