// How a conversation turn is cut into blocks while it arrives, and what
// each block is allowed to cost (lane md-render).
//
// **Why cut at all.** The renderer used to parse the whole turn on every
// chunk: a 50 000-character answer arriving in 200-character drips was
// parsed from the top 256 times, and the last of those parses alone was
// the longest task on the main thread. A turn only ever grows at its
// end, so everything above the block still arriving is finished text —
// each finished block is parsed once, kept by `memo`, and only the last
// one is parsed again per chunk. This is the shape every streaming
// renderer converged on (Streamdown, the ChatGPT and Claude faces): cut
// with a fast block lexer, render each block on its own.
//
// **The cut is `marked`'s block lexer, and remark still does the
// parse.** The lexer only answers *where* blocks begin; what a block
// means is decided by the same remark pipeline the file page uses, so
// the two faces cannot drift in what they draw. Where a cut cannot be
// trusted the text is not cut at all: footnotes and reference-style
// link definitions reach across blocks, so a text holding either is
// parsed whole, the way it always was.
//
// **Only the tail is lexed again.** A chunk appends; the blocks above
// the last blank-line boundary cannot be changed by text after them, so
// they are kept and the lexer restarts at that boundary. The gate checks
// this agrees with lexing the whole text, prefix by prefix
// (`scripts/check-md.mjs`).
import { Lexer } from "marked";
import remend from "remend";

/** A text cut into blocks, kept so the next, longer text can reuse it. */
export type Cut = { text: string; blocks: string[] };

/** A footnote's or a link's definition (`[^1]: …`, `[docs]: …`): read
    from anywhere in the text, so cutting would orphan the references to
    it. Keyed on the definition, not the reference — `[^a-z]` is far more
    often a regular expression in a code block than a footnote, and a
    reference with no definition is plain text anyway. */
const ACROSS = /^ {0,3}\[[^\]\n]+\]:/m;

/** `$$` pairs in a run of text. */
function dollars(s: string): number {
  let n = 0;
  for (let i = 0; i < s.length - 1; i++) {
    if (s[i] === "$" && s[i + 1] === "$") {
      n += 1;
      i += 1;
    }
  }
  return n;
}

/** Whether display math opened in `text` and has not closed: an odd
    count of `$$` outside code fences (a shell's `$$` is a pid, not a
    formula). */
