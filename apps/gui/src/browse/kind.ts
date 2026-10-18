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
// not from what a browser landing might. Nothing here was speculative:
// every member below had a live caller in `views/FilesKind.tsx`.
//
// **That caller left on 2026-09-11** (工作区替代文件): browsing a disk
// is not a browsing landing any more, it is what a workspace's file
// tree does beside a list worth watching. So this contract has **one**
// customer now, `views/PagesKind.tsx`, and the claim it was written to
// make — that these members are the shared half of two kinds — is
// un-tested until a second one returns. The VS Code tab is that second
// one and is on the account (docs/handoff/001.md). **If it does not
// land, this file has one customer and should be inlined into the one
// that uses it** — that is the line for whoever reads this next, and it
// is a line, not a hope.
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
  /** This tab's handle, for a resting face that has controls of its own
      to aim. The page kind's back, forward and reload act on one tab's
      webview, and there is nothing in an address that says which. */
  tab: string;
  /** Where the tab actually is, in this kind's own spelling. */
  at: string;
  /** Go somewhere else on the same machine. */
  onGo: (place: string) => void;
  /** Open the box. The shell has no way of its own into it from here —
      the resting face belongs to the kind, so the kind decides which
      part of it is the way in. */
  onEdit: () => void;
};

/** One open tab, as much of it as a kind needs to sweep what it holds. */
export type HeldTab = { id: string; device: string };

/** What one tab shows. */
export type BodyProps = {
  /** This tab's own handle. A body that holds something per tab — a page
      kind holds a live webview — needs to name which tab it is holding
      for, and cannot derive it: two tabs on one machine at one place are
      two tabs (`BrowseTab.id`). */
  tab: string;
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
  /** Open a **new** tab here, on this same machine, and focus it.
      What a page asking for a new window gets: whether a link opens
      where you are or beside you is the page author's call, and the
      body is where that call arrives. */
  onPopup: (place: string) => void;
  /** What the page calls itself, as it says so (docs/SHELL.md 第四轮:
      「标题是网页的标题而非网址」). A kind whose places have no title
      never calls it. */
  onTitle?: (title: string) => void;
  /** The page's icon, as something an `<img>` can draw, for the corner of
      its row. */
  onIcon?: (src: string, at: string) => void;
  /** khor is fetching something for this tab, or has stopped
      (docs/SHELL.md 第十二轮). **What the body knows and the shell does
      not**: the shell asked for a place, and only the body can say
      whether anything is still on its way. A kind whose places arrive
      instantly never calls it, and its rows never breathe. */
  onBusy?: (busy: boolean) => void;
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
  /**
   * Every word this landing says that names *its own* kind of thing.
   *
   * **The shell writes none of these, and that is the correction 浏览器
   * 接壳批 made.** The first version of this contract carried exactly
   * one word (the completion menu's heading) and left five more baked
   * into the shell — so the browser landing mounted correctly and then
   * told people it had 「还没有钉住的目录」. Measured on screen, not
   * argued: `data-browse-landing="page"` with the files landing's
   * sentences under it.
   *
   * The direction is what makes this growth rather than a broken line:
   * every one of these moves instance knowledge **out** of the shell.
   * The shell ends up knowing less about directories, not more — the
   * same direction the boundary gate enforces. Welding a second
   * landing's knowledge *in* would be the real break.
   */
  words: {
    /** Names the address bar and stands in as its hint. */
    address: string;
    /** The second axis's heading in the completion menu. */
    segment: string;
    /** The fresh tab's one section heading. ⚠️ 墓碑: there were two, and
        the other was 「置顶的…」 — retired with its empty sentence on
        2026-09-22 (docs/SHELL.md 第六轮: a pin is the sidebar's pinned
        group and nothing else). */
    recent: string;
    // ⚠️ 墓碑: `go`, the way onward from an empty fresh tab as a thing
    // to press — retired 2026-09-22 with the button (docs/SHELL.md 第七轮).
    /** The tab column's search box (docs/SHELL.md 第二轮) — it finds a
        tab by what it is called and where it is, and 「where」 is this
        kind's own word. */
    search: string;
    /** The row menu's copy item: a place in this kind's spelling is
        paste material. */
    copy: string;
  };
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
  useSecond: (
    machine: DeviceRow | null,
    typed: string,
    /**
     * What this screen already remembers, on every machine.
     *
     * **Handed in rather than fetched, because a kind may complete from
     * memory instead of from a machine.** A directory's next step is a
     * fact only that disk knows, so the files kind asks it; a URL's next
     * step is not knowable at all — nobody enumerates the web — so the
     * pages kind completes from the pins and recents this person keeps,
     * and the shell is already holding exactly those (`usePlaces`: one
     * fetch, two customers). Fetched again inside the kind they would be
     * two answers a poll apart, and the box could offer an address the
     * page below it is not showing.
     */
    remembered: Place[],
  ) => SecondSegment;
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
  /**
   * What this kind is holding for the tabs that are open, told the
   * machines they point at right now. Optional: a kind that holds
   * nothing on a machine leaves it out, and files does — a listing is
   * asked for and answered, and nothing survives the answer.
   *
   * **A hook over the whole set, and it has to be.** The shell mounts
   * only the open tab's `Body`, so a body unmounts on every tab switch;
   * a kind that released what it holds there would give it back each
   * time somebody looked at another tab. "Is anybody still pointed at
   * this machine" is a question only the set can answer.
   *
   * The list is the open tabs that name a machine, blank ones left out.
   * **Both fields, because a kind can hold two different things**: a page
   * kind holds one borrow per *machine* and one webview per *tab*, and
   * the two are swept by different questions.
   */
  useHeld?: (open: readonly HeldTab[]) => void;
};
