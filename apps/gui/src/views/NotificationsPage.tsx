import { useState } from "react";

import type { SessionRow } from "@/api";
import { GlassCircle } from "@/components/Glass";
import { IconCheck } from "@/components/icons";
import { SideList, type SideRow } from "@/components/SideList";
import { sessionKind } from "@/components/SessionFace";
import { gui } from "@/gen/catalog";
import { notificationsEnabled, pendingNotification, type Inbox, type InboxItem } from "@/lib/notification-inbox";
import { readNotification, removeNotification, setSessionNotifications } from "@/lib/notifications";
import { MoreFrame } from "@/views/more/MorePage";
import { ago } from "@/words";

export function notificationText(item: InboxItem): string {
  switch (item.kind) {
    case "needs_input": return item.resolved ? gui.notification_handled : gui.notification_needs_input;
    case "turn_completed": return gui.notification_completed;
    case "failed": return gui.notification_failed;
    case "command_finished": return item.exit !== null && item.exit !== 0 ? gui.notification_failed : gui.notification_command;
    case "notice": return item.text ?? gui.notification_notice;
  }
}

export function NotificationsPage({ inbox, sessions, onOpen, onClose }: {
  inbox: Inbox;
  sessions: SessionRow[];
  onOpen: (item: InboxItem) => boolean;
  onClose: () => void;
}) {
  const [missing, setMissing] = useState<string | null>(null);
  const rows: SideRow[] = [...inbox.items].sort((a,b) =>
    Number(pendingNotification(b)) - Number(pendingNotification(a)) || b.at_ms - a.at_ms,
  ).map((item) => {
    const session = sessions.find((row) => row.id === item.session && row.execution_home === item.home);
    const pending = pendingNotification(item);
    const enabled = session && notificationsEnabled(inbox, session);
    const text = notificationText(item);
    const detail = item.kind === "command_finished" && item.text ? `${text} · ${item.text}` : text;
    return {
      id: item.key, group: pending ? "pending" : "recent", pinned: false,
      face: { machine: session?.face ?? null },
      mark: session ? sessionKind(session.kind, session.category) : undefined,
      title: session?.title ?? item.title,
      sub: missing === item.key ? gui.notification_missing : detail,
      corners: { top: item.read ? undefined : gui.notification_unread, bottom: ago(item.at_ms) },
      find: `${item.title} ${detail}`,
      onOpen: () => {
        readNotification(item.key);
        setMissing(onOpen(item) ? null : item.key);
      },
      menu: [
        ...(session ? [{ key: "notifications", label: enabled ? gui.notification_mute : gui.notification_follow,
          run: () => setSessionNotifications(session, !enabled) }] : []),
        ...(!pending ? [{ key: "remove", label: gui.notification_remove, destructive: true,
          run: () => removeNotification(item.key) }] : []),
      ],
      rowData: { "data-notification": item.key, "data-pending": pending, "data-read": item.read },
    };
  });
  return <MoreFrame page="notifications" onClose={onClose} actions={
    inbox.items.some((item) => !item.read) ? <GlassCircle aria-label={gui.notification_read_all} onClick={() => readNotification()}><IconCheck /></GlassCircle> : undefined
  }>
    <SideList rows={rows} label={(group) => group === "pending" ? gui.waiting_on_you : gui.group_recent}
      empty={gui.notification_empty} emptyHow={gui.notification_empty_how} mark={{ "data-notification-list": "" }} />
  </MoreFrame>;
}
