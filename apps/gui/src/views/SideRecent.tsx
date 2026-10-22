/**
 * 最近, the 附栏's first seat and its landing (UX-30 §3): one list, types
 * mixed, newest first — files, workspaces, pages — each row with the
 * kind mark on the left (UX-32's assembly: the base is the machine, the
 * mark is what) and the time on the right. The rows come from what the
 * app already remembers (`use-side-recent`).
 *
 * **Everything opens in place** (§4): a press opens the thing inside the
 * column — a file as the column's file page, a workspace as its listing,
 * a page in the column's browser. The main area remains the session.
 *
 * **删除 forgets a row** (UX-44 §3): slid left under a finger (a full
 * swipe at once), or the menu's last item under a mouse. Only the record
 * goes; what it pointed at is untouched.
 *
 * **This session's slice, and only it** (UX-37 §1, UX-39 §3): what was
 * opened from this session — from its terminal's paths and URLs, or from
 * this 附栏 while the session was in front — as the record says
 * (`session` on a recent, `use-side-recent`). It starts with the day
 * (`today`, 过去24小时, UX-39 §4), or at the first opened item when the
 * day has no record (UX-42). A page opened from a row wears ‹ back
 * here (`App`'s `sideFrom`). ⚠️ 墓碑 (UX-37 §1 → UX-39 §3): ‹ from the
 * slice to the whole, 全部最近.
 */
import { useState, type ReactNode } from "react";

import type { Avatar } from "@/gen/bindings/Avatar";
import { PathCut } from "@/components/PathCut";
import { SideFind } from "@/components/SideFind";
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

/** **Where it is** (UX-48, the row's bottom-left): a page's address, a
    directory's or a file's path. ⚠️ 墓碑: the machine (and the folder
    within the workspace) as the name's aside — the face says the machine. */
function whereOf(it: RecentItem): string {
  return it.kind === "page" ? it.url : it.path;
}

export function SideRecent({
  items,
  slice,
  today,
  faceOf,
  onOpen,
  onForget,
}: {
  /** `null` until the first answer. */
  items: RecentItem[] | null;
  /** The session whose slice this is. */
  slice: string;
  /** The first row, the day (`DayToday`, UX-39 §4). */
  today?: ReactNode;
  /** The machine's face, by device id (the row's base, UX-32). */
  faceOf: (device: string) => Avatar | null;
  onOpen: (it: RecentItem) => void;
  /** 删除 on a row: the record only (`use-side-recent`'s forget). */
  onForget: (it: RecentItem) => Promise<void>;
}) {
  const [query, setQuery] = useState("");
  const shown = items?.filter((it) => it.session === slice) ?? null;
  const rows: SideRow[] = (shown ?? []).map((it) => ({
    id: it.key,
    group: GROUP,
    pinned: false,
    face: { machine: faceOf(it.device) },
    mark: it.kind === "page" ? pageKind(it.icon) : it.kind === "workspace" ? workspaceKind(it.repo) : fileKind(nameOf(it)),
    title: nameOf(it),
    name: nameOf(it),
    // **UX-48** (the owner 2026-10-07): bottom-left where it is; no state
    // on the right; bottom-right how long ago it was opened. A file tab
    // carries no time of its own (`use-side-recent`): its corner stays
    // empty rather than wearing its workspace's. A file the record holds
    // (no workspace root) has its own visit's.
    sub:
      it.kind === "page" ? (
        <span data-recent-where className="block min-w-0 truncate">
          {whereOf(it)}
        </span>
      ) : (
        <PathCut data-recent-where text={whereOf(it)} />
      ),
    corners: { bottom: (it.kind !== "file" || it.root === "") && it.at !== null ? <span data-recent-at>{ago(it.at)}</span> : undefined },
    find: `${nameOf(it)} ${whereOf(it)}`,
    menu: [
      // **删除 forgets the record, nothing else** (UX-44 §3): the file,
      // the page, the directory stay where they are. Asked nothing — what
      // goes is a line of memory, and going there again brings it back.
      // The menu is the mouse's way (the Mac column); a finger slides.
      { key: "forget", label: gui.delete, destructive: true, run: () => onForget(it) },
    ],
    swipe: { key: "forget", label: gui.delete, run: () => onForget(it) },
    onOpen: () => onOpen(it),
    rowData: { "data-recent-row": it.key, "data-recent-kind": it.kind },
  }));
  return (
    <div
      data-side-recent
      className="flex min-h-0 flex-1 flex-col"

    >
      {/* No head of its own: the 附栏's one head says 最近 (UX-39 §2).
          The find row, with no ＋: nothing is made here (UX-40 §1); while
          it holds words, 过去24小时 steps aside and the rows narrow. */}
      <SideFind label={gui.search_recent} query={query} onQuery={setQuery} />
      <div data-nav-clear className="min-h-0 flex-1 overflow-y-auto">
        {query.trim() === "" && today}
        <SideList
          rows={rows}
          query={query}
          loading={items === null}
          label={() => gui.group_recent}
          empty={gui.side_recent_here_none}
          mark={{ "data-list": "recent", "data-recent-slice": "here" }}
        />
      </div>
    </div>
  );
}
