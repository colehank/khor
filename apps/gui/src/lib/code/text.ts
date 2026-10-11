// Two texts' worth of arithmetic that needs no editor (CM-BRIEF §2: the
// bundle split). `textBefore` rebuilt a file's earlier text from the
// hunks git handed over — plain string and array work, and it sat in
// `lib/code/diff.ts` beside the CodeMirror extensions, so every importer
// of it (`DiffView`) pulled the whole editor core into the entry chunk.
// It lives on its own so the diff tab can lay HEAD out before the merge
// view arrives.
import type { GitHunk } from "@/gen/bindings/GitHunk";

/** HEAD's text (or a revision's), rebuilt from the file as it is now and
    the hunks that lead from that to this — all `git_diff` and `git_show`
    hand over. Each hunk's `was` goes where its `lines` are. */
export function textBefore(now: string, hunks: GitHunk[]): string {
  const lines = now.split("\n");
  const out: string[] = [];
  let cut = 0;
  for (const h of [...hunks].sort((a, b) => a.at - b.at)) {
    const upto = Math.min(h.lines === 0 ? h.at : h.at - 1, lines.length);
    for (; cut < upto; cut += 1) out.push(lines[cut]);
    out.push(...h.was);
    cut += h.lines;
  }
  for (; cut < lines.length; cut += 1) out.push(lines[cut]);
  return out.join("\n");
}
