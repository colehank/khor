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
// that later disappear.
import { useState, type ReactNode } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";

import { openLink } from "@/api";
import { cn } from "@/lib/utils";

/** The shape every component override receives from react-markdown. */
type Node = { children?: ReactNode; className?: string };

const parts: Components = {
  p: ({ children }) => <p className="my-1.5 first:mt-0 last:mb-0 leading-relaxed">{children}</p>,
  // Three levels of heading, then flat: a chat turn that needs a fourth
  // is not a document the pane should try to typeset.
  h1: ({ children }) => <h1 className="mt-3 mb-1 text-lg font-semibold first:mt-0">{children}</h1>,
  h2: ({ children }) => <h2 className="mt-3 mb-1 text-base font-semibold first:mt-0">{children}</h2>,
  h3: ({ children }) => <h3 className="mt-2 mb-1 font-semibold first:mt-0">{children}</h3>,
  h4: ({ children }) => <h4 className="mt-2 mb-1 font-semibold first:mt-0">{children}</h4>,
  h5: ({ children }) => <h5 className="mt-2 mb-1 font-semibold first:mt-0">{children}</h5>,
  h6: ({ children }) => <h6 className="mt-2 mb-1 font-semibold first:mt-0">{children}</h6>,
  ul: ({ children }) => <ul className="my-1.5 list-disc space-y-0.5 pl-5">{children}</ul>,
  ol: ({ children }) => <ol className="my-1.5 list-decimal space-y-0.5 pl-5">{children}</ol>,
  li: ({ children }) => <li className="leading-relaxed">{children}</li>,
  strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
  em: ({ children }) => <em className="italic">{children}</em>,
  blockquote: ({ children }) => (
    <blockquote className="my-1.5 border-l-2 pl-2 text-muted-foreground">{children}</blockquote>
  ),
  hr: () => <hr className="my-3 border-t" />,
  // `pre` is the block's frame — the scroller, the paper and the corner.
  // Its `code` child drops back to a bare run so the inline treatment
  // below does not paint a second box inside this one.
  pre: ({ children }) => (
    <pre className="my-2 overflow-x-auto rounded-md bg-muted p-2 font-mono text-xs leading-normal">
      {children}
    </pre>
  ),
  code: ({ children, className }: Node) => {
    // react-markdown marks a fenced block's code with `language-*`, and
    // an indented block with nothing — but an indented block's parent is
    // still `pre`, and there it must not wear the inline box either. The
    // `[pre_&]` variant asks the DOM instead of guessing from the class.
    const fenced = /language-/.test(className ?? "");
    return (
      <code
        className={cn(
          "font-mono text-xs",
          !fenced && "rounded-sm bg-muted px-1 py-0.5 [pre_&]:bg-transparent [pre_&]:p-0",
        )}
      >
        {children}
      </code>
    );
  },
  // A table is the one part that can be wider than the pane; it scrolls
  // inside its own frame rather than pushing the conversation sideways.
  table: ({ children }) => (
    <div className="my-2 overflow-x-auto">
      <table className="w-full border-collapse text-xs">{children}</table>
    </div>
  ),
  th: ({ children }) => <th className="border px-2 py-1 text-left font-semibold">{children}</th>,
  td: ({ children }) => <td className="border px-2 py-1 align-top">{children}</td>,
  a: ({ children, href }) => <Link href={href}>{children}</Link>,
  // An image is a remote fetch from an agent's text; its alt is the part
  // that was written here.
  img: ({ alt }) => <span className="text-muted-foreground">{alt || ""}</span>,
};

/** A link in an agent's text: pressed, it goes to this machine's own
    browser. A refusal (a scheme that is not a page) puts the address on
    screen instead of doing nothing — the person is then the one who
    decides what it is. */
function Link({ href, children }: { href?: string; children?: ReactNode }) {
  const [refused, setRefused] = useState(false);
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
      className="cursor-pointer underline underline-offset-2"
      onClick={() => openLink(href).catch(() => setRefused(true))}
    >
      {children}
    </button>
  );
}

export function Markdown({ text, className }: { text: string; className?: string }) {
  return (
    <div data-md className={cn("min-w-0 break-words", className)}>
      <ReactMarkdown remarkPlugins={[remarkGfm, remarkBreaks]} components={parts}>
        {text}
      </ReactMarkdown>
    </div>
  );
}
