// A notebook as the preview draws it (lane nb-view): nbformat 4's JSON
// read into cells, and for each output the **one** representation that
// will be drawn.
//
// **Read-only, and nothing in it runs.** Not a kernel, not a widget,
// not the notebook's own JavaScript: an output that only a script can
// draw (Plotly, ipywidgets, Bokeh, `application/javascript`) is drawn
// from the picture or the text it was saved with, and a sentence says
// the rest needs a script.
//
// **Which representation: nbconvert's order, GitHub's result.** A
// bundle carries several, and Jupyter, nbconvert and GitHub all draw
// the first they can in a fixed order — HTML, Markdown, SVG, LaTeX,
// PNG, JPEG, then text. The one deviation: a bundle that also holds a
// script-only type has its HTML skipped too, because that HTML is the
// script's stage (a Plotly `<div>` and its `<script>`), and drawn
// without the script it is an empty box.
//
// nbformat 3 (2014 and before; worksheets, `input`, `prompt_number`)
// is not read: it is answered with a sentence and the source face is
// one press away.

export type Out =
  | { kind: "image"; mime: "image/png" | "image/jpeg" | "image/gif" | "image/svg+xml"; src: string; width?: number; height?: number }
  | { kind: "html"; mime: "text/html"; html: string }
  | { kind: "markdown"; mime: "text/markdown"; text: string }
  | { kind: "latex"; mime: "text/latex"; text: string }
  | { kind: "json"; mime: "application/json"; text: string }
  | { kind: "text"; mime: "text/plain"; text: string; script?: boolean }
  | { kind: "script"; mime: string }
  | { kind: "stream"; name: "stdout" | "stderr"; text: string }
  | { kind: "error"; ename: string; evalue: string; text: string };

export type Cell =
  | { kind: "markdown"; source: string; attachments?: Record<string, string> }
  | { kind: "code"; source: string; count: number | null; outputs: Out[] }
  | { kind: "raw"; source: string };

export type Notebook = { language: string | null; minor: number; cells: Cell[] };

export type Read = { ok: true; nb: Notebook } | { ok: false; why: "json" | "version" | "shape"; version?: number };

type Json = Record<string, unknown>;

const joined = (v: unknown): string => (Array.isArray(v) ? v.join("") : typeof v === "string" ? v : "");

/** Types only a script can draw. Their bundle's HTML is the script's
    stage, so it is skipped with them. */
export function needsScript(mime: string): boolean {
  return /^application\/(javascript|ecmascript|vnd\.(plotly|jupyter\.widget|bokehjs|holoviews|vega|vegalite|dataresource|nteract))/.test(mime) || mime === "text/javascript";
}

const IMAGE = ["image/svg+xml", "image/png", "image/jpeg", "image/gif"] as const;

/** nbconvert's `display_data_priority`, with GIF after JPEG and JSON
    before plain text. */
const ORDER = ["text/html", "text/markdown", "image/svg+xml", "text/latex", "image/png", "image/jpeg", "image/gif", "application/json", "text/plain"] as const;

function size(meta: unknown, mime: string): { width?: number; height?: number } {
  const m = (meta as Json | undefined)?.[mime] as Json | undefined;
  const n = (v: unknown) => (typeof v === "number" && v > 0 ? v : undefined);
  return { width: n(m?.width), height: n(m?.height) };
}

/** The picture as an address `<img>` can take. **An SVG too**: it is
    drawn through `<img>`, never as markup in the page, so its scripts
    and its links stay inside it (the same rule as an `.svg` file). */
