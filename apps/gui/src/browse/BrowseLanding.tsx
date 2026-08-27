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

import { forgetRecent, recentVisit, type DeviceRow } from "@/api";
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
  // Whether the switcher is up.
  //
  // **Both faces now, and the correction is worth keeping.** This read
  // "narrow only — the strip is already every tab at once, and a
  // switcher over it would be a second way to do a thing that is not
  // hidden". The second clause is still right and the first was a
  // condition read as a fact: measured, the strip stops showing every
  // tab as soon as it scrolls, and the tabs it does show wear a label
  // 12 wide (`TabStrip`, tokens.css `--tab-w-min`). So the switcher is
  // offered on the wide face exactly when the strip has stopped being
  // able to do what the old comment credited it with, and `TabStrip`
  // owns that condition because it is the only thing that can measure
  // it.
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
    hint: pinned ? kind.words.pinned : kind.words.recent,
  }));

  // Pointing the open tab somewhere puts the switcher away on either
  // face: it was covering the tab that just moved, and leaving it up
  // would hide the thing the press was for. No `narrow` test — the
  // switcher being down already makes this a no-op.
  const goTo = (device: string, place: string) => {
    tabs.goTo(device, place);
    setSwitching(false);
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
        narrow={narrow}
        onOpen={goTo}
        // **The ledger is written here, like every other visit.** The
        // fresh tab is a reader of `usePlaces`; giving it the write as
        // well would put two files in charge of one account (the same
        // rule that keeps `recent_visit` out of the bodies).
        //
        // No local optimism: the row is not struck from the list on the
        // way out, the ledger is re-read and the screen redraws from
        // what it says. That makes a refused write show as the row
        // staying — which is the truth — instead of a row vanishing off
        // a screen whose account still has it (docs/UX.md 不知道就说不
        // 知道, and 「做了但没变化」和「失败」长不一样的脸).
        onForget={(place) => {
          forgetRecent(kind.kind, place.name, place.place)
            .catch(() => {})
            .finally(places.reload);
        }}
        onGo={me ? () => goTo(me.id, "") : undefined}
      />
    );

  // **One switcher, written once**, whichever face asked for it. Spelled
  // twice it would be two places to keep the closing rules in step, and
  // the wide face's copy would be the one nobody notices drifting.
  const region = switching ? (
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
  );

  if (narrow) {
    return (
      <section data-browse-landing={kind.kind} className="flex h-full min-h-0 flex-col">
        {address(<TabCount n={tabs.tabs.length} onPress={() => setSwitching(true)} />)}
        <div className="min-h-0 flex-1 overflow-hidden">{region}</div>
      </section>
    );
  }

  return (
    <section data-browse-landing={kind.kind} className="flex h-full min-h-0 flex-col">
      <TabStrip
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
        // The strip decides *whether* to offer this — it is the only
        // thing that can measure whether it is still showing every tab.
        // This side only says what pressing it does.
        onSwitch={() => setSwitching(true)}
      />
      {address()}
      <div className="min-h-0 flex-1 overflow-hidden">{region}</div>
    </section>
  );
}
