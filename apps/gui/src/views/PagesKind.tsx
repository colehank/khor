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

import { fetchWebPins, pageStep } from "@/api";
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
        // `items-baseline` lines the three spans up with each other;
        // `self-stretch` used to also stretch this button to the row's
        // full height, and a baseline-aligned group inside a stretched
        // box sits at its top — so the address rode high above the three
        // buttons beside it. Its own height, centred by the row.
        className="flex min-w-0 flex-1 cursor-text items-baseline gap-0 text-left text-sm"
      >
        <span className="flex-none text-muted-foreground">{before}</span>
        <span data-page-host className="flex-none font-label">
          {host}
        </span>
        <span className="min-w-0 truncate text-muted-foreground">{rest}</span>
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

function Surf({ tab, machine, place, onPlace, onLanded, onPopup }: BodyProps) {
  return (
    <BrowserPane
      tab={tab}
      machine={machine.name}
      device={machine.id}
      url={place}
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
  words: {
    address: gui.address_page,
    segment: gui.browser_tab,
    pinned: gui.pinned_webs,
    // The one heading both landings share — see `recent-places`.
    recent: gui.recent_places,
    empty: gui.no_pinned_webs,
    go: gui.pick_exit,
  },
  pins: async () => {
    const rows = await fetchWebPins();
    return rows.map((r) => ({ device: r.device, name: r.name, place: r.url, at_ms: null }));
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
