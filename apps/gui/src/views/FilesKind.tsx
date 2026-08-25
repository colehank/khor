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
          <span key={c.to} className="flex flex-none items-center">
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
              onClick={last ? onEdit : () => onGo(c.to)}
              className="cursor-pointer rounded-sm px-0.5 text-sm whitespace-nowrap hover:bg-muted"
            >
              {c.label}
            </button>
          </span>
        );
      })}
      {/* The rest of the strip is one more way in, so that pressing
          "somewhere on the address" does what it does in every other
          address bar. */}
      <button
        type="button"
        data-address-rest
        aria-label={gui.address_dir}
        onClick={onEdit}
        className="min-w-0 flex-1 cursor-text self-stretch"
      />
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
