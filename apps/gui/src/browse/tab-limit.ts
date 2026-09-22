// 标签页上限 (docs/SHELL.md 第三轮 第 4 条): the one rule every tab strip
// in the window shares, written once.
//
// The browse tabs keep theirs in the node (`khor_node::tabs`), because
// `khor tabs open` opens them too and has to obey the same limit. The
// session tabs and the workspace's file tabs are this window's alone,
// so their half is here — the same judgment in the same words: past the
// limit, the tab **touched longest ago** goes, never one a person said
// to keep (a pinned session, a file with unsaved work), never the one in
// front; and when every candidate is kept, nothing goes.
import { useSetting } from "@/settings";

/** The limit, or `null` for `off` — and for a setting that has not
    arrived, which reads as the default rather than as a limit appearing
    late. */
export function useTabsLimit(): number | null {
  return tabsLimitOf(useSetting("tabs-limit"));
}

/** The same reading for a caller that holds the settings rows itself.
    ⚠️ **`App` is one** (docs/SHELL.md 第五轮 第 11 条): it renders the
    settings provider, so `useSetting` there reads the context *above*
    the provider — always nothing — and the session tabs never had a
    limit at all. The face gate that found it is 4Q. */
export function tabsLimitOf(v: string | null): number | null {
  const n = v === null ? Number.NaN : Number(v);
  return Number.isInteger(n) && n > 0 ? n : null;
}

/**
 * Which tab to close so `ids` fits under `limit`, or `null` when it
 * already fits or everything is kept.
 *
 * `touched` is when each was last in front; a tab with no entry reads
 * as never, the honest answer for one restored from storage. Ties go to
 * the one further left.
 */
export function stalestTab(
  ids: readonly string[],
  limit: number | null,
  touched: ReadonlyMap<string, number>,
  keep: (id: string) => boolean,
): string | null {
  if (limit === null || ids.length <= limit) return null;
  let pick: string | null = null;
  let at = Number.POSITIVE_INFINITY;
  for (const id of ids) {
    if (keep(id)) continue;
    const t = touched.get(id) ?? 0;
    if (t < at) {
      at = t;
      pick = id;
    }
  }
  return pick;
}
