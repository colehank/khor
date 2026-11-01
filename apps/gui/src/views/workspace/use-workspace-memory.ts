import { useCallback, useLayoutEffect, useRef, useSyncExternalStore, type Dispatch, type SetStateAction } from "react";

import { reportDrafts } from "@/lib/drafts";
import { WorkspaceMemory, type WorkspaceFields } from "./workspace-memory";

export function useWorkspaceField<K extends keyof WorkspaceFields>(
  memory: WorkspaceMemory,
  key: K,
  initialize: () => WorkspaceFields[K],
): [WorkspaceFields[K], Dispatch<SetStateAction<WorkspaceFields[K]>>] {
  memory.ensure(key, initialize);
  const snapshot = useCallback(() => memory.read(key), [memory, key]);
  const value = useSyncExternalStore(memory.subscribe, snapshot, snapshot);
  const update = useCallback((next: SetStateAction<WorkspaceFields[K]>) => memory.update(key, next), [memory, key]);
  return [value, update];
}

/** Draft reporting follows the model, including when its view is detached. */
export function WorkspaceDraftOwner({ memory }: { memory: WorkspaceMemory }) {
  const owner = useRef(Symbol("workspace-drafts"));
  useLayoutEffect(() => {
    const report = () => reportDrafts(owner.current, memory.draftCount());
    report();
    const detach = memory.subscribe(report);
    return () => {
      detach();
      reportDrafts(owner.current, 0);
    };
  }, [memory]);
  return null;
}
