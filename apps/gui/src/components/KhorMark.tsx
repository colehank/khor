// The app's own mark, at the head of the rail: the mandala — outer ring,
// inner square, four gate beads, heart bead — in liquid glass. It is the
// same artwork the Dock shows, which is the point of putting it here: the
// window and the icon the user launched are one thing.
//
// **It is not a copy.** The `<img>` points at the design source the icon
// pipeline itself renders (`src-tauri/icons/src/README.md`), so there is
// exactly one mandala in this repo. Inlining the markup would be the
// second one, and a second copy of an identity mark is how the app ends
// up with two faces — the same reasoning `Avatar.tsx` spells out for the
// two vein paths, applied to the one asset that does *not* have to be
// written twice.
//
// **It takes no clicks, and must not look as though it does.** What it
// will eventually open (the mandala map and token usage) is a later
// batch; until then it gets no hover state, no pointer cursor and no
// button semantics, because an affordance that answers nothing teaches
// people to stop trying the ones that do.
import markUrl from "../../src-tauri/icons/src/mandala-glass.svg";

import { cn } from "@/lib/utils";

export function KhorMark({ className }: { className?: string }) {
  return (
    <div data-rail-mark className={cn("flex-none", className)}>
      <img
        src={markUrl}
        // Decorative, deliberately. The rail's landings each say their
        // name out loud because each is somewhere to go; this one is
        // nowhere to go, and naming it would add a stop on the screen
        // reader's tour that leads nowhere. The app's name is already on
        // the window.
        alt=""
        aria-hidden="true"
        // An image that can be dragged out of the window is the last
        // remaining hint that it is a thing you operate.
        draggable={false}
        // No rounding here: the artwork carries its own corner radius
        // (22.4%, Apple's icon grid) inside its clip path, and a second
        // radius on top of that one shaves the mark's own silhouette.
        className="block size-icon-mark"
      />
    </div>
  );
}
