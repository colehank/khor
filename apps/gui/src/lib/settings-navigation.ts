/** Settings navigation belongs to the viewing device, never a remote node. */
export const settingsPages = ["session", "workspace", "appearance", "connection", "security", "about"] as const;
/** Terminal remains a compatible deep link into expanded Session settings. */
export type SettingsPage = "terminal" | typeof settingsPages[number];
export type SettingsRequest = { pane?: string | null; machine?: string | null };
const KEY = "khor.settings-pane";

export function isSettingsPage(value: unknown): value is SettingsPage {
  return value === "terminal" || typeof value === "string" && (settingsPages as readonly string[]).includes(value);
}

export function lastSettingsPage(): SettingsPage {
  try {
    const page = window.localStorage.getItem(KEY);
    return isSettingsPage(page) ? page === "terminal" ? "session" : page : "session";
  } catch { return "session"; }
}

export function rememberSettingsPage(page: SettingsPage): void {
  try { window.localStorage.setItem(KEY, page); } catch { /* Navigation still works without storage. */ }
}
