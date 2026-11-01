import type { SessionRow } from "@/api";
import type { ActivityEvent } from "@/gen/bindings/ActivityEvent";

export const INBOX_LIMIT = 200;
const SOURCE_LIMIT = 1024;
export type InboxItem = ActivityEvent & {
  key: string;
  session: string;
  home: string;
  title: string;
  read: boolean;
};
type Cursor = { at: number; ids: string[] };
export type Inbox = {
  version: 1;
  items: InboxItem[];
  cursors: Record<string, Cursor>;
  preferences: Record<string, boolean>;
};
export const emptyInbox = (): Inbox => ({ version: 1, items: [], cursors: {}, preferences: {} });
export const sourceKey = (row: Pick<SessionRow, "id" | "execution_home">): string => JSON.stringify([row.execution_home, row.id]);
export const pendingNotification = (item: ActivityEvent): boolean => item.kind === "needs_input" && !item.resolved;
export const notificationsEnabled = (inbox: Inbox, row: SessionRow): boolean =>
  inbox.preferences[sourceKey(row)] ?? (!row.imported && row.observation_owner === null);

function validEvent(event: ActivityEvent): boolean {
  return !!event && typeof event.id === "string" && event.id.length <= 256 && Number.isSafeInteger(event.at_ms) && event.at_ms > 0 &&
    ["needs_input", "turn_completed", "failed", "command_finished", "notice"].includes(event.kind) &&
    typeof event.resolved === "boolean" && (event.text === null || typeof event.text === "string");
}

export function restoreInbox(raw: string | null): Inbox {
  if (raw && raw.length > 4 * 1024 * 1024) return emptyInbox();
  try {
    const stored = JSON.parse(raw ?? "null") as Inbox | null;
    if (!stored || stored.version !== 1 || !Array.isArray(stored.items)) return emptyInbox();
    const items = stored.items.filter((item) => validEvent(item) && typeof item.key === "string" &&
      typeof item.session === "string" && typeof item.home === "string" && typeof item.title === "string" && typeof item.read === "boolean");
    const cursors = Object.fromEntries(Object.entries(stored.cursors ?? {}).filter(([, cursor]) =>
      cursor && Number.isSafeInteger(cursor.at) && Array.isArray(cursor.ids) && cursor.ids.every((id) => typeof id === "string"))
      .slice(-SOURCE_LIMIT).map(([key, cursor]) => [key, { at: cursor.at, ids: cursor.ids.slice(-24) }]));
    const preferences = Object.fromEntries(Object.entries(stored.preferences ?? {}).filter(([, enabled]) => typeof enabled === "boolean").slice(-SOURCE_LIMIT));
    return { version: 1, items: items.slice(-INBOX_LIMIT), cursors, preferences };
  } catch {
    return emptyInbox();
  }
}

export function collectNotifications(
  inbox: Inbox,
  rows: SessionRow[],
  options: { commandSeconds: number; onScreen: (id: string) => boolean },
): Inbox {
  const items = new Map(inbox.items.map((item) => [item.key, item]));
  const cursors = { ...inbox.cursors };
  for (const row of rows) {
    if (!row.execution_home) continue;
    const source = sourceKey(row);
    const previous = cursors[source];
    let cursor = previous ?? { at: 0, ids: [] };
    const enabled = notificationsEnabled(inbox, row);
    const events = (row.activity ?? []).filter(validEvent);
    const latestDisposition = Math.max(0, ...events.filter((event) =>
      event.kind === "turn_completed" || event.kind === "failed" || event.kind === "needs_input",
    ).map((event) => event.at_ms));
    for (const [key, item] of items) {
      if (item.session === row.id && item.home === row.execution_home && pendingNotification(item) && latestDisposition > item.at_ms) {
        items.set(key, { ...item, resolved: true });
      }
    }
    for (const event of events) {
      const key = JSON.stringify([source, event.id]);
      const old = items.get(key);
      if (old) {
        const read = old.read || options.onScreen(row.id);
        if ((event.resolved && !old.resolved) || read !== old.read) items.set(key, { ...old, read, resolved: old.resolved || event.resolved });
        continue;
      }
      const newEvent = event.at_ms > (previous?.at ?? 0) || (event.at_ms === previous?.at && !previous.ids.includes(event.id));
      if (event.at_ms > cursor.at) cursor = { at: event.at_ms, ids: [event.id] };
      else if (event.at_ms === cursor.at && !cursor.ids.includes(event.id)) cursor = { ...cursor, ids: [...cursor.ids, event.id].slice(-24) };
      const record = enabled || (!row.imported && row.observation_owner === null);
      if (!newEvent || !record) continue;
      if (event.kind === "command_finished" && (options.commandSeconds === 0 ||
        (event.duration_ms ?? 0) < options.commandSeconds * 1000 || event.exit === 130 || event.exit === 141)) continue;
      const visible = options.onScreen(row.id);
      const item: InboxItem = { ...event, text: event.text?.slice(0,160) ?? null, key,
        session: row.id, home: row.execution_home, title: row.title.slice(0,200),
        read: !enabled || visible || (!previous && !pendingNotification(event)),
      };
      items.set(key, item);

    }
    cursors[source] = cursor;
  }
  return {
    ...inbox, items: [...items.values()].sort((a,b) => Number(pendingNotification(a)) - Number(pendingNotification(b)) || a.at_ms - b.at_ms).slice(-INBOX_LIMIT),
      cursors: Object.fromEntries(Object.entries(cursors).sort((a,b) => a[1].at - b[1].at).slice(-SOURCE_LIMIT)),
  };
}
