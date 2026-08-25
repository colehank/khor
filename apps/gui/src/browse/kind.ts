// The line between the browsing shell and one landing that uses it.
//
// **This file is the whole contract**, and everything in the directory
// around it is the shell's side of it. A browsing landing (docs/UX.md
// 浏览型) is tabs across the top, one address bar, and a fresh tab that
// offers where this person already goes — and none of that differs
// between browsing a disk and browsing the web. Written twice, the two
// would drift apart on the first change to either, which is the failure
// this exists to make impossible rather than to be careful about.
//
// **What the shell deliberately does not take is the second segment.**
// A path and a URL are not the same shape and must not be forced into
// one: docs/KHOR.md's judgment about the session kinds applies here word
// for word — 统一的是呈现与协议，不是实现，硬统一会毁掉第一个. So the
// second half of an address arrives as a provider (`useSecond`) and the
// shell never looks inside what it hands back. What the shell *does*
// take is only what has been confirmed identical on both sides: the tab
// model, the strip and the switcher, the store, the **machine** segment,
// the fresh tab's skeleton, and how "could not get there" is said.
//
// **The parameters grew from what the files landing actually needed**,
// not from what a browser landing might. Nothing here is speculative:
// every member below has a live caller in `views/FilesKind.tsx` today.
// When the browser batch arrives and needs a member this does not have,
// adding one is ordinary interface growth — the alternative, guessing
// now, is a parameter nobody can delete later because nobody can prove
// it is unused.
import type { ComponentType } from "react";

import type { DeviceRow } from "@/api";
import type { Candidate } from "@/components/Omnibox";

/**
 * One place a person keeps, whichever list it came from.
 *
 * Part of the contract rather than of the store, because it is what a
 * kind's `pins` hands back: the shell mixes pins and recents into one
 * ordering and neither list may arrive in a shape only one of them
 * uses.
 */
export type Place = {
  /** Device id — what both ledgers key by, because a machine can be
      renamed (`khor_sync::recents`, `dirpins`). */
  device: string;
  /** That machine's current name, looked up on the node side. */
  name: string;
  /** The place itself, in this kind's own spelling. */
  place: string;
  /** When it was last visited, or `null` on a pin — a pin is a
      decision, and a decision has no recency to sort by. */
  at_ms: number | null;
};

/** What the second segment has to offer for what has been typed. */
export type SecondSegment = {
  /** Ready to be completed, in the omnibox's own shape. A `key` is the
      text that committing writes into the box, so it must be a whole
      address on its own. */
  candidates: Candidate[];
  /**
   * The machine did not answer.
   *
   * **Not the same as no candidates, and the shell is the one that acts
   * on the difference**: it swaps in the places it remembers on that
   * machine, dimmed and each saying where it came from. A provider that
   * folded a refusal into an empty list would leave the shell painting
   * "that machine is silent" and "that place has nothing under it" the
   * same way.
   */
  failed: boolean;
};

/** What the resting address bar draws, and the two ways out of it. */
export type RestProps = {
  /** Where the tab actually is, in this kind's own spelling. */
  at: string;
  /** Go somewhere else on the same machine. */
  onGo: (place: string) => void;
  /** Open the box. The shell has no way of its own into it from here —
      the resting face belongs to the kind, so the kind decides which
      part of it is the way in. */
  onEdit: () => void;
};

/** What one tab shows. */
export type BodyProps = {
  machine: DeviceRow;
  /** Where to open; `""` means this kind's own starting point (for a
      disk, that machine's home). */
  place: string;
  narrow: boolean;
  /** Where the tab should now be. Called both for a step the person
      took and for what a `""` resolved to. */
  onPlace: (place: string) => void;
  /** Somewhere was really reached — the shell records the visit and
      re-reads its places. Separate from `onPlace` because the two are
      different facts: one is an intention that may still be refused. */
  onLanded: (place: string) => void;
};

export type BrowseKind = {
  /**
   * The word the recents ledger is keyed by — "dir" today, "page" when
   * the browser lands (`khor_sync::recents::Kind`, refused by name).
   *
   * It is also what the tab store is keyed by, so the two landings
   * cannot read each other's tabs. One word, both jobs: a second
   * spelling for "which kind of browsing is this" is a second thing to
   * keep in step.
   */
  kind: string;
  /** The second axis's heading in the completion menu. */
  segmentLabel: string;
  /** Every place kept on purpose, for the fresh tab and for the
      fallback when a machine goes quiet. */
  pins: () => Promise<Place[]>;
  /**
   * Completions for the second segment.
   *
   * **A hook, and that is not an accident**: a path segment has to ask
   * the machine, settle the keystrokes and keep what it has walked, and
   * all three of those are state. The shell calls it exactly once per
   * render, unconditionally.
   *
   * The price is that the descriptor must be **stable for the life of
   * the landing** — swapping one kind for another under a mounted shell
   * would change which hooks run, in the one way React cannot survive.
   * `App.tsx` keys the shell on `kind` so that a swap is a remount.
   */
  useSecond: (machine: DeviceRow | null, typed: string) => SecondSegment;
  /** What a place is called in one word: the tab's title, the lead of a
      row on the fresh tab. */
  leafOf: (place: string) => string;
  /**
   * The resting face of the address bar, beside the machine's chip.
   *
   * **A component, and it must be rendered as an element** — `<Rest …/>`,
   * never `Rest({…})`. Called, its hooks would join the *address bar's*
   * hook list, and the address bar only draws a resting face half the
   * time: React counted a different number of hooks the first time the
   * box opened and tore the landing down. Measured, on the run that
   * introduced this file. `ComponentType` is what says so in a way the
   * compiler can hold.
   */
  Rest: ComponentType<RestProps>;
  /** The tab's content, on the same footing as `Rest` and for the same
      reason — a body with state of its own is the ordinary case. */
  Body: ComponentType<BodyProps>;
};
