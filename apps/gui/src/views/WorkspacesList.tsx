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
import { RowMenu, type RowAction } from "@/components/RowMenu";
import { gui } from "@/gen/catalog";
import { useHidden } from "@/hooks/use-hidden";
import { cn } from "@/lib/utils";
import { ago } from "@/words";

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
  reloadAt = 0,
  selected,
  onOpen,
}: {
  /** Only for the faces: a row's avatar is its machine's, and the
      device table is where faces live. The list itself comes from the
      workspace table and not from here. */
  devices: DeviceRow[];
  /** Bumped by the shell when **it** has changed the table — the pane
      bar's 「+」 lives up there (`App`). Every other read this list
      makes is its own; this is the one change it has no way to
      notice. */
  reloadAt?: number;
  /** The open workspace, or null. Two fields rather than one joined
      key, for `keyOf`'s reason. */
  selected: { device: string; path: string } | null;
  onOpen: (row: WorkspaceRow) => void;
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
        // No confirm: nothing is destroyed. The directory is untouched
        // (user, 2026-09-14) and the row comes back with one `ws add`.
        label: gui.ws_remove,
        note: refused.get(k),
        run: run(() => wsRm(r.device, r.path)),
      },
    ];
  };

  if (rows.length === 0) {
    if (!read) return <div className="flex min-h-0 flex-1 flex-col" />;
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
        const changed = git ? git.rows.length : 0;
        return (
          <RowMenu key={k} items={actions(r)}>
            <button
              data-ws-row
              data-ws-pinned={r.pinned}
              type="button"
              onClick={() => onOpen(r)}
              className={cn(
                "flex min-h-touch w-full flex-col items-start gap-0.5 border-b border-border/50 px-3 py-2.5 text-left",
                isOpen && "bg-card",
              )}
            >
              <span className="flex w-full items-center gap-1.5">
                <MachineAvatar
                  face={devices.find((d) => d.id === r.device)?.face ?? null}
                  className="size-kind-mark flex-none"
                />
                <span data-ws-name className="truncate text-fact font-title text-foreground">
                  {r.name}
                </span>
                {/* **The branch, or nothing at all.** A directory that
                    is not a repository, a machine that could not be
                    asked, a detached HEAD with no name to say: all
                    three arrive here as nothing, and 「没有就不显示」
                    is the rule for all three. The corner keeps its
                    anchor so the absence is a readable fact rather than
                    a missing element. */}
                <span
                  data-ws-git
                  className="ml-auto shrink-0 truncate text-aux text-muted-foreground"
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
        );
      })}
    </div>
  );
}
