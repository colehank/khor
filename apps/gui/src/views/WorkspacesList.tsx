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
import { IconBranch, IconChevronRight, IconFiles } from "@/components/icons";
import { type RowAction } from "@/components/RowMenu";
import { SideList, type SideRow } from "@/components/SideList";
import { gui } from "@/gen/catalog";
import { useHidden } from "@/hooks/use-hidden";
import { cn } from "@/lib/utils";
import { ago, groupLabel } from "@/words";
import { IGNORED } from "@/views/fileRow";

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
  // The box's rule is `SideList`'s now (`find` below reads the same
  // three things `matches` did); this only orders.
  const rows = [...list].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
    const ta = times[keyOf(a)] ?? 0;
    const tb = times[keyOf(b)] ?? 0;
    return tb - ta || a.path.localeCompare(b.path);
  });

  const said = (k: string, e: unknown) =>
    setRefused((m) => new Map(m).set(k, String(e instanceof Error ? e.message : e)));


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
        // What deleting a workspace does is the window's sentence: the
        // row comes off the list and the directory is not touched (the
        // user, 2026-09-14) — the fact 移出 used to carry in the word.
        label: gui.delete,
        ask: gui.ws_remove_confirm,
        note: refused.get(k),
        run: run(() => wsRm(r.device, r.path)),
      },
    ];
  };

  if (!read) return <div className="flex min-h-0 flex-1 flex-col" />;

  const side: SideRow[] = rows.map((r) => {
    const k = keyOf(r);
    const isOpen = selected?.device === r.device && selected.path === r.path;
    const git = marks[k];
    const at = times[k] ?? 0;
    // **Ignored paths are rows too since 2026-09-16, and they are not
    // changes** (`khor_node::git::IGNORED`).
    const changed = git ? git.rows.filter((w) => w.mark !== IGNORED).length : 0;
    const repo = !!git && git.branch !== "";
    return {
      // Not `keyOf`: that joins on a NUL, which no attribute selector can
      // name. The machine's id and the path, with a colon between — the
      // shape the CLI prints.
      id: `${r.device}:${r.path}`,
      group: r.pinned ? "pin" : "",
      pinned: r.pinned,
      face: (
        <MachineAvatar
          face={devices.find((d) => d.id === r.device)?.face ?? null}
          className="size-avatar"
        />
      ),
      // 「工作区: 文件夹, 是 git 仓库时换分支」 (docs/SHELL.md 第四轮 第 2 条).
      badge: repo
        ? { mark: "branch", icon: <IconBranch className="size-kind-mark-glyph" /> }
        : { mark: "folder", icon: <IconFiles className="size-kind-mark-glyph" /> },
      title: (
        <span className="flex w-full items-center gap-1.5">
          {/* **The name gives way last** (「list 宽度折叠的时候, 优先折叠
              branch 名,保留工作区名」— 2026-09-16): the branch beside it
              starts at a base of zero and takes only the leftover, so
              the name is the one item that shrinks, and only once the
              branch has given up everything it had. */}
          <span data-ws-name className="min-w-0 truncate text-fact font-title text-foreground">
            {r.name}
          </span>
          {/* **The branch, or nothing at all** — not a repository, a
              machine that could not be asked, a detached HEAD: all
              three arrive here as nothing. */}
          <span data-ws-git className="min-w-0 flex-1 truncate text-right text-aux text-muted-foreground">
            {git && git.branch !== ""
              ? changed > 0
                ? `${git.branch} · ${gui.ws_marked(changed)}`
                : git.branch
              : ""}
          </span>
        </span>
      ),
      // Where it is, in full — the machine and the absolute path are one
      // fact, and this pair is what the CLI prints. **Not shortened to
      // `~`**: whose home that is, is a fact about the far machine.
      sub: (
        <span className="flex w-full items-baseline gap-1.5">
          <span data-ws-where className="truncate">
            {r.machine === "" ? r.path : `${r.machine}:${r.path}`}
          </span>
          <span data-ws-touched className="ml-auto shrink-0 tabular-nums">
            {at > 0 ? ago(at) : ""}
          </span>
        </span>
      ),
      // **The three things the row says out loud**: its name, its
      // machine, its path (2026-09-17 ruling 50).
      find: `${r.name} ${r.machine} ${r.path}`,
      selected: isOpen,
      menu: actions(r),
      // **The row is the control, and pressing the open one folds its
      // tree** (2026-09-16): a row that is not open means 「open this
      // workspace」 and nothing else; only the one already in front has
      // a second meaning left, and the selection never changes under it.
      onOpen: () => (isOpen && onTree !== undefined ? onTree() : onOpen(r)),
      // The tree's state, on the open row only — a drawing, not a
      // button (「『<』保留为状态展示」), pointing at the tree when it is
      // out and back at this list when it is folded.
      after:
        isOpen && onTree !== undefined ? (
          <span
            data-ws-tree-mark
            data-on={treeOpen}
            aria-hidden="true"
            className="flex min-w-touch flex-none items-center justify-center self-stretch text-muted-foreground"
          >
            <IconChevronRight className={cn("size-4 transition-transform", !treeOpen && "rotate-180")} />
          </span>
        ) : undefined,
      // Whether this row's tree is out, said on the line rather than only
      // drawn: the chevron is `aria-hidden` and may not be there at all.
      rowData: {
        "data-ws-line": "",
        "data-tree": isOpen && onTree !== undefined ? treeOpen : undefined,
        "data-pinned": r.pinned,
      },
      openData: { "data-ws-row": "", "data-ws-pinned": r.pinned, "data-selected": isOpen },
    };
  });

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
      <SideList
        rows={side}
        query={query}
        label={groupLabel}
        empty={
          <div className="flex flex-1 flex-col items-center justify-center gap-2 px-6 py-10 text-center">
            <span className="text-fact font-title text-foreground">{gui.workspaces_empty}</span>
            <span className="text-aux text-muted-foreground">{gui.workspaces_how}</span>
          </div>
        }
        // **A list emptied by the box is not an empty list** (ruling 50):
        // 「还没有工作区」 under a typed word would be khor telling somebody
        // they have no projects while three are one backspace away.
        none={
          <div
            data-ws-none-match
            className="flex flex-1 flex-col items-center justify-center px-6 py-10 text-center text-fact text-muted-foreground"
          >
            {gui.workspace_none_match}
          </div>
        }
      />
    </div>
  );
}
