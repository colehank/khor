// **The navigation's order** (the redesign, 2026-10-02). The floating
// navigation seats at most four: the first `NAV_SHOWN` landings in this
// order, then 更多, which holds the rest with 设置 and 这台机器. The owner
// set the default — 机器、会话、额度 — and the user reorders it under
// 外观 (`NavOrderRows`).
//
// Kept in this face's `localStorage`, like the list's arrangement
// (`lib/session-filter`), not in `khor config`: it is a way this screen is
// laid out, and the store has no key for it yet. A face that refuses
// storage simply keeps the default.
import { useEffect, useState } from "react";

/** The five landings, by key. */
export type NavKey = "sessions" | "devices" | "workspaces" | "browser" | "quota";

/** How many landings sit on the bar itself; the fourth seat is 更多. */
export const NAV_SHOWN = 3;

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
