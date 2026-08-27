// Browsing a disk, as an instance of the browsing shell
// (`browse/kind.ts`, docs/DESIGN.md 浏览型外壳).
//
// **Everything here is the half that is genuinely about directories.**
// Tabs, the strip, the switcher, the store, the machine chip, the fresh
// tab and how a refusal is shown are all the shell's and appear nowhere
// in this file. What is left is exactly five things: the ledger word,
// what completes a half-typed path, what one path is called, what a
// path looks like at rest, and what a directory listing is.
//
// The browser landing's sibling of this file is what the second batch
// writes; if it needs the shell changed, that is the interface growing,
// not this one having guessed wrong.
import { useEffect, useRef } from "react";

import { fetchDirPins } from "@/api";
import type { BodyProps, BrowseKind, RestProps, SecondSegment } from "@/browse/kind";
import { gui } from "@/gen/catalog";
import { usePathCandidates } from "@/hooks/use-path-candidates";
import { cn } from "@/lib/utils";
import { FilesPane } from "@/views/FilesPane";

/** `/a/b/c` → the root and each step, each carrying the path pressing
    it goes to. */
function crumbs(at: string): { label: string; to: string }[] {
  const out = [{ label: "/", to: "/" }];
  let sofar = "";
  for (const part of at.split("/").filter(Boolean)) {
    sofar += `/${part}`;
    out.push({ label: part, to: sofar });
  }
  return out;
}

/**
 * A path at rest: every step, each one a place to go.
 *
 * **A breadcrumb is a directory's answer, not the shell's.** A path is
 * a chain of real containers a person can stand in, so every step is
 * somewhere to press; a URL is not that shape, and the browser landing
 * will draw its address some other way. That is the whole reason the
 * resting face is a parameter.
 */
function PathCrumbs({ at, onGo, onEdit }: RestProps) {
  // The strip's own scroller, held at its right end: the step you are
  // standing on is the one worth seeing, and a scroller left at 0 shows
  // the part every path on that machine has in common.
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = box.current;
    if (el) el.scrollLeft = el.scrollWidth;
  }, [at]);

  const path = crumbs(at);
  return (
    // The steps have **no gap between them** — that is what makes this
    // read as a path rather than as a row of words. The slashes are the
    // whole of the spacing, and one `gap-1` per step (the first
    // spelling) put four pixels on either side of every separator and
    // turned `/var/folders/ml` into a list. Measured on a screenshot.
    //
    // **The floor under a finger bought part of that back, and the
    // button below says what the price was.** A step narrower than
    // `--touch` is widened to it, so a one- or two-letter step now
    // carries slack — the difference from the `gap-1` spelling is that
    // it falls after a step rather than around every separator, and
    // only on the steps too short to press.
    //
    // **The strip cannot hand the floor out sideways.**
    // `overflow-x-auto` makes this a scroll container on both axes: a
    // hit area larger than the box it sits in would be clipped or made
    // scrollable, never allowed to hang over its neighbours — which is
    // the same wall the tab strip hit. So the width is really spent,
    // and spending it is the judgment, not an oversight.
    //
    // **And no step truncates. The strip scrolls instead.** Letting them
    // share the width was the first spelling and it fails exactly where
    // a path is longest: at 390 every middle step shrank to two
    // characters and an ellipsis, so the whole address read
    // `,v/f…/ı/w_5l2…/ʔkho…/b/tree-beta` — six truncations that name
    // nothing, and the one segment worth reading squeezed between them.
    // Measured on a screenshot at 390. Scrolled, the end of the path —
    // the part that says where you are — is what shows, and the
    // ancestors are still there to the left, still pressable.
    <div ref={box} className="flex min-w-0 flex-1 items-center overflow-x-auto">
      {path.map((c, i) => {
        const last = i === path.length - 1;
        return (
          <span
            key={c.to}
            // The last step **grows and never shrinks** (`grow shrink-0`,
            // not `flex-1`): it is the one that takes the blank tail when
            // there is one, and a zero basis would have let a leaf longer
            // than the floor be squeezed to the floor with its own name
            // hanging out of it.
            className={cn("flex items-center", last ? "grow shrink-0" : "flex-none")}
          >
            {i > 1 && <span className="flex-none text-muted-foreground">/</span>}
            <button
              type="button"
              data-crumb={c.to}
              // **The step you are standing on opens the box instead of
              // going anywhere.** Going to where you already are is the
              // one press on this strip that could do nothing at all,
              // and an address bar needs a reliable way into its own
              // editor — on a path deep enough to fill the row there is
              // no blank tail left to press.
              //
              // **And it is the whole tail, not a filler beside it.**
              // There used to be a second button after the steps, spread
              // over whatever width was left, so that pressing "somewhere
              // on the address" opened the editor the way it does in
              // every other address bar. It did the same thing as this
              // one, and it could not be given the floor: on a path that
              // fills the row it measured zero wide (measured at 390 —
              // an affordance that is not there at all is worse than a
              // small one), and reserving the floor for it would have
              // taken a whole step off the visible end of the strip on
              // the face with the least room. One target, both jobs, and
              // it carries the bar's name because pressing it is how the
              // bar is opened — the one press on this strip that is not
              // a step.
              data-address-rest={last || undefined}
              aria-label={last ? gui.address_dir : undefined}
              onClick={last ? onEdit : () => onGo(c.to)}
              // **The floor under a finger, in both directions**
              // (tokens.css `--touch`). The tall half is free — the bar
              // is already that tall around the machine chip — and the
              // wide half is the one with a price, paid here rather than
              // dodged: a step whose name is one or two letters is a
              // target of about that many pixels otherwise, and a
              // breadcrumb is the only thing a person can press on this
              // face at rest.
              //
              // **Where the slack goes, once a step is wider than its
              // name.** Held to the start, it always falls *after* a
              // step and never between a slash and the name it
              // introduces, so `/var` and `/folders` stay whole and the
              // gaps land at the joins — centred, it would fall on both
              // sides of every short step, which is the spelling this
              // file already threw out for reading as a row of words.
              //
              // The root is the exception because it is the one step
              // whose name *is* the separator: with nothing to stay
              // joined to, held to the start it sits against the chip
              // with its whole width between it and the first name.
              // Centred, that distance is halved and falls on both
              // sides, which is what a lone slash wants — unless the
              // root is also where you are standing, in which case it
              // is the step that takes the whole tail and centring
              // would strand one slash in the middle of the bar.
              className={cn(
                "inline-flex min-h-touch min-w-touch cursor-pointer items-center rounded-sm px-0.5 text-sm whitespace-nowrap hover:bg-muted",
                i === 0 && !last ? "justify-center" : "justify-start",
                last && "grow shrink-0",
              )}
            >
              {c.label}
            </button>
          </span>
        );
      })}
    </div>
  );
}

