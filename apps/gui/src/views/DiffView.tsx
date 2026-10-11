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
// **The texts are the node's; the alignment is the editor's** (cm-a,
// CM-BRIEF step 2). HEAD is rebuilt from the node's hunks
// (`textBefore`), never from a second git; which lines stand opposite
// which is `@codemirror/merge`'s own diff of those two texts, which is
// what lets it fold the unchanged runs and mark the words that changed
// inside a line. Over two texts that differ exactly by the node's hunks
// the two diffs draw the same chunks.
//
// # It is not an editor
//
// Read-only, both sides. A person who wants to change something presses
// the file's own tab, which is one press away and is an editor. Two
// boxes holding one file is how two of them end up disagreeing.
import { useEffect, useMemo, useState } from "react";

import { fetchFileText, fetchGitDiff, said } from "@/api";
import { CodeDiff, preloadDiff } from "@/components/CodeDiff.lazy";
import { gui } from "@/gen/catalog";
import type { GitHunk } from "@/gen/bindings/GitHunk";
// **Not `lib/code/diff`**, which carries the CodeMirror extensions: this
// module is string arithmetic, and the diff tab must be able to lay HEAD
// out before the merge view's chunk arrives (CM-BRIEF §2).
import { textBefore } from "@/lib/code/text";

/* ⚠️ 墓碑 (cm-a, 2026-10-10): `sideBySide` and `Side`, two self-drawn
   columns kept level by padding the shorter side with empty tinted rows.
   `CodeDiff` aligns, folds and marks the changed words itself. */

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
  against,
}: {
  machine: string;
  /** Absolute, on that machine. */
  path: string;
  /** One column instead of side by side. Two 40-column halves of a phone
      screen are two columns of nothing (docs/UX.md 窄档). */
  narrow: boolean;
  /** **Another revision than HEAD** (round UX-4, git 历史's 「与当前比较」):
      its name for the left side and the hunks from it to the file as it
      is now, already answered (`git_show`), so only the file is read. */
  against?: { label: string; hunks: GitHunk[] };
}) {
  const [now, setNow] = useState<string | null>(null);
  const [hunks, setHunks] = useState<GitHunk[] | null>(null);
  const [refused, setRefused] = useState<string | null>(null);
  // The merge view's chunk, fetched while the two texts are read
  // (`fetchFileText` above): the tab should open drawn, not hollow.
  useEffect(() => {
    preloadDiff();
  }, []);
  useEffect(() => {
    let live = true;
    setNow(null);
    setHunks(null);
    setRefused(null);
    Promise.all([fetchFileText(machine, path), against ? Promise.resolve(against) : fetchGitDiff(machine, path)]).then(
      ([text, diff]) => {
        if (!live) return;
        setNow(text.binary ? "" : text.text);
        setHunks(diff.hunks);
      },
      (e: unknown) => live && setRefused(said(e)),
    );
    return () => {
      live = false;
    };
  }, [machine, path, against]);
  const before = useMemo(() => (now === null || hunks === null ? null : textBefore(now, hunks)), [now, hunks]);
  if (refused !== null) {
    return (
      <div data-diff-refused data-selectable className="p-3 text-callout text-state-failed">
        {refused}
      </div>
    );
  }
  if (now === null || before === null) return <div data-diff-tab className="flex-1" />;
  return (
    <CodeDiff
      path={path}
      before={before}
      after={now}
      beforeLabel={against?.label ?? gui.diff_head}
      afterLabel={gui.diff_work}
      narrow={narrow}
    />
  );
}
