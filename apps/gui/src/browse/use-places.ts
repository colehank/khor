// Places this person keeps going back to.
//
// ⚠️ 墓碑: this also held the ones pinned on purpose (「置顶的网页 /
// 置顶的目录」), until 2026-09-22 (docs/SHELL.md 第六轮:「置顶的浏览器
// 不就是和 session list 一样的置顶吗」). A pin is the sidebar's pinned
// group now; the old rows were moved there once (`Node::
// migrate_place_pins`) and this file does not read that table any more.
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
 *
 * **2026-08-27: that person's history was put in front of it, and it is
 * one row.** Asked of the live node on the development machine — the
 * only machine anybody has actually browsed from — through the same op
 * this file calls:
 *
 *     recents(dir)  → 1        dir_pins → 0
 *     recents(page) → 0        web_pins → 0
 *
 * So the number still cannot be moved on evidence, and now it is known
 * *why* rather than assumed: **the cap has never once been reached**,
 * by anyone, on the machine that would reach it first. Twelve is not
 * defended by this — it is simply not yet under any pressure, and a
 * cap nothing has pressed against is a cap nobody can size.
 *
 * **What would change the answer**, so the next person does not have to
 * re-derive it: a single dig three levels deep spends three of these
 * slots within milliseconds (the paragraph above), so the first real
 * signal is not "twelve rows are hard to scan" — it is somebody finding
 * that the place they went yesterday is gone while three layers of one
 * detour are still on the screen. Watch for that sentence, not for a
 * number.
 */
const MOST_RECENT = 12;

export type Places = {
  /** `null` while nobody has answered, and after a refusal — see the
      file head for why that is not `[]`. */
  recent: Place[] | null;
  /**
   * Somebody has answered, one way or the other. Until then 「no
   * places」 is a fact about a request still in flight, not about this
   * person, and the fresh tab draws nothing rather than claim it.
   */
  asked: boolean;
  reload: () => void;
};

export function usePlaces(kind: BrowseKind): Places {
  const [recent, setRecent] = useState<Place[] | null>(null);
  const [asked, setAsked] = useState(false);
  const [turn, setTurn] = useState(0);
  const reload = useCallback(() => setTurn((n) => n + 1), []);

  useEffect(() => {
    let stopped = false;
    fetchRecents(kind.kind, MOST_RECENT)
      .then((rows) => {
        if (stopped) return;
        setRecent(
          rows.map((r) => ({ device: r.device, name: r.name, place: r.place, at_ms: r.at_ms })),
        );
        setAsked(true);
      })
      .catch(() => {
        // Refused: either the node is older than this screen, or it is
        // unreachable. Both mean "no answer", and neither is "no
        // places" — so nothing is painted rather than an empty section
        // that claims this person has been nowhere.
        if (!stopped) setAsked(true);
      });
    return () => {
      stopped = true;
    };
  }, [kind, turn]);

  return { recent, asked, reload };
}

/** The recents, most lately first. The node's order is by key; the
    one this screen needs is by when. */
export function placesInOrder(p: Places): Place[] {
  return [...(p.recent ?? [])].sort((a, b) => (b.at_ms ?? 0) - (a.at_ms ?? 0));
}
