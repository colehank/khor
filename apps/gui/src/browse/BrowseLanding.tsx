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
import { useEffect, useRef, useState, type ReactNode } from "react";

import { fetchPageIcons, onPageWord, openLink, openTab, pageDeck, pageGround, pageStep, recentVisit, reveal, type DeviceRow } from "@/api";
import { revealHere } from "@/views/chat-files";
import { lastPageMachine, PageField, reachable, rememberPageMachine } from "@/browse/PageField";
import { nativeDeckHere, readDeck, seatControl } from "@/lib/native-deck";
import { pageFail, type PageFail } from "@/browse/page-fail";
import { pageDownload, type PageSaid } from "@/browse/page-download";
import { Tabs } from "@/browse/Tabs";
import type { TabFace } from "@/browse/TabStrip";
import type { BrowseKind, HeldTab } from "@/browse/kind";
import type { BrowseTab, BrowseTabs } from "@/browse/use-browse-tabs";
import { useLoading } from "@/browse/use-loading";
import { IconArrow, IconPlus, IconReload } from "@/components/icons";
import { GlassCircle, GlassItem } from "@/components/Glass";
import { SideFind } from "@/components/SideFind";
import { PathCut } from "@/components/PathCut";
import { BarSeat, SessionScreen } from "@/components/SessionBar";
import { FOLD_AT, foldStep, inkOver, UnfoldHold, type DeckInk, type FoldMemo } from "@/lib/deck-fold";
import { pageKind } from "@/components/SessionFace";
import { ListPane } from "@/components/ListPane";
import { PaneBar } from "@/components/PaneBar";
import type { RowAction } from "@/components/RowMenu";
import { SideList, type SideRow } from "@/components/SideList";
import type { PaneWidth } from "@/hooks/use-list-width";
import { gui } from "@/gen/catalog";
import { useTabHost } from "@/lib/shortcuts";
import { groupLabel } from "@/words";

/** A page's kind mark, on its machine's face — in the list and on its tab
    (the redesign): the face says which machine, the mark that it is a
    page, the way a session wears its kind. */
/** A page's host, as a person names a site (`www.baidu.com`); the
    address as it stands when it is not a URL. */
