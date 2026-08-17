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
// **It takes clicks now, and everything that used to say otherwise is
// gone.** It was written inert on a stated judgment — no hover, no
// pointer, no button semantics, and not drawn at all on the narrow face,
// because an affordance that answers nothing teaches people to stop
// trying the ones that do. What it was waiting for has landed (the
// mandala map and what the agents cost), so every one of those went with
// it, including the narrow rail's absence: that rail is a row of places
// to go, and the only reason to keep this out of it was that it went
// nowhere.
import markUrl from "../../src-tauri/icons/src/mandala-glass.svg";

/**
 * Just the artwork. **The button around it is `RailItem`**, the same one
 * every other glyph in the rail sits in, so the mark hovers, focuses and
 * lights exactly as its neighbours do — one implementation of "a place in
 * the rail", not a second one that has to be kept in step.
 */
export function KhorMark() {
  return (
    <img
      data-rail-mark
      src={markUrl}
      // Decorative: the button around it carries the name, so reading the
      // artwork out too would announce the same thing twice.
      alt=""
      aria-hidden="true"
      // An image that can be dragged out of the window is still not
      // something anybody means to do with a control.
      draggable={false}
      // No rounding here: the artwork carries its own corner radius
      // (22.4%, Apple's icon grid) inside its clip path, and a second
      // radius on top of that one shaves the mark's own silhouette.
      className="block size-icon-mark"
    />
  );
}
