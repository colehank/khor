/** Settings navigation belongs to the viewing device, never a remote node. */
export const settingsPages = ["appearance", "terminal", "session", "workspace", "connection", "security", "about"] as const;
export type SettingsPage = typeof settingsPages[number];
export type SettingsRequest = { pane?: string | null; machine?: string | null };
const KEY = "khor.settings-pane";

export function isSettingsPage(value: unknown): value is SettingsPage {
  return typeof value === "string" && (settingsPages as readonly string[]).includes(value);
}

export function lastSettingsPage(): SettingsPage {
  try {
    const page = window.localStorage.getItem(KEY);
    return isSettingsPage(page) ? page : "appearance";
  } catch { return "appearance"; }
}

export function rememberSettingsPage(page: SettingsPage): void {
  try { window.localStorage.setItem(KEY, page); } catch { /* Navigation still works without storage. */ }
}
