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
// **The 「+」 is at the head now, beside a search box** (docs/SHELL.md
// 第二轮, 2026-09-21): every sidebar has one look, this one included.
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
import { useEffect, useState } from "react";

import { fetchPageIcons, forgetRecent, openLink, recentVisit, type DeviceRow } from "@/api";
import { AddressBar, type Remembered } from "@/browse/AddressBar";
import type { BrowseKind, HeldTab } from "@/browse/kind";
import { NewTabPage } from "@/browse/NewTabPage";
import { TabCount } from "@/browse/TabStrip";
import type { BrowseTab, BrowseTabs } from "@/browse/use-browse-tabs";
import { useLoading } from "@/browse/use-loading";
import { placesInOrder, usePlaces } from "@/browse/use-places";
import { MachineAvatar } from "@/components/Avatar";
import { IconBrowser, IconClose } from "@/components/icons";
import { ListPane } from "@/components/ListPane";
import { PaneBar } from "@/components/PaneBar";
import { Region } from "@/components/Region";
import type { RowAction } from "@/components/RowMenu";
import { SideList, type SideRow } from "@/components/SideList";
import { Button } from "@/components/ui/button";
import type { PaneWidth } from "@/hooks/use-list-width";
import { gui } from "@/gen/catalog";
import { groupLabel } from "@/words";

/** The site a page is on, spelled as the node keys its kept icons
    (`khor_node::favicon::origin`: `scheme://host[:port]`); `""` for
    anything that is not an http(s) page. */
