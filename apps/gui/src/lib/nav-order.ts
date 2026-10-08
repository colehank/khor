// **The navigation's order** (the redesign, 2026-10-02). The narrow
// face's floating navigation seats the first `NAV_SHOWN` landings in this
// order, then 更多, which holds the rest with 设置 and 这台机器. The wide
// face's strip seats all of them, in this order, with 更多 a circle of
// its own beside it. The owner set the default and the user reorders it
// under 外观 (`NavOrderRows`); 更多 is always last.
//
// **Five seats since UX-11** (the owner on ux-9's I1, 2026-10-04:
// 「底栏5项吧，是不是默认机器、会话、额度、工作区就行了」): four landings —
// 机器, 会话, 额度, 工作区 by default — then 更多 (浏览器, 设置, 这台
// 机器). HIG Tab bars: avoid overflow tabs, and keep 更多 rare; four
// landings leave one in it. ⚠️ 墓碑: three landings and 更多, the
// redesign's first bar (2026-10-02).
//
// Kept in this face's `localStorage`, like the list's arrangement
// (`lib/session-filter`), not in `khor config`: it is a way this screen is
// laid out, and the store has no key for it yet. A face that refuses
// storage simply keeps the default.
import { useEffect, useState } from "react";

import { heardFrom } from "@/lib/other-windows";

/** The five landings, by key. */
export type NavKey = "sessions" | "devices" | "workspaces" | "browser" | "quota";

/** How many landings sit in the narrow bar before 更多, its fifth seat;
    the rest are under 更多. */
export const NAV_SHOWN = 4;

export const NAV_DEFAULT: NavKey[] = ["devices", "sessions", "quota", "workspaces", "browser"];

const KEY = "khor.nav-order";
const CHANGED = "khor:nav-order";

/** The stored order, mended: unknown keys dropped, missing ones appended
    in the default's order, so a newer or older build's list still names
    every landing exactly once. */
export function storedNavOrder(): NavKey[] {
  let saved: unknown = null;
  try {
    saved = JSON.parse(window.localStorage.getItem(KEY) ?? "null");
  } catch {
    saved = null;
  }
  const known = Array.isArray(saved) ? saved.filter((k): k is NavKey => NAV_DEFAULT.includes(k as NavKey)) : [];
  const seen = [...new Set(known)];
  return [...seen, ...NAV_DEFAULT.filter((k) => !seen.includes(k))];
}

export function rememberNavOrder(order: NavKey[]) {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(order));
  } catch {
    // A face without storage keeps the order for this session only.
  }
  window.dispatchEvent(new CustomEvent(CHANGED, { detail: order }));
}

// Reordered in another window of this app (the Mac's settings window,
// `lib/other-windows`): this window's strip takes it at once.
heardFrom(KEY, () => window.dispatchEvent(new CustomEvent(CHANGED, { detail: storedNavOrder() })));

/** The order, live: the settings rows and the navigation are not each
    other's children, and a reorder shows on the bar at once. */
export function useNavOrder(): NavKey[] {
  const [order, setOrder] = useState<NavKey[]>(storedNavOrder);
  useEffect(() => {
    const on = (e: Event) => setOrder((e as CustomEvent<NavKey[]>).detail);
    window.addEventListener(CHANGED, on);
    return () => window.removeEventListener(CHANGED, on);
  }, []);
  return order;
}
