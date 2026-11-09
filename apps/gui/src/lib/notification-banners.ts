import type { SessionRow } from "@/api";
import { sessionContentPreview } from "@/lib/session-preview";
import type { ActivityEvent } from "@/gen/bindings/ActivityEvent";

const SOURCE_LIMIT = 1024;
export const BANNER_LIMIT = 3;
export const BANNER_DURATION = 8000;
export const QUEUE_LIMIT = 32;
export const SUMMARY_THRESHOLD = 5;
const FRESH_FOR = 120000;
type Cursor = { at: number; ids: string[] };
/** Only watermarks and explicit subscription choices may leave memory. */
export type NotificationPreferences = { version: 1; cursors: Record<string, Cursor>; preferences: Record<string, boolean> };
export type NotificationMember = ActivityEvent & { session: string; home: string; title: string; preview: string | null };
export type NotificationBanner = NotificationMember & {
  key: string; expires: number | null; remaining: number; paused: boolean;
  members?: NotificationMember[];
};
export type NotificationState = NotificationPreferences & {
  items: NotificationBanner[]; queue: NotificationMember[]; capacity: number; focused: boolean;
};
export const emptyNotifications = (): NotificationState => ({ version: 1, items: [], queue: [], capacity: BANNER_LIMIT, focused: true, cursors: {}, preferences: {} });
export const sourceKey = (row: Pick<SessionRow, "id" | "execution_home">): string => JSON.stringify([row.execution_home, row.id]);
export const notificationsEnabled = (state: NotificationPreferences, row: SessionRow): boolean =>
  state.preferences[sourceKey(row)] ?? (!row.imported && row.observation_owner === null);

