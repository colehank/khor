// The one pin control. Every list pane draws this exact button — the
// same reason `PaneBar` is one file: mandala grew two search boxes that
// looked alike and behaved differently, and the second copy is where
// that starts.
//
// **It is on every row, always, not on hover.** Hover is a pointer-only
// affordance, and this app's narrow face has no pointer at all — a pin
// that appears when you point at a row is a pin the phone cannot reach,
// which is precisely where a list is long and one screen wide. It stays
// quiet instead of hidden: muted until it is either pointed at or
// actually pinned, at which point it takes the brand color and the
// upright shape.
//
// On a machine row it is also the *only* thing that can be operated —
// those rows have nowhere to go until machine cards land, and a row that
// hints at a destination it does not have is a promise the app breaks
// (docs/handoff 细栏定形).
import { IconPin } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";

export function PinButton({
  pinned,
  onToggle,
}: {
  pinned: boolean;
  onToggle: () => void;
}) {
  return (
    <Button
      variant="ghost"
      size="icon"
      // The name is the effect, and it changes with the state: a button
      // called the same thing in both states tells a screen reader
      // nothing about what just happened.
      aria-label={pinned ? gui.unpin : gui.pin}
      aria-pressed={pinned}
      data-pin
      data-on={pinned}
      onClick={(e) => {
        // The row around this button is clickable on the sessions pane;
        // without this, pinning also opens what you pinned.
        e.stopPropagation();
        onToggle();
      }}
      className="size-ctl-sm flex-none text-muted-foreground data-[on=true]:text-primary"
    >
      <IconPin pinned={pinned} />
    </Button>
  );
}
