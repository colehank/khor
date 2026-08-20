// The corner strip: what is happening that nobody is watching.
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
import { IconClose } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { word } from "@/words";
import { STATUS_STICKS, type StatusItem } from "@/hooks/use-status-bar";

export function StatusBar({
  items,
  onDismiss,
}: {
  items: StatusItem[];
  onDismiss: (id: string) => void;
}) {
  if (items.length === 0) return null;
  return (
    <div
      data-statusbar
      // Over the corner, not in the layout: it must not move the thing
      // somebody is reading to announce something they did not ask
      // about. Pointer events only on the lines themselves.
      className="pointer-events-none fixed right-3 bottom-3 z-30 flex flex-col items-end gap-1"
    >
      {items.map((i) => (
        <div
          key={i.id}
          data-status-item={i.id}
          data-word={i.word}
          data-settled={i.settled}
          className="pointer-events-auto flex max-w-80 items-center gap-2 rounded-md border bg-popover px-2 py-1 text-sm shadow-md"
        >
          <span data-word-text style={{ color: `var(--state-${i.word})` }} className="flex-none">
            {word(i.word)}
          </span>
          <span className="min-w-0 flex-1 truncate text-muted-foreground">{i.title}</span>
          {/* Only what stays gets a way off. A line that leaves on its
              own does not need a control, and giving it one would invite
              a race between the hand and the timer. */}
          {STATUS_STICKS.includes(i.word) && (
            <Button
              size="icon"
              variant="ghost"
              data-status-dismiss={i.id}
              aria-label={`${gui.remove} ${i.title}`}
              className="size-ctl-sm flex-none"
              onClick={() => onDismiss(i.id)}
            >
              <IconClose />
            </Button>
          )}
        </div>
      ))}
    </div>
  );
}
