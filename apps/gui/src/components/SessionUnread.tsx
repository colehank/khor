import { cli } from "@/gen/catalog";

/** The node's unread count, covered by local read receipts; independent of work state. */
export function SessionUnread({ count }: { count: number }) {
  if (count <= 0) return null;
  return (
    <span
      data-unread
      role="img"
      aria-label={cli.unread(count)}
      data-unread-dot
      className="size-1.5 flex-none rounded-full bg-badge"
    />
  );
}

export function SessionUnreadCount({ count }: { count: number }) {
  if (count <= 1) return null;
  return <span data-unread-count aria-hidden className="flex-none text-caption font-body tabular-nums text-muted-foreground">{count}</span>;
}
