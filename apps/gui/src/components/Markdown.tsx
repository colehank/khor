// Agents write markdown. Until this file, the conversation face painted
// their text as one pre-wrapped run: a table arrived as pipes, a code
// block as three backticks and its indentation, a numbered list as the
// numbers the model typed. The TUI renders all of it — so the chat face
// was the *worse* reading of the same words, which is the one thing it
// may not be (it is the default face; 会话身份批 ruling).
//
// Three judgments live here:
//
// - **A single newline breaks the line** (`remark-breaks`). CommonMark
//   folds it into a space; the face it replaces showed every newline.
//   Without this, switching to markdown would silently re-flow text
//   that used to have shape — a rendering change that reads as the
//   agent having said something else.
// - **No raw HTML.** `react-markdown` refuses it by default and nothing
//   here re-enables it: this is a *remote* agent's output painted into
//   the app's own window (`_cagent` relays whatever the model emits).
//   The one `dangerouslySetInnerHTML` below is not a hole in that and
//   the difference is worth stating precisely: what it sets is not the
//   agent's markup, it is markup highlight.js *generated* after escaping
//   every character the agent wrote. Nothing an agent types can reach
//   the DOM as an element, and that is asserted rather than believed —
//   the gate paints a block containing a script tag and reads back the
//   element count and the text (`scripts/check-read.mjs`).
// - **A link opens where this machine opens pages.** Not by navigating
//   this window (there is nothing here to navigate *to*) and not
//   through 借网 either: borrowing another machine's network is a
//   deliberate act with a machine to choose, while clicking a link in
//   a conversation is the ordinary one. The scheme is whitelisted on
//   the Rust side — the address came out of a model's words, and a
//   scheme this machine has registered can be an action rather than a
//   page (`khor_gui_core::web::open_link`). A refusal is not silent:
//   the label falls back to showing the address, which is what the
//   first version of this file did for every link.
//
// Streaming is safe by construction: an unterminated fence is a code
// block that runs to the end of the text (CommonMark), so a half-arrived
// block paints as a growing code block rather than as three backticks
// that later disappear. **That same fact is what keeps syntax colouring
// out of the streaming path**: the block that is still arriving is
// exactly the block whose fence has not closed, so it is left plain and
// the highlighter is called once, when the fence lands.
import { memo, useEffect, useMemo, useState, type ReactNode } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";

import { launchOn, openLink } from "@/api";
import { PathLink, remarkPaths } from "@/components/PathLinks";
import { useLinkHome } from "@/views/linkHome";
import { languageOf, openFenceAt, paint, painted } from "@/lib/highlight";
import { cn } from "@/lib/utils";

/** The shape every component override receives from react-markdown. */
type Node = { children?: ReactNode; className?: string };

/** What react-markdown hands down beside the props above. `code` is the
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

const parts: Components = {
  // The element `remarkPaths` makes. Not an HTML tag, so it is spelled
  // with a hyphen (the one shape a custom element may have) and the
  // table's type has to be widened for it — react-markdown's
  // `Components` is keyed by tag names it knows.
  ["khor-path" as keyof Components]: PathLink as Components[keyof Components],
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
  p: ({ children, node }) => <p {...at(node)} className="my-2 first:mt-0 last:mb-0">{children}</p>,
  // Three levels of heading, then flat: a chat turn that needs a fourth
  // is not a document the pane should try to typeset.
  h1: ({ children, node }) => <h1 {...at(node)} className="mt-3 mb-1 text-title-md font-title first:mt-0">{children}</h1>,
  h2: ({ children, node }) => <h2 {...at(node)} className="mt-3 mb-1 text-body font-title first:mt-0">{children}</h2>,
  h3: ({ children, node }) => <h3 {...at(node)} className="mt-2 mb-1 font-title first:mt-0">{children}</h3>,
  h4: ({ children, node }) => <h4 {...at(node)} className="mt-2 mb-1 font-title first:mt-0">{children}</h4>,
  h5: ({ children, node }) => <h5 {...at(node)} className="mt-2 mb-1 font-title first:mt-0">{children}</h5>,
  h6: ({ children, node }) => <h6 {...at(node)} className="mt-2 mb-1 font-title first:mt-0">{children}</h6>,
  ul: ({ children, node }) => <ul {...at(node)} className="my-1.5 list-disc space-y-0.5 pl-5">{children}</ul>,
  ol: ({ children, node }) => <ol {...at(node)} className="my-1.5 list-decimal space-y-0.5 pl-5">{children}</ol>,
  li: ({ children }) => <li>{children}</li>,
  strong: ({ children }) => <strong className="font-title">{children}</strong>,
  em: ({ children }) => <em className="italic">{children}</em>,
  // **The rule down the left is this element's alone now.** A tool call
  // in `ChatView` wore the same two utilities, down to the same border
  // and the same gutter, so a record of something the agent *did* and a
  // passage somebody *said* were one picture. The tool line gave the
  // shape up rather than tinting it: a left rule is what quotation has
  // always been marked with, and colour in this app is spoken for.
  blockquote: ({ children, node }) => (
    <blockquote {...at(node)} className="my-2 border-l-2 pl-3 text-muted-foreground">{children}</blockquote>
  ),
  hr: ({ node }) => <hr {...at(node)} className="my-3 border-t" />,
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
  pre: ({ children, node }) => (
    <pre {...at(node)} className="my-2 overflow-x-auto rounded-md border bg-muted px-3 py-2 font-mono text-mono">
      {children}
    </pre>
  ),
  // A table is the one part that can be wider than the pane; it scrolls
  // inside its own frame rather than pushing the conversation sideways.
  table: ({ children, node }) => (
    <div {...at(node)} className="my-2 overflow-x-auto">
      <table className="w-full border-collapse text-callout">{children}</table>
    </div>
  ),
  th: ({ children }) => <th className="border px-2 py-1 text-left font-title">{children}</th>,
  td: ({ children }) => <td className="border px-2 py-1 align-top">{children}</td>,
  a: ({ children, href }) => <Link href={href}>{children}</Link>,
  // An image is a remote fetch from an agent's text; its alt is the part
  // that was written here.
  img: ({ alt }) => <span className="text-muted-foreground">{alt || ""}</span>,
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
    // react-markdown marks a fenced block's code with `language-*`, and
    // an indented block with nothing — but an indented block's parent is
    // still `pre`, and there it must not wear the inline box either. The
    // `[pre_&]` variant asks the DOM instead of guessing from the class.
    const fenced = /language-/.test(className ?? "");
    const dress = cn(
      "font-mono text-mono",
      !fenced && "rounded-sm bg-muted px-1 py-0.5 [pre_&]:bg-transparent [pre_&]:p-0",
    );
    const lang = fenced ? langOf(className ?? "", node, openAt) : null;
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

/** The painted HTML for a closed, whitelisted block — else `null`. */
function langOf(className: string, node: Positioned["node"], openAt: number | null): string | null {
  if (openAt !== null) {
    const at = node?.position?.start?.offset;
    if (at === undefined || at >= openAt) return null;
  }
  return languageOf(/language-(\S+)/.exec(className)?.[1] ?? "");
}

