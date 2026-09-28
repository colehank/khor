// 点 M 看整文件对 HEAD 的 diff (工作区第五轮 第 38 条). The letter beside
// a name in the tree — and on the tab — is pressable, and what it opens
// is the whole file side by side with what HEAD has: HEAD on the left,
// the working copy on the right.
//
// # Why the whole file and not the hunks
//
// `git_diff` answers hunks, and the peek in `FileView` already shows one
// of them where it happened. That is the right shape for 「what did I
// change on this line」 and the wrong one for 「what does this file look
// like now compared to then」, which is the question somebody presses a
// status letter to ask. So the hunks are laid back into the file: the
// working copy is what `read_text` returned, HEAD is that same file with
// every hunk's current lines swapped back for the lines it replaced.
//
// **Nothing is re-derived.** The diff is still the node's
// (`khor_gui_core::git` says why a diff computed in the frontend is a
// diff `khor` cannot print); this reads its answer and lays it out.
//
// # It is not an editor
//
// Read-only, both sides. A person who wants to change something presses
// the file's own tab, which is one press away and is an editor. Two
// boxes holding one file is how two of them end up disagreeing.
import { useEffect, useMemo, useState } from "react";

import { fetchFileText, fetchGitDiff, said } from "@/api";
import { gui } from "@/gen/catalog";
import type { GitHunk } from "@/gen/bindings/GitHunk";
import { cn } from "@/lib/utils";

/** One row of one side. `null` text is **a line that side does not
    have** — the file is shorter there by a line the other side gained —
    and it is drawn as an empty tinted row so the two columns stay in
    step with each other all the way down. */
type Row = { num: number | null; text: string | null; changed: boolean };

/** The two sides, laid out so that row `i` of one is opposite row `i` of
    the other.
 *
 * Walks the working copy once, in `at` order. Between hunks both sides
 * take the same line; at a hunk the left side takes `was` and the right
 * takes the `lines` current lines, and whichever is shorter is padded
 * with holes. A deletion (`lines === 0`) is all left and all hole; an
 * insertion (`was` empty) is all right and all hole — the same three
 * kinds `GitHunk` spells by which half is empty, and no fourth field to
 * disagree with them. */
export function sideBySide(
  now: string[],
  hunks: GitHunk[],
): { head: Row[]; work: Row[] } {
  const head: Row[] = [];
  const work: Row[] = [];
  // Line numbers count their own side: HEAD's numbering is HEAD's, and
  // pretending the two agree is what made the peek's numbering wrong
  // before it was given none at all.
  let headNum = 1;
  let cut = 0;
  const ordered = [...hunks].sort((a, b) => a.at - b.at);
  for (const h of ordered) {
    // A deletion sits *after* line `at` (`GitHunk`), so the run of
    // untouched lines before it ends one line later than a change's.
    const upto = h.lines === 0 ? h.at : h.at - 1;
    for (let i = cut; i < Math.min(upto, now.length); i += 1) {
      head.push({ num: headNum, text: now[i], changed: false });
      work.push({ num: i + 1, text: now[i], changed: false });
      headNum += 1;
    }
    cut = Math.max(cut, Math.min(upto, now.length));
    const mine = now.slice(cut, cut + h.lines);
    const tall = Math.max(h.was.length, mine.length);
    for (let i = 0; i < tall; i += 1) {
      const old = h.was[i];
      const fresh = mine[i];
      head.push({
        num: old === undefined ? null : headNum,
        text: old ?? null,
        changed: true,
      });
      work.push({
        num: fresh === undefined ? null : cut + i + 1,
        text: fresh ?? null,
        changed: true,
      });
      if (old !== undefined) headNum += 1;
    }
    cut += h.lines;
  }
  for (let i = cut; i < now.length; i += 1) {
    head.push({ num: headNum, text: now[i], changed: false });
    work.push({ num: i + 1, text: now[i], changed: false });
    headNum += 1;
  }
  return { head, work };
}

/** The name a diff tab wears: `main.rs ⇄ HEAD`. Exported because the
    strip builds its label and this view builds its heading, and two
    spellings of one name is one restyle away from two different names
    for one tab. */
