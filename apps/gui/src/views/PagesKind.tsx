// Browsing the web through somebody else's network, as an instance of
// the browsing shell (`browse/kind.ts`, docs/DESIGN.md 浏览型外壳).
//
// **The sibling of `FilesKind`, and the whole point of that file having
// been written as a descriptor.** Tabs, the strip, the switcher, the
// store, the machine chip and the fresh tab are the shell's and appear
// nowhere here. What is left is the half that is genuinely about pages.
//
// The two instances differ in exactly the places the contract said they
// would, and it is worth naming them because they are what would have
// been broken by forcing one shape on both:
//
//   - **the second segment does not ask a machine.** A directory's next
//     step is a fact only that disk knows, so files calls `ls`; a URL's
//     next step is not knowable at all — nobody can enumerate the web —
//     so what completes here is what this person already keeps.
//   - **the resting face truncates the other way.** A path is read from
//     its tail (the leaf says where you are) and a URL from its head
//     (the host says whose page it is), so one strip scrolls to the end
//     and this one does not.
import { useEffect } from "react";

import { pageStep } from "@/api";
import type { BodyProps, BrowseKind, HeldTab, RestProps, SecondSegment } from "@/browse/kind";
import { Button } from "@/components/ui/button";
import { IconBack, IconForward, IconReload } from "@/components/icons";
import { gui } from "@/gen/catalog";
import { releaseBorrows, releasePages } from "@/views/held";
import { BrowserPane, normalize } from "@/views/BrowserPane";

/**
 * What a page is called in a word: its host.
 *
 * **Not the whole URL and not a title.** A tab wears a name, and the
 * name a person recognises a page by is the site it is on — khor has no
 * titles to use (nothing here ever loads the page; the borrow hands
 * back a proxy port and the app's own window is what points at it), so
 * inventing one would mean inventing it.
 *
 * A string that will not parse is handed back whole rather than
 * blanked: a tab with no name at all is worse than a tab wearing
 * something odd, and the odd thing is what the person typed.
 */
export function hostOf(place: string): string {
  try {
    return new URL(place).host || place;
  } catch {
    return place;
  }
}

/**
 * A URL at rest: the host in full ink, the rest of it quieter.
 *
 * **Chrome's treatment, and it is the opposite of a breadcrumb.** Every
 * step of a path is a real container somebody can stand in, so every
 * step is somewhere to press; the parts of a URL are not places — you
 * cannot "go up" from `/a/b?q=1` to `/a` and expect anything in
 * particular — so nothing here is a link, and the whole strip is one
 * way into the box.
 *
 * It truncates from the **tail**, which is the other half of that: what
 * identifies a page is at the front.
 */
function PageAddress({ tab, at, onEdit }: RestProps) {
  const host = hostOf(at);
  const rest = at.slice(at.indexOf(host) + host.length) || "";
  const before = at.slice(0, at.indexOf(host));
  const step = (way: "back" | "forward" | "reload") => () => {
    pageStep(tab, way).catch(() => {});
  };
  return (
    <div className="flex min-w-0 flex-1 items-center gap-1 self-stretch">
      {/* **Where a browser puts them, and they belong to the kind rather
          than to the shell**: a directory listing has no history of its
          own to walk, so the files landing has no use for these and the
          address bar must not grow a slot only one instance fills.

          ⚠️ **The arrows do not know whether there is anywhere to go.**
          tauri exposes no `can_go_back`, so they cannot grey out, and a
          press at the start of a history quietly does nothing. That is a
          gap in the platform, not a decision — the honest version of it
          is a lit button that no-ops rather than a disabled one that
          lies about knowing. */}
      <Button
        size="icon"
        variant="ghost"
        data-page-back
        aria-label={gui.page_back}
        className="min-h-touch min-w-touch flex-none text-muted-foreground"
        onClick={step("back")}
      >
        <IconBack />
      </Button>
      <Button
        size="icon"
        variant="ghost"
        data-page-forward
        aria-label={gui.page_forward}
        className="min-h-touch min-w-touch flex-none text-muted-foreground"
        onClick={step("forward")}
      >
        <IconForward />
      </Button>
      <Button
        size="icon"
        variant="ghost"
        data-page-reload
        aria-label={gui.page_reload}
        className="min-h-touch min-w-touch flex-none text-muted-foreground"
        onClick={step("reload")}
      >
        <IconReload />
      </Button>
      <button
        type="button"
        data-address-rest
        aria-label={gui.address_page}
        onClick={onEdit}
        // **`min-h-touch`, and it was missing for four batches without
        // anything noticing** (2026-09-21). This is the way into the
        // address box — the control a person aims at to type somewhere
        // else — and it measured 18 tall. `check-face` section 3 could
        // not see it because the browser stop's fixture had no tab
        // pointed anywhere, so the resting face never drew: 「a floor
        // nothing walks to is not a floor」, the same hole 額度's lower
        // half was in until 2026-09-08. What made it visible was the
        // tabs moving to the node (第 2 条) and the fixture therefore
        // serving a tab with an address on it.
        //
        // ⚠️ **And the floor immediately broke the alignment, which the
        // comment sitting here had already explained** (验收方
        // 2026-09-21, on a real install: the address flush against the
        // top of the window, a good few pixels above 后退 / 前进 /
        // 刷新). A baseline-aligned group in a box taller than itself
        // sits at that box's **top** — which is why `self-stretch` came
        // off this button once before. `min-h-touch` made it a tall box
        // again and handed the same bug back.
        //
        // So the two jobs are two elements now, and neither can undo the
        // other: the button is the finger's floor and centres what is in
        // it, the row inside is the baseline the three spans share.
        className="flex min-h-touch min-w-0 flex-1 cursor-text items-center gap-0 text-left text-callout"
      >
        <span className="flex min-w-0 flex-1 items-baseline gap-0">
          <span className="flex-none text-muted-foreground">{before}</span>
          <span data-page-host className="flex-none font-label">
            {host}
          </span>
          <span className="min-w-0 truncate text-muted-foreground">{rest}</span>
        </span>
      </button>
    </div>
  );
}

