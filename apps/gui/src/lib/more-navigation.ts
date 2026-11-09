import {settingsPages} from "./settings-navigation";

/** One destination, with stable grouped navigation for every More page. */
export const morePageOrder = ["me", ...settingsPages, "store", "notifications", "doctor"] as const;
export type MorePage = typeof morePageOrder[number];
const KEY = "khor.more-page";
export function isMorePage(value: unknown): value is MorePage {
  return typeof value === "string" && (morePageOrder as readonly string[]).includes(value);
}
export function lastMorePage(): MorePage {
  try { const page = window.localStorage.getItem(KEY); return isMorePage(page) ? page : "me"; }
  catch { return "me"; }
}
export function rememberMorePage(page: MorePage): void {
  try { window.localStorage.setItem(KEY, page); } catch { /* Navigation works without storage. */ }
}