/** Also accepts the old inbox for migration, deliberately discarding all content. */
export function restoreNotifications(raw: string | null): NotificationState {
  try {
    if (!raw || raw.length > 4 * 1024 * 1024) return emptyNotifications();
    const stored = JSON.parse(raw) as NotificationPreferences | null;
    if (!stored || stored.version !== 1) return emptyNotifications();
    const cursors = Object.fromEntries(Object.entries(stored.cursors ?? {}).filter(([, c]) =>
      c && Number.isSafeInteger(c.at) && c.at >= 0 && Array.isArray(c.ids) && c.ids.every(id => typeof id === "string" && id.length <= 256))
      .slice(-SOURCE_LIMIT).map(([k,c]) => [k, { at: c.at, ids: c.ids.slice(-24) }]));
    const preferences = Object.fromEntries(Object.entries(stored.preferences ?? {}).filter(([, v]) => typeof v === "boolean").slice(-SOURCE_LIMIT));
    return { ...emptyNotifications(), cursors, preferences };
  } catch { return emptyNotifications(); }
}
export function notificationMetadata(state: NotificationState): NotificationPreferences {
  return { version: 1, cursors: state.cursors, preferences: state.preferences };
}
function validEvent(event: ActivityEvent): boolean {
  return !!event && typeof event.id === "string" && event.id.length <= 256 && Number.isSafeInteger(event.at_ms) && event.at_ms > 0 &&
    ["needs_input", "turn_completed", "failed", "command_finished", "notice"].includes(event.kind) &&
    typeof event.resolved === "boolean" && (event.text === null || typeof event.text === "string");
}
const memberKey = (item: NotificationMember): string => JSON.stringify([item.home, item.session]);
const members = (item: NotificationBanner): NotificationMember[] => item.members ?? [item];
const banner = (item: NotificationMember): NotificationBanner => ({ ...item, key: memberKey(item), expires: null, remaining: BANNER_DURATION, paused: false });
/** Removing a presentation never resolves or marks a host event read. */
export function filterNotifications(state: NotificationState, keep: (item: NotificationMember) => boolean): NotificationState {
  return { ...state, queue: state.queue.filter(keep), items: state.items.flatMap(item => {
    const kept = members(item).filter(keep);
    if (!kept.length) return [];
    return [item.members ? { ...item, members: kept } : item];
  }) };
}
/** Scheduling is separate from collection: a queued notice has no dismissal clock. */
export function scheduleNotifications(state: NotificationState, now: number): NotificationState {
  let items = state.items.filter(item => item.expires === null || item.expires > now);
  let queue = state.queue.filter(item => now - item.at_ms <= FRESH_FOR).slice(-QUEUE_LIMIT);
  // Coalesce in place, but hold the newest text until interaction ends.
  items = items.map(item => {
    if (item.paused || !state.focused) return item;
    if (item.members) {
      const pending = queue.filter(next => item.members!.some(old => memberKey(old) === memberKey(next)));
      if (!pending.length) return item;
      const merged = new Map(item.members.map(member => [memberKey(member), member]));
      for (const next of pending) merged.set(memberKey(next), next);
      queue = queue.filter(next => !merged.has(memberKey(next)));
      return { ...item, members: [...merged.values()], remaining: BANNER_DURATION, expires: null };
    }
    const update = queue.find(next => memberKey(next) === memberKey(item));
    if (!update) return item;
    queue = queue.filter(next => memberKey(next) !== memberKey(item));
    return { ...banner(update), key: item.key };
  });
  const protectedItems = items.filter(item => item.paused || !state.focused);
  const held = queue.filter(next => protectedItems.some(item => members(item).some(member => memberKey(member) === memberKey(next))));
  const available = [...items.filter(item => !protectedItems.includes(item)).flatMap(members), ...queue.filter(next => !held.includes(next))];
  const merged = new Map(available.map(item => [memberKey(item), item]));
  if (merged.size >= SUMMARY_THRESHOLD || items.some(item => item.members && !protectedItems.includes(item)) && queue.length) {
    const previous = items.find(item => item.members && !protectedItems.includes(item));
    const grouped = [...merged.values()].slice(-QUEUE_LIMIT);
    const summary = { ...banner(grouped.at(-1)!), key: "notification-summary", members: grouped };
    // Do not repeatedly reset a quiet summary on ordinary polls.
    const unchanged = previous && JSON.stringify(previous.members) === JSON.stringify(grouped);
    if (protectedItems.some(item => item.members)) {
      queue = [...held, ...merged.values()];
      items = protectedItems;
    } else {
      items = [...protectedItems, unchanged ? previous : summary];
      queue = held;
    }
  }
  // Shrinking to phone width keeps an interacted banner; others return to the queue.
  const capacity = Math.max(1, Math.min(BANNER_LIMIT, state.capacity));
  if (items.length > capacity) {
    const retained = [...items.filter(item => item.paused), ...items.filter(item => !item.paused)].slice(0, capacity);
    queue = [...items.filter(item => !retained.includes(item)).flatMap(members), ...queue];
    items = retained;
  }
  while (items.length < capacity && state.focused) {
    const index = queue.findIndex(next => !items.some(item => members(item).some(member => memberKey(member) === memberKey(next))));
    if (index < 0) break;
    const [next] = queue.splice(index, 1);
    items.push(banner(next));
  }
  items = items.map(item => item.paused || !state.focused
    ? { ...item, remaining: item.expires === null ? item.remaining : Math.max(0, item.expires - now), expires: null }
    : item.expires === null ? { ...item, expires: now + item.remaining } : item);
  return { ...state, items, queue: queue.slice(-QUEUE_LIMIT) };
}
export type NotificationPresence = { waiting: boolean; snapshot: unknown };
/** On resume, stale cached rows are not the fresh host snapshot we must consume. */
export function notificationPresence(previous: NotificationPresence, snapshot: unknown, present: boolean): { next: NotificationPresence; deliver: boolean } {
  if (!present) return { next: { waiting: true, snapshot }, deliver: false };
  if (previous.waiting) return { next: { waiting: previous.snapshot === snapshot, snapshot }, deliver: false };
  return { next: { waiting: false, snapshot }, deliver: true };
}
export function collectNotifications(state: NotificationState, rows: SessionRow[], options: {
  now: number; commandSeconds: number; present: boolean; onScreen: (id: string) => boolean;
}): NotificationState {
  let next = state;
  const cursors = { ...state.cursors };
  for (const row of rows) {
    if (!row.execution_home) continue;
    const source = sourceKey(row), previous = cursors[source];
    let cursor = previous ?? { at: 0, ids: [] };
    const events = (row.activity ?? []).filter(validEvent).sort((a,b) => a.at_ms - b.at_ms);
    const disposition = Math.max(0, ...events.filter(e => ["turn_completed","failed","needs_input"].includes(e.kind)).map(e => e.at_ms));
    next = filterNotifications(next, item => {
      if (item.session !== row.id || item.home !== row.execution_home) return true;
      const resolved = item.kind === "needs_input" && (disposition > item.at_ms || events.some(e => e.id === item.id && e.resolved));
      return !resolved && !options.onScreen(row.id) && notificationsEnabled(state,row);
    });
    for (const event of events) {
      const fresh = !!previous && (event.at_ms > previous.at || (event.at_ms === previous.at && !previous.ids.includes(event.id)));
      if (event.at_ms > cursor.at) cursor = { at: event.at_ms, ids: [event.id] };
      else if (event.at_ms === cursor.at && !cursor.ids.includes(event.id)) cursor = { ...cursor, ids: [...cursor.ids,event.id].slice(-24) };
      if (!fresh || !options.present || !notificationsEnabled(state,row) || options.onScreen(row.id) ||
        options.now - event.at_ms > FRESH_FOR || event.at_ms > options.now + FRESH_FOR ||
        event.resolved && event.kind === "needs_input" || event.kind === "needs_input" && disposition > event.at_ms) continue;
      if (event.kind === "command_finished" && (options.commandSeconds === 0 || (event.duration_ms ?? 0) < options.commandSeconds * 1000 || event.exit === 130 || event.exit === 141)) continue;
      const item = { ...event, text: event.text?.slice(0,160) ?? null, session: row.id, home: row.execution_home, title: row.title.slice(0,200), preview: sessionContentPreview(row)?.slice(0,320) ?? null };
      next = { ...next, queue: [...next.queue.filter(old => memberKey(old) !== memberKey(item)), item].slice(-QUEUE_LIMIT) };
    }
    cursors[source] = cursor;
  }
  if (!options.present) next = { ...next, items: [], queue: [] };
  return scheduleNotifications({ ...next, cursors: Object.fromEntries(Object.entries(cursors).sort((a,b)=>a[1].at-b[1].at).slice(-SOURCE_LIMIT)) }, options.now);
}
export function expireNotifications(state: NotificationState, now: number): NotificationState {
  return scheduleNotifications(state, now);
}
