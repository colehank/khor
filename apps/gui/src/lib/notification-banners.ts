import type { SessionRow } from "@/api";
import type { ActivityEvent } from "@/gen/bindings/ActivityEvent";

const SOURCE_LIMIT = 1024;
export const BANNER_LIMIT = 3;
export const BANNER_DURATION = 8000;
const FRESH_FOR = 120000;
type Cursor = { at: number; ids: string[] };
/** Only watermarks and explicit subscription choices may leave memory. */
export type NotificationPreferences = { version: 1; cursors: Record<string, Cursor>; preferences: Record<string, boolean> };
export type NotificationBanner = ActivityEvent & {
  key: string; session: string; home: string; title: string;
  expires: number | null; remaining: number;
};
export type NotificationState = NotificationPreferences & { items: NotificationBanner[] };
export const emptyNotifications = (): NotificationState => ({ version: 1, items: [], cursors: {}, preferences: {} });
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
    return { version: 1, items: [], cursors, preferences };
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
export function collectNotifications(state: NotificationState, rows: SessionRow[], options: {
  now: number; commandSeconds: number; present: boolean; onScreen: (id: string) => boolean;
}): NotificationState {
  const items = new Map(state.items.map(item => [item.key,item]));
  const cursors = { ...state.cursors };
  for (const row of rows) {
    if (!row.execution_home) continue;
    const source = sourceKey(row), previous = cursors[source];
    let cursor = previous ?? { at: 0, ids: [] };
    const events = (row.activity ?? []).filter(validEvent);
    const disposition = Math.max(0, ...events.filter(e => ["turn_completed","failed","needs_input"].includes(e.kind)).map(e => e.at_ms));
    for (const [key,item] of items) {
      if (item.session !== row.id || item.home !== row.execution_home) continue;
      const resolved = item.kind === "needs_input" && (disposition > item.at_ms || events.some(e => e.id === item.id && e.resolved));
      if (resolved || options.onScreen(row.id) || !notificationsEnabled(state,row)) items.delete(key);
    }
    for (const event of events) {
      const fresh = !!previous && (event.at_ms > previous.at || (event.at_ms === previous.at && !previous.ids.includes(event.id)));
      if (event.at_ms > cursor.at) cursor = { at: event.at_ms, ids: [event.id] };
      else if (event.at_ms === cursor.at && !cursor.ids.includes(event.id)) cursor = { ...cursor, ids: [...cursor.ids,event.id].slice(-24) };
      if (!fresh || !options.present || !notificationsEnabled(state,row) || options.onScreen(row.id) ||
        options.now - event.at_ms > FRESH_FOR || event.at_ms > options.now + FRESH_FOR ||
        event.resolved && event.kind === "needs_input" || event.kind === "needs_input" && disposition > event.at_ms) continue;
      if (event.kind === "command_finished" && (options.commandSeconds === 0 || (event.duration_ms ?? 0) < options.commandSeconds * 1000 || event.exit === 130 || event.exit === 141)) continue;
      const key = JSON.stringify([source,event.id]);
      items.set(key,{ ...event, text: event.text?.slice(0,160) ?? null, key, session:row.id, home:row.execution_home,
        title:row.title.slice(0,200), expires:options.now+BANNER_DURATION, remaining:BANNER_DURATION });
    }
    cursors[source] = cursor;
  }
  return { ...state, items: [...items.values()].slice(-BANNER_LIMIT),
    cursors:Object.fromEntries(Object.entries(cursors).sort((a,b)=>a[1].at-b[1].at).slice(-SOURCE_LIMIT)) };
}
export function expireNotifications(state: NotificationState, now: number): NotificationState {
  return { ...state, items:state.items.filter(item=>item.expires === null || item.expires > now) };
}
