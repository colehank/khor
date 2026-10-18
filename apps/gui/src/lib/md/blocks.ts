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
export type Cut = {
  text: string;
  /** The text's own blocks: they join back into `text` exactly. */
  blocks: string[];
  /** Each block as the conversation parses it: its single dollars read
      by pandoc's rule (`dollarsPandoc`), code left as written. */
  shown: string[];
  /** Whether a block is code — fenced or indented — and nothing else. */
  code: boolean[];
};

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

type Lexed = { blocks: string[]; shown: string[]; code: boolean[] };

function lex(text: string): Lexed {
  const out: Lexed = { blocks: [], shown: [], code: [] };
  let odd = false;
  for (const t of new Lexer({ gfm: true }).blockTokens(text)) {
    const last = out.blocks.length - 1;
    // **Code is known by the lexer's own word for it**, fenced or
    // indented: a `$` in a pasted log is a dollar, and an escape added to
    // it would show — code prints its backslashes.
    const code = t.type === "code";
    const shown = code || t.type === "space" ? t.raw : dollarsPandoc(t.raw);
    // Blank lines ride with the block above them, and display math that
    // opened swallows what follows until it closes.
    if (last >= 0 && (t.type === "space" || odd)) {
      out.blocks[last] += t.raw;
      out.shown[last] += shown;
      out.code[last] &&= code || t.type === "space";
    } else {
      out.blocks.push(t.raw);
      out.shown.push(shown);
      out.code.push(code);
    }
    if (!code && t.type !== "space" && dollars(t.raw) % 2 === 1) odd = !odd;
  }
  // The lexer tidies a few raws it was handed (a list whose last item is
  // only its marker, `- `, comes back as `-\n`) — exactly the shape a
  // turn has while it arrives. The blocks are taken as cuts of the text
  // itself: where one stops matching, the rest of the text is one block.
  let at = 0;
  for (let i = 0; i < out.blocks.length; i++) {
    if (!text.startsWith(out.blocks[i], at)) {
      out.blocks.length = out.shown.length = out.code.length = i;
      break;
    }
    at += out.blocks[i].length;
  }
  if (at < text.length) {
    out.blocks.push(text.slice(at));
    out.shown.push(dollarsPandoc(text.slice(at)));
    out.code.push(false);
  }
  return out;
}

/**
 * The blocks of `text`, reusing `prev` when `text` only grew from it.
 * The blocks always join back into `text` exactly; anything else falls
 * back to one block, which is the old whole-text parse.
 */
export function cut(text: string, prev: Cut | null): Cut {
  if (text === "") return { text, blocks: [], shown: [], code: [] };
  if (ACROSS.test(text)) {
    const all = lex(text);
    return { text, blocks: [text], shown: [all.shown.join("")], code: [false] };
  }
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
  const tail = lex(text.slice(kept));
  const head = (k: "blocks" | "shown" | "code") => (prev === null ? [] : prev[k].slice(0, keep));
  return {
    text,
    blocks: (head("blocks") as string[]).concat(tail.blocks),
    shown: (head("shown") as string[]).concat(tail.shown),
    code: (head("code") as boolean[]).concat(tail.code),
  };
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
    `$$` the parser reads. Agents write both, and neither has a price to
    be mistaken for (`dollarsPandoc` is the single `$`'s rule). Code —
    fenced or inline — is left alone. */
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

/**
 * A conversation's single `$`, read by pandoc's rule (`tex_math_dollars`,
 * the owner's ruling 2026-10-19): a `$` opens a formula only with a
 * non-space on its right, closes one only with a non-space on its left
 * and no digit after it, and `\$` is a dollar. So 「$5 和 $10」,
 * 「$5-$10」 and 「costs $5, $10」 are prices and `$x^2$` is a formula.
 *
 * The parser (`remark-math`) pairs any two dollars, so every `$` that
 * this rule leaves unpaired is escaped before it parses; a pair is left
 * as written. Lines inside a fence and runs inside backticks are code
 * and are not touched; `$$` is display math's and is left alone. One
 * line at a time — a formula in a conversation does not span lines.
 *
 * Streaming: an opening `$` whose close has not arrived is a dollar for
 * that frame — `$x^2` shows as text and turns into the formula when its
 * `$` lands, never into a broken one (`mend` is told not to close it).
 * A close that is the last character of the text so far counts, so
 * 「$5-$」 draws as a formula for the one frame before 「10」 arrives;
 * pandoc's rule cannot know either until the next character.
 *
 * A code block — fenced or indented — never reaches here: `cut` asks the
 * lexer which blocks are code and passes those through untouched. The
 * fence walk below is for the fences inside other blocks (a list item's).
 */
export function dollarsPandoc(text: string): string {
  if (!text.includes("$")) return text;
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
    out.push(
      line
        .split(/(`+[^`]*`+)/)
        .map((piece, i) => (i % 2 === 1 ? piece : pairDollars(piece)))
        .join(""),
    );
  }
  return out.join("\n");
}

const blank = (c: string | undefined) => c === undefined || /\s/.test(c);

/** One run of prose: pandoc pairs kept, every other single `$` escaped. */
function pairDollars(s: string): string {
  if (!s.includes("$")) return s;
  /** Where each kept dollar is. */
  const keep = new Set<number>();
  const single = (i: number) => s[i] === "$" && s[i - 1] !== "$" && s[i + 1] !== "$";
  for (let i = 0; i < s.length; i++) {
    if (s[i] === "\\") {
      i += 1;
      continue;
    }
    if (s[i] === "$" && s[i + 1] === "$") {
      // Display math's, and the run of dollars it belongs to.
      while (s[i + 1] === "$") i += 1;
      continue;
    }
    if (!single(i) || blank(s[i + 1])) continue;
    for (let j = i + 1; j < s.length; j++) {
      if (s[j] === "\\") {
        j += 1;
        continue;
      }
      if (single(j) && !blank(s[j - 1]) && !/[0-9]/.test(s[j + 1] ?? "")) {
        keep.add(i).add(j);
        i = j;
        break;
      }
    }
  }
  let out = "";
  for (let i = 0; i < s.length; i++) {
    if (s[i] === "\\") {
      out += s.slice(i, i + 2);
      i += 1;
    } else if (s[i] === "$" && (s[i + 1] === "$" || s[i - 1] === "$")) {
      out += "$";
    } else if (s[i] === "$" && !keep.has(i)) {
      out += "\\$";
    } else {
      out += s[i];
    }
  }
  return out;
}

/** What a text asks for beyond the parser everyone gets; each is a
    chunk loaded the first time some text needs it (`lib/md/extras`).
    The file page's own chunk (HTML, anchors) is not here: every file
    needs it. */
export type Needs = { math: boolean; emoji: boolean };

/** Roughly a pandoc pair (`dollarsPandoc`), to decide whether the math
    chunk is worth fetching; the exact test runs on the blocks. */
const PANDOC_PAIR = /\$[^\s$](?:[^$\n]*[^\s\\$])?\$(?!\d)/;

export function needsOf(text: string, file: boolean): Needs {
  return {
    math: (file ? text.includes("$") : PANDOC_PAIR.test(text) || text.includes("$$") || /\\[([]/.test(text)) || /^ {0,3}(`{3,}|~{3,})\s*math\b/m.test(text),
    // A letter first, so a clock (`10:30:45`) does not fetch the table.
    emoji: /:(?:[a-z][a-z0-9_+-]*|[+-]1):/i.test(text),
  };
}