function Listing({ machine, place, narrow, onPlace, onLanded }: BodyProps) {
  return (
    <FilesPane
      machine={machine.name}
      device={machine.id}
      path={place}
      narrow={narrow}
      onPath={onPlace}
      onLanded={onLanded}
    />
  );
}

/**
 * A disk, for the browsing shell.
 *
 * **A module constant, not something built per render.** The shell calls
 * `useSecond` as a hook, so the descriptor has to be the same object for
 * the life of the landing (`BrowseKind.useSecond` says what breaks
 * otherwise).
 */
export const FILES: BrowseKind = {
  // The word `khor_sync::recents` refuses by name, and the word the tab
  // store is keyed by. One spelling, both jobs.
  kind: "dir",
  words: {
    address: gui.address_dir,
    segment: gui.files_tab,
    pinned: gui.pinned_dirs,
    // Shared with the browser landing: 「最近去过」 is about the going,
    // not about what was gone to (`recent-places` in the catalog).
    recent: gui.recent_places,
    empty: gui.no_pinned_dirs,
    go: gui.go_pin_a_dir,
  },
  pins: async () => {
    const rows = await fetchDirPins();
    return rows.map((r) => ({ device: r.device, name: r.name, place: r.path, at_ms: null }));
  },
  // `remembered` is unused here: a directory's next step is a fact only
  // that disk knows, so this asks the disk. The pages kind is the one
  // that completes from memory (`BrowseKind.useSecond`).
  useSecond: (machine, typed): SecondSegment => {
    const { dir, entries, failed } = usePathCandidates(machine?.name ?? null, typed);
    return {
      // Only directories complete: a file is where a path stops, so
      // offering one as a step to continue from would be offering a
      // step nobody can take.
      candidates: entries
        .filter((e) => e.dir)
        .map((e) => ({ key: `${dir}${e.name}/`, label: `${e.name}/` })),
      failed,
    };
  },
  leafOf: (place) => place.slice(place.lastIndexOf("/") + 1) || place,
  Rest: PathCrumbs,
  Body: Listing,
};