function picture(mime: (typeof IMAGE)[number], raw: unknown): string {
  const data = joined(raw);
  if (mime === "image/svg+xml") return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(data)}`;
  return `data:${mime};base64,${data.replace(/\s+/g, "")}`;
}

/** The one representation of a bundle that is drawn. */
export function pick(data: Json, meta?: unknown): Out | null {
  const types = Object.keys(data);
  const script = types.find(needsScript);
  for (const mime of ORDER) {
    if (!(mime in data)) continue;
    if (mime === "text/html" && script) continue;
    const raw = data[mime];
    if (mime === "text/html") return { kind: "html", mime, html: joined(raw) };
    if (mime === "text/markdown") return { kind: "markdown", mime, text: joined(raw) };
    if (mime === "text/latex") return { kind: "latex", mime, text: joined(raw) };
    if (mime === "application/json") return { kind: "json", mime, text: JSON.stringify(raw, null, 2) };
    if (mime === "text/plain") return { kind: "text", mime, text: joined(raw), ...(script ? { script: true } : {}) };
    return { kind: "image", mime, src: picture(mime, raw), ...size(meta, mime) };
  }
  return script ? { kind: "script", mime: script } : null;
}

function output(o: Json): Out | null {
  const type = o.output_type;
  if (type === "stream") return { kind: "stream", name: o.name === "stderr" ? "stderr" : "stdout", text: joined(o.text) };
  if (type === "error") {
    const tb = Array.isArray(o.traceback) ? (o.traceback as unknown[]).map(String).join("\n") : "";
    return { kind: "error", ename: String(o.ename ?? ""), evalue: String(o.evalue ?? ""), text: tb };
  }
  if ((type === "execute_result" || type === "display_data") && o.data && typeof o.data === "object") return pick(o.data as Json, o.metadata);
  return null;
}

/** Consecutive writes to the same stream as one, the way Jupyter shows
    them: a progress bar is hundreds of `\r`-led writes, and only joined
    does its last state replace the ones before it. */
function merged(outs: Out[]): Out[] {
  const out: Out[] = [];
  for (const o of outs) {
    const last = out[out.length - 1];
    if (o.kind === "stream" && last?.kind === "stream" && last.name === o.name) out[out.length - 1] = { ...last, text: last.text + o.text };
    else out.push(o);
  }
  return out;
}

/** A markdown cell's `attachments`, as `name → data:` addresses. Only
    pictures; anything else in a bundle is not something `![](…)` can
    show. */
function attachments(raw: unknown): Record<string, string> | undefined {
  if (!raw || typeof raw !== "object") return undefined;
  const out: Record<string, string> = {};
  for (const [name, bundle] of Object.entries(raw as Json)) {
    if (!bundle || typeof bundle !== "object") continue;
    const mime = IMAGE.find((m) => m in (bundle as Json));
    if (mime) out[name] = picture(mime, (bundle as Json)[mime]);
  }
  return Object.keys(out).length ? out : undefined;
}

export function readNotebook(text: string): Read {
  let doc: unknown;
  try {
    doc = JSON.parse(text);
  } catch {
    return { ok: false, why: "json" };
  }
  if (!doc || typeof doc !== "object" || Array.isArray(doc)) return { ok: false, why: "shape" };
  const nb = doc as Json;
  const major = typeof nb.nbformat === "number" ? nb.nbformat : null;
  if (major !== 4) return major === null ? { ok: false, why: "shape" } : { ok: false, why: "version", version: major };
  if (!Array.isArray(nb.cells)) return { ok: false, why: "shape" };
  const meta = (nb.metadata ?? {}) as Json;
  const info = (meta.language_info ?? {}) as Json;
  const kernel = (meta.kernelspec ?? {}) as Json;
  const language = typeof info.name === "string" ? info.name : typeof kernel.language === "string" ? kernel.language : null;
  const cells: Cell[] = [];
  for (const c of nb.cells as unknown[]) {
    if (!c || typeof c !== "object") continue;
    const cell = c as Json;
    const source = joined(cell.source);
    if (cell.cell_type === "markdown") cells.push({ kind: "markdown", source, attachments: attachments(cell.attachments) });
    else if (cell.cell_type === "code") {
      const outs = Array.isArray(cell.outputs) ? (cell.outputs as unknown[]) : [];
      cells.push({
        kind: "code",
        source,
        count: typeof cell.execution_count === "number" ? cell.execution_count : null,
        outputs: merged(outs.flatMap((o) => (o && typeof o === "object" ? (output(o as Json) ?? []) : []))),
      });
    } else cells.push({ kind: "raw", source });
  }
  return { ok: true, nb: { language, minor: typeof nb.nbformat_minor === "number" ? nb.nbformat_minor : 0, cells } };
}
