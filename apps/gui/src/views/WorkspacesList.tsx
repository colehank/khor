// The workspace landing's list: every declared workspace in one column
// (docs/UX.md 收件箱型 — this list is the thing being watched, because
// what happens inside a workspace happens while you are not looking).
//
// **One read, and no machine is asked for it.** A workspace is a
// decision and decisions replicate (`khor_sync::workspaces`), so the
// whole list is on this disk. The four states this file used to keep —
// 没问过 / 在问 / 它答不了 / 它答了 — are gone with the question: there
// is no machine that could refuse a list nobody asks it for.
//
// **What still needs a far machine is what a row shows**, and those are
// asked one question per row: its git marks and when it last changed.
// Either can fail, and when it does the row keeps its place and loses
// that corner — never the row. A workspace whose machine is asleep is
// still a workspace, and it is still openable.
//
// **Re-asked on the way back into the window, never on a timer** — the
// same rule `WorkspacePane` holds for the git marks, and the same
// reason: a beat is a dial plus a walk of somebody else's disk for an
// answer that is almost always the one already on screen, and coming
// back to the window is the moment staleness starts to mislead.
import { useCallback, useEffect, useState } from "react";

import {
  fetchGitStatus,
  fetchTouched,
  fetchWorkspaces,
  wsPin,
  wsRm,
  type DeviceRow,
  type GitStatus,
  type WorkspaceRow,
} from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { IconChevronRight } from "@/components/icons";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import { gui } from "@/gen/catalog";
import { useHidden } from "@/hooks/use-hidden";
import { cn } from "@/lib/utils";
import { useSetting } from "@/settings";
import { ago } from "@/words";
import { IGNORED } from "@/views/fileRow";

/** Whether a row answers what is typed in the box (2026-09-17 ruling
    50).
 *
 * **The three things the row says out loud**: its name, the machine it
 * is on, and its path. Nothing hidden — a box that matched a field
 * nobody can see would leave a person staring at a row wondering what
 * it has to do with the word they typed (`visibleQuota`'s rule, one
 * landing over).
 *
 * `machine` is the **shown** name, because that is the one on the glass
 * — a row whose machine has left the device table carries an empty one
 * and is then findable by its name and its path, which is all it says.
 *
 * Case-insensitive, a plain substring, any one of the three: the same
 * reading `khor search` gives a typed word, and the same reading every
 * other box in this app gives one. */
function matches(r: WorkspaceRow, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (q === "") return true;
  return [r.name, r.machine, r.path].some((s) => s.toLowerCase().includes(q));
}

/** One row's identity, the same pair the table is keyed by: two
    machines can hold the same path, and the machine is half the answer.
    Joined on `\u0000`, written as an escape rather than as itself
    (`scripts/check-bytes.mjs`) — it is the one byte a path cannot
    contain, so no pair of rows can collide here. */
function keyOf(r: { device: string; path: string }): string {
  return `${r.device}\u0000${r.path}`;
}

