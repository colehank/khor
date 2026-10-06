/**
 * 最近, the 附栏's first seat and its landing (UX-30 §3): one list, types
 * mixed, newest first — files, workspaces, pages — each row with the
 * kind mark on the left (UX-32's assembly: the base is the machine, the
 * mark is what) and the time on the right. The rows come from what the
 * app already remembers (`use-side-recent`).
 *
 * **Everything opens in place** (§4): a press opens the thing inside the
 * column — a file as the column's file page, a workspace as its listing,
 * a page in the column's browser. The main area is the session, always;
 * the one way there is a double-click, or the row menu's 「在主区打开」.
 */
import type { Avatar } from "@/gen/bindings/Avatar";
import { SideList, type SideRow } from "@/components/SideList";
import { fileKind, pageKind, workspaceKind } from "@/components/SessionFace";
import { gui } from "@/gen/catalog";
import type { RecentItem } from "@/hooks/use-side-recent";
import { ago } from "@/words";

/** One group, unnamed: the seat already says 最近, and a sole unnamed
    group draws no head (`SideList`). */
const GROUP = "";

function nameOf(it: RecentItem): string {
  switch (it.kind) {
    case "page":
      return it.title;
    case "workspace":
      return it.name || it.path.slice(it.path.lastIndexOf("/") + 1) || it.path;
    case "file":
      return it.path.slice(it.path.lastIndexOf("/") + 1);
  }
}

/** The muted words after the name: where it is. */
function asideOf(it: RecentItem): string {
  switch (it.kind) {
    case "page":
      return it.machine;
    case "workspace":
      return `${it.machine} · ${it.path}`;
    case "file":
      return it.root !== "" && it.path.startsWith(`${it.root}/`) ? it.path.slice(it.root.length + 1, it.path.lastIndexOf("/")) || it.machine : it.machine;
  }
}

export function SideRecent({
  items,
  faceOf,
  onOpen,
  onOpenMain,
}: {
  /** `null` until the first answer. */
  items: RecentItem[] | null;
  /** The machine's face, by device id (the row's base, UX-32). */
  faceOf: (device: string) => Avatar | null;
  onOpen: (it: RecentItem) => void;
  onOpenMain: (it: RecentItem) => void;
}) {
  const rows: SideRow[] = (items ?? []).map((it) => ({
    id: it.key,
    group: GROUP,
    pinned: false,
    face: { machine: faceOf(it.device) },
    mark: it.kind === "page" ? pageKind(it.icon) : it.kind === "workspace" ? workspaceKind(it.repo) : fileKind(nameOf(it)),
    title: nameOf(it),
    name: nameOf(it),
    aside: asideOf(it),
    // A file tab carries no time of its own (`use-side-recent`): its
    // corner stays empty rather than wearing its workspace's.
    corners: { bottom: it.kind !== "file" && it.at !== null ? <span data-recent-at>{ago(it.at)}</span> : undefined },
    find: `${nameOf(it)} ${asideOf(it)}`,
    menu: [{ key: "open-main", label: gui.open_in_main, run: () => onOpenMain(it) }],
    onOpen: () => onOpen(it),
    rowData: { "data-recent-row": it.key, "data-recent-kind": it.kind },
  }));
  const byKey = new Map((items ?? []).map((it) => [it.key, it]));
  return (
    <div
      data-side-recent
      className="flex min-h-0 flex-1 flex-col"
      // The double-click is the way to the main area (§4): read off the
      // row it landed on, so the list keeps its one press for opening.
      onDoubleClick={(e) => {
        const row = (e.target as HTMLElement).closest<HTMLElement>("[data-recent-row]");
        const it = row ? byKey.get(row.dataset.recentRow ?? "") : undefined;
        if (it) onOpenMain(it);
      }}
    >
      <SideList rows={rows} loading={items === null} label={() => gui.group_recent} empty={gui.recent_none} mark={{ "data-list": "recent" }} />
    </div>
  );
}