function originOf(path: string): string {
  try {
    const u = new URL(path);
    return u.protocol === "http:" || u.protocol === "https:" ? u.origin : "";
  } catch {
    return "";
  }
}

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
  /** What the column's search box holds (docs/SHELL.md 第二轮). This
      screen's, like every other pane's query — a filter over rows that
      are already here, not a question for the node. */
  const [query, setQuery] = useState("");
  /** A row action the node refused, in its own sentence, by tab and
      action — `RowMenu`'s `note`: it outlives one opening of the menu,
      so it is kept here and not there. The one a person meets is
      「删除」 on a pinned tab (`khor_node::tabs::close`). */
  const [refused, setRefused] = useState<Map<string, string>>(new Map());
  /** Each tab's site icon, once its page has loaded here in this window. */
  const [icons, setIcons] = useState<Map<string, string>>(new Map());
  /** **And every icon fetched before, read off this disk by site**
      (docs/SHELL.md 第五轮 第 8 条:「favicon 存盘…重开 app 就有」). The
      node keeps what `pageIcon` got; this is a read of that file and asks
      no site anything, so a restart draws each row's icon without a
      request leaving from here. Read once when the landing mounts. */
  const [kept, setKept] = useState<Record<string, string>>({});
  useEffect(() => {
    let live = true;
    fetchPageIcons().then(
      (m) => live && setKept(m),
      // A cache that cannot be read is rows wearing the generic mark.
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);
  /** Which tabs are loading, for the breath on their rows (docs/SHELL.md
      第十二轮). Kept by the landing rather than by the body, because the
      body is only ever mounted for the tab in front and a tab loading
      behind one has to breathe too. */
  const load = useLoading();
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

  /**
   * A tab's own menu: **置顶, 删除, then what this column has of its own**
   * (docs/SHELL.md 第二轮:「每一行都有右键菜单,头两项永远是置顶、删除,
   * 顺序固定」).
   *
   * 删除 asks nothing first. What it does to a tab is close it — the
   * same act as the × beside it, which has never asked — and a question
   * on one road and not the other would make the cheap act read as the
   * costly one depending on where it was pressed. A pinned tab is
   * refused by the node; that sentence is the note.
   *
   * No 「在系统浏览器中打开」: the ruling asked for it where a road
   * already exists, and none does — a page here leaves through the
   * machine the tab names, and the system browser would take it out
   * through this one.
   */
  const menuOf = (t: BrowseTab): RowAction[] => {
    const k = (action: string) => `${t.id}:${action}`;
    const answered = (action: string) => (p: Promise<void>) =>
      p.then(
        () =>
          setRefused((m) => {
            if (!m.has(k(action))) return m;
            const next = new Map(m);
            next.delete(k(action));
            return next;
          }),
        (e: unknown) => {
          setRefused((m) => new Map(m).set(k(action), String(e instanceof Error ? e.message : e)));
          // Rejecting is how an action says it did not happen (`RowMenu`).
          throw e;
        },
      );
    return [
      {
        key: "pin",
        // What pressing it does, so the opposite of the tab's state —
        // every row's pin says it this way.
        label: t.pinned ? gui.unpin : gui.pin,
        note: refused.get(k("pin")),
        run: () => answered("pin")(tabs.pin(t.id, !t.pinned)),
      },
      {
        key: "close",
        label: gui.delete,
        ask: gui.tab_remove_confirm,
        note: refused.get(k("close")),
        run: () => answered("close")(tabs.close(t.id)),
      },
      // A tab pointed nowhere has nothing to copy, and an item that
      // would put an empty string on the clipboard is not offered.
      ...(t.path
        ? [{ key: "copy-place", label: kind.words.copy, run: () => navigator.clipboard.writeText(t.path) }]
        : []),
      // **Only on a tab that goes out of this machine**: a borrowed page
      // handed to the system browser would leave through a different
      // network, and that is a different page (`open_link`).
      ...(t.path && deviceOf(t.device)?.me
        ? [{ key: "open-outside", label: gui.open_in_browser, run: () => openLink(t.path).then(() => {}) }]
        : []),
    ];
  };

  /**
   * Every tab as a row of the one sidebar list (`SideList`, docs/SHELL.md
   * 第四轮): the column on the wide face, the screen on the narrow one.
   * The pinned group, its fold and the box's rule are that list's.
   */
  const side: SideRow[] = tabs.tabs.map((t) => {
    const d = deviceOf(t.device);
    const label = tabLabel(kind, t.path, d);
    const on = t.id === tabs.current.id;
    // 「标题是网页的标题而非网址, 副标题是现在的网址, 但不用显示http(s)://」
    // (docs/SHELL.md 第四轮, Chrome's own arrangement): the page's own
    // title when it has said one, else the bare address, else what a tab
    // pointed nowhere is called.
    // A site's root reads as its host, the way Chrome prints it.
    const bare = t.path.replace(/^https?:\/\//i, "").replace(/^([^/]+)\/$/, "$1");
    const src = icons.get(t.id) ?? kept[originOf(t.path)];
    return {
      id: t.id,
      group: t.pinned ? "pin" : "",
      pinned: t.pinned,
      face: <MachineAvatar face={d?.face ?? null} className="size-avatar" />,
      // **The same ring a 忙碌 session wears** (docs/SHELL.md 第十二轮:
      // 「用同一个光晕…不新造一种转圈」) — one state word, one look,
      // and the reduced-motion guard comes with it (`app.css`). Only
      // this row: the strip's own tab is 80 wide and has no room for a
      // ring around its face (第十二轮 第 3 条).
      halo: load.busy(t.id) ? "busy" : undefined,
      badge: {
        mark: src ? "favicon" : "page",
        icon: src ? (
          <img src={src} alt="" className="size-kind-mark-glyph rounded-sm" />
        ) : (
          <IconBrowser className="size-kind-mark-glyph" />
        ),
      },
      title: (
        <span data-tab-label data-tab-name className="block truncate text-callout font-label">
          {t.title || bare || label}
        </span>
      ),
      sub: bare || undefined,
      // What the row shows — its name and its place — and nothing else.
      find: `${label} ${t.title} ${t.path}`,
      selected: on,
      menu: menuOf(t),
      onOpen: () => {
        tabs.focus(t.id);
        setSwitching(false);
      },
      // **A pinned tab has no ×, and no pin either** (docs/SHELL.md 第四轮
      // 第 8 条:「浏览器不要置顶按钮了呀, 完全交给侧边栏list」): the pinned
      // group it sits under is its mark, and it is unpinned from its menu
      // before it can go. The seat stays, empty, so the rows keep one
      // shape. The × is fire and forget — never drawn on a pinned tab, so
      // the node's one refusal for it cannot be met here — and it closes
      // without asking, as Chrome's does.
      after: t.pinned ? (
        <span aria-hidden="true" className="min-h-touch min-w-touch flex-none" />
      ) : (
        <Button
          size="icon"
          variant="ghost"
          data-switch-close={t.id}
          aria-label={gui.close_tab}
          className="min-h-touch min-w-touch flex-none"
          onClick={() => void tabs.close(t.id).catch(() => {})}
        >
          <IconClose />
        </Button>
      ),
      rowData: { "data-switch-tab": t.id, "data-on": on, "data-pinned": t.pinned },
      openData: { "data-switch-open": t.id },
    };
  });

  // Every place this screen remembers, each carrying the word for which
  // list it came from. The address bar falls back to these when a
  // machine stops answering, and the hint is what stops a remembered
  // address passing for a live one.
  const remembered: Remembered[] = placesInOrder(places).map((place) => ({
    place,
    hint: kind.words.recent,
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
          // The title the tab already has rides along: the page names
          // itself on its own beat (`onTitle`), and a move that cleared it
          // would leave the row on its bare address until the next word.
          onPlace={(place) => tabs.goTo(machine.id, place, tabs.current.title)}
          onTitle={(title) => tabs.goTo(machine.id, tabs.current.path, title)}
          onIcon={(src) => {
            const id = tabs.current.id;
            setIcons((m) => (m.get(id) === src ? m : new Map(m).set(id, src)));
          }}
          // What khor itself has out for this tab. Whether that is what
          // the row breathes on is `use-loading`'s call: in the app the
          // page's own report wins, and this only lights it.
          onBusy={(busy) => load.errand(tabs.current.id, busy)}
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
      />
    );

  const tabList = (
    <div data-tab-switcher className="h-full min-h-0 overflow-y-auto">
      <SideList rows={side} query={query} label={groupLabel} none={gui.no_matches} />
    </div>
  );

  /**
   * **The column's head, the one every sidebar wears** (docs/SHELL.md
   * 第二轮:「浏览器侧边栏也要和其他侧边栏一样有搜索和新建的组件在最上面」):
   * a search box and a 「+」, in `PaneBar`, at `--pane-head`.
   *
   * ⚠️ 墓碑:this column had no head on purpose for one batch — Chrome's
   * vertical tabs put the one thing it can do at the foot, and that was
   * the user's own picture. The user overruled it: one kind of thing (a
   * sidebar) has one look. The 「+」 moved up rather than being drawn
   * twice; a foot *and* a head would be two places for one act.
   */
  const head = (
    <PaneBar
      seat="side"
      searchLabel={kind.words.search}
      query={query}
      onQuery={setQuery}
      create={{
        label: gui.new_tab,
        onSelect: () => {
          tabs.add();
          // A tab that did not match would open behind the box that
          // hid it; clearing it is what makes the new row visible.
          setQuery("");
          setSwitching(false);
        },
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
        {tabs.ready &&
          address(
            // A switch (第四轮 第 10 条): the tab screen it opens, it closes.
            <TabCount n={tabs.tabs.length} onPress={() => setSwitching((v) => !v)} open={switching} />,
          )}
        <div className="min-h-0 flex-1 overflow-hidden">
          {switching ? (
            // The narrow face's tab screen is this landing's sidebar,
            // one screen wide — so it wears the same head (seated in the
            // first row, 第十三轮) and the sidebar's ground.
            <div className="flex h-full min-h-0 flex-col bg-side">
              {head}
              <div className="min-h-0 flex-1">{tabList}</div>
            </div>
          ) : (
            body
          )}
        </div>
      </section>
    );
  }

  return (
    <section data-browse-landing={kind.kind} className="flex h-full min-h-0 min-w-0 flex-col">
      {/* Written here, drawn in the left block (第十三轮): the tabs
          column beside the rail, its head in the block's first row. */}
      <ListPane
        mark="data-browse-tabs"
        name={kind.words.segment}
        pane={pane}
        narrow={false}
        seam={{ "data-list-seam": "" }}
        seat="side"
        head={head}
        // The switcher holds its own scroller.
        scroll={false}
      >
        {tabList}
      </ListPane>
      {/* **文档 (C)** (docs/DESIGN.md 第五节, D7): the row in the column
          *is* the tab, so there is no strip — R0 is the address bar and
          the page is the board under it. Until the tabs have been read
          there is no address to show, and R0 says the landing's name. */}
      <Region
        kind="doc"
        title={kind.words.segment}
        head={tabs.ready ? <div className="flex min-w-0 flex-1 flex-col">{address()}</div> : undefined}
      >
        <div className="h-full min-h-0 overflow-hidden">{body}</div>
      </Region>
    </section>
  );
}
