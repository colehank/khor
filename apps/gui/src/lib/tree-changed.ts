// **Every tree of one machine's files moves together** (UX-43 §1, the
// owner 2026-10-07 13:40: 「底栏的工作区应该会和主区工作区的变化同步对吧」).
//
// The 附栏's tree and the main area's are both `FileTree`, but only a
// *watched* machine (`dirsWait`, NET-4) tells a tree that a level changed;
// this machine and an unwatched one are not watched. And git's letters are
// read per holder (`SideWorkspace`, `WorkspacePane`), each re-asked only
// after its own acts. So a file made, renamed, deleted or saved in one
// place stayed out of the other until it was mounted again.
//
// This is the one in-process signal for khor's own acts on files: the act
// says which level changed (absolute, on that machine — not relative to
// any tree's root, since two trees of one machine can have different
// ceilings), every mounted tree holding that level reads it again, and
// every git holder whose checkout holds it asks git again. Changes made
// outside khor are not this signal's: the watch says those, or the next
// open does.

const CHANGED = "khor:tree-changed";

/** Which machine: its name as the trees are handed it, and whether it is
    the one khor runs on (`here`), so a caller that only knows the name
    and one that only knows `here` still meet. `here` unknown is false. */
export type TreeMachine = { machine: string; here?: boolean };

/** A level that changed: the directory whose listing is not what it was. */
export type TreeChange = TreeMachine & { dir: string };

/** The directory a path is in. */
export const levelOf = (path: string) => path.slice(0, path.lastIndexOf("/")) || "/";

/** Says that `dir` on that machine changed. Synchronous, so a caller's
    own state changes in the same turn are rendered once with the
    answers'. */
export function treeChanged(c: TreeChange) {
  window.dispatchEvent(new CustomEvent<TreeChange>(CHANGED, { detail: c }));
}

/** Whether two holders mean the same machine. */
export function sameMachine(a: TreeMachine, b: TreeMachine): boolean {
  return (a.here === true && b.here === true) || a.machine === b.machine;
}

/** Whether `dir` is `root` or under it. */
export function holds(root: string, dir: string): boolean {
  return root === "/" || dir === root || dir.startsWith(`${root}/`);
}

/** Calls `run` for every change said, until the answer is called. */
export function onTreeChanged(run: (c: TreeChange) => void): () => void {
  const on = (e: Event) => run((e as CustomEvent<TreeChange>).detail);
  window.addEventListener(CHANGED, on);
  return () => window.removeEventListener(CHANGED, on);
}