export function mathOpen(text: string): boolean {
  if (!text.includes("$$")) return false;
  let n = 0;
  let fence: string | null = null;
  for (const line of text.split("\n")) {
    const f = /^ {0,3}(`{3,}|~{3,})/.exec(line);
    if (fence !== null) {
      if (f && f[1][0] === fence[0] && f[1].length >= fence.length && line.slice(f[0].length).trim() === "") fence = null;
    } else if (f) fence = f[1];
    else n += dollars(line);
  }
  return n % 2 === 1;
}

function lex(text: string): string[] {
  const out: string[] = [];
  let odd = false;
  for (const t of new Lexer({ gfm: true }).blockTokens(text)) {
    const last = out.length - 1;
    // Blank lines ride with the block above them, and display math that
    // opened swallows what follows until it closes.
    if (last >= 0 && (t.type === "space" || odd)) out[last] += t.raw;
    else out.push(t.raw);
    if (t.type !== "code" && t.type !== "space" && dollars(t.raw) % 2 === 1) odd = !odd;
  }
  // The lexer tidies a few raws it was handed (a list whose last item is
  // only its marker, `- `, comes back as `-\n`) — exactly the shape a
  // turn has while it arrives. The blocks are taken as cuts of the text
  // itself: where one stops matching, the rest of the text is one block.
  let at = 0;
  for (let i = 0; i < out.length; i++) {
    if (!text.startsWith(out[i], at)) {
      out.length = i;
      break;
    }
    at += out[i].length;
  }
  if (at < text.length) out.push(text.slice(at));
  return out;
}

/**
 * The blocks of `text`, reusing `prev` when `text` only grew from it.
 * The blocks always join back into `text` exactly; anything else falls
 * back to one block, which is the old whole-text parse.
 */
export function cut(text: string, prev: Cut | null): Cut {
  if (text === "" || ACROSS.test(text)) return { text, blocks: text === "" ? [] : [text] };
  let keep = 0;
  let kept = 0;
  if (prev !== null && text.startsWith(prev.text)) {
    // The last block can still grow and the one before it can still be
    // joined (a list item, a setext underline); a block that ended on a
    // blank line before those two is finished.
    for (let i = prev.blocks.length - 3; i >= 0; i--) {
      if (prev.blocks[i].endsWith("\n\n")) {
        keep = i + 1;
        break;
      }
    }
    for (let i = 0; i < keep; i++) kept += prev.blocks[i].length;
  }
  const blocks = (prev === null ? [] : prev.blocks.slice(0, keep)).concat(lex(text.slice(kept)));
  if (blocks.join("") !== text) return { text, blocks: [text] };
  return { text, blocks };
}

/**
 * The tail of a turn as it should read *while it arrives*: an opened
 * `**` or `` ` `` closed for this frame, so the reader never sees the
 * markers flash and then turn into weight. Only ever the last block —
 * every block above it is finished text and is drawn as written.
 *
 * Off on purpose: display math (`katex`), which would close a formula
 * that is still being typed and draw it per chunk; HTML tags, which a
 * conversation never renders; and the single-tilde escape, because the
 * conversation's parser does not strike on one tilde to begin with.
 * Links show their words until the address has arrived.
 */
export function mend(tail: string): string {
  return remend(tail, { katex: false, htmlTags: false, singleTilde: false, linkMode: "text-only" });
}

/** Display math written the LaTeX way, `\[ … \]` and `\( … \)`, as the
    `$$` the parser reads. Agents write both; the conversation does not
    read a single `$` as math (「$5 和 $10」), so the bracket forms are how
    inline math reaches it. Code — fenced or inline — is left alone. */
export function bracketMath(text: string): string {
  if (!text.includes("\\(") && !text.includes("\\[")) return text;
  const out: string[] = [];
  let fence: string | null = null;
  for (const line of text.split("\n")) {
    const f = /^ {0,3}(`{3,}|~{3,})/.exec(line);
    if (fence !== null) {
      if (f && f[1][0] === fence[0] && f[1].length >= fence.length && line.slice(f[0].length).trim() === "") fence = null;
      out.push(line);
      continue;
    }
    if (f) {
      fence = f[1];
      out.push(line);
      continue;
    }
    // Outside inline code only: split on backtick runs and convert the
    // even pieces.
    out.push(
      line
        .split(/(`+[^`]*`+)/)
        .map((piece, i) =>
          i % 2 === 1
            ? piece
            : piece
                .replace(/^(\s*)\\\[\s*$/, "$1$$$$")
                .replace(/^(\s*)\\\]\s*$/, "$1$$$$")
                .replace(/\\\[(.+?)\\\]/g, "$$$$$1$$$$")
                .replace(/\\\((.+?)\\\)/g, "$$$$$1$$$$"),
        )
        .join(""),
    );
  }
  return out.join("\n");
}

/** What a text asks for beyond the parser everyone gets; each is a
    chunk loaded the first time some text needs it (`lib/md/extras`).
    The file page's own chunk (HTML, anchors) is not here: every file
    needs it. */
export type Needs = { math: boolean; emoji: boolean };

export function needsOf(text: string, file: boolean): Needs {
  return {
    math: (file ? text.includes("$") : text.includes("$$") || /\\[([]/.test(text)) || /^ {0,3}(`{3,}|~{3,})\s*math\b/m.test(text),
    // A letter first, so a clock (`10:30:45`) does not fetch the table.
    emoji: /:(?:[a-z][a-z0-9_+-]*|[+-]1):/i.test(text),
  };
}
