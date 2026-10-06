// 最近, the 附栏's first seat (UX-30 §3): one list, types mixed, newest
// first — files, workspaces, pages — read from what the app already
// remembers. **No new store**: the pages are the browser's history
// (`recents` of kind `page`, `khor_sync::recents`, the same on every
// device), the workspaces are the workspace list with its last change,
// and the files are the workspaces' open tabs (`WorkspacePane`'s
// `khor.ws-tabs.*`, this device's). A file tab carries no time of its own
// — the strip never needed one — so a file row sits with its workspace
// and shows no time; adding a clock to the tab store would be the store
// the ruling forbids.
import { useEffect, useState } from "react";

import { fetchPageIcons, fetchRecents, fetchWorkspaces } from "@/api";
import type { WorkspaceRow } from "@/gen/bindings/WorkspaceRow";
import { storedTabs, fileOf } from "@/views/WorkspacePane";

export type RecentItem =
  | { kind: "page"; key: string; device: string; machine: string; url: string; title: string; icon?: string; at: number }
  | { kind: "workspace"; key: string; device: string; machine: string; path: string; name: string; repo: boolean; at: number | null }
  | { kind: "file"; key: string; device: string; machine: string; path: string; root: string; at: number | null };

/** How many pages are asked for: the list is a glance, not a history page. */
const PAGES = 30;

function originOf(url: string): string {
  try {
    const u = new URL(url);
    return u.protocol === "http:" || u.protocol === "https:" ? u.origin : "";
  } catch {
    return "";
  }
}

/** The rows, newest first; `null` until the first answer. Asked when
    `on` turns true (the seat shown) and again each time it does. */
export function useSideRecent(on: boolean): RecentItem[] | null {
  const [items, setItems] = useState<RecentItem[] | null>(null);
  useEffect(() => {
    if (!on) return;
    let live = true;
    void Promise.all([
      fetchRecents("page", PAGES).catch(() => []),
      fetchWorkspaces().catch(() => [] as WorkspaceRow[]),
      fetchPageIcons().catch(() => ({}) as Record<string, string>),
    ]).then(([pages, workspaces, icons]) => {
      if (!live) return;
      const out: RecentItem[] = [];
      for (const p of pages) {
        out.push({
          kind: "page",
          key: `page ${p.device} ${p.place}`,
          device: p.device,
          machine: p.name,
          url: p.place,
          title: p.place,
          icon: icons[originOf(p.place)],
          at: p.at_ms,
        });
      }
      for (const w of workspaces) {
        const machine = w.machine || w.device;
        // An old node's row may lack the field; nothing is not a time.
        const at = typeof w.changed_ms === "number" && Number.isFinite(w.changed_ms) ? w.changed_ms : null;
        out.push({
          kind: "workspace",
          key: `ws ${w.device} ${w.path}`,
          device: w.device,
          machine,
          path: w.path,
          name: w.name,
          repo: w.branch !== null,
          at,
        });
        // The workspace's open files, last opened last in the strip —
        // reversed here, so the one a person was just in comes first.
        for (const tab of storedTabs(machine, w.path).map(fileOf).reverse()) {
          out.push({ kind: "file", key: `file ${w.device} ${tab}`, device: w.device, machine, path: tab, root: w.path, at });
        }
      }
      // Newest first; what has no time keeps its place after what has.
      out.sort((a, b) => (b.at ?? -1) - (a.at ?? -1));
      setItems(out);
    });
    return () => {
      live = false;
    };
  }, [on]);
  return items;
}
