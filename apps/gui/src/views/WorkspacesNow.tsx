// 工作区落地页的总览 (2026-09-28). What the board shows on the workspaces
// landing when no workspace is open — it was a blank slab, the one
// landing with no overview (10-layout 「每个落地页有一个总览」).
//
// # Facts, not an illustration
//
// The house view is `WorkspaceEmpty`'s: a blank is the app having
// nothing to say at the moment a person arrives, and a picture says the
// same nothing on every visit. What khor already knows, and what a
// person comes to this landing for, is **which project to go back to**:
// the workspaces this face opened last, each with where it is (机器:路径),
// what branch it is on and when its tree last changed — pressable.
//
// # Nothing is asked again
//
// Every fact here is the list's (`WorkspacesList` publishes what it
// heard, `useWorkspaceFacts`): the branch and the last change are one
// question each to a far machine, and the list beside this board has
// already asked them. The one read made here is the table itself, and
// only while the list has not published yet (a list put away at start).
//
// # Two headings, each true
//
// 「最近打开」 once this face has opened a workspace — the order is its own
// memory (`storedWsOpened`). Before that there is nothing to remember,
// so the rows are the workspaces by when their tree last changed and the
// heading says exactly that, 「最近改动」, rather than borrowing a claim.
import { useEffect, useState } from "react";

import { fetchWorkspaces, type DeviceRow, type WorkspaceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { EmptyState, RegionState } from "@/components/RegionState";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { ago } from "@/words";
import { factKey, storedWsOpened, useWorkspaceFacts } from "@/views/WorkspacesList";

/** Six rows: the overview is read at a glance on the way into one
    project; the list beside it holds the rest. */
const SHOWN = 6;

export function WorkspacesNow({
  devices,
  onOpen,
  onAdd,
}: {
  /** Only for the faces, as the list uses it. */
  devices: DeviceRow[];
  onOpen: (row: WorkspaceRow) => void;
  /** The landing's 「+」: the cross-machine directory picker. */
  onAdd: () => void;
}) {
  const facts = useWorkspaceFacts();
  const [own, setOwn] = useState<WorkspaceRow[] | null>(null);
  useEffect(() => {
    if (facts.read) return;
    let live = true;
    fetchWorkspaces().then(
      (rows) => live && setOwn(rows),
      () => live && setOwn([]),
    );
    return () => {
      live = false;
    };
  }, [facts.read]);
  // The list's reading once it has one; before that, what it last knew
  // (`WorkspaceFacts.known`, remembered across a restart) until this
  // pane's own read lands.
  const list = facts.read || (own === null && facts.known) ? facts.list : own;

  if (list === null) return <RegionState state="loading-never" />;
  if (list.length === 0) {
    // **No workspace at all**: one sentence and the one way to add one
    // (components/EmptyState — at most one button).
    return (
      <EmptyState
        data-ws-now-empty
        action={
          <Button data-ws-now-add onClick={onAdd} className="min-h-touch">
            {gui.ws_add_ask}
          </Button>
        }
      >
        {gui.workspaces_empty}
      </EmptyState>
    );
  }

  const opened = storedWsOpened()
    .map((o) => list.find((r) => r.device === o.device && r.path === o.path))
    .filter((r): r is WorkspaceRow => r !== undefined);
  const recent = opened.length > 0;
  const rows = (
    recent
      ? opened
      : [...list].sort(
          (a, b) => (facts.times[factKey(b)] ?? 0) - (facts.times[factKey(a)] ?? 0) || a.path.localeCompare(b.path),
        )
  ).slice(0, SHOWN);

  return (
    // MeshNow's shape: one column centred in the board, `my-auto` so a
    // short one sits in the middle and a long one keeps its head
    // reachable.
    <div data-ws-now className="flex h-full min-h-0 flex-col items-center overflow-y-auto p-pane">
      <section data-ws-now-list={recent ? "opened" : "changed"} className="my-auto flex w-full max-w-md flex-col">
        <h2 className="mb-in px-in text-caption font-label text-muted-foreground">
          {recent ? gui.ws_empty_recent : gui.ws_now_changed}
        </h2>
        {rows.map((r) => {
          const k = factKey(r);
          const git = facts.marks[k];
          const at = facts.times[k] ?? 0;
          return (
            <button
              key={k}
              type="button"
              data-ws-now-row={`${r.device}:${r.path}`}
              onClick={() => onOpen(r)}
              className="flex min-h-touch w-full min-w-0 items-center gap-row rounded-md px-in py-xxs text-left hover:bg-fill"
            >
              <MachineAvatar
                face={devices.find((d) => d.id === r.device)?.face ?? null}
                className="size-avatar flex-none"
              />
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="flex min-w-0 items-baseline gap-in">
                  <span className="min-w-0 truncate text-body font-title">{r.name}</span>
                  {/* The branch, or nothing: not a repository, a machine
                      that could not be asked and a detached HEAD all
                      arrive as nothing (the list's rule). */}
                  {git && git.branch !== "" && (
                    <span data-ws-now-branch className="min-w-0 truncate font-mono text-caption text-muted-foreground">
                      {git.branch}
                    </span>
                  )}
                </span>
                <span className="flex min-w-0 items-baseline gap-in text-callout text-muted-foreground">
                  <span className="min-w-0 truncate">{r.machine === "" ? r.path : `${r.machine}:${r.path}`}</span>
                  <span data-ws-now-touched className="ml-auto flex-none tabular-nums">
                    {at > 0 ? ago(at) : ""}
                  </span>
                </span>
              </span>
            </button>
          );
        })}
      </section>
    </div>
  );
}
