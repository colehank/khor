import {isSettingsPage} from "./settings-navigation";

/** One destination, with stable grouped navigation for every More page. */
export const morePageOrder = ["session","workspace","appearance","connection","security","store","doctor","about"] as const;
/** My Khor retains its persisted route, but is a separate menu destination. */
export type MorePage = "me" | "terminal" | typeof morePageOrder[number];
const KEY = "khor.more-page";
export function isMorePage(value: unknown): value is MorePage {
  return value === "me" || isSettingsPage(value) || typeof value === "string" && (morePageOrder as readonly string[]).includes(value);
}
export function lastMorePage(): MorePage {
  try { const page = window.localStorage.getItem(KEY); return isMorePage(page) && page !== "me" ? page === "terminal" ? "session" : page : "session"; }
  catch { return "session"; }
}
export function rememberMorePage(page: MorePage): void {
  if (page === "me") return;
  if (page === "terminal") page = "session";
  try { window.localStorage.setItem(KEY, page); } catch { /* Navigation works without storage. */ }
}