/** A link in an agent's text.
 *
 * **Inside a workspace it opens on the machine the session is running
 * on** (工作区第五轮 第 35 条: 都在宿主机器上打开). That is the machine
 * the conversation happened on, and the one the agent was talking about
 * — this face can be a browser on a phone, and a phone is not where
 * anything in that sentence lives.
 *
 * Everywhere else it goes to this machine's own browser, which is what
 * it has always done and is still the right answer: a link written in a
 * conversation nobody has a project open for is a page to read, here.
 *
 * A refusal (a scheme that is not a page) puts the address on screen
 * instead of doing nothing — the person is then the one who decides
 * what it is. */
function Link({ href, children }: { href?: string; children?: ReactNode }) {
  const [refused, setRefused] = useState(false);
  const home = useLinkHome();
  if (!href || refused) {
    return (
      <span className="underline decoration-dotted underline-offset-2">
        {children}
        {href && typeof children === "string" && children !== href ? ` (${href})` : null}
      </span>
    );
  }
  return (
    <button
      type="button"
      data-md-link={href}
      title={href}
      // **The pointing hand, and one of only two in the app** (the other
      // is `PathLink`): a link in prose. Every control is the arrow, the
      // way AppKit draws it (design-system 30-behavior 选字与手).
      // **A `button` is an atomic box, so a long address in it wraps only
      // if it may break anywhere** (afix-a, `check-face` afix a: an agent's
      // URL ran 838 px past its paragraph): `anywhere`, and the wrapped
      // lines start on the left like the prose around them.
      className="cursor-pointer text-left underline underline-offset-2 [overflow-wrap:anywhere]"
      onClick={() =>
        (home === null ? openLink(href) : launchOn(home.machine, href)).catch(() =>
          setRefused(true),
        )
      }
    >
      {children}
    </button>
  );
}

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

export const Markdown = memo(function Markdown({
  text,
  className,
  lines = false,
}: {
  text: string;
  className?: string;
  /** Mark each top-level block with its source line (`rehypeSourceLines`). */
  lines?: boolean;
}) {
  // One table per text rather than one for the module, because `code`
  // has to be told where the unterminated fence is. It is rebuilt
  // exactly when the text changes — which is exactly when the parse
  // below runs anyway, so it rides along with work already being done
  // rather than adding a pass. The scan itself is one walk of the
  // lines, against a parse that is proportional to the same string.
  const components = useMemo(
    () => ({ ...parts, code: codePart(openFenceAt(text)) }),
    [text],
  );
  return (
    <div data-md data-selectable className={cn("min-w-0 break-words", className)}>
      <ReactMarkdown
        // **After gfm**, so the addresses it autolinks are already
        // `link` nodes and the path pass leaves them alone.
        remarkPlugins={[remarkGfm, remarkBreaks, remarkPaths]}
        rehypePlugins={lines ? [rehypeSourceLines] : []}
        components={components}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
});
