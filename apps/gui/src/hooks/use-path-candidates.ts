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
import { useEffect, useState } from "react";

import { fetchLs } from "@/api";
import type { DirRow } from "@/gen/bindings/DirRow";

/** How long a keystroke has to be the last one before the ask goes out.
    Only ever paid when the *directory* changed — typing inside one asks
    nothing at all. */
const SETTLE_MS = 150;

/** The directory part of a typed path, and the fragment after it. */
export function splitPath(typed: string): { dir: string; leaf: string } {
  const cut = typed.lastIndexOf("/");
  return cut < 0
    ? { dir: "", leaf: typed }
    : { dir: typed.slice(0, cut + 1), leaf: typed.slice(cut + 1) };
}

export function usePathCandidates(
  machine: string | null,
  typed: string,
): { dir: string; entries: DirRow[]; answered: boolean } {
  const { dir } = splitPath(typed);
  const [got, setGot] = useState<{ machine: string; dir: string; entries: DirRow[] } | null>(null);

  useEffect(() => {
    if (!machine) return;
    let stopped = false;
    const t = window.setTimeout(() => {
      fetchLs(machine, dir)
        .then((listing) => {
          if (!stopped) setGot({ machine, dir, entries: listing.entries });
        })
        .catch(() => {
          // A directory that cannot be read offers no completions, and
          // says nothing about why — the pane that opens it is where a
          // refusal belongs, not a dropdown under the caret.
        });
    }, SETTLE_MS);
    return () => {
      stopped = true;
      window.clearTimeout(t);
    };
  }, [machine, dir]);

  const answered = got !== null && got.machine === machine && got.dir === dir;
  return { dir, entries: answered ? got.entries : [], answered };
}
