// Places this person keeps: the ones pinned on purpose, and the ones
// they keep going back to.
//
// **One fetch, two customers.** The fresh tab paints them and the
// address bar falls back to them when a machine stops answering — and
// those are two readings of one fact, not two lists. Fetched twice they
// would be two answers a poll apart, and the box could offer a path the
// page below it is not showing.
//
// **"The node has no such verb" is not "there is nothing".** The
// recents ledger exists now (`khor_sync::recents`), so this is the
// older-node path rather than the ordinary one — and it still has to be
// kept apart: nothing means an empty section, refused means no section
// at all. Collapsed, a screen talking to a node that predates the
// ledger would say 最近去过 over a blank strip and read as broken
// (docs/UX.md 不知道就说不知道).
import { useCallback, useEffect, useState } from "react";

import { fetchRecents } from "@/api";
import type { BrowseKind, Place } from "@/browse/kind";

/**
 * How many recent places a fresh tab offers. A screenful, not a
 * history: the list is a shortcut, and a shortcut nobody can scan is a
 * list. The node caps too — this is the ask, not the trust.
 *
 * **What makes this number harder to pick than it looks**, in
 * `tabs-node`'s own words: a visit is recorded **per layer**, and that
 * is deliberate — every layer really is a place somebody went, and
 * which one was "the destination" is only knowable afterwards (a person
 * who stops at `/a/b`, reads, then goes deeper was at both). No
 * debounce can tell, so none pretends to.
 *
 * The consequence lands here: **one dig down three levels takes three
 * of these slots**, all within milliseconds of each other, and at a
 * small cap that single act can push everywhere else this person
 * actually goes off the screen. The knob is this one; the ledger is
 * not going to change.
 *
 * Left at 12 on purpose for now: the only customer is the fresh tab's
 * one screen, twelve rows still scan at both widths, and there is no
 * real usage to pick a better number from — raising it today would
 * trade one guess for another. Somebody with a person's actual history
 * in front of them should move it.
 */
const MOST_RECENT = 12;

export type Places = {
  pinned: Place[];
  /** `null` while nobody has answered, and after a refusal — see the
      file head for why that is not `[]`. */
  recent: Place[] | null;
  /**
   * Somebody has answered about the pins, one way or the other.
   *
   * **What it is for is the sentence that says there is nothing.** Until
   * this is true, "no places" is not a fact about this person, it is a
   * fact about a request still in flight — and painting 还没有钉住的目录
   * over it is the same 不知道-drawn-as-没有 the six words are forbidden
   * from doing (docs/UX.md). Measured on a real run: switching landings
   * and coming back flashed the empty state over a mesh with a pin in
   * it, because the fetch starts again on the way in.
   */
  asked: boolean;
  reload: () => void;
};

export function usePlaces(kind: BrowseKind): Places {
  const [pinned, setPinned] = useState<Place[]>([]);
  const [recent, setRecent] = useState<Place[] | null>(null);
  const [asked, setAsked] = useState(false);
  const [turn, setTurn] = useState(0);
  const reload = useCallback(() => setTurn((n) => n + 1), []);

  useEffect(() => {
    let stopped = false;
    kind
      .pins()
      .then((rows) => {
        if (stopped) return;
        setPinned(rows);
        setAsked(true);
      })
      .catch(() => {
        // Answered, even though the answer was a refusal: what this
        // gates is "is it still coming", and it is not.
        if (!stopped) setAsked(true);
        // A pin list that cannot be read is not an empty pin list
        // either — but this one has no second face to fall back to, so
        // the last good answer stands rather than being erased. A blink
        // in the backend must not empty a shortcut list.
      });
    fetchRecents(kind.kind, MOST_RECENT)
      .then((rows) => {
        if (stopped) return;
        setRecent(
          rows.map((r) => ({ device: r.device, name: r.name, place: r.place, at_ms: r.at_ms })),
        );
      })
      .catch(() => {
        // Refused: either the node is older than this screen, or it is
        // unreachable. Both mean "no answer", and neither is "no
        // places" — so nothing is painted rather than an empty section
        // that claims this person has been nowhere.
      });
    return () => {
      stopped = true;
    };
  }, [kind, turn]);

  return { pinned, recent, asked, reload };
}

/**
 * Pins first, then the recents that are not already pinned, most
 * lately first.
 *
 * **Pins are on top by rule, not by recency**, and that is the whole
 * ordering argument: a pin says "I decided this matters", a visit says
 * "I was here", and letting the second sort the first would move a
 * shortcut around under the hand that put it there. Within the pins the
 * node's own order stands (this layer never re-sorts — docs/UX.md).
 *
 * The de-duplication is by (machine, path) and it matters: a directory
 * you pinned is a directory you visit, so every pin would otherwise
 * appear twice on the one screen whose job is to be scannable.
 */
export function placesInOrder(p: Places): { place: Place; pinned: boolean }[] {
  const seen = new Set(p.pinned.map((x) => `${x.device}\n${x.place}`));
  const rest = (p.recent ?? [])
    .filter((x) => !seen.has(`${x.device}\n${x.place}`))
    .sort((a, b) => (b.at_ms ?? 0) - (a.at_ms ?? 0));
  return [
    ...p.pinned.map((place) => ({ place, pinned: true })),
    ...rest.map((place) => ({ place, pinned: false })),
  ];
}
