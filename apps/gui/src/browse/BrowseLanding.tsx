// A browsing landing, whole: tabs, an address bar, and whatever the open
// tab is showing.
//
// **One shell, one per landing that browses** (docs/UX.md 浏览型,
// docs/DESIGN.md for what it owns and what a kind must bring). A
// browsing landing has **no list pane** — 会话 and 机器 are lists a
// person keeps an eye on, so their list is furniture and stays on
// screen, while a place is not watched but walked: which machine is a
// step on the way, and a permanent column of machines is a column spent
// on a question that was answered the moment the tab opened. What
// replaces it is the browser's own answer, tabs and an address bar, and
// the column goes to the content.
//
// One tab is (a machine, a place). That pair is the tab's whole
// identity, it is what the strip titles, what the address bar draws,
// and what survives a reload (`useBrowseTabs`) — one fact with three
// faces, held in one place so they cannot disagree.
//
// **What is not here is as deliberate as what is.** Nothing in this
// file knows what a place looks like: no slash, no URL, no listing. All
// of that arrives through `kind`, which is what lets the second
// browsing landing be a descriptor rather than a second copy of this.
import { useState } from "react";

import { recentVisit, type DeviceRow } from "@/api";
import { AddressBar, type Remembered } from "@/browse/AddressBar";
import type { BrowseKind } from "@/browse/kind";
import { NewTabPage } from "@/browse/NewTabPage";
import { TabCount, TabStrip, TabSwitcher, type TabFace } from "@/browse/TabStrip";
import type { BrowseTabs } from "@/browse/use-browse-tabs";
import { placesInOrder, usePlaces } from "@/browse/use-places";
import { gui } from "@/gen/catalog";

/** What a tab calls itself: the way a browser tab wears a page's title
    rather than its URL. A machine with nothing open on it wears its own
    name; a tab pointed nowhere wears the word for what it is. */
function tabLabel(kind: BrowseKind, place: string, machine: DeviceRow | null): string {
  if (!machine) return gui.new_tab;
  if (!place) return machine.name;
  return kind.leafOf(place);
}

export function BrowseLanding({
  kind,
  devices,
  tabs,
  narrow,
}: {
  /**
   * Which kind of browsing this is.
   *
   * **Must not change under a mounted landing** — `kind.useSecond` is a
   * hook, so a swap would change which hooks run. `App.tsx` keys this
   * on `kind.kind`, which makes a swap a remount instead.
   */
  kind: BrowseKind;
  devices: DeviceRow[];
  tabs: BrowseTabs;
  narrow: boolean;
}) {
  // Narrow only: the switcher is up. Wide ignores it — the strip is
  // already every tab at once, and a switcher over it would be a second
  // way to do a thing that is not hidden.
  const [switching, setSwitching] = useState(false);
  const places = usePlaces(kind);
  // An element, never a call — see `BrowseKind.Body`.
  const Body = kind.Body;

  const deviceOf = (id: string | null) => (id ? (devices.find((d) => d.id === id) ?? null) : null);
  const machine = deviceOf(tabs.current.device);
  const me = devices.find((d) => d.me) ?? null;

  const faces: TabFace[] = tabs.tabs.map((t) => {
    const d = deviceOf(t.device);
    return {
      id: t.id,
      label: tabLabel(kind, t.path, d),
      face: d?.face ?? null,
      path: t.path,
    };
  });

  // Every place this screen remembers, each carrying the word for which
  // list it came from. The address bar falls back to these when a
  // machine stops answering, and the hint is what stops a remembered
  // address passing for a live one.
  const remembered: Remembered[] = placesInOrder(places).map(({ place, pinned }) => ({
    place,
    hint: pinned ? gui.pinned_dirs : gui.recent_dirs,
  }));

  const goTo = (device: string, place: string) => {
    tabs.goTo(device, place);
    if (narrow) setSwitching(false);
  };

  const address = (trailing?: React.ReactNode) => (
    <AddressBar
      kind={kind}
      devices={devices}
      machine={machine}
      at={tabs.current.path}
      remembered={remembered}
      fallback={me}
      onGo={goTo}
      onClear={tabs.clear}
      trailing={trailing}
    />
  );

  const content =
    machine && tabs.current.device ? (
      <div
        // Keyed by the tab, not by where the tab is: a tab that moves is
        // the same tab, and re-keying on the place would throw away the
        // scroll position on every step. Keyed by *machine* too, because
        // an address from one machine means nothing on another and the
        // body must not carry one across.
        key={`${tabs.current.id}:${tabs.current.device}`}
        className="h-full min-h-0"
      >
        <Body
          machine={machine}
          place={tabs.current.path}
          narrow={narrow}
          onPlace={(place) => tabs.goTo(machine.id, place)}
          onLanded={(place) => {
            // **The visit is recorded here, once, for every kind.** The
            // ledger is the node's (`khor_sync::recents`) and the body
            // has no business writing to it — a kind that forgot to
            // would silently have no history, which looks like the
            // feature being broken somewhere else entirely. Fire and
            // forget: a history that would not write must never be the
            // reason a place does not open.
            recentVisit(kind.kind, machine.name, place).catch(() => {});
            // …and the memory this screen is holding is now a visit out
            // of date. Asked again rather than patched locally: the node
            // keeps that ledger, and a local copy would be a second one.
            places.reload();
          }}
        />
      </div>
    ) : (
      <NewTabPage
        kind={kind}
        places={places}
        devices={devices}
        onOpen={goTo}
        onGo={me ? () => goTo(me.id, "") : undefined}
      />
    );

  if (narrow) {
    return (
      <section data-browse-landing={kind.kind} className="flex h-full min-h-0 flex-col">
        {address(<TabCount n={tabs.tabs.length} onPress={() => setSwitching(true)} />)}
        <div className="min-h-0 flex-1 overflow-hidden">
          {switching ? (
            <TabSwitcher
              faces={faces}
              active={tabs.current.id}
              onFocus={(id) => {
                tabs.focus(id);
                setSwitching(false);
              }}
              onClose={tabs.close}
              onNew={() => {
                tabs.add();
                setSwitching(false);
              }}
            />
          ) : (
            content
          )}
        </div>
      </section>
    );
  }

  return (
    <section data-browse-landing={kind.kind} className="flex h-full min-h-0 flex-col">
      <TabStrip
        faces={faces}
        active={tabs.current.id}
        onFocus={tabs.focus}
        onClose={tabs.close}
        onNew={tabs.add}
      />
      {address()}
      <div className="min-h-0 flex-1 overflow-hidden">{content}</div>
    </section>
  );
}
