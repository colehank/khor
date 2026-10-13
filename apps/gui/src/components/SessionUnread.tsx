import { cli } from "@/gen/catalog";
import { cn } from "@/lib/utils";

/** The operator's unread content, independent of work state and pill
 * interactions. The node supplies the count: agents currently report
 * 0/1, broadcast counts messages. Never infer unread from a state word.
 */
export function SessionUnread({ count }: { count: number }) {
  if (count <= 0) return null;
  return (
    <span
      data-unread
      role="img"
      aria-label={cli.unread(count)}
      className={cn(
        "inline-block flex-none rounded-full bg-badge text-center text-caption font-body tabular-nums text-badge-foreground",
        count === 1 ? "size-1.5" : "min-w-4 px-1 leading-4",
      )}
    >
      {count > 1 ? count : null}
    </span>
  );
}
