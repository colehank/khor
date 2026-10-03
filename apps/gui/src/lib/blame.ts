// **Blame beside the line numbers** (round UX-4, the file page's 「…」):
// whether the column is shown, per device like the file's 字号, and how
// HEAD's blame lands on the lines of the file as it is now.
//
// Mac only for now (the brief): a phone's column is too narrow to give
// a name and a hash a column of their own.
import { useEffect, useState } from "react";

import type { GitBlame } from "@/gen/bindings/GitBlame";
import type { GitCommit } from "@/gen/bindings/GitCommit";
import type { GitHunk } from "@/gen/bindings/GitHunk";

const KEY = "khor.blame";
const CHANGED = "khor:blame";

function stored(): boolean {
  try {
    return window.localStorage.getItem(KEY) === "1";
  } catch {
    return false;
  }
}

/** Whether the column is on, and a way to turn it; every open file follows. */
export function useBlameShown(): { on: boolean; set: (on: boolean) => void } {
  const [on, setOn] = useState(stored);
  useEffect(() => {
    const again = () => setOn(stored());
    window.addEventListener(CHANGED, again);
    return () => window.removeEventListener(CHANGED, again);
  }, []);
  const set = (next: boolean) => {
    try {
      window.localStorage.setItem(KEY, next ? "1" : "0");
    } catch {
      // A private window keeps it for this page only.
    }
    setOn(next);
    window.dispatchEvent(new Event(CHANGED));
  };
  return { on, set };
}

/**
 * Who wrote each line of the file **as it is now**, from a blame of HEAD
 * and the hunks between HEAD and now (`GitHunk`'s coordinates): a line
 * outside every hunk is HEAD's line at the matching offset, a line inside
 * one is not committed (`null`) — what `git blame` says of a working copy.
 */
export function blameOfLines(lines: number, blame: GitBlame, hunks: GitHunk[]): (GitCommit | null)[] {
  const byHead: number[] = [];
  for (const r of blame.runs) for (let k = 0; k < r.len; k++) byHead[r.start + k] = r.commit;
  const out: (GitCommit | null)[] = [];
  let head = 0;
  const take = () => {
    const c = byHead[head++];
    out.push(c === undefined ? null : (blame.commits[c] ?? null));
  };
  for (const h of [...hunks].sort((a, b) => a.at - b.at)) {
    // A deletion sits after line `at`; anything else starts on it.
    const start = h.lines === 0 ? h.at : h.at - 1;
    while (out.length < start && out.length < lines) take();
    for (let k = 0; k < h.lines && out.length < lines; k++) out.push(null);
    head += h.was.length;
  }
  while (out.length < lines) take();
  return out;
}
