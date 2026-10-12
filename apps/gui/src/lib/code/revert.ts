// **「还原」 puts HEAD's lines back** (cm-e item 2): the peek's head bar
// replaces one hunk's current lines with what HEAD had, as VS Code's
// dirty-diff peek does — one change, so one ⌘Z takes it back. Here only
// the change it is; the editor dispatches it (`git.ts`).
//
// No imports: `scripts/check-revert.mjs` runs this as it is.

/** The slice of a CodeMirror `Text` this needs. */
export type Lines = { length: number; lines: number; line(n: number): { from: number; to: number } };
/** A `GitHunk`'s shape: `at` the first changed line (or, removed lines
    only, the line they went after, 0 for the top), `lines` how many lines
    of the text are the change, `was` HEAD's lines in their place. */
export type Hunk = { at: number; lines: number; was: string[] };

/** The change that turns this hunk's lines back into HEAD's, or null when
    the hunk names lines this text does not have. A modification swaps the
    lines for `was`; an insertion takes its lines out with one line break;
    a deletion puts `was` back after line `at`. */
export function revertChange(doc: Lines, h: Hunk): { from: number; to: number; insert: string } | null {
  if (h.lines === 0) {
    if (h.at === 0) return { from: 0, to: 0, insert: h.was.join("\n") + "\n" };
    if (h.at > doc.lines) return null;
    const end = doc.line(h.at).to;
    return { from: end, to: end, insert: "\n" + h.was.join("\n") };
  }
  const last = h.at + h.lines - 1;
  if (h.at < 1 || last > doc.lines) return null;
  if (h.was.length > 0) return { from: doc.line(h.at).from, to: doc.line(last).to, insert: h.was.join("\n") };
  // An insertion: its lines go, and the break after the last of them —
  // the file's own break at the foot stays, that is text of its own.
  if (last < doc.lines) return { from: doc.line(h.at).from, to: doc.line(last + 1).from, insert: "" };
  return { from: doc.line(h.at).from, to: doc.line(last).to, insert: "" };
}
