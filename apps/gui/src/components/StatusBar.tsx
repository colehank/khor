// The window's foot: what is happening that nobody is watching.
//
// What may appear here and for how long is `use-status-bar`'s — that
// file holds the two admission rules and the reasons. This one paints.
//
// **Absent, not empty.** With nothing to say the strip does not render
// at all: no box, no zero, nothing to look at. That is what "can go to
// zero" means for a surface, and it is why this is safe to leave on
// screen forever — the only thing it can ever be is a thing that is
// actually happening.
//
// The state word is the row's own, in the state's own clothes: the same
// `data-word` / `data-word-text` pair the session row and the detail
// header wear, so the colour, the breath and the reduced-motion guard
// are the doctrine's rather than a third copy of it (app.css).
import { StatusWord } from "@/components/StatusWord";
import { gui } from "@/gen/catalog";
import { STATUS_STICKS, type StatusItem } from "@/hooks/use-status-bar";
import { cn } from "@/lib/utils";

export function StatusBar({
  items,
  onDismiss,
  at = "foot",
}: {
  items: StatusItem[];
  onDismiss: (id: string) => void;
  /** **Where the strip hangs** (UX-31, the owner 2026-10-06): the
      window's foot on the wide face and the narrow face's lists; under
      the deck's pill inside a session on the phone, in the pill's own
      glass (`SessionScreen`'s `status`), like Safari's progress bar. */
  at?: "foot" | "pill";
}) {
  if (items.length === 0) return null;
  return (
    <div
      data-statusbar
      data-at={at}
      // **The window's foot, in the layout** (docs/design-system
      // components/StatusBar, 2026-09-26): one `ctl-sm` line on the
      // `canvas`, `caption` in `tertiary` — not a box floating over the
      // corner with a border and a shadow, which is the toast the system
      // rules out. It takes its line only while there is something on it.
      // Under the pill it is the same line on the pill's glass instead.
      className={cn(
        "flex h-ctl-sm min-w-0 flex-none items-center gap-base overflow-hidden px-row text-caption text-tertiary",
        at === "foot" ? "bg-background" : "flex-1",
      )}
    >
      {items.map((i) => {
        const sticks = STATUS_STICKS.includes(i.word);
        const line = (
          <>
            <StatusWord word={i.word} />
            <span className="min-w-0 truncate">{i.title}</span>
          </>
        );
        // No button on the bar. A line that leaves on its own needs no
        // way off; one that stays (a failure) is taken off by pressing
        // the line itself — the whole line is the target, named for it.
        return sticks ? (
          <button
            key={i.id}
            type="button"
            data-status-item={i.id}
            data-status-dismiss={i.id}
            data-word={i.word}
            data-settled={i.settled}
            aria-label={`${gui.remove} ${i.title}`}
            onClick={() => onDismiss(i.id)}
            className="flex min-w-0 items-center gap-xxs rounded-xs hover:text-muted-foreground"
          >
            {line}
          </button>
        ) : (
          <span
            key={i.id}
            data-status-item={i.id}
            data-word={i.word}
            data-settled={i.settled}
            className="flex min-w-0 items-center gap-xxs"
          >
            {line}
          </span>
        );
      })}
    </div>
  );
}