function hostOf(path: string): string {
  try {
    return new URL(path).host || path;
  } catch {
    return path.replace(/^https?:\/\//i, "").replace(/\/.*$/, "");
  }
}

/** **The pill's word for a page** (UX-35 §4, the owner 2026-10-06: iOS
    Safari's compact bar): the host alone — no scheme, no `www.`, no path
    — with its port kept (`localhost:5467`). `""` for a place that is not
    an http(s) page, which keeps its leaf (`tabLabel`). An address typed
    without its scheme reads as the http one it opens. */
function pillHost(path: string): string {
  const at = /^[a-z][a-z0-9+.-]*:\/\//i.test(path) ? path : /^[\w.-]+(?::\d+)?(?:[/?#]|$)/.test(path) ? `http://${path}` : "";
  try {
    const u = new URL(at);
    return u.protocol === "http:" || u.protocol === "https:" ? u.host.replace(/^www\./i, "") : "";
  } catch {
    return "";
  }
}

/** **The site's icon, where there is one** (UX-22, the owner 2026-10-05:
    every row wore khor's browser glyph): the mark on the machine's face is
    the page's favicon once it has been fetched — this window, or any
    time before (the node keeps them, `favicons.json`) — and the browser's
    mark only before or without one. The mark itself is the assembly's
    (`pageKind`, UX-32). ⚠️ 墓碑 (2026-10-02 → UX-22): the browser's mark
    on every row. */
const markOf = pageKind;

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
  caption,
  screen = "list",
  onOpen,
  onBack,
  bare,
}: {
  /**
   * **Drawn inside the session's 附栏** (UX-37 §6, UX-39 §3, UX-40,
   * narrow only): the list is the tabs opened from `here` (a session's
   * id) under the 附栏's one head and its find row — this session's pages,
   * ＋ a new page that is this session's (UX-40 §1). A page wears the main
   * area's narrow deck at the 附栏's foot: ‹ · ← → host ↻ · the tabs
   * circle with this session's pages, never folded, no native deck (the
   * session's deck is away while the sheet is up; UX-40 §4). ⚠️ 墓碑
   * (UX-37 §6 → UX-40 §4): ‹ and the host in a head at the page's top.
   * ⚠️ 墓碑 (UX-37 §1 → UX-39 §3): ‹ from the slice to every tab (`here:
   * null`, 全部网页).
   */
  bare?: { here: string };
  /** Narrow only: which of the pair is up — the tabs or the page — and
      the shell's two moves between them. */
  screen?: "list" | "detail";
  onOpen?: () => void;
  onBack?: () => void;
  /** 过期 / 离线: the shell's one line for R0's right end, after the
      address bar (`Region caption`; components/RegionState). */
  caption?: ReactNode;
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
  /** Narrow: the list was chosen from, or ＋ was pressed — the page is
      the next screen (the shell's list ⇄ detail pair). */
  const opened = () => onOpen?.();
  // ⌘W, ⌃⇥ / ⌃⇧⇥ on this landing's tabs (`lib/shortcuts`, 2026-09-28): the
  // tab in front closes — a pinned one is refused by the node, as its
  // row menu's 删除 is — and the steps walk the column's order.
  useTabHost(
    tabs.ready
      ? {
          count: tabs.tabs.length,
          front: true,
          close: () => {
            tabs.close(tabs.current.id).catch(() => {});
          },
          step: (by) => {
            const at = tabs.tabs.findIndex((t) => t.id === tabs.current.id);
            const next = tabs.tabs[(at + by + tabs.tabs.length) % tabs.tabs.length];
            if (next !== undefined) tabs.focus(next.id);
          },
        }
      : null,
  );
  /** What the column's search box holds (docs/SHELL.md 第二轮). This
      screen's, like every other pane's query — a filter over rows that
      are already here, not a question for the node. */
  const [query, setQuery] = useState("");
  /** A row action the node refused, in its own sentence, by tab and
      action — `RowMenu`'s `note`: it outlives one opening of the menu,
      so it is kept here and not there. The one a person meets is
      「删除」 on a pinned tab (`khor_node::tabs::close`). */
  const [refused, setRefused] = useState<Map<string, string>>(new Map());
  // ⚠️ 墓碑 (the redesign, 2026-10-02): each row's mark was its site's
  // favicon, fetched and kept (`pageIcon`, `fetchPageIcons`). A row's face
  // is the machine the page goes through now, and its mark the browser's
  // kind, the way a session row carries its kind (the owner: 「现在完全不
  // 知道是在那台机器」).
  /** Which tabs are loading, for the breath on their rows (docs/SHELL.md
      第十二轮). Kept by the landing rather than by the body, because the
      body is only ever mounted for the tab in front and a tab loading
      behind one has to breathe too. */
  const load = useLoading();
  /** Each tab's site icon, once its page has loaded here in this window. */
  const [icons, setIcons] = useState<Map<string, string>>(new Map());
  /** **And every icon fetched before, read off this disk by site**
      (docs/SHELL.md 第五轮 第 8 条:「favicon 存盘…重开 app 就有」): a read
      of the node's file that asks no site anything. Once, on mount. */
  const [kept, setKept] = useState<Record<string, string>>({});
  useEffect(() => {
    let live = true;
    fetchPageIcons().then(
      (m) => live && setKept(m),
      // A cache that cannot be read is rows wearing the browser's mark.
      () => {},
    );
    return () => {
      live = false;
    };
  }, []);
  const iconOf = (t: { id: string; path: string }) => icons.get(t.id) ?? kept[originOf(t.path)];
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

  /**
   * A tab's own menu: **置顶, what this column has of its own, then 删除
   * last behind a line** (2026-09-28 audit 第 16 条, Apple HIG; until then
   * docs/SHELL.md 第二轮's 「头两项永远是置顶、删除」).
   *
   * ⚠️ 墓碑 (2026-10-02): the last item was 删除, asked first since 第四轮
   * (`tab_remove_confirm`), beside a × that never asked. The × went to
   * this menu (one act, one home) and the owner named the act 关闭 — a
   * page is closed, nothing is deleted — so it asks nothing, like
   * Chrome's. A pinned tab is refused by the node; that sentence is the
   * note.
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
      // 删除 last, behind a line (2026-09-28 audit 第 16 条, Apple HIG).
      {
        // **关闭, not 删除** (the owner, 2026-10-02): a page is closed, like
        // a tab in the session switcher — nothing is deleted, so nothing
        // is asked first and the item is not drawn as destructive.
        key: "close",
        label: gui.close,
        note: refused.get(k("close")),
        run: () => answered("close")(tabs.close(t.id)),
      },
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
    const host = hostOf(t.path);
    return {
      id: t.id,
      group: t.pinned ? "pin" : "",
      pinned: t.pinned,
      face: { machine: d?.face ?? null },
      // **The same ring a 忙碌 session wears** (docs/SHELL.md 第十二轮:
      // 「用同一个光晕…不新造一种转圈」) — one state word, one look,
      // and the reduced-motion guard comes with it (`app.css`). Only
      // this row: the strip's own tab is 80 wide and has no room for a
      // ring around its face (第十二轮 第 3 条).
      halo: load.busy(t.id) ? "busy" : undefined,
      // The machine's face with the site's icon on it, as a session row
      // wears its kind (`FaceMark`, on the face or its corner by 外观).
      mark: markOf(iconOf(t)),
      // **The page's title over its host** (F2.4; N16, W22): one line
      // where the two would say the same thing.
      title: (
        <span data-tab-label data-tab-name className="block truncate">
          {t.title || host || label}
        </span>
      ),
      name: t.title || host || label,
      sub: t.title && host && t.title !== host ? <PathCut text={host} /> : undefined,
      // What the row shows — its name and its place — and nothing else.
      find: `${label} ${t.title} ${t.path}`,
      selected: on,
      menu: menuOf(t),
      onOpen: () => {
        tabs.focus(t.id);
        opened();
      },
      // ⚠️ 墓碑 (the redesign, 2026-10-02, the owner: one act, one home):
      // an unpinned row carried a ×, a pinned one an empty seat. Closing a
      // page is the row's menu's 删除 — right-click, or a long press —
      // like every other list's row acts.
      rowData: { "data-switch-tab": t.id, "data-on": on, "data-pinned": t.pinned },
      openData: { "data-switch-open": t.id },
    };
  });

  /** The open page's machine: chosen first, then the address (the
      owner: 「点击框，弹出可选机器，选择一个机器…然后，用户再输入地址」).
      Choosing another on an open page reloads the same address through
      it — the body is keyed by the machine, so it opens again. */
  const field = (
    <PageField
      key={tabs.current.id}
      tab={tabs.current.id}
      machine={machine}
      devices={devices}
      at={tabs.current.path}
      onGo={(place) => machine && tabs.goTo(machine.id, place)}
      onMachine={(d) => tabs.goTo(d.id, tabs.current.path, tabs.current.title)}
    />
  );

  /** Whether the page can step back and forward, as the iOS page says
      (`page:steps`); `null` where nothing says, and both are then drawn
      lit as they always were. */
  const [steps, setSteps] = useState<{ back: boolean; forward: boolean } | null>(null);
  useEffect(() => {
    setSteps(null);
    return onPageWord("page:steps", tabs.current.id, (t) => setSteps({ back: t[0] === "1", forward: t[1] === "1" }));
  }, [tabs.current.id]);
  /** **The row folds while the page is read** (round UX-3 §3): down past
      the fold's threshold folds it, a scroll up by a hand's width or the
      top of the page brings it back — the phone page's `page:scroll`. */
  const [folded, setFolded] = useState(false);
  /** The strip pressed holds the deck through the flick (`UnfoldHold`). */
  const hold = useRef(new UnfoldHold());
  /** **The deck's light is the row's** (UX-22): one rule, held across a
      load's hops and bounded by the ceiling (`use-loading`). ⚠️ 墓碑: its
      own state, set on every `page:load` and with no ceiling — a load
      that started and never finished pulsed forever. */
  const pageLoading = load.busy(tabs.current.id);
  /** **Why the page could not be opened** (UX-22), until the next load
      starts: said in the pill (narrow) or the caption (wide), with 重试. */
  const [failed, setFailed] = useState<PageFail | null>(null);
  /** **What the page's download did** (UX-42 §16): never silent — said
      where the caption stands until the next load or the next word. */
  const [said, setSaid] = useState<PageSaid | null>(null);
  const [finder, setFinder] = useState(false);
  useEffect(() => void revealHere().then(setFinder), []);
  /** **The folded capsule's ink, from the page's own ground** (UX-29):
      the phone's native page is asked what it paints under the capsule
      (`pageGround`, as the fold happens and a few times a second as the
      page scrolls on) and answers `page:ground`; the DOM cannot see into
      it. Dark ink until the page says otherwise (a white page). */
  const [ink, setInk] = useState<DeckInk>("dark");
  const askedAt = useRef(0);
  const askGround = () => {
    const bar = document.querySelector<HTMLElement>("[data-session-bar][data-native-deck]");
    const cap = bar?.querySelector<HTMLElement>("[data-deck-strip]");
    if (!bar || !cap) return;
    const c = cap.getBoundingClientRect();
    void pageGround(tabs.current.id, bar.getBoundingClientRect().bottom - (c.top + c.height / 2)).catch(() => {});
  };
  useEffect(() => {
    setFolded(false);
    setFailed(null);
    setSaid(null);
    setInk("dark");
    const id = tabs.current.id;
    // The one deck rule (UX-13, `lib/deck-fold`): a page rests at its top,
    // and the top's first hand's width still counts as rest.
    const memo: FoldMemo = { turned: null };
    const offScroll = onPageWord("page:scroll", id, (t) => {
      const [y, way] = t.split(",");
      const at = Number(y);
      if (!Number.isFinite(at)) return;
      const next = foldStep(memo, at, way === "down", FOLD_AT);
      if (!hold.current.allows() && next === true) return;
      if (next !== null) setFolded(next);
      const now = performance.now();
      if (now - askedAt.current > 250) {
        askedAt.current = now;
        // After the fold's own render, so the capsule is there to measure.
        requestAnimationFrame(askGround);
      }
    });
    const offGround = onPageWord("page:ground", id, (t) => setInk(inkOver(t)));
    const offLoad = onPageWord("page:load", id, (t) => {
      // A page starting to load is a page at its top: the deck unfolds
      // (measured on the simulator: a new address typed into a folded
      // tab arrived folded, its first scroll word lost to the editing).
      if (t === "started") {
        setFailed(null);
        setSaid(null);
        setFolded(false);
      }
    });
    const offFail = onPageWord("page:fail", id, (t) => setFailed(pageFail(t)));
    const offDown = onPageWord("page:download", id, (t) => setSaid(pageDownload(t)));
    // Arrived somewhere: whatever failed before is over (UX-38 §1; a load
    // quicker than the phone's poll used to leave the word standing).
    const offWent = onPageWord("page:went", id, () => setFailed(null));
    return () => {
      offScroll();
      offGround();
      offLoad();
      offFail();
      offDown();
      offWent();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tabs.current.id]);
  /** Whether this tab has somewhere to be: its ← → and reload are drawn
      then, and the editor is not needed to start. */
  const somewhere = !!(machine && tabs.current.device && tabs.current.path);
  /** The failure, where the caption stands — the pill's end narrow, the
      tabs row's end wide — never silence (UX-22). 重试 goes where the load
      failed to go. */
  const failedWord = failed && (
    <span data-page-failed className="flex min-w-0 items-center gap-in">
      <span data-page-failed-word title={failed.url || undefined} className="min-w-0 truncate text-caption text-muted-foreground">
        {/* The phone's pill has room for the reason and 重试, not the
            whole sentence (measured: 「打不开：连接被拒绝」 is cut to
            「打不开：…」 there). */}
        {narrow ? failed.reason : gui.page_failed(failed.reason)}
      </span>
      {/* An untrusted certificate is passed, not retried (UX-42 §14):
          the same seat, so the phone's native deck draws and presses it
          as 重试 (`native-deck` SEATS). */}
      <button
        type="button"
        data-page-retry
        data-page-pass={failed.pass ? "" : undefined}
        onClick={() => {
          setFailed(null);
          void pageStep(tabs.current.id, failed.pass ? "pass" : "retry").catch(() => {});
        }}
        className="grid min-h-touch flex-none place-items-center px-in text-caption font-label text-brand-strong"
      >
        {failed.pass ? gui.page_pass : gui.region_retry}
      </button>
    </span>
  );
  /** The download's sentence, in the same seat (UX-42 §16); 在访达中显示
      where Finder is in front of the person. */
  const saidWord = said && (
    <span data-page-said className="flex min-w-0 items-center gap-in">
      <span data-page-said-word title={said.reveal} className="min-w-0 truncate text-caption text-muted-foreground">
        {said.text}
      </span>
      {said.reveal && finder && (
        <button
          type="button"
          data-page-reveal
          onClick={() => void reveal(said.reveal ?? "").catch(() => {})}
          className="grid min-h-touch flex-none place-items-center px-in text-caption font-label text-brand-strong"
        >
          {gui.chat_file_reveal}
        </button>
      )}
    </span>
  );
  /** **The pill is the address editor** (UX-18, the owner 2026-10-05:
      one deck, no second row): a tap on the pill turns it into the
      machine's face and the address, Go on the keyboard, 取消 beside it.
      A fresh tab opens with the pill already editing. */
  const [editing, setEditing] = useState(!somewhere);
  useEffect(() => setEditing(!somewhere), [tabs.current.id]); // eslint-disable-line react-hooks/exhaustive-deps
  /** **The machine defaults to the last one used** (UX-18): a fresh tab
      with no machine takes it, once, if it is still reachable. */
  const defaulted = useRef<string | null>(null);
  useEffect(() => {
    if (!narrow || !tabs.ready || tabs.current.device || defaulted.current === tabs.current.id) return;
    defaulted.current = tabs.current.id;
    const last = lastPageMachine();
    if (last && reachable(devices).some((d) => d.id === last)) tabs.goTo(last, "", "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [narrow, tabs.ready, tabs.current.id, tabs.current.device]);
  /** **On an iPhone the deck is drawn natively over the page** (UX-13,
      `lib/native-deck`): read from the DOM deck after every change to it
      and handed to the page, put away while the pill edits the address
      (the DOM deck is the deck then) or where there is no page. A press
      on it comes back as the DOM control it stands for. */
  const native = nativeDeckHere && narrow && tabs.ready && somewhere && !editing && !bare;
  const nativeTab = tabs.current.id;
  /** **The DOM deck's own layout is read again when it changes** (UX-38
      §4): 取消 put the editor away while the keyboard was still up, the
      deck was `display: none` under it (`app.css`, the deck steps aside
      for the keyboard), the reading came back empty, and nothing read it
      again when the keyboard went — the native deck was gone. Any change
      of the deck's box is a reading now. */
  const [laid, setLaid] = useState(0);
  useEffect(() => {
    if (!native) return;
    const bar = document.querySelector<HTMLElement>("[data-session-bar]");
    const deck = bar?.querySelector<HTMLElement>("[data-session-deck]");
    if (!bar) return;
    let first = true;
    const ro = new ResizeObserver(() => {
      // The observer's first call is the box as it already is.
      if (first) first = false;
      else setLaid((n) => n + 1);
    });
    ro.observe(bar);
    if (deck) ro.observe(deck);
    return () => ro.disconnect();
  }, [native, nativeTab]);
  const nativeKey = `${folded}|${ink}|${pageLoading}|${failed?.reason ?? ""}|${said?.text ?? ""}|${JSON.stringify(steps)}|${tabs.current.title}|${tabs.current.path}|${tabs.tabs.length}|${laid}`;
  useEffect(() => {
    if (!nativeDeckHere || !narrow) return;
    if (!native) {
      void pageDeck(nativeTab, null).catch(() => {});
      return;
    }
    let gone = false;
    // After the layout this render made, and after the face's own glyphs
    // have their size.
    const frame = requestAnimationFrame(() => {
      const screen = document.querySelector<HTMLElement>("[data-session-screen]");
      if (!screen) return;
      void readDeck(screen).then((d) => {
        if (!gone && d) void pageDeck(nativeTab, d).catch(() => {});
      });
    });
    return () => {
      gone = true;
      cancelAnimationFrame(frame);
    };
  }, [native, nativeTab, nativeKey, narrow]);
  useEffect(() => {
    if (!native) return;
    return onPageWord("page:deck", nativeTab, (t) => {
      const screen = document.querySelector<HTMLElement>("[data-session-screen]");
      if (!screen) return;
      if (t.startsWith("swipe:")) {
        const by = Number(t.slice("swipe:".length));
        const at = tabs.tabs.findIndex((x) => x.id === nativeTab);
        const next = tabs.tabs[at + by];
        if (next) tabs.focus(next.id);
        return;
      }
      seatControl(screen, Number(t))?.click();
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [native, nativeTab]);
  const pillEditor = (
    <PageField
      key={tabs.current.id}
      tab={tabs.current.id}
      machine={machine}
      devices={devices}
      at={tabs.current.path}
      steps={false}
      inPill
      autoFocus={editing}
      // The keyboard put away with the caret out of the field ends the
      // editing, as Safari's does — but not a press on the machine's face
      // or its picker, which takes the focus on its way to the menu.
      onLeave={() =>
        window.setTimeout(() => {
          if (document.activeElement?.closest("[data-deck-editing], [data-page-machines]")) return;
          if (document.querySelector("[data-page-machines]")) return;
          setEditing(false);
        }, 150)
      }
      onGo={(place) => {
        setEditing(false);
        if (machine) {
          rememberPageMachine(machine.id);
          tabs.goTo(machine.id, place);
        }
      }}
      onMachine={(d) => {
        rememberPageMachine(d.id);
        tabs.goTo(d.id, tabs.current.path, tabs.current.title);
      }}
    />
  );

  const content =
    machine && tabs.current.device && tabs.current.path ? (
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
          // draws between the two (`BrowseTabs`) — and in this one's
          // session, so it is in the same 附栏 slice (UX-42 §15).
          onPopup={(place) => tabs.open(machine.id, place, { session: tabs.current.session, after: tabs.current.id })}
          onLanded={(place) => {
            // **The visit is recorded here, once, for every kind.** The
            // ledger is the node's (`khor_sync::recents`); fire and forget.
            // A tab opened from a session carries it into the record, so
            // the page is in that session's 附栏 slice (UX-37 §1).
            recentVisit(kind.kind, machine.name, place, tabs.current.session || undefined).catch(() => {});
          }}
        />
      </div>
    ) : (
      // ⚠️ 墓碑 (the redesign): a page with no address showed the new-tab
      // page — the places visited and the machines to start on. The
      // machine is chosen in the field now, the one way in; the page is
      // empty until there is something to show.
      <div data-page-empty className="h-full" />
    );

  const tabList = (
    <div data-tab-switcher className="h-full min-h-0 overflow-y-auto">
      <SideList
        rows={side}
        query={query}
        label={groupLabel}
        // **Nothing open is a state of its own** (the redesign: no page is
        // made for an empty landing any more, `useBrowseTabs`): one quiet
        // sentence, and where the way to start is — the ＋ above, as the
        // sessions' and the workspaces' columns say it.
        empty={<span data-page-none>{gui.page_none}</span>}
        emptyHow={gui.page_none_how}
        none={gui.no_matches}
      />
    </div>
  );

  /**
   * **The column's head, the one every sidebar wears** (docs/SHELL.md
   * 第二轮): the landing's name, ＋ for a new page, and search.
   */
  const head = (
    <PaneBar
      seat="side"
      title={gui.browser_tab}
      // ⌘T: a new page is a new tab (`lib/shortcuts`).
      newIsTab
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
          opened();
        },
      }}
    />
  );

  /** **The tabs as the session page draws them** (the owner: 「浏览器页做
      成标签页逻辑算了，挪用 session 的即可」): a page is one more kind of tab
      face — its machine's face with the browser's mark. */
  const faces: TabFace[] = tabs.tabs.map((t) => {
    const d = deviceOf(t.device);
    return {
      id: t.id,
      // **The pill writes the host** (UX-35 §4), on the phone's deck and
      // the wide strip alike — one pill, one word; the page's own title
      // is the tabs sheet's row and the hover title (`title`). The
      // address editor still opens on the whole address (UX-18). ⚠️ 墓碑:
      // the title, else the address without its scheme.
      label: pillHost(t.path) || tabLabel(kind, t.path, d),
      title: t.title || undefined,
      face: d?.face ?? null,
      path: t.path,
      machine: d?.shown ?? null,
      mark: markOf(iconOf(t)),
      menu: menuOf(t),
      // **The third seat** (UX-33): reload, or × while the page loads.
      act: load.busy(t.id)
        ? { kind: "stop" as const, label: gui.page_stop, onPress: () => void pageStep(t.id, "stop").catch(() => {}) }
        : { kind: "reload" as const, label: gui.page_reload, onPress: () => void pageStep(t.id, "reload").catch(() => {}) },
      loading: load.busy(t.id),
    };
  });

  // **Nothing is drawn against the stand-in tab** (`useBrowseTabs`'s
  // `ready`): everything here is keyed by the open tab's id, and a face
  // built over the stand-in would be torn down when the real one lands.
  const body = tabs.ready ? content : <div data-browse-waiting className="h-full" />;

  // **Narrow: the session page's layout** — the list is the first screen;
  // a page is the next, its content over the whole top, and at its foot
  // the session bar: ‹, the tabs, the count; under them the address
  // field, in the seat a conversation's box takes, riding the keyboard.
  // The last page closed under the page screen (from the terminal, or
  // another window, between two beats): back to the list, which says so.
  // Only on the way from some to none — ＋ on an empty list opens the
  // page screen a round trip before its page exists.
  const had = useRef(false);
  const count = tabs.tabs.length;
  useEffect(() => {
    if (count > 0) {
      had.current = true;
      return;
    }
    if (!tabs.empty || !had.current) return;
    had.current = false;
    if (narrow && screen === "detail") onBack?.();
  }, [count, tabs.empty, narrow, screen, onBack]);
  /** In the 附栏: this session's pages (`bare.here`), and a new page is
      this session's, its address field up at once (UX-40 §1). */
  const mine = bare ? new Set(tabs.tabs.filter((t) => t.session === bare.here).map((t) => t.id)) : null;
  const newHere = () => {
    if (!bare) {
      tabs.add();
      opened();
      return;
    }
    void openTab(kind.kind, machine?.id ?? "", "", bare.here)
      .then((id) => {
        setQuery("");
        tabs.focus(id);
        setEditing(true);
        opened();
      })
      .catch(() => {});
  };
  if (narrow && mine && !(screen === "detail" && !tabs.empty)) {
    return (
      <section data-browse-tabs data-browse-bare="here" className="flex h-full min-h-0 flex-col">
        {/* The 附栏's head is the seat's title (`SideBody`); this list has
            none of its own, only the find row (UX-40 §1). ⚠️ 墓碑 (UX-37
            §1 → UX-39 §3): ‹ to 全部网页. */}
        <SideFind
          label={gui.search_tabs}
          query={query}
          onQuery={setQuery}
          plus={
            <GlassCircle aria-label={gui.new_tab} data-side-new onClick={newHere}>
              <IconPlus />
            </GlassCircle>
          }
        />
        <div data-tab-switcher data-nav-clear className="min-h-0 flex-1 overflow-y-auto">
          <SideList
            query={query}
            rows={side.filter((r) => mine.has(r.id))}
            label={groupLabel}
            empty={<span data-page-none>{gui.side_pages_here_none}</span>}
            none={gui.no_matches}
          />
        </div>
      </section>
    );
  }
  if (narrow) {
    // **A page in the 附栏 wears the main area's deck** (UX-40 §4, the
    // owner 2026-10-07 02:10): the narrow screen below, its tabs this
    // session's, never folded — the sheet is small already.
    return screen === "detail" && !tabs.empty ? (
      <SessionScreen
        faces={mine ? faces.filter((f) => mine.has(f.id)) : faces}
        active={tabs.current.id}
        onFocus={(id) => tabs.focus(id)}
        onClose={(id) => {
          if (tabs.tabs.length === 1) onBack?.();
          void tabs.close(id).catch(() => {});
        }}
        // ＋ in the tabs sheet: a new page, as the list's ＋ opens one.
        onNew={newHere}
        onBack={() => onBack?.()}
        onTitle={() => setEditing(true)}
        caption={failedWord || saidWord || caption}
        editor={tabs.ready && editing ? pillEditor : undefined}
        nativeDeck={native}
        // The page is a native view the capsule cannot float over: it
        // keeps its own room under the page (UX-29); on the phone the
        // native deck draws it over the page from this layout.
        foldOver={false}
        ink={native ? ink : undefined}
        onEditCancel={() => setEditing(false)}
        folded={folded && !editing && !bare}
        onUnfold={() => {
          hold.current.asked();
          setFolded(false);
        }}
        loading={pageLoading}
      >
        <section data-browse-landing={kind.kind} data-browse-bare={bare ? "page" : undefined} className="flex h-full min-h-0 flex-col">
          <div data-nav-clear={bare ? true : undefined} className="min-h-0 flex-1 overflow-hidden">
            {body}
          </div>
          {/* **The page's own things are the pill's** (round UX-3): ← → at
              its left, reload at its right; the address is edited under
              the deck only when asked for (the title pressed, or a page
              with nowhere to be yet), riding the keyboard. */}
          {tabs.ready && somewhere && (
            <>
              <BarSeat at="pill">
                {/* **← grey when there is nowhere back, → only when there is
                    somewhere ahead** (round UX-3: the one agreed exception to
                    「不灰显」, so the pair never jumps sideways when the first
                    step back is taken). */}
                <span data-deck-history className="flex items-center">
                  <GlassItem
                    aria-label={gui.page_back}
                    aria-disabled={steps?.back === false || undefined}
                    data-page-step="back"
                    className="aria-disabled:pointer-events-none aria-disabled:text-tertiary"
                    onClick={() => void pageStep(tabs.current.id, "back").catch(() => {})}
                  >
                    <IconArrow to="left" />
                  </GlassItem>
                  {steps?.forward !== false && (
                    <GlassItem aria-label={gui.page_forward} data-page-step="forward" onClick={() => void pageStep(tabs.current.id, "forward").catch(() => {})}>
                      <IconArrow to="right" />
                    </GlassItem>
                  )}
                </span>
              </BarSeat>
              <BarSeat at="lead">
                <GlassItem aria-label={gui.page_reload} data-page-step="reload" onClick={() => void pageStep(tabs.current.id, "reload").catch(() => {})}>
                  <IconReload />
                </GlassItem>
              </BarSeat>
            </>
          )}
        </section>
      </SessionScreen>
    ) : (
      <ListPane mark="data-browse-tabs" name={kind.words.segment} pane={null} narrow seat="side" head={head} scroll={false}>
        {tabList}
      </ListPane>
    );
  }

  // **Wide: the session landing's arrangement** — the list column, and
  // over the content the tabs capsule; under it one row with the address
  // field, then the page.
  return (
    <section data-browse-landing={kind.kind} className="flex h-full min-h-0 min-w-0 flex-col">
      <ListPane
        mark="data-browse-tabs"
        name={kind.words.segment}
        pane={pane}
        narrow={false}
        seam={{ "data-list-seam": "" }}
        seat="side"
        head={head}
        scroll={false}
      >
        {tabList}
      </ListPane>
      {tabs.ready ? (
        <Tabs
          glass
          faces={faces}
          active={tabs.current.id}
          onFocus={(id) => tabs.focus(id)}
          onClose={(id) => void tabs.close(id).catch(() => {})}
          stripData={{ "data-page-tabs": "" }}
          caption={failedWord || saidWord || caption}
          bodyClass="flex flex-col"
        >
          <div data-page-field-seat className="flex flex-none px-page pt-xxs pb-in">
            {field}
          </div>
          <div className="min-h-0 flex-1 overflow-hidden">{body}</div>
        </Tabs>
      ) : tabs.empty ? (
        // Nothing open: the column says so; the content stays empty, with
        // no tabs capsule and no field to aim at nothing.
        <div data-page-none-body className="h-full" />
      ) : (
        <div data-browse-waiting className="h-full" />
      )}
    </section>
  );
}
