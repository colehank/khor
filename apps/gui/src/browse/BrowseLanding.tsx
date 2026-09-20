// A browsing landing, whole: tabs, an address bar, and whatever the open
// tab is showing.
//
// **One shell, one per landing that browses** (docs/UX.md 浏览型,
// docs/DESIGN.md for what it owns and what a kind must bring).
//
// # The tabs are a column now (docs/SHELL.md 第 2 条, 2026-09-20)
//
// 「浏览器改版为侧边栏. 标签页移动到侧边栏, 参考chrome的最新模式.」— the
// user, with Chrome's vertical tabs as the picture: a column of tabs on
// the left, each a face and a name with a close on it, a 「+」 at the
// foot, the page filling the rest and the address bar above it.
//
// ⚠️ **This overturns what this file used to argue, and the old
// argument is worth keeping because it was not wrong about its own
// subject.** It said a browsing landing has no list pane: 会话 and 机器
// are lists a person keeps an eye on, so their list is furniture, while
// a place is not watched but walked — 「which machine」 is a step on the
// way, and a permanent column of machines is a column spent on a
// question the tab answered the moment it opened.
//
// Every word of that is about **a column of machines**, which is what
// the first version of this landing had. A column of **tabs** is not
// that: it is the thing a person keeps an eye on here, exactly the way
// the sessions list is on its own landing — what is open, which one is
// in front, what to close. So the landing did not stop being 浏览型; it
// grew the one list it turns out to have.
//
// **And the column is `TabSwitcher`, the component the narrow face was
// already using.** Chrome's vertical tab is a face, a name, a second
// line and a close — which is the switcher's row, drawn for a phone
// two batches ago. One component, two seats: a pane on the wide face
// and a screen on the narrow one. A second drawing of it would be the
// failure `PaneBar`'s head warns about, one landing over.
//
// **What left with the strip**: `TabStrip` is not drawn here any more,
// and neither is the way into the switcher on the wide face (`onSwitch`,
// `TabCount`) — the column *is* every tab at once, at every number of
// tabs, so the condition the strip's switcher existed for cannot arise.
// The strip is still one component and still has two customers (the
// sessions landing and the workspace's files).
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
import type { BrowseKind, HeldTab } from "@/browse/kind";
import { NewTabPage } from "@/browse/NewTabPage";
import { TabCount, TabSwitcher, type TabFace } from "@/browse/TabStrip";
import type { BrowseTab, BrowseTabs } from "@/browse/use-browse-tabs";
import { placesInOrder, usePlaces } from "@/browse/use-places";
import { ListPane } from "@/components/ListPane";
import type { PaneWidth } from "@/hooks/use-list-width";
import { gui } from "@/gen/catalog";

/** What a tab calls itself: the way a browser tab wears a page's title
    rather than its URL. A machine with nothing open on it wears its own
    name; a tab pointed nowhere wears the word for what it is. */
function tabLabel(kind: BrowseKind, place: string, machine: DeviceRow | null): string {
  if (!machine) return gui.new_tab;
  if (!place) return machine.name;
  return kind.leafOf(place);
}

/** The stand-in for a kind that holds nothing, so the shell calls one
    hook either way (`BrowseKind.useHeld`). It takes the list it is not
    going to read, because a signature that differed would let the two
    drift. */
function holdsNothing(_open: readonly HeldTab[]): void {}

