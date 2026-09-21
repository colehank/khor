// 设置 (docs/MORE.md) as the rest of the app reads it.
//
// **A context rather than props, because the readers are not near each
// other.** The four keys this batch lands are read on the sessions
// list, the workspace list, the new-session wizard and the file tree —
// four different subtrees, none of them a child of another. Threading
// one array through five layers of props would put a `config` prop on
// components that have nothing to do with settings, and the first one
// somebody forgot to pass down would be a setting that silently stopped
// working on one screen (`linkHome` is the same shape for the same
// reason).
//
// **One fetch, in the shell**, beside the lock's: both are read straight
// off the store with no node opened, so they cost one file read each and
// the answer is there before anything that reads it is drawn.
//
// ⚠️ **Every reader tests for the value that is *not* the default.**
// `useSetting` answers `null` while khor has not answered yet — and a
// reader that spelled out its own default would be a second copy of a
// number only `khor_node::config::KEYS` decides. Asking 「is it the
// other one」 makes not-yet-answered and the default the same case,
// which is what they are.
import { createContext, useContext } from "react";

import type { ConfigRow } from "@/api";

/** `null` until the first answer. Not an empty array: 「no settings」 and
    「not asked yet」 are two different things, and only one of them
    should ever make a switch disappear. */
const Settings = createContext<ConfigRow[] | null>(null);

export const SettingsProvider = Settings.Provider;

/** Ask the shell to read the settings again — for a surface that changes
    one outside the settings pages (the delete window's 不再提醒). */
const Reread = createContext<() => void>(() => {});
export const SettingsRereadProvider = Reread.Provider;
export function useSettingsReread(): () => void {
  return useContext(Reread);
}

/** Every setting, or `null` before the first answer. The two pages that
    draw rows want the whole list; everything else wants one key. */
export function useSettings(): ConfigRow[] | null {
  return useContext(Settings);
}

/**
 * What one setting is set to, or `null` when khor has not said — which
 * covers both 「the answer has not arrived」 and 「this build has no such
 * key」.
 *
 * Both of those are cases a reader must treat as the default, and they
 * are `null` rather than the default itself on purpose: handing back a
 * default from here would be this file holding a copy of the node's
 * table.
 */
export function useSetting(name: string): string | null {
  const rows = useContext(Settings);
  return rows?.find((r) => r.name === name)?.value ?? null;
}