/**
 * Gives back what no open tab needs any more (`BrowseKind.useHeld`,
 * `views/held`): the network of a machine nothing names, and the page
 * webview of a tab that is gone.
 *
 * **Two sweeps because they are two questions.** A borrow is per exit
 * machine and outlives any one tab; a page is per tab. Neither can live
 * on the pane: the shell mounts only the open tab's body, so a pane
 * unmounts on every tab switch, and releasing there would tear both down
 * each time somebody glanced at another tab.
 *
 * Keyed on the joined lists rather than the arrays — the shell builds
 * fresh ones every render, and an effect keyed on those would ask a
 * machine to close a session on every render. Tab ids and device ids are
 * both minted from hex and a counter, so a comma cannot appear in one.
 */
function useBorrowsHeld(open: readonly HeldTab[]): void {
  const machines = [...new Set(open.map((t) => t.device))].sort().join(",");
  const tabs = open.map((t) => t.id).sort().join(",");
  useEffect(() => {
    releaseBorrows(machines ? machines.split(",") : []);
  }, [machines]);
  useEffect(() => {
    releasePages(tabs ? tabs.split(",") : []);
  }, [tabs]);
}

function Surf({ tab, machine, place, onPlace, onLanded, onPopup, onTitle, onIcon, onBusy }: BodyProps) {
  return (
    <BrowserPane
      retentionOwner="page"
      tab={tab}
      machine={machine.name}
      device={machine.id}
      url={place}
      // 第十二轮: what khor is doing for this tab, so the row in the
      // sidebar can breathe while it is doing it.
      onBusy={(busy) => onBusy?.(busy)}
      // This machine's tab goes out of this machine's own network
      // (docs/SHELL.md 第四轮 第 5 条).
      here={machine.me}
      onTitle={(t) => onTitle?.(t)}
      onIcon={onIcon}
      onOpened={onLanded}
      // Where the page went is where the tab is, so it is `onPlace` and
      // not `onLanded`: the shell's two callbacks split on *intention*
      // versus *arrival*, and a page that has already moved has arrived.
      // Recording it as a visit as well would put every redirect a site
      // passes through into 常去的地方.
      onWent={onPlace}
      onPopped={onPopup}
    />
  );
}

/**
 * The web, for the browsing shell.
 *
 * A module constant for `FilesKind`'s reason: the shell calls
 * `useSecond` as a hook, so the descriptor has to be the same object for
 * the life of the landing.
 */
export const PAGES: BrowseKind = {
  // The word `khor_sync::recents` refuses by name (`PlaceKind::parse`
  // takes exactly "dir" and "page"), and the word the tab store is
  // keyed by — which is what keeps this landing's tabs and the files
  // landing's out of each other's storage.
  kind: "page",
  // ⚠️ **Getters, not plain properties** (批 D3). This descriptor is
  // built once and must stay stable for the life of the landing (the
  // type's own note says why), so a word read here would be read once —
  // and would then keep whichever language the page loaded in while the
  // rest of the screen changed around it. A getter keeps every call
  // site (`kind.words.address`) exactly as it was and moves the read to
  // the moment it is painted.
  words: {
    get address() {
      return gui.address_page;
    },
    get segment() {
      return gui.browser_tab;
    },
    // The one heading both landings share — see `recent-places`.
    get recent() {
      return gui.recent_places;
    },
    get search() {
      return gui.search_tabs;
    },
    get copy() {
      return gui.copy_url;
    },
  },
  useSecond: (machine, _typed, remembered): SecondSegment => ({
    // **What this person already keeps on that machine**, handed in by
    // the shell rather than fetched again (`BrowseKind.useSecond` says
    // why). Nothing is asked of the machine: a URL's next step is not
    // something anybody can enumerate, so the only honest completions
    // are the addresses already known.
    //
    // Not filtered by what has been typed — the box does that, and it
    // does it by prefix on the key, which for an address is the whole
    // address (`Omnibox`: a completion is *continued*, not *found*).
    candidates: remembered
      .filter((p) => p.device === machine?.id)
      .map((p) => ({ key: p.place, label: p.place })),
    // **Never true here, and that is a fact rather than a stub.** For a
    // directory `failed` means "that machine did not answer the `ls`";
    // nothing is asked of a machine to complete a URL, so there is no
    // call that can fail and nothing to degrade *from*. Reporting a
    // refusal that did not happen would make the shell swap in
    // remembered addresses to replace… remembered addresses.
    failed: false,
  }),
  leafOf: hostOf,
  Rest: PageAddress,
  Body: Surf,
  useHeld: useBorrowsHeld,
};

/** Re-exported so the address bar's submit and this file agree on what
    counts as an address (a bare host means https). */
export { normalize };