export function BrowseLanding({
  kind,
  devices,
  tabs,
  narrow,
  pane,
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
  /**
   * The tab column's width, **the shell's own** (`components/ListPane`,
   * docs/SHELL.md 第 6 条).
   *
   * It is the same object the rail toggles, which is what makes 「再按
   * 一次收起」 work here too (第 4 条) — this was the one landing left
   * out of that rule, and it was left out because it had no column to
   * put away rather than by anybody's decision. `null` on the narrow
   * face, where there is one screen and nothing beside it.
   */
  pane: PaneWidth | null;
}) {
  /**
   * Whether the switcher's **screen** is up. Narrow only, and this time
   * that really is a fact about the face rather than a condition read
   * as one.
   *
   * ⚠️ **It was on both faces for two batches, and the reason it is not
   * any more is the tabs moving into a column** (第 2 条). The wide
   * face's switcher existed because the strip stopped showing every tab
   * as soon as it scrolled (tokens.css `--tab-w`: fixed stubs, so it
   * scrolls rather than shrinks) — a switcher was the answer to 「too
   * many to find one in」, which is a question about the number of tabs
   * and not about the width of the window. A column scrolls **down**
   * and shows every tab at every number of them, so the question cannot
   * arise there and a second way in would be clutter over something
   * that is not hidden. That was this file's original judgment, and it
   * is true again for a reason it was not true for before.
   */
  const [switching, setSwitching] = useState(false);
  const places = usePlaces(kind);
  // An element, never a call — see `BrowseKind.Body`.
  const Body = kind.Body;
  // What the kind is holding on the machines the tabs name, so it can
  // give back what nothing points at any more (`BrowseKind.useHeld`).
  // A kind that holds nothing gets a hook that does nothing, rather than
  // a call the shell makes only half the time: `kind` is one object for
  // the life of this landing (`PAGES`, `FILES` are module constants),
  // which is what lets either be called unconditionally here.
  const useHeld = kind.useHeld ?? holdsNothing;
  useHeld(
    tabs.tabs
      .filter((t): t is BrowseTab & { device: string } => t.device !== null)
      .map((t) => ({ id: t.id, device: t.device })),
  );

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
      // The exit machine's name, for the tab's hover title. **The face
      // was already there and said which machine without saying its
      // name** (2026-09-09) — a coloured disc is recognisable once you
      // know it and unreadable before that.
      machine: d?.name ?? null,
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
      tab={tabs.current.id}
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
          tab={tabs.current.id}
          machine={machine}
          place={tabs.current.path}
          narrow={narrow}
          onPlace={(place) => tabs.goTo(machine.id, place)}
          // A new tab beside this one, on the same machine — `open`
          // rather than `goTo`, which is the whole difference the shell
          // draws between the two (`BrowseTabs`).
          onPopup={(place) => tabs.open(machine.id, place)}
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

  /**
   * Every tab as a row: **the column on the wide face, the screen on
   * the narrow one, and one component either way.**
   *
   * Chrome's vertical tab is a face, a name, a second line and a close,
   * with a 「+」 under the list — which is precisely what `TabSwitcher`
   * has drawn for phones since before this ruling. Written twice, the
   * two would be two places to keep the closing rules in step, and the
   * wide one would be the copy nobody notices drifting.
   *
   * `setSwitching(false)` on every action is a no-op on the wide face
   * (the screen is never up there) and is what puts the screen away on
   * the narrow one. Said once rather than branched: a handler that
   * differed per face is the second spelling this whole file is about.
   */
  const tabList = (
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
  );

  // **One screen at a time** (docs/UX.md 姿势, docs/WORKSPACE.md 第 33
  // 条): the tab list is one screen and the page is the other, with the
  // count in the address bar as the way between them.
  // **Nothing is drawn against the stand-in tab** (`useBrowseTabs`'s
  // `ready`). The list is a round trip now rather than a synchronous
  // read of this window's own storage, and everything on this landing
  // is keyed by the open tab's id — so a face built over the stand-in
  // is torn down and rebuilt when the real one lands, taking whatever
  // was being typed into the address box with it. One frame of the
  // landing's own ground says nothing about anybody's tabs, which is
  // the same choice the lock screen makes (`App.tsx`).
  const body = tabs.ready ? content : <div data-browse-waiting className="h-full" />;

  if (narrow) {
    return (
      <section data-browse-landing={kind.kind} className="flex h-full min-h-0 flex-col">
        {tabs.ready && address(<TabCount n={tabs.tabs.length} onPress={() => setSwitching(true)} />)}
        <div className="min-h-0 flex-1 overflow-hidden">{switching ? tabList : body}</div>
      </section>
    );
  }

  return (
    <section data-browse-landing={kind.kind} className="flex h-full min-h-0">
      {/* **No head on this pane, and that is a decision rather than an
          omission.** `ListPane`'s head is where a landing puts what you
          can *do* there — a search box, a filter, a 「+」. Chrome's
          vertical tabs put the one thing this column can do at the
          foot, under the last tab, and the user's own picture is that
          one; `TabSwitcher` has drawn it there since it was a phone
          screen. A search box over the tabs would be a control this
          ruling did not ask for, on the one list in the app short
          enough to read at a glance. */}
      <ListPane
        mark="data-browse-tabs"
        name={kind.words.segment}
        pane={pane}
        narrow={false}
        seam={{ "data-list-seam": "" }}
        // The switcher holds its own scroller, under a foot that must
        // not scroll away with the tabs.
        scroll={false}
      >
        {tabList}
      </ListPane>
      <div className="flex min-w-0 flex-1 flex-col">
        {tabs.ready && address()}
        <div className="min-h-0 flex-1 overflow-hidden">{body}</div>
      </div>
    </section>
  );
}
