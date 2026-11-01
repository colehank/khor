import { useSyncExternalStore } from "react";

import type { SessionRow } from "@/api";
import { finishedAfter } from "@/lib/finished";
import { collectNotifications, emptyInbox, notificationsEnabled, pendingNotification, restoreInbox, sourceKey, type Inbox } from "@/lib/notification-inbox";
import { heardFrom } from "@/lib/other-windows";

const KEY = "khor.notification-inbox";
let cached = emptyInbox();
const listeners = new Set<() => void>();
let initialized = false;
let storageAvailable = true;

function load(): Inbox {
  if (!storageAvailable) return cached;
  try { return restoreInbox(window.localStorage.getItem(KEY)); } catch { storageAvailable = false; return cached; }
}
function snapshot(): Inbox {
  if (!initialized) { cached = load(); initialized = true; }
  return cached;
}
function publish(next: Inbox): void {
  if (JSON.stringify(next) === JSON.stringify(snapshot())) return;
  cached = next;
  try { window.localStorage.setItem(KEY, JSON.stringify(next)); } catch { storageAvailable = false; }
  for (const listener of listeners) listener();
}
heardFrom(KEY, () => {
  cached = load(); initialized = true;
  for (const listener of listeners) listener();
});
function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}
export function useNotificationInbox(): Inbox { return useSyncExternalStore(subscribe, snapshot); }
export function sessionNotificationsEnabled(row: SessionRow): boolean { return notificationsEnabled(load(), row); }

export function receiveNotifications(rows: SessionRow[], onScreen: (id: string) => boolean): void {
  const current = load();
  const result = collectNotifications(current, rows, { commandSeconds: finishedAfter(), onScreen });
  publish(result);
}
export function readNotification(key?: string): void {
  const current = load();
  publish({ ...current, items: current.items.map((item) => key === undefined || key === item.key ? { ...item, read: true } : item) });
}
export function removeNotification(key: string): void {
  const current = load();
  publish({ ...current, items: current.items.filter((item) => item.key !== key || pendingNotification(item)) });
}
export function setSessionNotifications(row: SessionRow, enabled: boolean): void {
  const current = load();
  publish({ ...current,
    items: current.items.map((item) => !enabled && item.session === row.id && item.home === row.execution_home ? { ...item, read: true } : item),
    preferences: Object.fromEntries([...Object.entries(current.preferences).filter(([key]) => key !== sourceKey(row)), [sourceKey(row), enabled]].slice(-1024)),
  });
}
