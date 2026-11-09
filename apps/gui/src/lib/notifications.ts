import { useSyncExternalStore } from "react";
import type { SessionRow } from "@/api";
import { finishedAfter } from "@/lib/finished";
import { collectNotifications, emptyNotifications, expireNotifications, notificationMetadata, notificationsEnabled,
  restoreNotifications, sourceKey, scheduleNotifications, filterNotifications, type NotificationState } from "@/lib/notification-banners";
import { heardFrom } from "@/lib/other-windows";

const KEY = "khor.notification-preferences";
const LEGACY = "khor.notification-inbox";
let cached = emptyNotifications(), initialized = false;
let timer: ReturnType<typeof setTimeout> | undefined;
const listeners = new Set<() => void>();
function snapshot(): NotificationState {
  if (!initialized) {
    initialized = true;
    try {
      cached = restoreNotifications(window.localStorage.getItem(KEY) ?? window.localStorage.getItem(LEGACY));
      window.localStorage.setItem(KEY,JSON.stringify(notificationMetadata(cached)));
      window.localStorage.removeItem(LEGACY);
    } catch { /* In-memory banners and preferences remain usable. */ }
  }
  return cached;
}
function publish(next: NotificationState): void {
  next = scheduleNotifications(next, Date.now());
  if (JSON.stringify(next) === JSON.stringify(snapshot())) return;
  cached = next;
  try { window.localStorage.setItem(KEY, JSON.stringify(notificationMetadata(next))); } catch { /* Optional persistence. */ }
  clearTimeout(timer);
  const due = next.items.flatMap(item=>item.expires === null ? [] : [item.expires]);
  if (due.length) timer = setTimeout(()=>publish(expireNotifications(snapshot(),Date.now())),Math.max(0,Math.min(...due)-Date.now()));
  for (const listener of listeners) listener();
}
heardFrom(KEY, () => {
  try {
    const stored = restoreNotifications(window.localStorage.getItem(KEY));
    publish(filterNotifications({ ...snapshot(), ...notificationMetadata(stored) }, item=>notificationsEnabled(stored,{id:item.session,execution_home:item.home} as SessionRow)));
  } catch { /* Keep this window's choices. */ }
});
function subscribe(listener: () => void): () => void { listeners.add(listener); return ()=>{listeners.delete(listener);}; }
export function useNotifications(): NotificationState { return useSyncExternalStore(subscribe,snapshot); }
export function sessionNotificationsEnabled(row: SessionRow): boolean { return notificationsEnabled(snapshot(),row); }
export function receiveNotifications(rows: SessionRow[], onScreen: (id: string)=>boolean, present = true): void {
  publish(collectNotifications(snapshot(),rows,{now:Date.now(),commandSeconds:finishedAfter(),onScreen,present}));
}
export function dismissNotification(key: string): void {
  const current = snapshot(), item = current.items.find(item => item.key === key);
  const sources = item?.members ?? (item ? [item] : []);
  publish({ ...current, items: current.items.filter(item => item.key !== key),
    queue: current.queue.filter(next => !sources.some(source => source.session === next.session && source.home === next.home)) });
}
export function clearNotifications(): void { publish({...snapshot(),items:[],queue:[]}); }
/** Each view combines pointer, hover and focus before changing this flag. */
export function pauseNotification(key: string, pause: boolean): void {
  publish({...snapshot(),items:snapshot().items.map(item=>item.key===key ? {...item,paused:pause} : item)});
}
export function configureNotifications(capacity: number): void { publish({...snapshot(),capacity}); }
export function focusNotifications(focused: boolean): void { publish({...snapshot(),focused}); }
export function setSessionNotifications(row: SessionRow, enabled: boolean): void {
  const current=snapshot(), source=sourceKey(row);
  publish({...filterNotifications(current,item=>enabled || item.session!==row.id || item.home!==row.execution_home),
    preferences:Object.fromEntries([...Object.entries(current.preferences).filter(([key])=>key!==source),[source,enabled]].slice(-1024))});
}
