// The parts a row of files is made of, wherever the row is standing.
//
// **Two trees read the same disk** (工作区批⑦): `FilesPane` is the one
// that browses a whole machine — it has 上一级 and an address bar,
// because「where am I」is the question there — and `FileTree` is the one
// inside a workspace, rooted at the project with no way above it. What
// they share is not the walking, which is exactly what the user ruled
// apart; it is what a *row* is: git's letter, the way to delete, the way
// to pull a file over.
//
// So those three live here rather than in either pane. A second copy of
// the delete flow is how 「本机进废纸篓,远端直删」 ends up true on one
// tree and not the other, and neither screen would say so.
import { useEffect, useState } from "react";

import { fetchTrashHere, pullFile, removePath } from "@/api";
import { neverAsks, type RemoveKind } from "@/components/RemoveDialog";
import { type RowAction } from "@/components/RowMenu";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/** What a directory wears instead of a letter. Not a catalog string: it
    is a glyph, and the catalog holds sentences a person reads.

    **A filled circle since 2026-09-16, where it was a middle dot**:
    「文件 list 的状态点太小了,小到现在都看不清是什么颜色」(the user).
    The whole of what this mark says is its colour, and a `·` in the
    row's own font is three or four pixels of ink — small enough that
    the one thing it carries cannot be read off it.

    **Drawn as a box rather than as a character.** How big a `·` comes
    out is the font's decision and khor has no way to ask; a circle
    asked for at `size-2` is 8 across wherever this face is drawn,
    which is the size VS Code gives it. The character stays here as the
    attribute's value, because that is what tells a folded mark from a
    letter to anything reading the DOM. */
export const DOT = "●";

/** How heavy one letter is, for the dot a directory wears: **C > D > M
    > A/U** (docs/WORKSPACE.md 第二轮). A folder's dot can only say one
    thing about a whole subtree, so it says the thing that would be read
    first if the subtree were open. `R` ranks with `M`: a rename is a
    change to a file that is still there. Anything unknown ranks lowest
    rather than throwing — a letter this face has not met is still a
    letter the node meant. */
const WEIGHT: Record<string, number> = { C: 4, D: 3, M: 2, R: 2, A: 1, U: 1 };

/** git's own letter for a path `.gitignore` covers (`khor_node::git`'s
    `IGNORED`, porcelain's `!!` with one of its two characters).

    **It is a letter that is never drawn.** What it has to say is said
    by the name going grey — VS Code does the same, and a mark beside it
    would be a second answer to a question the colour has finished. It
    is here because the two places that decide a row's look both have to
    recognise it: `markInk` paints it, [`Mark`] declines to draw it. */
export const IGNORED = "!";

/** The heavier of two letters. */
export function heavier(a: string, b: string): string {
  return (WEIGHT[b] ?? 0) > (WEIGHT[a] ?? 0) ? b : a;
}

/** The ink one letter is drawn in (tokens.css `--git-*`).
    **Returned as a class rather than painted here**, because the row's
    *name* wears the same ink — 「颜色与字符表示」 is one decoration in
    two places, and a function that painted only the letter would leave
    the other half to be remembered. */
export function markInk(mark: string | undefined): string {
  switch (mark) {
    // **Grey, and grey is the whole message** (2026-09-16:「文件树中,
    // 被.gitignore的文件/目录名要变灰」— the user). Not one of the four
    // git inks: those four say *how* a file differs from HEAD, and this
    // one says git is not looking at the file at all. The muted ink is
    // what this face already uses for 「here, and not the point」 (a
    // dotfile row wears it), which is exactly the claim.
    case IGNORED:
      return "text-muted-foreground";
    case "M":
    // A rename shares 修改's ink: the file is still there and still
    // differs from HEAD, which is what the colour says; `R` is what
    // says the path moved, and that is the letter's job.
    case "R":
      return "text-git-modified";
    case "A":
    case "U":
      return "text-git-added";
    case "D":
      return "text-git-deleted";
    case "C":
      return "text-git-conflict";
    default:
      return "";
  }
}

/** git's letter on one row, or nothing.
    **Nothing is the common case and it takes no width**: most files in
    a checkout match HEAD, and a column of blanks reserved for the few
    that do not would push every name left for no reader's benefit.

    A directory wears [`DOT`] in the ink of the heaviest letter under it
    — git has no opinion about directories, so any letter on a folder
    would be khor inventing a claim, while「something under here
    differs」is both true and the only thing worth walking for.

    **The letter is pressable where somebody handed it an `onPress`**
    (工作区第五轮 第 38 条): 「点击 M 看整文件对 HEAD 的 diff」. The dot
    on a directory is not — a directory has no diff, and a press that
    opened「the first changed file under here」would be khor picking one.
    Where no handler is given it stays a span, because a button nothing
    happens on is worse than a letter. */
