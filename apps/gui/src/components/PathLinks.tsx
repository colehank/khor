// 会话里的路径可点 (工作区第五轮 第 35 条). The remark pass that finds
// paths in an agent's prose, and the thing it turns them into.
//
// # Why a remark pass and not a components override
//
// `react-markdown` lets a caller replace *elements* — a paragraph, a
// link, a code block. What has to be replaced here is a run of
// characters in the middle of a text node, which can be anywhere: inside
// a list item, a table cell, a bolded clause. A pass over the parsed
// tree is the one place all of those are the same shape.
//
// # What is refused, and why the list is short
//
// Only these become links:
//
// - an absolute path already **inside this project**;
// - a relative one with a `/` in it (`src/main.rs`, `./notes.md`);
// - either of those with a `:12` line suffix.
//
// A bare word is not a path (`main` is a word in a sentence), a path
// that climbs is refused before it is resolved (`underRoot`), and — the
// rule the ruling is most explicit about — **a path the machine does not
// actually have is not painted**: 一个指着不存在的东西的下划线是骗人的.
// So the answer is asked for and the run stays plain text until it comes
// back yes.
//
// Code spans and code blocks are left alone. A path inside backticks is
// a path somebody wrote *as text* — very often a command line, where the
// word beside it matters as much as the path does — and turning part of
// a command into a button is the kind of helpfulness that makes people
// stop copying from the screen. Links are left alone too: `remark-gfm`
// has already made those, and they belong to the other half of this
// ruling (`Markdown`'s `Link`).
import { useEffect } from "react";

import { pathsIn } from "@/lib/paths";
import { useLinkHome, underRoot } from "@/views/linkHome";
import { cn } from "@/lib/utils";

// What a path looks like in prose is `lib/paths`' `pathsIn`, the one
// detector the conversation's files and these links share.

type Mdast = {
  type: string;
  value?: string;
  children?: Mdast[];
  data?: Record<string, unknown>;
};

/** The element a matched run becomes. `hName` is what
    `mdast-util-to-hast` puts on the wire to react-markdown, and
    `hProperties` is what reaches the component as props — the documented
    seam for a plugin that wants to draw something of its own. */
function pathNode(written: string): Mdast {
  return {
    type: "khorPath",
    value: written,
    data: { hName: "khor-path", hProperties: { written } },
  };
}

/** Splits the text nodes of a tree, in place.
 *
 * Takes no options and knows no root: whether a candidate is really a
 * path in this project is decided where the answer lives, one component
 * down. This pass only says 「this run has the shape of one」, which is
 * a fact about the characters and can be made once per parse. */
export function remarkPaths() {
  const walk = (node: Mdast) => {
    const kids = node.children;
    if (kids === undefined) return;
    // The three kinds a path may not be found inside, said here rather
    // than tested per child: the head says why each is out.
    if (node.type === "link" || node.type === "linkReference") return;
    const out: Mdast[] = [];
    for (const kid of kids) {
      if (kid.type !== "text" || kid.value === undefined) {
        if (kid.type !== "code" && kid.type !== "inlineCode") walk(kid);
        out.push(kid);
        continue;
      }
      const text = kid.value;
      let at = 0;
      for (const hit of pathsIn(text)) {
        if (hit.index > at) out.push({ type: "text", value: text.slice(at, hit.index) });
        out.push(pathNode(hit.written));
        at = hit.index + hit.written.length;
      }
      if (at === 0) {
        out.push(kid);
        continue;
      }
      if (at < text.length) out.push({ type: "text", value: text.slice(at) });
    }
    node.children = out;
  };
  return (tree: Mdast) => walk(tree);
}

/** One candidate, drawn as a link once the machine says it is a file.
 *
 * **Text until then, and text for good if the answer is no.** The
 * ruling's own sentence: 一个指着不存在的东西的下划线是骗人的. So this
 * never flickers the other way — nothing is underlined and then taken
 * back — and a conversation outside a workspace paints every one of
 * these as the prose it already was. */
export function PathLink({ written }: { written: string }) {
  const home = useLinkHome();
  const inRoot = home === null ? null : underRoot(home.root, written);
  // The conversation's files folder, for a path written whole.
  const more = home?.more && inRoot === null && written.startsWith("/") ? home.more : null;
  const found = inRoot ?? (more ? underRoot(more.root, written) : null);
  const abs = found?.abs ?? null;
  const see = more ? more.see : home?.see;
  const known = more ? more.known : home?.known;
  useEffect(() => {
    if (abs !== null) see?.(abs);
  }, [abs, see]);
  if (home === null || found === null || known?.(found.abs) !== true) {
    return <>{written}</>;
  }
  return (
    // **Inline text, not a `<button>`** (round CHAT, seen on a phone): a
    // button is an atomic box, so a path longer than what was left of the
    // line jumped whole to lines of its own, centred, and left the word
    // before it stranded. A link in a sentence wraps with the sentence,
    // anywhere in the path if it must; Enter presses it like a button.
    <span
      role="link"
      tabIndex={0}
      data-link={found.abs}
      data-link-line={found.line ?? undefined}
      title={found.abs}
      // Underlined like a link and nothing else: it is a link. The
      // dotted variant next door means「this one refused」 (`Link`),
      // and one decoration meaning two things is the ambiguity that
      // rule exists to avoid. **And it wears the pointing hand**, which
      // only a link in prose does (`Markdown`'s `Link` is the other): a
      // control is the arrow (design-system 30-behavior 选字与手).
      className={cn("cursor-pointer underline underline-offset-2 [overflow-wrap:anywhere]")}
      // **`⌘`+press always goes to the host's own opener** (ruling 35:
      // 「`⌘`+点 或 网址→在宿主机器上打开」; 2026-09-18 ruling 54
      // repeats it in so many words). `ctrlKey` beside it for the same
      // reason the save shortcut carries both: this face is a browser
      // on Linux as often as an app on a Mac, and one gesture with two
      // spellings is one gesture.
      onClick={(e) =>
        e.metaKey || e.ctrlKey ? home.launch(found.abs) : home.open(found.abs, found.line)
      }
      onKeyDown={(e) => {
        if (e.key !== "Enter") return;
        e.preventDefault();
        if (e.metaKey || e.ctrlKey) home.launch(found.abs);
        else home.open(found.abs, found.line);
      }}
    >
      {written}
    </span>
  );
}
