// 退出时还有草稿要拦 (工作区第五轮 第 39 条). How many files are open
// with something typed in them that has not been saved.
//
// # Why a register and not a prop
//
// The drafts themselves live where they are used — `WorkspacePane` holds
// a Map keyed by path, because it owns the tabs (`FileView`'s `draft`
// says why they are lifted that far and no further). What the quit guard
// needs is one number, from wherever in the app it happens to be true,
// and it is read at the moment a window is closing — outside React's
// render, by a listener the shell installed.
//
// Threading a count up through the shell would mean every pane that ever
// grows a draft box has to remember to pass it, and a pane that forgot
// would be a window that closed over somebody's work with no warning and
// no red anywhere. One register with one writer per pane cannot be
// forgotten quietly: the pane either reports or it does not, and the
// number is on screen in the dialog when it is wrong.
//
// **It is a count, not the drafts.** What the guard asks is 「is there
// anything to lose」, and what it says is how many files — nothing here
// can save anything, and holding the text would invite a second road
// into the save that is not `FileView`'s.

/** Per reporter, so two panes open at once add up and a pane that goes
    away takes its own number with it. */
const owed = new Map<string, number>();
const watchers = new Set<(n: number) => void>();

function total(): number {
  let n = 0;
  for (const v of owed.values()) n += v;
  return n;
}

/** How many files are open with unsaved changes, across the whole face.
    Read by the quit guard at the moment a window is closing. */
export function draftCount(): number {
  return total();
}

/** One pane says how many of its files are unsaved. `0` is a real
    answer and is how a pane says 「none of mine」; a pane going away
    calls it with `0` too, which is the same fact. */
export function reportDrafts(who: string, n: number): void {
  const was = total();
  if (n === 0) owed.delete(who);
  else owed.set(who, n);
  const now = total();
  if (now === was) return;
  for (const w of watchers) w(now);
}

/** Told whenever the count changes — for anything that wants to draw it
    rather than ask at the last moment. Answers the way to stop. */
export function watchDrafts(said: (n: number) => void): () => void {
  watchers.add(said);
  return () => {
    watchers.delete(said);
  };
}
