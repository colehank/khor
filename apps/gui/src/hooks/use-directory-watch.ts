import { useLayoutEffect, useState, useSyncExternalStore } from "react";
import { dirsBegin, dirsWait, dirsLeave, inApp } from "@/api";
import type { DirWatchScope } from "@/gen/bindings/DirWatchScope";
import { DirectoryWatch, directoryWatchKey } from "@/lib/directory-watch";
import { useHidden } from "@/hooks/use-hidden";

/** Hidden views release their server owner, including replies to a late begin. */
export function useDirectoryWatch(machine: string, scopes: DirWatchScope[], active = true) {
  const hidden = useHidden();
  const [owner] = useState(() => new DirectoryWatch({ begin: dirsBegin, wait: dirsWait, leave: dirsLeave }, inApp ? 20_000 : 0,
    (run, delay) => { const timer = window.setTimeout(run, delay); return () => window.clearTimeout(timer); }));
  const key = directoryWatchKey({ machine, scopes });
  useLayoutEffect(() => {
    owner.set(active && !hidden && scopes.length > 0 ? { machine, scopes } : null);
    return () => owner.set(null);
    // The serialized scope order is the server coverage index identity.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [owner, key, active, hidden]);
  const state = useSyncExternalStore(owner.subscribe, owner.snapshot, owner.snapshot);
  const wanted = active && !hidden && scopes.length > 0 ? key : null;
  return state.key === wanted ? state : {
    key: wanted, scopes: wanted === null ? [] : scopes,
    status: wanted === null ? "idle" as const : "pending" as const,
    batch: null, error: null,
  };
}
