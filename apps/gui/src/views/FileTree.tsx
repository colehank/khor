// The workspace's own tree (工作区批⑦). One project, one root, and no
// way above it.
//
// 「目录树不要有上一级。工作区不允许回退到工作区之外的地方。」「取消面包屑。」
// 「目录的打开同 vscode,是点击后展开。目录有类似 `>` 的符号表示可展开。」
// — the user, 2026-09-15.
//
// **This is not `FilesPane` with two controls taken off.** That pane
// answers 「where am I on this machine」 — it walks, it has 上一级, its
// address bar is the point — and it is still what 看它的文件 opens,
// because browsing a whole disk is that question. This one answers
// 「what is in this project」, which has no *where*: the root is the
// ceiling, so there is nothing for a breadcrumb to say and nowhere for
// an up row to go. The two share what a row *is* (`views/fileRow`) and
// nothing else, which is the split the ruling actually drew.
//
// **One directory, one `ls`, and only once it is open.** A project is a
// tree somebody opens two or three branches of; listing the whole thing
// would be a walk of somebody else's disk for rows nobody asked to see.
// Which branches are open is remembered per workspace in this face's own
// storage, so reopening a project finds it where it was left — the same
// class of memory as a column width, not a decision about the mesh.
import { Fragment, useEffect, useMemo, useRef, useState } from "react";

import { EmptyState, RegionState, Waiting } from "@/components/RegionState";
import { dirsWait, fetchLs, makeDir, makeFile, renamePath, type DirListing } from "@/api";
import { useHidden } from "@/hooks/use-hidden";
import { EditableName } from "@/components/EditableName";
import { RemoveDialog } from "@/components/RemoveDialog";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import { FileKindIcon } from "@/components/FileKindIcon";
import { IconChevronRight } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { useNarrow } from "@/hooks/use-narrow";
import { pathDrag } from "@/lib/path-drag";
import { onTreeChanged, sameMachine, treeChanged } from "@/lib/tree-changed";
import { cn } from "@/lib/utils";
import { useSetting } from "@/settings";
import { Mark, markInk, useRemoveFlow, useTake } from "@/views/fileRow";

/** One row, already placed: the tree is flattened before it is drawn so
    that a row is a row whatever level it is on — the same reason the
    sessions list flattens its groups. */
type Row = { name: string; full: string; dir: boolean; depth: number };

/** The indent guides (2026-09-16:「目录展开后,有一条竖线,表示这些目录下
    文件属于该目录。如 vscode 一致」): one hairline per level above this
    row, each on the centre line of that level's chevron, so the rows
    under an open directory hang off a single line that runs down from
    it. **Drawn per row, not per directory**: the rows are a flat list
    (`walk`), so the only thing that knows a row's ancestry is its
    depth, and a line that every row at that depth draws for itself is
    continuous because the rows touch. The offset is the chevron's half
    width (12/2) past the level's own padding. */
function Guides({ depth }: { depth: number }) {
  return (
    <>
      {Array.from({ length: depth }, (_, k) => (
        <span
          key={k}
          data-guide={k}
          aria-hidden="true"
          className="pointer-events-none absolute inset-y-0 w-px bg-border"
          style={{ left: `calc(var(--tree-inset) + var(--tree-step) * ${k} + var(--tree-slot) / 2)` }}
        />
      ))}
    </>
  );
}

/** Where the open branches are kept, per workspace. **Keyed by the pair
    that is the workspace's identity** (docs/WORKSPACE.md 身份与名字):
    the same checkout on two machines is two projects, and remembering
    one tree's shape under the other's name would open branches that are
    not there. */
const openKey = (machine: string, root: string) => `khor.ws-tree.${machine}:${root}`;

function remembered(machine: string, root: string): Set<string> {
  try {
    const raw = window.localStorage.getItem(openKey(machine, root));
    const list: unknown = raw === null ? [] : JSON.parse(raw);
    // Only paths inside this root, whatever storage holds. The ceiling
    // is the whole ruling, and a path that wandered in from somewhere
    // else would be one row the tree cannot explain.
    return new Set(
      Array.isArray(list)
        ? list.filter((p): p is string => typeof p === "string" && p.startsWith(`${root}/`))
        : [],
    );
  } catch {
    // Storage refused (a private window, a locked profile). Then
    // nothing is remembered, which is the shape the tree opens in
    // anyway — a shut tree is never wrong, only less convenient.
    return new Set();
  }
}

/**
 * **What each tree last held, and where its watch had got to** (round
 * NET-4): a tree coming back on screen — the narrow face walking back
 * from a file, a workspace reopened — starts from this, and asks its
 * machine's watch what changed since (`dirsWait`, answered by the
 * resident without crossing the road) instead of listing every open
 * level again. Only while that machine is watched: where it cannot be,
 * the tree asks as it always did.
 */
