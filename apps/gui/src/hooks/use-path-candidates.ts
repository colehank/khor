// Completions for a path being typed, read off the machine that owns it.
//
// **Keyed by the directory, not by the keystroke.** Typing `pro` then
// `proj` then `proje` is three keystrokes in *one* directory, and the
// answer is the same for all three — so the ask happens once and the
// narrowing happens locally. That is not an optimisation: a request per
// keystroke is what makes candidates flicker, because each answer
// replaces the list a moment after the one before it.
//
// **An answer is kept only if it is still the directory being typed in.**
// `ls` is a remote call; answers arrive late and out of order, and
// "latest wins" is not enough — the latest answer can be for a directory
// the caret has already left, and then the box offers names from
// somewhere else entirely.
//
// **"Nothing matches" and "nobody has answered yet" are two facts.**
// While a directory is still being asked about, this offers nothing and
// claims nothing: no stale entries from the previous directory, and no
// statement that the directory is empty. The two look identical in a
// single frame, and only one of them is ever true.
//
// **And a third fact: "that machine did not answer at all."** It used
// to be swallowed here, which was right while the only caller was a
// dropdown under a caret. It is not right now that the caller is an
// address bar: a machine that is not answering has to degrade to what
// is remembered about it rather than to an empty list, and an empty
// list is exactly what "no completions" looks like. So the refusal is
// reported, and what to put on screen instead is the caller's judgment
// (`AddressBar` — pins and recents, each saying where it came from).
import { useEffect, useState } from "react";

import { fetchLs } from "@/api";
import type { DirRow } from "@/gen/bindings/DirRow";

/** How long a keystroke has to be the last one before the ask goes out.
    Only ever paid when the *directory* changed — typing inside one asks
    nothing at all. */
const SETTLE_MS = 150;

/**
 * Layers already pulled, across every box on this page and across the
 * life of the tab.
 *
 * **Module-level and never evicted, on purpose.** A tab strip walks the
 * same few trees over and over — down into a directory, back up, into
 * its sibling — and the way up is the case the per-hook state could
 * never serve: the parent was listed a second ago and asking again
 * makes climbing feel slower than descending. The cost of keeping it is
 * a few hundred names per directory visited in one sitting.
 *
 * **What it cannot do is stand in for a live answer.** A cached layer
 * is served immediately *and* re-asked; the answer replaces it. So a
 * directory that changed under the caret is briefly a layer old — which
 * is the correct trade for a completion list, and would not be for the
 * pane that actually opens the directory (`FilesPane` never reads
 * this).
 */
const cache = new Map<string, DirRow[]>();
const layer = (machine: string, dir: string) => `${machine}\n${dir}`;

export type PathCandidates = {
  dir: string;
  entries: DirRow[];
  answered: boolean;
  /** The last ask about this directory came back refused — the machine
      is not reachable, or the path is not one it will list. */
  failed: boolean;
};

/** The directory part of a typed path, and the fragment after it. */
export function splitPath(typed: string): { dir: string; leaf: string } {
  const cut = typed.lastIndexOf("/");
  return cut < 0
    ? { dir: "", leaf: typed }
    : { dir: typed.slice(0, cut + 1), leaf: typed.slice(cut + 1) };
}

export function usePathCandidates(machine: string | null, typed: string): PathCandidates {
  const { dir } = splitPath(typed);
  const [got, setGot] = useState<{ machine: string; dir: string; entries: DirRow[] } | null>(null);
  const [refused, setRefused] = useState<{ machine: string; dir: string } | null>(null);

  useEffect(() => {
    if (!machine) return;
    let stopped = false;
    // A layer already walked answers now. The ask below still goes out:
    // this is a head start, not a substitute for the truth.
    const held = cache.get(layer(machine, dir));
    if (held) setGot({ machine, dir, entries: held });
    const t = window.setTimeout(() => {
      fetchLs(machine, dir)
        .then((listing) => {
          cache.set(layer(machine, dir), listing.entries);
          if (stopped) return;
          setGot({ machine, dir, entries: listing.entries });
          setRefused(null);
        })
        .catch(() => {
          // Not swallowed any more, and still not spoken here: this
          // says *that* it failed, and the caller decides what a box
          // with nothing live to offer should show instead. A reason
          // under the caret would be a dropdown explaining itself.
          if (!stopped) setRefused({ machine, dir });
        });
    }, SETTLE_MS);
    return () => {
      stopped = true;
      window.clearTimeout(t);
    };
  }, [machine, dir]);

  const answered = got !== null && got.machine === machine && got.dir === dir;
  const failed = refused !== null && refused.machine === machine && refused.dir === dir;
  return { dir, entries: answered ? got.entries : [], answered, failed };
}
