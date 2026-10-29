import {assertPageUrl,openSystemUrl} from "@/lib/client-opener";
import {linkIntent} from "@/lib/content-link-intent";
// Agents write markdown. Until this file, the conversation face painted
// their text as one pre-wrapped run: a table arrived as pipes, a code
// block as three backticks and its indentation, a numbered list as the
// numbers the model typed. The TUI renders all of it — so the chat face
// was the *worse* reading of the same words, which is the one thing it
// may not be (it is the default face; 会话身份批 ruling).
//
// The judgments that live here:
//
// - **One renderer, two faces** (lane md-render, the owner's ruling: 统一
//   一套). The conversation and the file page draw through the same
//   remark/rehype pipeline and the same component table; what differs
//   is a handful of options, set in `kitOf` and nowhere else. A file is
//   read the way GitHub reads it; a conversation is read the way the
//   terminal showed it. Where those disagree is listed there.
// - **A single newline breaks the line — in the conversation**
//   (`remark-breaks`). CommonMark folds it into a space; the face it
//   replaces showed every newline. Without this, switching to markdown
//   would silently re-flow text that used to have shape — a rendering
//   change that reads as the agent having said something else. A file
//   folds it, as GitHub does: a README is wrapped at 80 columns by its
//   author and was never meant to break there.
// - **No raw HTML in the conversation.** The pipeline turns it back into
//   its own text (`lib/md/render`) and nothing on that face parses it:
//   this is a *remote*
//   agent's output painted into the app's own window (`_cagent` relays
//   whatever the model emits). The file page does render it, cleaned
//   with GitHub's own allow-list, from a chunk the conversation never
//   loads (`lib/md/file`). The one `dangerouslySetInnerHTML` below is
//   not a hole in either and the difference is worth stating precisely:
//   what it sets is not the agent's markup, it is markup the highlighter
//   *generated* after escaping every character the agent wrote. Nothing
//   an agent types can reach the DOM as an element, and that is
//   asserted rather than believed — the gates paint a block containing
//   a script tag and read back the element count and the text
//   (`scripts/check-read.mjs`, `scripts/check-md.mjs`).
// - **A link opens where this machine opens pages.** Not by navigating
//   this window (there is nothing here to navigate *to*) and not
//   through 借网 either: borrowing another machine's network is a
//   deliberate act with a machine to choose, while clicking a link in
//   a conversation is the ordinary one. The scheme is whitelisted on
//   the Rust side — the address came out of a model's words, and a
//   scheme this machine has registered can be an action rather than a
//   page (`khor_gui_core::web::open_link`). A refusal is not silent:
//   the label falls back to showing the address, which is what the
//   first version of this file did for every link. An address that is
//   only an anchor (`#安装`, a footnote) moves within the text instead.
//
// **Streaming.** A turn is cut into blocks and each block is drawn and
// memoised on its own, so a chunk re-parses only the block it lands in
// (`lib/md/blocks`); that block's half-written `**` or `` ` `` is closed
// for the frame (`mend`), so markers never flash before they turn into
// weight. An unterminated fence is a code block that runs to the end of
// the text (CommonMark), so a half-arrived block paints as a growing
// code block rather than as three backticks that later disappear. **That
// same fact is what keeps the heavy work out of the streaming path**:
// the block that is still arriving is exactly the block whose fence (or
// `$$`) has not closed, so it is left plain — no syntax colour, no
// formula, no diagram — and each of those is done once, when it lands.
import { createContext, memo, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { unified } from "unified";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";

import { fetchPreview } from "@/api";
import { onDisk } from "@/components/HtmlView";
import { MermaidBlock } from "@/components/Mermaid";
import { PathLink, remarkPaths } from "@/components/PathLinks";
import { gui } from "@/gen/catalog";
import { useLinkHome } from "@/views/linkHome";
import { languageOf, openFenceAt, paint, painted } from "@/lib/highlight";
import { rehypeAlerts } from "@/lib/md/alerts";
import { bracketMath, cut, dollarsPandoc, mathOpen, mend, needsOf, type Cut } from "@/lib/md/blocks";
import { useExtras, type Loaded } from "@/lib/md/extras";
import { pipeline, render, type Components, type Pipeline, type PluggableList } from "@/lib/md/render";
import type * as FileChunk from "@/lib/md/file";
import { cn } from "@/lib/utils";

/** The shape every component override receives from the renderer:
    the element's own properties, its children, and the hast node. */
type Node = { children?: ReactNode; className?: string; node?: unknown };

/** What the renderer hands down beside the props above. `code` is the
    one override here that needs it: `mdast-util-to-hast` patches the
    fenced block's source position onto this element, which is how a
    block that is still arriving is told from one that has closed. */
type Positioned = { node?: { position?: { start?: { offset?: number } } } };

/** The source line a top-level block was marked with (`rehypeSourceLines`),
    as the attribute the file page reads; nothing when not marked. */
const at = (node: unknown) => {
  const line = (node as { properties?: { dataSrcLine?: unknown } } | undefined)?.properties?.dataSrcLine;
  return line === undefined ? {} : { "data-src-line": String(line) };
};

/** An element's own properties, passed through. Each override below used
    to keep only its children, which lost what the parser had said about
    the element: a list's `start` (「3.」 renumbered from 1), a table
    cell's alignment, a heading's anchor, the footnotes' ids, and on the
    file page the `align` a README centres its logo with. */
function own<P extends { node?: unknown; className?: string; children?: ReactNode }>(
  { node, className, children: _children, ...rest }: P,
  dress?: string,
) {
  return { ...rest, ...at(node), className: cn(dress, className) || undefined };
}

const parts: Components = {
  // The element `remarkPaths` makes. Not an HTML tag, so it is spelled
  // with a hyphen (the one shape a custom element may have) and the
  // table's type has to be widened for it — `Components` is keyed by
  // the tag names React knows.
  ["khor-path" as "span"]: PathLink as unknown as Components["span"],
  // **No leading of its own.** It used to carry `leading-relaxed`, which
  // is a ratio and therefore a second opinion about the line height for
  // whatever size the pane happened to hand down — the thing the type
  // scale forbids ("a size that floats between leadings is two sizes
  // wearing one name", tokens.css). The reading size arrives with the
  // leading it was chosen with; a paragraph's job is not to re-decide it.
  //
  // The gap between paragraphs grew with that leading: at the old one it
  // was a third of a line, which is barely a gap, and blocks of prose
  // this long need to be told apart from the lines inside them.
  p: (e) => <p {...own(e, "my-2 first:mt-0 last:mb-0")}>{e.children}</p>,
  // Three levels of heading, then flat: a chat turn that needs a fourth
  // is not a document the pane should try to typeset.
  h1: (e) => <h1 {...own(e, "mt-3 mb-1 text-title-md font-title first:mt-0")}>{e.children}</h1>,
  // The footnotes' heading is for a screen reader: the rule above the
  // notes and their numbers already say what they are (GitHub hides it
  // the same way).
  h2: (e) => <h2 {...own(e, cn("mt-3 mb-1 text-body font-title first:mt-0", e.id === "footnote-label" && "sr-only"))}>{e.children}</h2>,
  h3: (e) => <h3 {...own(e, "mt-2 mb-1 font-title first:mt-0")}>{e.children}</h3>,
  h4: (e) => <h4 {...own(e, "mt-2 mb-1 font-title first:mt-0")}>{e.children}</h4>,
  h5: (e) => <h5 {...own(e, "mt-2 mb-1 font-title first:mt-0")}>{e.children}</h5>,
  h6: (e) => <h6 {...own(e, "mt-2 mb-1 font-title first:mt-0")}>{e.children}</h6>,
  ul: (e) => <ul {...own(e, "my-1.5 list-disc space-y-0.5 pl-5")}>{e.children}</ul>,
  ol: (e) => <ol {...own(e, "my-1.5 list-decimal space-y-0.5 pl-5")}>{e.children}</ol>,
  li: (e) => <li {...own(e)}>{e.children}</li>,
  // A task list's box is the writer's record, not a control: it shows
  // the state and cannot be changed here.
  input: (e) => <input {...own(e)} readOnly />,
  strong: (e) => <strong {...own(e, "font-title")}>{e.children}</strong>,
  em: (e) => <em {...own(e, "italic")}>{e.children}</em>,
  // **The rule down the left is this element's alone now.** A tool call
  // in `ChatView` wore the same two utilities, down to the same border
  // and the same gutter, so a record of something the agent *did* and a
  // passage somebody *said* were one picture. The tool line gave the
  // shape up rather than tinting it: a left rule is what quotation has
  // always been marked with, and colour in this app is spoken for.
  blockquote: (e) => (
    <blockquote {...own(e, "my-2 border-l-2 pl-3 text-muted-foreground")}>{e.children}</blockquote>
  ),
  hr: (e) => <hr {...own(e, "my-3 border-t")} />,
  // `pre` is the block's frame — the scroller, the paper and the corner.
  // Its `code` child drops back to a bare run so the inline treatment
  // below does not paint a second box inside this one.
  //
  // **An edge and room to breathe, with no library added.** The paper
  // alone is a faint tint, and against the reading column it read as a
  // paragraph that had gone slightly grey rather than as a different
  // kind of thing; a border draws the boundary the tint was failing to.
  // The inner room is asymmetric on purpose: the left gutter is where
  // the eye lands looking for the start of each line of code, and it was
  // the same as the gap above, which made the first line sit in the
  // corner.
  //
  // (This paragraph used to end「no highlighting, so nothing here
  // colours a token or parses a language」. That was true of the batch
  // that wrote it and stopped being true one batch later — the sentence
  // is kept as its own correction because a note that describes the
  // *previous* state reads exactly like a note describing this one.)
  pre: (e) => (
    <pre {...own(e, "my-2 overflow-x-auto rounded-md border bg-muted px-3 py-2 font-mono text-mono")}>
      {e.children}
    </pre>
  ),
  // A table is the one part that can be wider than the pane; it scrolls
  // inside its own frame rather than pushing the conversation sideways.
  table: ({ node, ...e }) => (
    <div {...at(node)} className="my-2 overflow-x-auto">
      <table {...own(e, "w-full border-collapse text-callout")}>{e.children}</table>
    </div>
  ),
  // A column's alignment arrives as the cell's own style (`:--:`), which
  // outranks the class: left is only the default.
  th: (e) => <th {...own(e, "border px-2 py-1 text-left font-title")}>{e.children}</th>,
  td: (e) => <td {...own(e, "border px-2 py-1 align-top")}>{e.children}</td>,
  a: ({ node: _node, children, ...rest }) => <Link {...rest}>{children}</Link>,
  // In a conversation an image is a remote fetch from an agent's text;
  // its alt is the part that was written here. On the file page a
  // picture beside the file is read from its machine (`FileImage`).
  img: (e) => <Picture {...(e as Pic)} />,
};

/**
 * The `code` override, which is the only one that has to know something
 * about the *whole* text: where the unterminated fence starts, if there
 * is one. Hence a factory rather than an entry in the table above.
 *
 * **A block is painted only once it has closed.** `openAt` is the offset
 * an unterminated fence begins at (`lib/highlight`), and a block at or
 * after it is still arriving — its language may be half-typed and its
 * last line is half a token. Skipping it is not only correctness: it is
 * what keeps the highlighter out of the streaming path entirely, so a
 * turn that drips twenty thousand characters calls it zero times and
 * then once.
 *
 * Where the position is missing it is treated as still arriving, which
 * is the conservative direction — plain rather than a guess. That
 * cannot fail silently: the gate asserts a *closed* block does get
 * classes, so a build where positions stopped arriving goes red rather
 * than quietly going plain (`scripts/check-read.mjs`).
 */
const codePart =
  (openAt: number | null): Components["code"] =>
  ({ children, className, node }: Node & Positioned) => {
    // remark marks a fenced block's code with `language-*`, and
    // an indented block with nothing — but an indented block's parent is
    // still `pre`, and there it must not wear the inline box either. The
    // `[pre_&]` variant asks the DOM instead of guessing from the class.
    const fenced = /language-/.test(className ?? "");
    const dress = cn(
      "font-mono text-mono",
      !fenced && "rounded-sm bg-muted px-1 py-0.5 [pre_&]:bg-transparent [pre_&]:p-0",
    );
    const label = fenced ? (/language-(\S+)/.exec(className ?? "")?.[1] ?? "") : "";
    const closed = fenced && isClosed(node, openAt);
    // A diagram is drawn by the same rule a block is coloured by: once
    // its fence has closed, and not a chunk before.
    if (closed && label.toLowerCase() === "mermaid") {
      return (
        <MermaidBlock code={String(children ?? "").replace(/\n$/, "")}>
          <code className={dress}>{children}</code>
        </MermaidBlock>
      );
    }
    const lang = closed ? languageOf(label) : null;
    return lang === null ? (
      <code className={dress}>{children}</code>
    ) : (
      <Painted code={String(children ?? "")} lang={lang} dress={dress}>
        {children}
      </Painted>
    );
  };

/**
 * A closed fence, painted when its grammar has arrived.
 *
 * **Text first, paint second.** Shiki loads a grammar on the first
 * block in a language, so a block draws as plain text on the frame it
 * lands and takes the paint when the promise settles; one already
 * painted (`painted`) never passes through the plain frame, which is
 * what keeps a long answer's early blocks steady while its prose keeps
 * arriving (the cache's reason, `lib/highlight.ts`).
 *
 * **The one place this file emits markup, and the ban it looks like it
 * breaks is intact.** The module head refuses raw HTML because the text
 * is a remote agent's; this is not the agent's markup, it is markup the
 * highlighter generated *from* the agent's text after escaping every
 * character of it. The difference is measured, not asserted: the gate
 * paints a block containing a script tag and an event handler and reads
 * back both the element count and the text, character for character.
 */
function Painted({
  code,
  lang,
  dress,
  children,
}: {
  code: string;
  lang: string;
  dress: string;
  children?: ReactNode;
}) {
  const [html, setHtml] = useState<string | null>(() => painted(code, lang));
  useEffect(() => {
    if (html !== null) return;
    let live = true;
    void paint(code, lang).then((h) => live && h !== null && setHtml(h));
    return () => {
      live = false;
    };
  }, [code, lang, html]);
  return html === null ? (
    <code className={dress}>{children}</code>
  ) : (
    <code className={dress} dangerouslySetInnerHTML={{ __html: html }} />
  );
}

/** Whether a fenced block has closed: it starts before the fence that
    is still open, if there is one. */
function isClosed(node: Positioned["node"], openAt: number | null): boolean {
  if (openAt === null) return true;
  const at = node?.position?.start?.offset;
  return at !== undefined && at < openAt;
}

/** A session link follows the user's destination intent. Local anchors
 * stay within the rendered document; other contexts retain system opening. */
function Link({ href, children, className: _className, ...rest }: { href?: string; children?: ReactNode; [prop: string]: unknown }) {
  const [refused, setRefused] = useState(false);
  const home = useLinkHome();
  if (!href || refused) {
    return (
      <span {...rest} className="underline decoration-dotted underline-offset-2">
        {children}
        {href && typeof children === "string" && children !== href ? ` (${href})` : null}
      </span>
    );
  }
  const anchor = href.startsWith("#");
  return (
    <button
      type="button"
      // The element's own attributes ride along: a footnote reference's
      // `id` is what its note's back-link jumps to.
      {...rest}
      data-md-link={href}
      title={typeof rest.title === "string" ? rest.title : href}
      // **The pointing hand, and one of only two in the app** (the other
      // is `PathLink`): a link in prose. Every control is the arrow, the
      // way AppKit draws it (design-system 30-behavior 选字与手).
      // **A `button` is an atomic box, so a long address in it wraps only
      // if it may break anywhere** (afix-a, `check-face` afix a: an agent's
      // URL ran 838 px past its paragraph): `anywhere`, and the wrapped
      // lines start on the left like the prose around them.
      className="cursor-pointer text-left underline underline-offset-2 [overflow-wrap:anywhere]"
      onClick={async (e) => {
        if (anchor) { jump(e.currentTarget, href); return; }
        try {
          assertPageUrl(href);
          if (linkIntent(e) === "internal" && home?.openPage) await home.openPage(href);
          else await openSystemUrl(href);
        } catch { setRefused(true); }
      }}
    >
      {children}
    </button>
  );
}

/** An anchor, followed inside the text it was written in: a heading
    (`rehype-slug`) or a footnote. What the author wrote as an `id` came
    through the file page's clean as `user-content-…` (GitHub's rule), so
    both spellings are looked for; nothing outside this text is. */
function jump(from: Element, href: string) {
  let id = href.slice(1);
  try {
    id = decodeURIComponent(id);
  } catch {
    // Written undecodable: looked for as written.
  }
  const want = new Set([id, `user-content-${id}`, id.replace(/^user-content-/, "")]);
  const root = from.closest("[data-md]");
  for (const el of root?.querySelectorAll<HTMLElement>("[id]") ?? []) {
    if (want.has(el.id)) {
      el.scrollIntoView({ block: "start" });
      return;
    }
  }
}

/** The file whose text this is, for the pictures beside it; `null` in a
    conversation. `asked` is this text's budget of reads. */
const FileAt = createContext<{ machine: string; dir: string; asked: Set<string> } | null>(null);

/** At most this many pictures are read for one text (HtmlView's cap). */
const FETCH_MAX = 24;
/** A picture that has not come back by then is left as its alt. */
const FETCH_MS = 8000;
/** Pictures already read, by machine and path: a preview drawn again
    (a re-render, the history's next version) does not read them again.
    `null` is a picture that could not be read. Bounded like the
    highlighter's cache. */
const READ = new Map<string, string | null>();
const READ_CAP = 64;

type Pic = { src?: string; alt?: string; node?: unknown; [prop: string]: unknown };

/**
 * **Pictures that travel inside the text** (lane nb-view): a notebook
 * cell's `attachments` (`![](attachment:plot.png)`) and the `data:`
 * pictures in a `text/html` output. Neither address survives the clean
 * — `attachment:` is no scheme at all and `data:` is not `https` — and
 * neither should be let through it in general, so the text is given a
 * relative stand-in (`ATTACHED` and a number) before it is parsed and
 * the picture is looked up here after. The bytes never went anywhere:
 * they are the file's own.
 */
const Attached = createContext<readonly string[] | null>(null);
export const ATTACHED = "?khor-attached=";

function Picture(e: Pic) {
  const file = useContext(FileAt);
  const attached = useContext(Attached);
  if (attached !== null && e.src?.startsWith(ATTACHED)) {
    const url = attached[Number(e.src.slice(ATTACHED.length))];
    const { node: _node, src: _src, ...rest } = e;
    // On the light paper whatever the ground: a plot saved with a clear
    // ground has black ink (`Notebook`'s image outputs do the same).
    if (url)
      return (
        <span data-ground="light" className="inline-block max-w-full rounded-sm bg-background align-top">
          <img {...rest} src={url} alt={e.alt ?? ""} decoding="async" className="block" />
        </span>
      );
  }
  return file === null ? <span className="text-muted-foreground">{e.alt || ""}</span> : <FileImage {...e} />;
}

/** `attachment:name` in a notebook cell's text, for each name it
    carries, as its stand-in; the pictures in the same order. A name is
    matched as written and percent-encoded, which is how Jupyter writes
    one with a space. */
export function attach(text: string, attachments: Record<string, string>): { text: string; pictures: string[] } {
  const pictures: string[] = [];
  let out = text;
  for (const [name, url] of Object.entries(attachments)) {
    const at = `${ATTACHED}${pictures.length}`;
    pictures.push(url);
    for (const spelled of new Set([name, encodeURI(name), encodeURIComponent(name)])) {
      out = out.split(`attachment:${spelled}`).join(at);
    }
  }
  return { text: out, pictures };
}

/**
 * A picture in a file. **Beside the file**, it is read from the machine
 * the file is on — the way the HTML preview reads its pictures
 * (`HtmlView`): relative to the file, through the file API, inlined as
 * `data:`. **On the network**, it is fetched by this machine if the
 * address is `https` and not at all if it is `http` (the owner's ruling,
 * 2026-10-19, VS Code's Strict default: a README's badges show, nothing
 * goes out in the clear). The clean already took `http` away
 * (`lib/md/file`); the test here is the same rule again for a file drawn
 * without the clean, which happens only when its chunk failed to load.
 * One that cannot be read — too large, not there, too slow — stays its
 * alt.
 */
function FileImage(e: Pic) {
  return /^https:\/\//i.test(e.src ?? "") ? <WebImage {...e} /> : <DiskImage {...e} />;
}

function WebImage({ src, alt, node: _node, ...rest }: Pic) {
  const [broken, setBroken] = useState(false);
  if (broken) return <span className="text-muted-foreground">{alt || ""}</span>;
  return <img {...rest} src={src} alt={alt ?? ""} referrerPolicy="no-referrer" loading="lazy" onError={() => setBroken(true)} />;
}

function DiskImage({ src, alt, node: _node, ...rest }: Pic) {
  const file = useContext(FileAt)!;
  const path = src ? onDisk(file.dir, src) : null;
  const key = path === null ? null : `${file.machine}\u0000${path}`;
  const [url, setUrl] = useState<string | null | undefined>(() => (key === null ? null : READ.get(key)));
  useEffect(() => {
    if (key === null || path === null) return setUrl(null);
    if (READ.has(key)) return setUrl(READ.get(key));
    if (!file.asked.has(key) && file.asked.size >= FETCH_MAX) return setUrl(null);
    file.asked.add(key);
    let live = true;
    const late = new Promise<undefined>((r) => setTimeout(() => r(undefined), FETCH_MS));
    void Promise.race([
      fetchPreview(file.machine, path).then(
        (p) => (!p.cut && p.data !== "" ? `data:${p.media};base64,${p.data}` : null),
        () => null,
      ),
      late,
    ]).then((u) => {
      // Too slow is not remembered: the next look may be quicker.
      if (u !== undefined) {
        if (READ.size >= READ_CAP) READ.delete(READ.keys().next().value as string);
        READ.set(key, u);
      }
      if (live) setUrl(u ?? null);
    });
    return () => {
      live = false;
    };
  }, [file, key, path]);
  if (!url) return <span className="text-muted-foreground">{alt || ""}</span>;
  return <img {...rest} src={url} alt={alt ?? ""} />;
}

/** **Each block says which source line it starts on** (round UX-3, the
    file page): a top-level block gets `data-src-line`, so the page can
    find the line a reader was at in the rendered file and open the editor
    there, and come back to the same block after saving. Positions are
    the parser's own, so nothing is counted twice. */
function rehypeSourceLines() {
  type Hast = { type: string; tagName?: string; properties?: Record<string, unknown>; position?: { start: { line: number } }; children?: Hast[] };
  return (tree: Hast) => {
    for (const kid of tree.children ?? []) {
      if (kid.type === "element" && kid.position) {
        kid.properties = { ...kid.properties, dataSrcLine: kid.position.start.line };
      }
    }
  };
}

/** Counted for the gate (`scripts/check-md.mjs`): blocks drawn. */
export const measured = { blocks: 0 };

/** What one face draws with: the parser's plugins, the passes over the
    tree, and the component table. Made once per face and per set of
    loaded chunks, so a block memoised on it stays put while text
    arrives. */
type Kit = {
  /** For a block that has closed: everything. */
  closed: Pipeline;
  /** For the block still arriving: no formula is laid out. */
  open: Pipeline;
  components: Components;
};

/**
 * **The one place the two faces differ** (统一一套, the owner's ruling).
 *
 * | | conversation | file |
 * |---|---|---|
 * | single newline | a line break | a space (GitHub) |
 * | `~one~` | text (「3~5 天」) | struck (GitHub) |
 * | `$x$` | math by pandoc's rule (`dollarsPandoc`): 「$5 和 $10」 text | math (GitHub) |
 * | `$$x$$`, ```` ```math ```` | math | math |
 * | `\\( \\)`, `\\[ \\]` | math (`bracketMath`) | text (GitHub) |
 * | raw HTML | text | GitHub's allow-list (`lib/md/file`) |
 * | heading anchors | none | GitHub's slugs |
 * | pictures | alt | read from the file's machine |
 * | arriving text | cut into blocks, tail mended | whole |
 *
 * Everything else — the components, alerts, footnotes, task lists,
 * emoji short codes, code colour, diagrams — is the same code.
 */
function kitOf(doc: boolean, lines: boolean, loaded: Loaded): Kit {
  const { math, file, emoji } = loaded;
  const alerts = rehypeAlerts({
    note: gui.md_note,
    tip: gui.md_tip,
    important: gui.md_important,
    warning: gui.md_warning,
    caution: gui.md_caution,
  });
  const remark: PluggableList = doc
    ? [remarkGfm, ...(math ? [math.remarkMath] : [])]
    : [
        [remarkGfm, { singleTilde: false }],
        ...(math ? [math.remarkMath] : []),
        remarkBreaks,
      ];
  if (emoji) remark.push(emoji);
  // **After gfm**, so the addresses it autolinks are already `link`
  // nodes and the path pass leaves them alone.
  remark.push(remarkPaths);
  const clean: PluggableList =
    doc && file ? [file.rehypeRaw, [file.rehypeSanitize, file.schema], file.rehypeSlug] : [];
  const tail = lines ? [rehypeSourceLines] : [];
  const bridge = {
    footnoteLabel: gui.md_footnotes,
    footnoteBackLabel: gui.md_footnote_back,
    // The clean prefixes ids itself (GitHub's `user-content-`); the
    // bridge doing it too would make it twice.
    ...(doc && file ? { clobberPrefix: "" } : {}),
  };
  return {
    closed: pipeline(remark, bridge, [...clean, alerts, ...(math ? [math.rehypeKatex] : []), ...tail]),
    open: pipeline(remark, bridge, [...clean, alerts, ...tail]),
    components: parts,
  };
}

/** Finished blocks are drawn in runs of about this many characters. */
const RUN = 2000;

/**
 * The blocks a turn is drawn as: its finished blocks in runs of about
 * `RUN` characters, and the run still growing with its tail mended
 * (`mend`). **Why runs and not single blocks.** Every parse pays a fixed
 * price before its first character (the parser's state, the tree's
 * conversion), and a 50 000-character turn is three hundred blocks: drawn
 * one by one, the finished turn took half as long again to appear as
 * drawn whole. Runs are cut greedily from the start over finished blocks
 * only, so a run, once closed, never changes and its memo holds; the
 * last run is re-parsed per chunk, which is two thousand characters, not
 * the turn.
 */
function grouped({ blocks, shown, code }: Cut): string[] {
  const out: string[] = [];
  let run = "";
  for (let i = 0; i < blocks.length - 1; i++) {
    run += shown[i];
    if (run.length >= RUN) {
      out.push(run);
      run = "";
    }
  }
  // The tail is mended before its dollars are read, so a backtick still
  // open is closed and the dollars inside it are seen as code.
  const last = blocks.length - 1;
  if (last >= 0) out.push(run + (code[last] ? blocks[last] : dollarsPandoc(mend(blocks[last]))));
  return out;
}

/**
 * One block of a text, drawn on its own. **Memoised on its text and the
 * kit**, so a block above the one still arriving is drawn once and left
 * alone; the gate counts this (`measured.blocks`).
 *
 * `code` is the one override that has to know where this block's
 * unterminated fence starts, if it has one — hence rebuilt per block —
 * and a block whose fence or `$$` is still open draws no formula.
 */
const Block = memo(function Block({ text, kit }: { text: string; kit: Kit }) {
  measured.blocks += 1;
  const openAt = openFenceAt(text);
  const components = useMemo(() => ({ ...kit.components, code: codePart(openAt) }), [kit, openAt]);
  return render(openAt === null && !mathOpen(text) ? kit.closed : kit.open, text, components);
});

/**
 * **Memoised, and that is load-bearing rather than tidy.** Parsing is
 * proportional to the whole string, and the conversation hands this
 * component one block per turn: without the guard, every poll re-parsed
 * every block that had not changed — a turn that streamed twenty
 * thousand characters was re-parsed from the top for each new chunk,
 * and again for each chunk of every turn after it.
 *
 * Measured on a 2044-frame conversation at the fast cadence, with the
 * pane's own arithmetic already made incremental — so the gap is
 * re-parsing and nothing else: the longest main-thread task while a
 * turn dripped fell from 21.3 ms to 8.7 ms when this wrapper went on.
 * (The lag is quoted rather than the end-to-end latency because it is
 * sampled thousands of times a run and the latency twenty times.)
 *
 * Both props are plain values, so the default shallow compare is the
 * right one: a block is the same block exactly when its text is.
 */
export const Markdown = memo(function Markdown({
  text,
  className,
  lines = false,
  machine,
  path,
  pictures = null,
}: {
  text: string;
  className?: string;
  /** Mark each top-level block with its source line (`rehypeSourceLines`). */
  lines?: boolean;
  /** The file this text is, when it is one: the file page's face, and
      where its pictures are read from. Two plain values rather than one
      object, so the memo's shallow compare still holds. */
  machine?: string;
  path?: string;
  /** The pictures `attach` stood in for, by number. */
  pictures?: readonly string[] | null;
}) {
  const doc = path !== undefined;
  const needs = useMemo(() => needsOf(text, doc), [text, doc]);
  const { loaded, version, ready } = useExtras(needs, doc);
  // `version` stands for `loaded`, which is one object that grows.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const kit = useMemo(() => kitOf(doc, lines, loaded), [doc, lines, version]);
  const file = useMemo(
    () => (path === undefined ? null : { machine: machine ?? "", dir: path.slice(0, path.lastIndexOf("/")) || "/", asked: new Set<string>() }),
    [machine, path],
  );
  // The last cut, so the next (longer) text re-lexes only its tail.
  const last = useRef<Cut | null>(null);
  const blocks = useMemo(() => {
    if (doc) return [text];
    last.current = cut(bracketMath(text), last.current);
    return grouped(last.current);
  }, [text, doc]);
  return (
    <div data-md data-selectable className={cn("min-w-0 break-words", className)}>
      {/* The file page waits for its own chunk (HTML, anchors) rather
          than drawing the source once and the page a moment later. */}
      {(!doc || ready) && (
        <FileAt.Provider value={file}>
          <Attached.Provider value={pictures}>
            {blocks.map((b, i) => (
              <Block key={i} text={b} kit={kit} />
            ))}
          </Attached.Provider>
        </FileAt.Provider>
      )}
    </div>
  );
});

const NO_NEEDS = { math: false, emoji: false };
const fragments = new WeakMap<typeof FileChunk, Pipeline>();

/**
 * **HTML that is not markdown, through the same clean and the same parts**
 * (lane nb-view): a notebook's `text/html` output — a pandas table, a
 * styled summary. Parsed as a fragment, cleaned with the file face's
 * schema (GitHub's allow-list: no script, no `on*`, no `style`, no
 * `iframe`, no form) narrowed to no network picture at all
 * (`fragmentSchema`), drawn with the component table above, so its
 * table is the README's table and its links open where this machine
 * opens links.
 *
 * Inline rather than in a sandboxed frame: a frame with an empty
 * allow-list cannot say how tall its content is (no script inside, no
 * reach in from here), and hundreds of them would be hundreds of
 * documents. What a frame would keep and this drops is the output's own
 * `<style>` — which is GitHub's result too.
 */
export const HtmlFragment = memo(function HtmlFragment({
  html,
  machine,
  path,
  pictures = null,
}: {
  html: string;
  machine: string;
  path: string;
  pictures?: readonly string[] | null;
}) {
  const { loaded, ready } = useExtras(NO_NEEDS, true);
  const chunk = loaded.file;
  const p = useMemo(() => {
    if (!chunk) return null;
    let made = fragments.get(chunk);
    if (!made) {
      made = unified().use(chunk.rehypeParseFragment).use(chunk.rehypeSanitize, chunk.fragmentSchema).freeze() as unknown as Pipeline;
      fragments.set(chunk, made);
    }
    return made;
  }, [chunk]);
  const file = useMemo(() => ({ machine, dir: path.slice(0, path.lastIndexOf("/")) || "/", asked: new Set<string>() }), [machine, path]);
  const drawn = useMemo(() => (p === null ? null : render(p, html, parts)), [p, html]);
  return (
    <div data-md data-nb-html className="min-w-0 break-words">
      {ready && (
        <FileAt.Provider value={file}>
          <Attached.Provider value={pictures}>{drawn}</Attached.Provider>
        </FileAt.Provider>
      )}
    </div>
  );
});