export function Mark({
  mark,
  dir = false,
  onPress,
}: {
  mark?: string;
  dir?: boolean;
  onPress?: () => void;
}) {
  if (mark === undefined) return null;
  // **An ignored path wears no mark at all, on a file or a directory.**
  // Its name is already grey (`markInk`), and that is the whole of what
  // there is to say; a dot or a `!` at the end of the row would spend a
  // column on 「and here is that again」. VS Code gives it no badge
  // either. **The fold above never sends one up a directory** either
  // (`khor_node::git::fold`): a folder holding an edited file and a
  // build output is 黄, not grey.
  if (mark === IGNORED) return null;
  if (dir) {
    return (
      <span
        data-git-mark={DOT}
        data-git-weight={mark}
        // Nothing to read aloud: the colour is the whole message and a
        // screen reader gets none of it. The letters beside it are
        // characters and are read as themselves.
        aria-hidden="true"
        // **`bg-current`, so `markInk` stays one decision.** The ink is
        // set as a text colour on this span — the same class the letter
        // and the file's name wear — and the circle takes its fill from
        // it. Painting the background from a second class would be the
        // same choice written twice, and the day one moved they would
        // disagree on one row.
        className={cn("size-2 flex-none rounded-full bg-current", markInk(mark))}
      />
    );
  }
  const look = cn("flex-none font-mono text-xs", markInk(mark));
  if (onPress === undefined) {
    return (
      <span data-git-mark={mark} data-git-weight={mark} className={look}>
        {mark}
      </span>
    );
  }
  return (
    <button
      type="button"
      data-git-mark={mark}
      data-git-weight={mark}
      aria-label={gui.diff_open}
      // The row under it opens the file; this opens the diff, and a
      // press is one or the other (`Ruler`'s marks, same rule).
      onClick={(e) => {
        e.stopPropagation();
        onPress();
      }}
      className={cn(look, "cursor-pointer")}
    >
      {mark}
    </button>
  );
}

/** Whether the machine khor runs on has a recycle bin. **Asked, not
    guessed**: this page is also served to phones looking at a desktop's
    face (`api.ts` `fetchTrashHere`). Starts false, which is the safe
    direction — 「直接删」 said of something that turns out to be
    recoverable costs a person nothing; the other way round costs them a
    file. */
export function useTrashHere(): boolean {
  const [yes, setYes] = useState(false);
  useEffect(() => {
    let live = true;
    fetchTrashHere()
      .then((has) => live && setYes(has))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, []);
  return yes;
}

/** The row's one menu item and the state the sheet behind it needs.
 *
 * **Its own dialog rather than `RowMenu`'s two-press confirm**: the
 * user asked for VS Code's shape — a sheet that names the thing and
 * carries a 「不再问」 tick — and a tick has nowhere to live on a menu
 * item that disappears when the menu closes.
 */
export function useRemoveFlow({
  machine,
  here,
  onGone,
}: {
  machine: string;
  /** Whether the files are on the machine khor runs on. */
  here: boolean;
  /** It really went. The caller re-reads whatever listing held it and
      closes a view onto it — a pane showing a file that is no longer
      there is the one state neither tree may leave behind. */
  onGone: (path: string) => void;
}) {
  const trashHere = useTrashHere();
  const [going, setGoing] = useState<{ path: string; kind: RemoveKind } | null>(null);
  const [refused, setRefused] = useState<string | null>(null);

  /** What deleting this path will actually do, in the node's terms.
      **Never a guess about the platform** — `here` says whose disk it
      is and `trashHere` says whether that disk's machine has a bin. */
  const kindOf = (isDir: boolean): RemoveKind =>
    !here || !trashHere ? "gone" : isDir ? "trash-dir" : "trash";

  /** Do it. Shared by the dialog's button and the press that skipped
      the dialog, so there is one place that knows what happens after. */
  const go = (full: string) =>
    removePath(machine, full).then(
      () => onGone(full),
      (e: unknown) => {
        setRefused(String(e instanceof Error ? e.message : e));
        throw e;
      },
    );

  const actions = (full: string, isDir: boolean): RowAction[] => [
    {
      key: "remove",
      label: gui.delete,
      run: () => {
        setRefused(null);
        const kind = kindOf(isDir);
        // The tick only ever covered the recoverable road, so it only
        // ever skips that one (`RemoveDialog`).
        if (kind !== "gone" && neverAsks()) return go(full);
        setGoing({ path: full, kind });
      },
    },
  ];

  return {
    actions,
    go,
    /** What the sheet needs, spread straight onto `RemoveDialog`. */
    dialog: {
      open: going !== null,
      name: going === null ? "" : (going.path.split("/").pop() ?? going.path),
      kind: going?.kind ?? ("gone" as RemoveKind),
      refused,
      onOpenChange: (o: boolean) => {
        if (!o) {
          setGoing(null);
          setRefused(null);
        }
      },
      onConfirm: () => go(going?.path ?? ""),
    },
  };
}

/** What one pressed row is doing or has done. */
export type Took = { path: string; state: "taking" | "landed" | "failed"; word: string };

/** 拉过来 — copy a file from that machine to this one. The landing (the
    downloads directory) was chosen silently, so the row says where the
    file went; a failure wears its words on the same row, the pin's rule,
    because this app has nowhere that collects messages. */
export function useTake(machine: string) {
  const [took, setTook] = useState<Took | null>(null);
  const take = (path: string) => {
    setTook({ path, state: "taking", word: "…" });
    pullFile(machine, path)
      .then((dest) => setTook({ path, state: "landed", word: gui.took_file(dest) }))
      .catch((e: unknown) =>
        setTook({ path, state: "failed", word: String(e instanceof Error ? e.message : e) }),
      );
  };
  return { took, take };
}
