import {composerFilesAtRisk} from './composer-files';
import {sessionDocumentsAtRisk} from '@/hooks/use-session-documents';
import { useLayoutEffect, useRef } from "react";
import type { UpdateBlocker } from "@/gen/bindings/UpdateBlocker";
import { checkpointSessionDrafts } from "@/hooks/use-session-draft";
import { draftCount } from "./drafts";
import { chatDraftsAtRisk } from "./chat-drafts";
import { RestartChecklist } from "./restart-checklist";

export const updateReadiness = new RestartChecklist();
updateReadiness.register(()=>composerFilesAtRisk()||sessionDocumentsAtRisk()?"attachments":null);
updateReadiness.register(() => draftCount() ? "unsaved-files" : null);
updateReadiness.register(() => chatDraftsAtRisk() ? "unconfirmed-sends" : null);
updateReadiness.register(() => checkpointSessionDrafts() ? null : "draft-storage");

export function useUpdateBlocker(reason: UpdateBlocker, blocked: boolean): void {
  const current = useRef(blocked);
  current.current = blocked;
  useLayoutEffect(() => updateReadiness.register(() => current.current ? reason : null), [reason]);
}

export function useUpdateCheckpoint(check: () => UpdateBlocker | null): void {
  const current = useRef(check);
  current.current = check;
  useLayoutEffect(() => updateReadiness.register(() => current.current()), []);
}