export function diffLabel(name: string): string {
  return `${name} ⇄ HEAD`;
}

export function DiffView({
  machine,
  path,
  narrow,
}: {
  machine: string;
  /** Absolute, on that machine. */
  path: string;
  /** Stacked instead of side by side. Two 40-column halves of a phone
      screen are two columns of nothing (docs/UX.md 窄档). */
  narrow: boolean;
}) {
  const [now, setNow] = useState<string[] | null>(null);
  const [hunks, setHunks] = useState<GitHunk[] | null>(null);
  const [refused, setRefused] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    setNow(null);
    setHunks(null);
    setRefused(null);
    // **Both, and both refusals are shown as they came.** An empty hunk
    // list means「this file matches HEAD」, which is a real answer and
    // the one thing that must never stand in for「khor could not read
    // the repository」 (`khor_gui_core::git`).
    Promise.all([fetchFileText(machine, path), fetchGitDiff(machine, path)]).then(
      ([text, diff]) => {
        if (!live) return;
        setNow(text.binary ? [] : text.text.split("\n"));
        setHunks(diff.hunks);
      },
      (e: unknown) => live && setRefused(said(e)),
    );
    return () => {
      live = false;
    };
  }, [machine, path]);

  const sides = useMemo(
    () => (now === null || hunks === null ? null : sideBySide(now, hunks)),
    [now, hunks],
  );

  if (refused !== null) {
    return (
      <div data-diff-refused className="p-3 text-callout text-state-failed">
        {refused}
      </div>
    );
  }
  if (sides === null) return <div data-diff-tab className="flex-1" />;

  return (
    <div
      data-diff-tab={path}
      className={cn("flex min-h-0 flex-1", narrow ? "flex-col" : "flex-row")}
    >
      <Side
        which="head"
        label={gui.diff_head}
        rows={sides.head}
        className={narrow ? "min-h-0 flex-1 border-b" : "min-w-0 flex-1 border-r"}
      />
      <Side
        which="work"
        label={gui.diff_work}
        rows={sides.work}
        className="min-w-0 min-h-0 flex-1"
      />
    </div>
  );
}

/** One half. Its own scroller: the two sides are the same number of rows
    tall, so they stay level without a line of code holding them
    together — and a shared scroller would have made the narrow face's
    stacked halves scroll each other, which is the opposite of what
    stacking is for. */
function Side({
  which,
  label,
  rows,
  className,
}: {
  which: "head" | "work";
  label: string;
  rows: Row[];
  className: string;
}) {
  return (
    <div className={cn("flex flex-col", className)}>
      <div
        data-diff-side={which}
        className="flex-none border-b px-3 py-1 text-caption text-muted-foreground"
      >
        {label}
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        <div className="flex min-w-full items-start py-2">
          <pre
            data-diff-numbers={which}
            aria-hidden="true"
            className="shrink-0 select-none border-r pl-2 pr-1 text-right font-mono text-mono leading-mono text-muted-foreground tabular-nums"
          >
            {rows.map((r, i) => (
              <div key={i} className="min-h-[var(--text-mono--line-height)]">
                {/* A hole carries no number: it is not a line of this
                    file, and numbering it would say it was. */}
                {r.num === null ? " " : r.num}
              </div>
            ))}
          </pre>
          <pre className="min-w-0 flex-1 font-mono text-mono leading-mono whitespace-pre-wrap [overflow-wrap:anywhere]">
            {rows.map((r, i) => (
              <div
                key={i}
                data-diff-line={r.num ?? ""}
                data-diff-changed={r.changed || undefined}
                className={cn(
                  "min-h-[var(--text-mono--line-height)] px-3",
                  // **Both sides are tinted, and with the same ink.**
                  // The question this view answers is「where is this
                  // file different」, and a colour that meant「added」on
                  // one side and「removed」on the other would be two
                  // answers to it. Which side a row is on is already
                  // said by the heading over it.
                  r.changed && "bg-git-modified/10",
                )}
              >
                {r.text ?? " "}
              </div>
            ))}
          </pre>
        </div>
      </div>
    </div>
  );
}