export function WorkspacesList({
  devices,
  query = "",
  reloadAt = 0,
  selected,
  onOpen,
  treeOpen,
  onTree,
}: {
  /** Only for the faces: a row's avatar is its machine's, and the
      device table is where faces live. The list itself comes from the
      workspace table and not from here. */
  devices: DeviceRow[];
  /** What is typed in the pane bar's box (2026-09-17 ruling 50:「工作区
      list上的搜索框没有生效. 它应该针对不同机器、工作区路径可以搜索」—
      the user). Matched against the three things a row says out loud;
      see `matches`. */
  query?: string;
  /** Bumped by the shell when **it** has changed the table — the pane
      bar's 「+」 lives up there (`App`). Every other read this list
      makes is its own; this is the one change it has no way to
      notice. */
  reloadAt?: number;
  /** The open workspace, or null. Two fields rather than one joined
      key, for `keyOf`'s reason. */
  selected: { device: string; path: string } | null;
  onOpen: (row: WorkspaceRow) => void;
  /** Whether the open workspace's file tree is showing, for the chevron
      on its row (2026-09-16:「再次点击工作区 list 的某个,也是收起和展开
      该工作区的文件 list」). **A chevron beside the name, not a second
      meaning on the row**: pressing a row opens it, everywhere in this
      app, and a row whose second press quietly folded a column would be
      a mode nothing on the glass announces. The chevron says the tree
      can be folded from here, and says which way it is. */
  treeOpen: boolean;
  /** Fold or unfold it. Absent on the narrow face, where the list is a
      screen and the tree is the next one — nothing to fold. */
  onTree?: () => void;
}) {
  const [list, setList] = useState<WorkspaceRow[]>([]);
  /** What the two per-row questions came back with. Absent means「not
      answered」— never asked yet, or that machine could not — and the
      corner is blank either way, because the two lead to the same next
      move: look at the row, not at the column. */
  const [marks, setMarks] = useState<Record<string, GitStatus>>({});
  const [times, setTimes] = useState<Record<string, number>>({});
  /** A row action that did not take, in the node's own sentence. It
      outlives one opening of the menu, which is why it is here and not
      in `RowMenu` (that file's `note` says so). */
  const [refused, setRefused] = useState<Map<string, string>>(new Map());
  /** Whether the table has been read once. **Not a fifth state** — the
      four this file used to keep were about machines, and no machine is
      asked any more. This is only the difference between「还没读」and
      「读了，是空的」, which is one frame long and would otherwise flash
      「还没有工作区」at somebody who has plenty. */
  const [read, setRead] = useState(false);
  const hidden = useHidden();
  const [round, setRound] = useState(0);

  const reload = useCallback(
    () =>
      fetchWorkspaces()
        .then(
          setList,
          // The table is on this disk; a read of it failing is the
          // backend being unreachable, which the shell already says
          // above this pane (`App`'s 后端 line). Nothing to add here.
          () => {},
        )
        .finally(() => setRead(true)),
    [],
  );

  useEffect(() => {
    void reload();
  }, [reload, round, reloadAt]);

  useEffect(() => {
    if (hidden) return;
    setRound((n) => n + 1);
  }, [hidden]);

  // One git question and one mtime question per row. **Both forget
  // their old answer when the new one fails**, rather than leaving the
  // last good one on screen: a corner that keeps saying `main` about a
  // machine that has stopped answering is the only reading here that
  // could be wrong rather than missing.
  //
  // **A row with no machine name is not asked at all.** Its machine has
  // left the device table, and an empty name means「this machine」to
  // every verb the node has — so asking would answer with *this* Mac's
  // branch and mtime for a path that lives on another one. That is the
  // failure this repo already has a name for: a branch that cannot
  // answer does not go blank, it becomes its neighbour. Here the
  // neighbour would be believable, which is worse.
  useEffect(() => {
    let live = true;
    const drop =
      <T,>(k: string) =>
      (m: Record<string, T>): Record<string, T> => {
        const next = { ...m };
        delete next[k];
        return next;
      };
    for (const r of list) {
      if (r.machine === "") continue;
      const k = keyOf(r);
      fetchGitStatus(r.machine, r.path).then(
        (g) => live && setMarks((m) => ({ ...m, [k]: g })),
        () => live && setMarks(drop<GitStatus>(k)),
      );
      fetchTouched(r.machine, r.path).then(
        (t) => live && setTimes((m) => ({ ...m, [k]: t })),
        () => live && setTimes(drop<number>(k)),
      );
    }
    return () => {
      live = false;
    };
  }, [list]);

  // Pinned lead; the rest by when they were last changed, newest
  // first; a row with no time sinks to the foot (0 and 「not answered」
  // are one answer here, `marks`' note). Ties by path, so two machines
  // holding the same tree print in one order.
  const rows = [...list].filter((r) => matches(r, query)).sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
    const ta = times[keyOf(a)] ?? 0;
    const tb = times[keyOf(b)] ?? 0;
    return tb - ta || a.path.localeCompare(b.path);
  });

  const said = (k: string, e: unknown) =>
    setRefused((m) => new Map(m).set(k, String(e instanceof Error ? e.message : e)));

  // 危险动作二次确认 · 移出工作区 (`khor_node::config`).
  const askRemove = useSetting("confirm-ws-remove") === "on";

  const actions = (r: WorkspaceRow): RowAction[] => {
    const k = keyOf(r);
    const run = (go: () => Promise<unknown>) => async () => {
      try {
        await go();
        setRefused((m) => {
          const next = new Map(m);
          next.delete(k);
          return next;
        });
        await reload();
      } catch (e) {
        said(k, e);
        // Rejecting is how an action says it did not happen: the menu
        // stays open where the press was made (`RowMenu`).
        throw e;
      }
    };
    // **Both are keyed by the row's own device id**, never by its
    // machine name: the row this matters most for is the one whose
    // machine has left the table, which has no name and which somebody
    // opened this menu to take off (`api.ts` `wsRm`).
    return [
      {
        key: "pin",
        // The word is what pressing it *does*, so it is the opposite of
        // the row's current state — the same rule the session rows'
        // pin follows.
        label: r.pinned ? gui.unpin : gui.pin,
        note: refused.get(k),
        run: run(() => wsPin(r.device, r.path, !r.pinned)),
      },
      {
        key: "remove",
        // **Off by default, and that is the ruling rather than an
        // omission**: nothing is destroyed here, the directory is
        // untouched (user, 2026-09-14) and the row comes back with one
        // `ws add`. `khor config set confirm-ws-remove on` asks anyway,
        // for somebody who wants the question — and only an explicit
        // 「on」 does, so an answer that has not arrived reads as the
        // default rather than as a question appearing late
        // (`SessionsList` makes the mirror-image test for the same
        // reason).
        label: gui.ws_remove,
        confirm: askRemove ? gui.ws_remove_confirm : undefined,
        note: refused.get(k),
        run: run(() => wsRm(r.device, r.path)),
      },
    ];
  };

  if (rows.length === 0) {
    if (!read) return <div className="flex min-h-0 flex-1 flex-col" />;
    // **A list emptied by the box is not an empty list** (ruling 50).
    // 「还没有工作区」 under a typed word would be khor telling somebody
    // they have no projects while three of them are one backspace away.
    if (query.trim() !== "" && list.length > 0) {
      return (
        <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
          <div
            data-ws-none-match
            className="flex flex-1 flex-col items-center justify-center px-6 py-10 text-center text-fact text-muted-foreground"
          >
            {gui.workspace_none_match}
          </div>
        </div>
      );
    }
    return (
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
        <div className="flex flex-1 flex-col items-center justify-center gap-2 px-6 py-10 text-center">
          <span className="text-fact font-title text-foreground">{gui.workspaces_empty}</span>
          <span className="text-aux text-muted-foreground">{gui.workspaces_how}</span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
      {rows.map((r) => {
        const k = keyOf(r);
        const isOpen = selected?.device === r.device && selected.path === r.path;
        const git = marks[k];
        const at = times[k] ?? 0;
        // **Ignored paths are rows too since 2026-09-16, and they are
        // not changes** (`khor_node::git::IGNORED`): 「N 个文件改过」
        // counting a build directory would be the corner reporting on
        // things git is not even looking at.
        const changed = git ? git.rows.filter((w) => w.mark !== IGNORED).length : 0;
        return (
          <div
            key={k}
            data-ws-line
            // Whether this row's tree is out, said on the line rather
            // than only drawn: the chevron is `aria-hidden` and may not
            // be there at all (the narrow face has no column to fold),
            // and a state nothing can read is a state a gate cannot
            // hold (2026-09-16).
            data-tree={isOpen && onTree !== undefined ? treeOpen : undefined}
            // **One wash for the whole line** (2026-09-17 ruling 48:
            // 「悬浮工作区list的时候, >符号的高亮要比别的区域深一些,
            // 应该一致」— the user). The row is two boxes — the button
            // and the chevron beside it — and the hover used to be the
            // button's, so the part of the line under the chevron did
            // not change while the rest did. A row is one thing to a
            // hand, so it lights as one: the wash is here, and nothing
            // inside paints a ground of its own.
            className={cn(
              "flex items-stretch border-b border-border/50",
              isOpen ? "bg-accent" : "hover:bg-accent/50",
            )}
          >
          <RowMenu items={actions(r)}>
            {/* **The open one is marked the way an open session is**
                (「一个工作区被选中了没有任何状态」— the user,
                2026-09-16). It wore `bg-card` until then, which is the
                ground this whole pane already stands on (`App`'s
                `data-list`) — a selected state painted in the colour of
                the thing behind it is no selected state at all, and
                nothing on the glass said which row the middle column
                belongs to. `bg-accent` is what `SessionsList` marks its
                own front row with; one gesture, one look. */}
            <button
              data-ws-row
              data-ws-pinned={r.pinned}
              data-selected={isOpen}
              type="button"
              // **The row is the control, and pressing the open one
              // folds its tree** (2026-09-16:「取消工作区list这个只能
              // 点击『<』展开文件树吧. 还是点击整个工作区行都可以. 但
              // 『<』保留为状态展示. 收起状态, 点击行来控制」— the user).
              //
              // The two meanings this was designed away from are still
              // not both on one press: a row that is **not** open means
              // 「open this workspace」 and nothing else, and it is only
              // once a row is the one already in front that its press
              // has anything else left to mean. So the second press is
              // the fold, the selection never changes under it, and the
              // middle column never moves.
              onClick={() => (isOpen && onTree !== undefined ? onTree() : onOpen(r))}
              className="flex min-h-touch min-w-0 flex-1 flex-col items-start gap-0.5 px-3 py-2.5 text-left"
            >
              <span className="flex w-full items-center gap-1.5">
                <MachineAvatar
                  face={devices.find((d) => d.id === r.device)?.face ?? null}
                  className="size-kind-mark flex-none"
                />
                {/* **The name gives way last** (「list 宽度折叠的时候,
                    优先折叠 branch 名,保留工作区名」— the user,
                    2026-09-16). Ordinary flex shrinking here, arranged
                    so the deficit has only one place to go: the branch
                    beside it starts at a base of zero (`flex-1`), so
                    the width it draws is whatever is left over and it
                    absorbs nothing when the row gets narrower. This one
                    keeps its natural base and `min-w-0`, so it is the
                    only item that can shrink — and it only starts to
                    once the branch has already given up everything it
                    had. */}
                <span data-ws-name className="min-w-0 truncate text-fact font-title text-foreground">
                  {r.name}
                </span>
                {/* **The branch, or nothing at all.** A directory that
                    is not a repository, a machine that could not be
                    asked, a detached HEAD with no name to say: all
                    three arrive here as nothing, and 「没有就不显示」
                    is the rule for all three. The corner keeps its
                    anchor so the absence is a readable fact rather than
                    a missing element. */}
                {/* **`flex-1` where this was `ml-auto shrink-0`.** A
                    base of zero that grows into the leftover is the
                    same right-hand corner on a wide row, and on a
                    narrow one it is the half that folds: it takes what
                    the name did not want and truncates inside it, down
                    to nothing. `text-right` is what `ml-auto` was doing
                    before — the box now reaches all the way across, so
                    the corner is where the text sits rather than where
                    the box starts. */}
                <span
                  data-ws-git
                  className="min-w-0 flex-1 truncate text-right text-aux text-muted-foreground"
                >
                  {git && git.branch !== ""
                    ? changed > 0
                      ? `${git.branch} · ${gui.ws_marked(changed)}`
                      : git.branch
                    : ""}
                </span>
              </span>
              <span className="flex w-full items-baseline gap-1.5 text-aux text-muted-foreground">
                {/* Where it is, in full: the machine and the absolute
                    path are one fact, and this pair is what the CLI
                    prints and what somebody pastes. **Not shortened to
                    `~`** — whose home that is, is a fact about the far
                    machine, and this list holds rows from several. */}
                <span data-ws-where className="truncate">
                  {r.machine === "" ? r.path : `${r.machine}:${r.path}`}
                </span>
                <span data-ws-touched className="ml-auto shrink-0 tabular-nums">
                  {at > 0 ? ago(at) : ""}
                </span>
              </span>
            </button>
          </RowMenu>
          {/* The tree's state, on the open row only — a closed row has
              no tree to be about, and a chevron on every row would read
              as 「expand this row」. Turned when the tree is out, the
              same glyph the tree's own directories turn.

              **A drawing, not a button** (2026-09-16, the ruling above):
              「『<』保留为状态展示」. It was a control and the row was
              not; now the row is the control and this says which way it
              went. `aria-hidden` because it is a picture of state a
              screen reader already has from the row's own press — and
              the state itself is on `[data-ws-line]`'s `data-tree`,
              where a reader of the DOM finds it whether or not this
              glyph is drawn. */}
          {isOpen && onTree !== undefined && (
            <span
              data-ws-tree-mark
              data-on={treeOpen}
              aria-hidden="true"
              // As tall as the row still, though nothing washes now:
              // the glyph sits in the middle of the row's right edge
              // whatever height the row came out at, and a box that
              // stretched with it is how that stays true. The `min-w-touch`
              // is not a target any more — it is the width the corner
              // was, kept so the two rows either side of a fold do not
              // step sideways as the glyph turns.
              className="flex min-w-touch flex-none items-center justify-center self-stretch text-muted-foreground"
            >
              {/* **Left and right, not up and down** (2026-09-16:「收起是
                  <,打开是 >」): the tree is the column to this list's
                  right, so the glyph points at it when it is out and
                  back at this list when it is folded. */}
              <IconChevronRight
                className={cn("size-4 transition-transform", !treeOpen && "rotate-180")}
              />
            </span>
          )}
          </div>
        );
      })}
    </div>
  );
}
