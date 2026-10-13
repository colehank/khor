import { cli } from "@/gen/catalog";

/** The operator's unread content, independent of work state and pill
 * interactions. The node supplies the count: agents currently report
 * 0/1, broadcast counts messages. Never infer unread from a state word.
 * The dot always has the same red area; a neutral number adds quantity
 * without making a busier channel look more urgent.
 */
export function SessionUnread({ count }: { count: number }) {
  if (count <= 0) return null;
  return (
    <span
      data-unread
      role="img"
      aria-label={cli.unread(count)}
      className="inline-flex flex-none items-center gap-xxs text-caption font-body tabular-nums text-muted-foreground"
    >
      <span data-unread-dot aria-hidden className="size-1.5 flex-none rounded-full bg-badge" />
      {count > 1 && <span data-unread-count aria-hidden>{count}</span>}
    </span>
  );
}