const trees = new Map<string, { listings: Map<string, DirListing>; seq: number }>();
const treeKey = (machine: string, root: string) => `${machine}\u0000${root}`;

/** How long one watch question is held when nothing changes. */
const WATCH_HOLD_MS = 20_000;

export function FileTree({
  machine,
  root,
  here,
  marks,
  open,
  onOpen,
  onDiff,
  onGone,
  onMoved,
  onMade,
  make,
  onMakeSeen,
}: {
  /** The machine's name — what the node resolves. */
  machine: string;
  /** The workspace's own directory, absolute. The ceiling. */
  root: string;
  /** Whether these files are on the machine khor is running on
      (`FilesPane`'s `here` says the rest). */
  here: boolean;
  /** Absolute path → git's letter, folded so a directory carries the
      heaviest letter under it (`WorkspacePane`).
      **`null` is「nobody asked」or「nobody could answer」, which is not
      the same as an empty map** — an empty map says every file here
      matches HEAD, and a tree khor could not read must not claim that.
      Since 工作区批⑦ that is also the whole of what it says: no marks
      and no sentence (「拿不到 git 就一个字不说」). */
  marks: Map<string, string> | null;
  /** Which file is in front, so the row can say so. */
  open: string | null;
  /** A file was pressed. `pinned` is true when the press was one that
      keeps the tab — a double click — which is the strip's business and
      not this tree's (`WorkspacePane`, 工作区批⑧). */
  onOpen: (path: string, pinned: boolean) => void;
  /** git's letter beside a file was pressed: open that file against
      HEAD (工作区第五轮 第 38 条). Absent on a tree where there is
      nowhere to open one — the letter stays a letter then. */
  onDiff?: (path: string) => void;
  /** That path is no longer on the disk. */
  onGone: (path: string) => void;
  /** A name moved on the disk (工作区第五轮 第 36 条). The tab strip
      follows the file rather than losing it, and the marks are stale the
      moment a path changes — both are the pane's business, not this
      tree's. Absent where nothing is listening. */
  onMoved?: (from: string, to: string) => void;
  /** A name was made on the disk. **git has never seen it**, so the
      marks are stale the moment it exists — a new file wears `U` and
      the tree cannot say so until somebody asks again. The same beat a
      save and a rename ride (`WorkspacePane`'s `gitAt`). */
  onMade?: (path: string) => void;
  /** 新建文件 / 新建目录 asked for from the ＋ at the head of this tree
      (2026-09-28), stamped because two presses are two asks. **A
      one-shot, like the pane's `reveal`**: the tree takes it and says so
      (`onMakeSeen`), or a remount — the narrow face walking back to the
      tree — would open a name box nobody asked for. */
  make?: { kind: "file" | "dir"; at: number } | null;
  onMakeSeen?: () => void;
}) {
  /** Every directory this tree has an answer for. The root is asked at
      once; the rest are asked when they are opened. */
  const [listings, setListings] = useState<Map<string, DirListing>>(
    () => new Map(trees.get(treeKey(machine, root))?.listings ?? []),
  );
  /** Whether this machine's levels are watched (`dirsWait`): `null`
      until the first answer, then a fact. Read by the asks after khor's
      own changes — a watched tree is told, not re-asked. */
  const watching = useRef<boolean | null>(null);
  /** The watch's counter this tree has caught up to. */
  const seq = useRef(trees.get(treeKey(machine, root))?.seq ?? 0);
  const hidden = useHidden();
  /** …and the ones that refused, in the node's own sentence. Kept apart
      from the listings for `WorkspacePane`'s reason: 「could not read」
      and 「empty」 are different facts and only one of them is a fact
      about the directory. */
  const [failed, setFailed] = useState<Map<string, string>>(new Map());
  const [expanded, setExpanded] = useState<Set<string>>(() => remembered(machine, root));
  /** Which directories this tree wants an answer for. Separate from the
      listings so that asking again after a removal is a change to *this*
      — the path did not move, and nothing else in the render would
      notice (`FilesPane`'s `again`, one level up). */
  const [want, setWant] = useState<Set<string>>(
    () => new Set([root, ...remembered(machine, root)]),
  );
  /** What is already on the wire. **A ref and not state**: two renders
      between the dial and the answer must not become two dials, and a
      piece of state for that would itself cause the render that starts
      the second one. */
  const asked = useRef(new Set<string>(listings.keys()));
  /** When the root was asked for, while it is out — what the wait says
      (`Waiting`, round NET-FIX) — and a counter 重试 turns, so the asking
      effect runs again for a path taken off `asked`. */
  const [rootAskedAt, setRootAskedAt] = useState<number | null>(null);
  const [tries, setTries] = useState(0);
  /** Asks for `path` again from the start: the refusal or the wait
      forgotten, the dial made anew. */
  const again = (path: string) => {
    asked.current.delete(path);
    setFailed((was) => {
      const next = new Map(was);
      next.delete(path);
      return next;
    });
    setTries((n) => n + 1);
  };
  const { took, take, takes } = useTake(machine);
  /** 单击文件: whether one click keeps the tab, or takes the preview
      seat the next click replaces (`khor config`). Double click pins
      either way, so 「预览」 is not a road with no exit. */
  const clickPins = useSetting("files-click") === "pin";
  // **The open file is not marked on the narrow face** (2026-10-02, the
  // lists' rule, `SideList`): there the tree and the file it opened are
  // two screens, never side by side, so coming back to the tree there is
  // no open file beside it to point at. `data-open` stays the fact.
  const narrow = useNarrow();

  // **The tree is remounted per workspace** (`WorkspacePane` keys it),
  // so `machine` and `root` are constants here and nothing resets: a
  // reset effect would be a second, later opinion about state that was
  // already initialised from the right props.
  // ⚠️ **No `live` flag on these answers, and that is the opposite of
  // every other fetch in this pane.** The usual shape — drop the answer
  // if the effect was torn down — is wrong the moment the dial is
  // deduplicated by a ref that outlives the teardown: React mounts every
  // effect twice in development, the first run starts the `ls` and marks
  // it asked, the cleanup would refuse its answer, and the second run
  // sees it already asked and dials nothing. The tree then never listed
  // anything at all, on the dev build only (found by `check-face`, which
  // is the build it runs against).
  //
  // Dropping the flag is safe here for a reason this component has and
  // most do not: it is remounted per workspace (`WorkspacePane` keys
  // it), so `machine` and `root` cannot change under an answer in
  // flight — every answer that arrives is about this tree.
  useEffect(() => {
    for (const path of want) {
      if (asked.current.has(path)) continue;
      asked.current.add(path);
      if (path === root) setRootAskedAt(Date.now());
      fetchLs(machine, path)
        .then((l) => {
          setListings((was) => new Map(was).set(path, l));
          // An answer that lands after 停止等待 is still the answer.
          setFailed((was) => {
            if (!was.has(path)) return was;
            const next = new Map(was);
            next.delete(path);
            return next;
          });
        })
        .catch((e: unknown) =>
          setFailed((was) => new Map(was).set(path, String(e instanceof Error ? e.message : e))),
        )
        .finally(() => {
          if (path === root) setRootAskedAt(null);
        });
    }
    // `root` is a constant here (the tree is remounted per workspace);
    // `tries` is 重试 asking for a path taken off `asked`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [machine, want, tries]);

  /**
   * **A directory remembered open and deleted since leaves quietly**
   * (2026-09-22: 「列不出 …/target/x86_64-unknown-linux-musl: No such
   * file or directory」, two build directories removed under an open
   * tree). Gone is not an error about the directory — there is no
   * directory — so it draws nothing, and the remembered set forgets it
   * (and everything under it) so the next opening does not ask again.
   *
   * **Told by the parent's listing, not by the refusal's words.** The
   * three roads say 「not there」 three ways — this machine's `io` error,
   * a peer's (in its own language, from whatever khor it runs), a
   * satellite's shell — and a sentence is not a thing to parse. The
   * parent listing is the fact itself: listed, not cut short, and the
   * name is not a directory in it. A refusal whose parent this tree has
   * not asked about gets its parent asked. Any other refusal — no
   * permission, a machine that does not answer — has its directory in
   * the parent listing, or no parent listing to judge by, and keeps its
   * sentence.
   */
  useEffect(() => {
    const parentOf = (p: string) => p.slice(0, p.lastIndexOf("/")) || "/";
    const gone: string[] = [];
    const ask: string[] = [];
    for (const dir of new Set([...failed.keys(), ...expanded])) {
      if (dir === root || !dir.startsWith(`${root}/`)) continue;
      const parent = parentOf(dir);
      const l = listings.get(parent);
      if (l === undefined) {
        if (failed.has(dir) && !failed.has(parent) && !want.has(parent)) ask.push(parent);
        continue;
      }
      if (l.truncated) continue;
      const name = dir.slice(dir.lastIndexOf("/") + 1);
      if (!l.entries.some((e) => e.dir && e.name === name)) gone.push(dir);
    }
    if (ask.length > 0) setWant((was) => new Set([...was, ...ask]));
    if (gone.length === 0) return;
    const under = (p: string) => gone.some((g) => p === g || p.startsWith(`${g}/`));
    for (const p of [...asked.current]) if (under(p)) asked.current.delete(p);
    setFailed((was) => new Map([...was].filter(([p]) => !under(p))));
    setWant((was) => new Set([...was].filter((p) => !under(p))));
    setExpanded((was) => {
      const now = new Set([...was].filter((p) => !under(p)));
      try {
        window.localStorage.setItem(openKey(machine, root), JSON.stringify([...now]));
      } catch {
        // As `toggle`: only the memory is lost.
      }
      return now;
    });
  }, [failed, listings, expanded, want, machine, root]);

  /** The level a watch said changed, asked again — that level alone,
      its listing on screen until the new one lands. */
  const refresh = (dirs: string[]) => {
    for (const d of dirs) asked.current.delete(d);
    setWant((was) => new Set(was));
  };

  // **khor's own acts, wherever they were made** (UX-43 §1,
  // `lib/tree-changed`): a level said to have changed is read again where
  // this tree holds it, its listing on screen until the new one lands. A
  // watched tree is told by the watch instead; a level this tree never
  // asked about is not its business. The tree's own acts say it too, after
  // their `reread` has already taken the level off `asked`, so nothing is
  // asked twice.
  useEffect(
    () =>
      onTreeChanged((c) => {
        if (!sameMachine(c, { machine, here }) || watching.current === true || !asked.current.has(c.dir)) return;
        refresh([c.dir]);
      }),
    // `refresh` only touches refs and a setter.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [machine, here],
  );

  // **Kept for the next mount** (`trees`): what this tree holds, and
  // how far its watch had got.
  useEffect(() => {
    trees.set(treeKey(machine, root), { listings, seq: seq.current });
  }, [listings, machine, root]);

  /**
   * **The watch** (round NET-4): while the tree is on screen, one
   * question at a time to this machine's watch — which of the levels
   * held changed since the counter this tree has — and the ones named
   * are asked again. The first question does not wait, so a tree that
   * came back from `trees` learns at once which levels went stale while
   * it was away. Where the machine cannot be watched (this machine, a
   * satellite, the web face, an older khor) the answer says so, and the
   * tree asks every level it held, as it always did.
   */
  useEffect(() => {
    if (hidden) return;
    let stopped = false;
    let first = true;
    const loop = async () => {
      while (!stopped) {
        const dirs = [...want].filter((d) => listings.has(d) || asked.current.has(d));
        if (dirs.length === 0) {
          await new Promise((r) => setTimeout(r, 500));
          continue;
        }
        try {
          const w = await dirsWait(machine, dirs, seq.current, first ? 0 : WATCH_HOLD_MS);
          if (stopped) return;
          if (w.unavailable) {
            // Cannot be watched: what came from `trees` is asked again
            // once, and the tree asks as it always did from here.
            if (watching.current === null && seq.current > 0) refresh(dirs);
            watching.current = false;
            return;
          }
          watching.current = true;
          first = false;
          seq.current = w.seq;
          if (w.dirs.length > 0) refresh(w.dirs.filter((d) => listings.has(d)));
        } catch {
          // Not reached just now: the next question will say.
          await new Promise((r) => setTimeout(r, 5_000));
        }
      }
    };
    void loop();
    return () => {
      stopped = true;
    };
    // The loop reads the latest `want` and `listings` on each turn
    // through this effect re-running when they change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [machine, hidden, want, listings.size]);

  /** Ask that directory again. Everything below it goes too: a listing
      for a path under one that just changed is an answer about a tree
      that may no longer be there. **Not on a watched tree**, after khor's
      own change: the watch says which level moved, a moment later. */
  const reread = (dir: string) => {
    if (watching.current === true) return;
    const under = (p: string) => p === dir || p.startsWith(`${dir}/`);
    for (const p of [...asked.current]) if (under(p)) asked.current.delete(p);
    const drop = <T,>(m: Map<string, T>) =>
      new Map([...m].filter(([p]) => !under(p)));
    setListings(drop);
    setFailed(drop);
    // A new set with the same members: the effect above keys on
    // identity, and what changed is not the membership but the answers.
    setWant((was) => new Set(was));
  };

  const { actions, dialog } = useRemoveFlow({
    machine,
    here,
    onGone: (full) => {
      onGone(full);
      reread(full.slice(0, full.lastIndexOf("/")) || "/");
    },
  });

  const toggle = (full: string) => {
    setExpanded((was) => {
      const now = new Set(was);
      if (now.has(full)) now.delete(full);
      else now.add(full);
      try {
        window.localStorage.setItem(openKey(machine, root), JSON.stringify([...now]));
      } catch {
        // The branch still opens; only the memory of it is lost, which
        // is the same bargain the column widths take.
      }
      return now;
    });
    setWant((was) => (was.has(full) ? was : new Set(was).add(full)));
  };

  /** 就地输名字 (工作区第五轮 第 36 条): which row is currently a box,
      and what it is for. `at` is the directory a new name goes into, or
      the path being renamed — the two never overlap, because a row is
      either the one being renamed or the parent of a new one.
   *
   * **VS Code's arrangement, which is what was asked for**: the row
   * grows an input, Enter takes it, Esc puts it back. Not a dialog —
   * a name is typed *at* a place in a tree, and a sheet takes the tree
   * off the screen to ask about it.
   */
  const [naming, setNaming] = useState<
    { at: string; kind: "file" | "dir" | "rename" } | null
  >(null);

  /** Make one, then list that directory again. **Nothing is guessed
      into the tree**: the row appears because the machine answered, not
      because the name was typed, so a refusal leaves the tree exactly
      as it was (`reread`). */
  const made = (dir: string, kind: "file" | "dir") => async (name: string) => {
    const full = `${dir === "/" ? "" : dir}/${name.trim()}`;
    await (kind === "dir" ? makeDir(machine, full) : makeFile(machine, full));
    // Open the directory it went into, or the new row is made somewhere
    // nobody can see.
    if (!expanded.has(dir)) toggle(dir);
    reread(dir);
    treeChanged({ machine, here, dir });
    onMade?.(full);
    return null;
  };

  /** Rename in place: the new name replaces the last segment, so a
      typed name is a name and never a path. Crossing directories is the
      same verb in the node (`khor mv`) and is deliberately **not** a
      thing this box can do — a `/` typed into a row would move a file
      somewhere the tree is not showing. */
  const moved = (full: string) => async (name: string) => {
    const dir = full.slice(0, full.lastIndexOf("/")) || "/";
    const to = `${dir === "/" ? "" : dir}/${name.trim()}`;
    if (to !== full) {
      await renamePath(machine, full, to);
      onMoved?.(full, to);
      reread(dir);
      treeChanged({ machine, here, dir });
    }
    return null;
  };

  /** The path whose 复制路径 the clipboard refused, said on that item
      until a copy takes. */
  const [copyRefused, setCopyRefused] = useState<string | null>(null);

  /** 新建文件 / 新建目录 / 复制路径 / 重命名, on top of 删掉. **A
      directory offers all five and a file three**: there is nothing to
      make inside a file, and a menu item that could only refuse is worse
      than no item.
   *
   * **复制路径** (UX-43 §2, the owner 2026-10-07: 「list里新增一个复制路径」):
   * the absolute path on its machine, the word the file page's 「…」 and
   * the tab menu already use, on the root's blank too. The menu closing
   * is the success face — it has no line to say 已复制 on — and a refusal
   * rejects, so the menu stays open with the item saying so (`RowMenu`). */
  const menu = (full: string, isDir: boolean): RowAction[] => [
    ...(isDir
      ? [
          {
            key: "new-file",
            label: gui.file_new,
            run: () => setNaming({ at: full, kind: "file" }),
          },
          {
            key: "new-dir",
            label: gui.dir_new,
            run: () => setNaming({ at: full, kind: "dir" }),
          },
        ]
      : []),
    {
      key: "copy-path",
      label: copyRefused === full ? gui.copy_failed : gui.tab_copy_path,
      failed: copyRefused === full || undefined,
      run: () =>
        navigator.clipboard.writeText(full).then(
          () => setCopyRefused(null),
          (e: unknown) => {
            setCopyRefused(full);
            throw e;
          },
        ),
    },
    {
      key: "rename",
      label: gui.rename,
      run: () => setNaming({ at: full, kind: "rename" }),
    },
    ...actions(full, isDir),
  ];

  /** The box a row grows, whichever of the three opened it. Shares
      `EditableName` with every other name in this app, so Enter, Esc
      and 「the refusal goes under the box」 cannot come to mean
      different things in two places (that component's head). */
  const nameBox = (write: (name: string) => Promise<string | null>, was: string) => (
    <EditableName
      id={was}
      title={was}
      write={write}
      open
      onClose={() => setNaming(null)}
      inputClassName="w-full text-callout"
      className="min-w-0 flex-1"
    />
  );


  /** The open branches, flattened in the node's order (it sorts, this
      never re-sorts — docs/UX.md). */
  const rows = useMemo(() => {
    const out: Row[] = [];
    const walk = (dir: string, depth: number) => {
      const l = listings.get(dir);
      if (l === undefined) return;
      for (const e of l.entries) {
        const full = `${dir === "/" ? "" : dir}/${e.name}`;
        out.push({ name: e.name, full, dir: e.dir, depth });
        if (e.dir && expanded.has(full)) walk(full, depth + 1);
      }
    };
    walk(root, 0);
    return out;
  }, [listings, expanded, root]);

  /** Where the head's ＋ makes a name (2026-09-28): **VS Code's rule**.
      The row pressed last is the selection: a folder means inside it, a
      file means beside it — the folder it is in — and nothing (the blank
      under the rows pressed, or nothing pressed yet) means the
      workspace's root. A row that has since gone — renamed, deleted,
      folded out of sight under a parent — is not a place a box can be
      drawn next to, so the root takes it. */
  const [picked, setPicked] = useState<{ full: string; dir: boolean } | null>(null);
  useEffect(() => {
    if (make == null) return;
    onMakeSeen?.();
    const row = picked === null ? undefined : rows.find((r) => r.full === picked.full && r.dir === picked.dir);
    const parent = (p: string) => p.slice(0, p.lastIndexOf("/")) || "/";
    const into = row === undefined ? root : row.dir ? row.full : parent(row.full);
    // Opened, so the box stands where the row it becomes will stand,
    // among its siblings — what 新建 from a folder's own menu does too
    // once the name is taken (`made`).
    if (into !== root && !expanded.has(into)) toggle(into);
    setNaming({ at: into, kind: make.kind });
    // Only a new ask is a reason to open a box; the rest is read as it is.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [make]);

  /** How far in one level sits (tokens.css `--tree-step`). */
  const indent = (depth: number) => ({
    // **The first level's disclosure mark on the page's left line** (the
    // redesign, 2026-10-02: one left line), via `--tree-inset`.
    paddingLeft: `calc(var(--tree-inset) + var(--tree-step) * ${depth})`,
  });

  const rootError = failed.get(root);
  return (
    <section data-file-tree className="flex h-full min-w-0 flex-col">
      <div data-files-list className="min-h-0 flex-1 overflow-y-auto">
        {rootError !== undefined ? (
          // **The one thing this must never be is blank** — the node's
          // own sentence, at the size of something meant to be read
          // (`FilesPane` says the rest), and 重试 under it. A project khor
          // cannot list is not a project with no files in it.
          <div data-cannot-reach className="h-full">
            <RegionState state="error" message={rootError} onRetry={() => again(root)} />
          </div>
        ) : !listings.has(root) && rootAskedAt !== null ? (
          // **Waiting is said too** (round NET-FIX, audit/net.md F9):
          // the shape of a tree at once, a spinner past a second, and
          // past eight which machine and how long, with a way out.
          <Waiting
            since={rootAskedAt}
            machine={here ? null : machine}
            onCancel={() =>
              setFailed((was) =>
                new Map(was).set(root, here ? gui.waiting_stopped_here : gui.waiting_stopped(machine)),
              )
            }
            onRetry={() => again(root)}
          />
        ) : (
          <>
            {listings.has(root) && rows.length === 0 && (
              <EmptyState data-empty>{gui.empty_dir}</EmptyState>
            )}
            {/* A name being typed into the root itself, which has no
                row of its own to hang under. */}
            {naming !== null && naming.at === root && naming.kind !== "rename" && (
              <div
                data-new-name={naming.kind}
                style={indent(0)}
                className="flex min-h-[var(--tree-row)] items-center gap-2 pr-3 text-callout"
              >
                {nameBox(made(root, naming.kind), "")}
              </div>
            )}
            {rows.map((r) => {
              const mark = marks?.get(r.full);
              const faded = r.name.startsWith(".");
              const on = expanded.has(r.full);
              // A name being typed into *this* directory sits directly
              // under it, one level in — which is where the row it is
              // about to become will be.
              const adding =
                naming !== null && naming.at === r.full && naming.kind !== "rename" ? (
                  <div
                    key={`${r.full} new`}
                    data-new-name={naming.kind}
                    style={indent(r.depth + 1)}
                    className="flex min-h-[var(--tree-row)] items-center gap-2 pr-3 text-callout"
                  >
                    {nameBox(
                      made(r.full, naming.kind === "dir" ? "dir" : "file"),
                      "",
                    )}
                  </div>
                ) : null;
              const row = r.dir ? (
                <div
                  key={r.full}
                  data-depth={r.depth}
                  style={indent(r.depth)}
                  // The wash is the row's and the target fills it, the
                  // arrangement both panes arrived at from opposite
                  // sides (`FilesPane`'s two rows).
                  className={cn(
                    "group relative flex min-h-[var(--tree-row)] items-center gap-2 pr-3 text-callout hover:bg-muted/50",
                    faded && "text-muted-foreground",
                  )}
                >
                  {/* **The whole row toggles, and the chevron is a
                      drawing on it.** VS Code's arrangement, and it is
                      also the only one with a target a finger can land
                      on: a 16-wide twisty of its own would be a control
                      under the floor sitting inside a row that is not
                      (`check-face` section 3).

                      The menu hangs on the button, never on the row
                      around it — `RowMenu`'s trigger carries an
                      `onPointerDown`, and a press handler on a bare
                      `<div>` is a target only a pointer can find. */}
                  <Guides depth={r.depth} />
                  <RowMenu items={menu(r.full, true)}>
                    <button
                      type="button"
                      // Dragged onto a terminal, the path (item 60).
                      {...pathDrag(here ? null : machine, r.full)}
                      data-dir={r.name}
                      data-on={on}
                      // The row the head's ＋ reads its place off (`picked`).
                      data-picked={picked?.full === r.full ? "" : undefined}
                      onClick={() => {
                        toggle(r.full);
                        setPicked({ full: r.full, dir: true });
                      }}
                      className="flex min-w-0 flex-1 items-center gap-2 self-stretch text-left"
                    >
                      {/* **12 across, not 16** (2026-09-16:「树里的 > 按钮
                          有点大了,占地方」), and the name with no
                          trailing slash — 「目录后面不用带 /,> 已经表明
                          了是目录」. */}
                      {/* The slot is 16 — the icon's size, text height —
                          and the twisty (`--affordance`) sits centred in it, so a
                          directory's name and a file's start on one
                          line (2026-09-16:「文件类型 icon 太小了,应该
                          和字等高」, after 12 proved unreadable). */}
                      <span data-tree-slot className="flex size-[var(--tree-slot)] flex-none items-center justify-center" aria-hidden="true">
                        <IconChevronRight
                          className={cn("size-affordance flex-none transition-transform", on && "rotate-90")}
                        />
                      </span>
                      {/* **The name wears the dot's ink** (2026-09-16:「目录
                          如果有状态,应该也上色,现在只有个状态灯,但目录仍
                          然是原色」) — VS Code paints the folder's name
                          in the heaviest change below it, and so does
                          this. */}
                      {naming?.at === r.full && naming.kind === "rename" ? (
                        nameBox(moved(r.full), r.name)
                      ) : (
                        <span
                          data-dir-name
                          className={cn("min-w-0 flex-1 truncate", markInk(mark))}
                        >
                          {r.name}
                        </span>
                      )}
                      <Mark mark={mark} dir />
                    </button>
                  </RowMenu>
                </div>
              ) : (
                <div
                  key={r.full}
                  data-file={r.name}
                  data-depth={r.depth}
                  data-open={open === r.full}
                  style={indent(r.depth)}
                  // **The open file's whole row is washed**
                  // (2026-09-16:「文件树中, 被选中的文件也要高亮」— the
                  // user). It was the name going bold and nothing else,
                  // which from across a column is not a state: bold is
                  // read as a kind of file before it is read as
                  // 「this one is in front」. `fill-strong` is what the
                  // workspace list and the sessions list mark their
                  // selected row with (components/ListRow, 2026-09-27) —
                  // one gesture, one look; it was the brand wash, which
                  // is the row being located to (README 颜色, 4K). It
                  // goes on the row rather than on the button so the
                  // indent is inside it, the way VS Code paints it. The
                  // weight stays: two marks of one state cost nothing
                  // and the row is legible without colour.
                  className={cn(
                    // Round like every list's chosen block (F4.7; W17).
                    "group relative isolate flex min-h-[var(--tree-row)] items-center gap-2 pr-3 text-callout",
                    // **Inset like a list's chosen block** (W-V9): 8 in
                    // from the pane's edges, drawn under the row.
                    "before:absolute before:inset-y-0 before:inset-x-2 before:-z-10 before:rounded-md before:content-['']",
                    open === r.full && !narrow ? "before:bg-fill-strong" : "hover:before:bg-fill",
                    faded && "text-muted-foreground",
                  )}
                >
                  <Guides depth={r.depth} />
                  <RowMenu items={menu(r.full, false)}>
                    <button
                      type="button"
                      // 单击文件 (`khor config set files-click
                      // preview|pin`). **Only an explicit 「pin」 moves
                      // it**, so an answer that has not arrived is the
                      // default rather than a second copy of it.
                      data-picked={picked?.full === r.full ? "" : undefined}
                      {...pathDrag(here ? null : machine, r.full)}
                      onClick={() => {
                        setPicked({ full: r.full, dir: false });
                        onOpen(r.full, clickPins);
                      }}
                      // 「双击、或在里面打了字,就钉住」— the second
                      // press is what says 「I am staying」, and the
                      // browser hands it over as its own event rather
                      // than making this count clicks against a clock.
                      onDoubleClick={() => onOpen(r.full, true)}
                      // **The name wears the letter's ink** — one
                      // decoration in two places (`views/fileRow`), and
                      // last in the list so it wins over the open row's
                      // own weight.
                      className={cn(
                        "flex min-w-0 flex-1 items-center gap-2 self-stretch text-left",
                        open === r.full && !narrow && "font-title text-foreground",
                        markInk(mark),
                      )}
                    >
                      {/* The chevron's seat on a row that has no
                          chevron: without it a file's name would start
                          16 to the left of its sibling directory's and
                          the level would stop reading as a level. */}
                      {/* **The chevron's slot is the row's type slot**
                          (2026-09-16). A directory puts its chevron
                          there; a file puts the icon VS Code would
                          draw for it, the same 12 across, so every
                          name at one level starts on one line — the
                          arrangement the user first asked for as
                          「文件名对齐到 > 的左沿」 was the answer to an
                          *empty* slot reading as membership; a slot
                          with the file's own mark in it is VS Code's
                          picture, and the ruling that followed. */}
                      {/* The kind's glyph starts its ink on the line with the
                          chevrons' (W-V9): its box begins where the
                          chevron's ink does (5.4 further in than its box,
                          measured), at the chevron's 16. */}
                      <FileKindIcon name={r.name} className="h-[var(--tree-slot)] w-[var(--tree-slot)] items-center ps-[calc(var(--tree-slot)/2-var(--affordance)/2+var(--tree-icon-nudge))] [&>svg]:!size-4" />
                      {naming?.at === r.full && naming.kind === "rename" ? (
                        nameBox(moved(r.full), r.name)
                      ) : (
                        <span data-file-name className="min-w-0 flex-1 truncate">
                          {r.name}
                        </span>
                      )}
                    </button>
                  </RowMenu>
                  <Mark
                    mark={mark}
                    onPress={onDiff === undefined ? undefined : () => onDiff(r.full)}
                  />
                  {took?.path === r.full ? (
                    <span
                      data-took={took.state}
                      className={cn(
                        "max-w-[50%] truncate text-caption",
                        took.state === "failed" ? "text-state-failed" : "text-muted-foreground",
                      )}
                    >
                      {took.word}
                    </span>
                  ) : (
                    !here && takes && (
                      <Button
                        size="sm"
                        variant="ghost"
                        data-take={r.name}
                        // **Drawn where somebody is, VS Code's row-action
                        // rule** (2026-09-28 audit 第 15 条): the row under
                        // the pointer, the row the keyboard is in, the open
                        // row — and every row under a finger, which has no
                        // hover to summon it with. On every row at once it
                        // was the loudest word in the tree, repeated down
                        // a column that is about names.
                        //
                        // **Hidden by opacity, never removed**: it stays in
                        // the tab order, and the focus that lands on it is
                        // what draws it (`group-focus-within`). Its place is
                        // kept too, so a row does not change shape when the
                        // pointer arrives.
                        className={cn(
                          "min-h-[var(--tree-row)] min-w-touch flex-none transition-opacity",
                          open !== r.full &&
                            "opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 pointer-coarse:opacity-100",
                        )}
                        onClick={() => take(r.full)}
                      >
                        {gui.take_file}
                      </Button>
                    )
                  )}
                </div>
              );
              // One row, or a row and the box being typed under it.
              return adding === null ? (
                row
              ) : (
                <Fragment key={r.full}>
                  {row}
                  {adding}
                </Fragment>
              );
            })}
            {/* A branch that refused, said where the branch is rather
                than over the whole tree: the rest of the project is
                still listed and still walkable. */}
            {[...failed].map(([dir, why]) =>
              dir === root || !expanded.has(dir) ? null : (
                <div
                  key={dir}
                  data-branch-refused={dir}
                  className="px-4 py-2 text-caption text-muted-foreground"
                >
                  {why}
                </div>
              ),
            )}
            {/* **The blank under the last row belongs to the root**
                (工作区第五轮 第 36 条: 「空处右键 = 根目录的菜单」). A
                tree that only answers a right-click on a row has no way
                to make the first file in an empty project, and no way to
                make one at the top level without picking some directory
                and then correcting it.

                A finger's height of it, so it is a place rather than
                a hairline — and it is the last thing in the scroller,
                so on a short project the whole stretch under the files
                is inside it. */}
            <RowMenu items={menu(root, true)}>
              {/* **A button, not a bare div.** `RowMenu`'s trigger
                  carries a press handler, and a press handler on a
                  `<div>` is a target only a pointer can find — this
                  file's own rule, three hundred lines up. So it is a
                  control, and what it does when it is pressed the
                  ordinary way is the thing people came to the blank for:
                  a new file at the top level. The right-click still
                  opens all four. */}
              <button
                type="button"
                data-tree-blank
                aria-label={gui.file_new}
                onClick={() => {
                  setPicked(null);
                  setNaming({ at: root, kind: "file" });
                }}
                className="min-h-touch w-full"
              />
            </RowMenu>
            {/* The cap said out loud, for `Response::Dir::truncated`'s
                reason: a listing that was cut and one that is simply
                short look exactly the same. */}
            {[...listings].map(([dir, l]) =>
              l.truncated ? (
                <div key={dir} className="p-4 text-caption text-muted-foreground">
                  {cli.dir_truncated(l.entries.length)}
                </div>
              ) : null,
            )}
          </>
        )}
      </div>
      {/* One dialog for the whole tree, not one per row: what is being
          asked about is state, and a sheet per row would be one mounted
          component per file in the project. */}
      <RemoveDialog {...dialog} />
    </section>
  );
}
