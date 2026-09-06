import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import {
  fetchDevices,
  fetchHooks,
  fetchSessions,
  fetchQuota,
  openTermOn,
  installHooks,
  closeSession,
  forgetDevice,
  markSeen,
  pinDevice,
  pinSession,
  uninstallHooks,
  type DeviceRow,
  type HooksState,
  type SessionRow,
  type QuotaAnswer,
} from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import {
  IconBrowser,
  IconDevices,
  IconFiles,
  IconMore,
  IconQuota,
  IconSessions,
} from "@/components/icons";
import { type OmniAxis } from "@/components/Omnibox";
import { PaneBar, type PaneAction } from "@/components/PaneBar";
import { StatusBar } from "@/components/StatusBar";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { BrowseLanding } from "@/browse/BrowseLanding";
import { TabCount, TabStrip, TabSwitcher, type TabFace } from "@/browse/TabStrip";
import { useBrowseTabs } from "@/browse/use-browse-tabs";
import { gui } from "@/gen/catalog";
import { useDayStrips, useStripWidth } from "@/hooks/use-day-strips";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";
import { useListWidth } from "@/hooks/use-list-width";
import { useSessionTabs } from "@/hooks/use-session-tabs";
import { useNarrow } from "@/hooks/use-narrow";
import { useStatusBar } from "@/hooks/use-status-bar";
import { cn } from "@/lib/utils";
import { axisOf, groupLabel, placeLabels, valueOf, word } from "@/words";
import { DetailPane } from "@/views/DetailPane";
import { FILES } from "@/views/FilesKind";
import { PAGES } from "@/views/PagesKind";
import { DevicesList } from "@/views/DevicesList";
import { FaceSettings } from "@/views/FaceSettings";
import { MachineCard } from "@/views/MachineCard";
import { MandalaMap } from "@/views/MandalaMap";
import { MeshNow } from "@/views/MeshNow";
import { MarkPlace } from "@/views/MarkPlace";
import { InviteDialog, JoinDialog } from "@/views/PairDialogs";
import { QuotaLanding } from "@/views/QuotaLanding";
import { SessionsList } from "@/views/SessionsList";
import { NewSessionDialog } from "@/views/NewSessionDialog";
import { TellDialog } from "@/views/TellDialog";

/**
 * The four landings, all present from the first day.
 *
 * Every one begins with the same question the devices pane answers —
 * which machine — and grows its own second step from there: a machine's
 * card, its disk, or its network. Files opens a directory; browser
 * borrows the network and opens a page through it (docs/NET.md 借网).
 */
type Landing = "sessions" | "devices" | "files" | "browser" | "quota";

/**
 * The app's mark opens a place of its own: the whole mesh, and what the
 * agents on it have cost.
 *
 * **Not a fifth `Landing`, and the difference is structural rather than
 * cosmetic.** A landing is a list beside a detail — that is what the
 * shell is built out of, and what the rail's four glyphs each open. This
 * one is neither half of that pair: there is no list of things to pick
 * from, because the thing it shows is the network itself. So it replaces
 * the list-and-detail pair rather than filling it, and the state that
 * says so is separate from the landing rather than another value of it.
 * Coming back from it restores whichever landing was open, which is why
 * that value is never overwritten.
 */
type Mark = boolean;

/** Which of the "+" dialogs is up, if any. */
type Sheet = "new" | "tell" | "invite" | "join" | null;

const POLL_MS = 2000;
/** How often the subscription's arcs are re-asked while a surface that
    can show them is open. **Named for the spending answer it used to
    share a beat with**, which left on 2026-08-29 with the 今天 line —
    kept rather than renamed on its own, because a constant's name is
    not worth a line of diff in a file five people edit; the next thing
    to touch this effect should rename it. Not the rows' beat: see where
    it is used. */
const USAGE_POLL_MS = 10_000;

/**
 * How to arrange the session list. The keys are the node's
 * (`khor_node::list::Arrange`) and the CLI's `--by` values — spelled
 * once there, echoed here, so the two faces cannot drift into different
 * vocabularies for one setting.
 */
const ARRANGE = [
  { key: "recent", label: gui.by_recent },
  { key: "category", label: gui.by_category },
  { key: "device", label: gui.by_device },
  { key: "state", label: gui.by_state },
  { key: "place", label: gui.by_place },
] as const;

/** WeChat's opening: the thing you touched last is the thing you want. */
const ARRANGE_DEFAULT = "recent";
const ARRANGE_KEY = "khor.sessions.arrange";

/**
 * The chosen arrangement, remembered on this device.
 *
 * **Local on purpose, unlike a pin.** A pin is a property of the session
 * it is stuck to, so it travels the network; how one screen sorts is
 * that screen's posture, and a phone held in a queue does not want the
 * desktop's answer (docs/handoff 置顶与分类 — the same split that keeps
 * collapse state local).
 *
 * An unreadable or unknown stored value falls back to the default rather
 * than being passed on: the backend refuses a mode it does not know, so
 * a stale key would leave the list empty with an error nobody asked for.
 */
function storedArrange(): string {
  try {
    const saved = window.localStorage.getItem(ARRANGE_KEY);
    return ARRANGE.some((a) => a.key === saved) ? saved! : ARRANGE_DEFAULT;
  } catch {
    return ARRANGE_DEFAULT;
  }
}

/**
 * The ticked filter words, remembered on this device — **one entry per
 * pane**, which is why the key carries the landing.
 *
 * Same mechanism and same reasoning as the arrangement above: a
 * preference the user set once should still be set the next time
 * (docs/handoff — 所有偏好保留一次设置). Only the sessions pane can
 * filter today, and the shape is per pane anyway because the alternative
 * is one pane silently inheriting another's ticks the day a second pane
 * grows a filter.
 *
 * **Not validated against a list of known words.** This layer does not
 * get to say which states exist (docs/UX.md 状态呈现) — the options come
 * from what the node sent — so all that can be checked here is the
 * shape. A key that no longer means anything simply matches no rows,
 * which the pane says out loud (`no_matches`) with the filter control
 * lit, and one tick undoes it.
 */
const FILTER_KEY = "khor.filter";
const filterKey = (landing: Landing) => `${FILTER_KEY}.${landing}`;

function storedWords(landing: Landing): string[] {
  try {
    const raw = window.localStorage.getItem(filterKey(landing));
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) && parsed.every((k) => typeof k === "string")
      ? (parsed as string[])
      : [];
  } catch {
    return [];
  }
}

function storedWordsAll(): Record<Landing, string[]> {
  return {
    sessions: storedWords("sessions"),
    devices: storedWords("devices"),
    files: storedWords("files"),
    browser: storedWords("browser"),
    quota: storedWords("quota"),
  };
}

// In the order the user set on 2026-09-02 —「分别是会话、机器、文件、
// 用量、浏览器」— which is also the fold order: the narrow bar shows
// the first `NARROW_SHOWN` and folds the rest behind「…」.
const LANDINGS: { key: Landing; name: string; glyph: ReactNode }[] = [
  { key: "sessions", name: gui.sessions_tab, glyph: <IconSessions /> },
  { key: "devices", name: gui.devices_tab, glyph: <IconDevices /> },
  { key: "files", name: gui.files_tab, glyph: <IconFiles /> },
  // 额度: the registered API accounts as rings (`QuotaLanding`). Like
  // the browsing landings it is one region rather than a list beside a
  // detail — its list is its own, and the rail only opens it.
  { key: "quota", name: gui.accounts_tab, glyph: <IconQuota /> },
  { key: "browser", name: gui.browser_tab, glyph: <IconBrowser /> },
];

/** How many landings the narrow bar seats before folding the rest
    (2026-09-02:「窄屏的时候,下面的侧边栏最多只显示四个icon,多余的
    折叠为...」). Four glyphs, the fold and this machine's face sit a
    390 bar comfortably; the wide rail is a column and folds nothing. */
const NARROW_SHOWN = 4;

const NO_QUERIES: Record<Landing, string> = {
  sessions: "",
  devices: "",
  quota: "",
  files: "",
  browser: "",
};

function RailItem({
  label,
  tab,
  on,
  narrow,
  badge,
  onClick,
  className,
  children,
}: {
  label: string;
  /** Which landing this glyph opens, when it opens one. */
  tab?: Landing;
  on?: boolean;
  narrow: boolean;
  badge?: number;
  onClick?: () => void;
  /**
   * Extra styling for one item. **Exists for the mark**, which is not a
   * fifth landing and therefore has to sit a group's distance from the
   * four that are. It is a margin rather than a gap because the gap
   * between rail items belongs to the nav, and one item cannot change
   * it for itself.
   *
   * It used to exist for the opposite kind of reason: the mark's glyph
   * is a picture, the selected look was a text colour, and a text colour
   * tints a drawn glyph while doing nothing at all to a picture — so
   * that one item said "I am open" its own way, and ended up the only
   * item in the rail with a block behind it while being the only one
   * that is not a place to go. Every item wears the block now
   * (`app.css`), so that reason is gone and this carries only spacing.
   */
  className?: string;
  children: React.ReactNode;
}) {
  const button = (
    <Button
      variant="ghost"
      aria-label={label}
      data-rail-item
      data-landing={tab}
      data-on={on}
      onClick={onClick}
      className={cn(
        // Two deliberate choices in one string, both measured.
        //
        // **`px-2 py-2`, not the spacing tokens** — the one place in
        // this rail that does not speak in them. `cn()` is
        // tailwind-merge, and tailwind-merge does not read `in` as a
        // padding, so `p-in` does not displace the vendored button's own
        // `px-4`: both survive into the markup and the winner is
        // whichever the stylesheet emitted last. Cost when that was
        // tried here: every rail item went from 44 wide to 60, and the
        // six of them no longer fit a phone's rail.
        //
        // **`shrink-0`** — what a rail out of room does about it. A row
        // that overflows shrinks its items, and this row's items hold a
        // button's padding but no text, so the first thing to give is
        // the mark's `<img>`: it went to **width 0** while the rail
        // reported no overflow at all, which is a missing app icon that
        // nothing on screen and no layout inspection would report.
        // (`flex-none` on the image itself was tried and is not the fix:
        // it holds the height, because height is the column button's
        // main axis, and the width still went to nothing.) With this,
        // the same squeeze overflows the rail visibly instead —
        // measured, and both outcomes are asserted in
        // `scripts/check-rail.mjs`.
        //
        // (Numbers here carry no unit on purpose: `check-px.mjs` reads
        // lines rather than syntax, so a number-and-unit is a build
        // error even inside a comment.)
        // **`min-h-touch min-w-touch`** — the finger floor (tokens.css
        // `--touch`): the hit area grows to 44, the glyph keeps its
        // `--icon-rail` size, and because the selected block and the
        // hover wash paint this very button box, the highlight is the
        // hit area — nothing is pressable that does not light up whole.
        // Measured before this: every rail item was 44×30, under every
        // guideline's floor (`check-face.mjs` now holds the line).
        "group relative h-auto min-h-touch min-w-touch flex-col justify-center gap-0 rounded-md px-2 py-2 shrink-0 text-rail-ink",
        // **Pointing at an item paints the ground, never the glyph.**
        // The `ghost` variant carries `hover:text-foreground`, which on
        // this rail made a hovered *resting* item heavier than the open
        // one — 12.20 against 10.01 (`check-rail` caught it the hour
        // the open glyph stopped being full ink). A person dragging the
        // pointer down the rail would have been shown a different
        // answer to 我在哪一页 under every item they passed.
        //
        // Said as a `hover:` of its own because it has to be: a
        // pseudo-class outranks a plain utility however the order runs
        // — the same rule that made `focus:` need saying twice in
        // `RowMenu`.
        "hover:text-rail-ink",
        // **Open takes the brand's hue, and since 2026-08-31 the glyph
        // is the whole of what says open** — the block behind it is
        // gone (`app.css` carries the tombstone and the numbers).
        //
        // It was full ink until 2026-09-01, on a measurement that was
        // right about one colour and wrong about the family: `--lq3` on
        // paper is 2.22, under the 3.0 SC 1.4.11 asks of a graphic — so
        // *that* green is a plate and not a mark. **A green deep enough
        // is a mark**, and the token carries one per ground for exactly
        // that reason (`--rail-on-ink`). The other half of the signal
        // is the body filling solid, one rule away in the same file.
        on && "text-[var(--rail-on-ink)] hover:text-[var(--rail-on-ink)]",
        className,
      )}
    >
      {children}
      {badge !== undefined && badge > 0 && (
        <span className="absolute right-1 top-1 min-w-4 rounded-full bg-badge px-1 text-center text-xs leading-4 tabular-nums text-badge-foreground">
          {badge}
        </span>
      )}
    </Button>
  );
  // The name shows itself the instant you point at it — the whole reason
  // this is Radix and not the OS title with its 1.5s wait. Narrow has no
  // pointer to hover with; the glyphs stand alone there, and the
  // aria-label still names them for anyone listening.
  return narrow ? (
    button
  ) : (
    <Tooltip>
      <TooltipTrigger asChild>{button}</TooltipTrigger>
      <TooltipContent side="right" data-rail-tip>
        {label}
      </TooltipContent>
    </Tooltip>
  );
}

export default function App() {
  const narrow = useNarrow();
  // **The breakpoint, published once.** `use-narrow` owns the number
  // and stylesheets need it too (the machine card's part rows drop
  // their shared grid narrow) — so the shell stamps the answer on the
  // root and CSS keys off the attribute, rather than a `@media`
  // repeating a number that can drift with nothing to catch it. The px
  // gate refuses the second copy out loud.
  useEffect(() => {
    const root = document.documentElement;
    if (narrow) root.setAttribute("data-narrow", "");
    else root.removeAttribute("data-narrow");
  }, [narrow]);
  const listPane = useListWidth(narrow);
  const [landing, setLanding] = useState<Landing>("sessions");
  const [rows, setRows] = useState<SessionRow[]>([]);
  const [devices, setDevices] = useState<DeviceRow[]>([]);
  // The sessions a person has open, as tabs, and which is in front:
  // `selected` is the front tab (`useSessionTabs` — iTerm's arrangement,
  // asked for 2026-09-02). Opening a row opens or focuses its tab;
  // closing a tab is a view leaving, never a session ending.
  const sessionTabs = useSessionTabs();
  const { tabs: openTabs, drop: dropTabs } = sessionTabs;
  const selected = sessionTabs.active;
  // Whether the open tabs are up as a list to pick from (`TabSwitcher`)
  // instead of the front tab's content. The narrow face's only way to
  // another tab — a phone has no strip — and the wide face's answer
  // once the strip has stopped showing every tab (`TabStrip.onSwitch`,
  // which is the side that can measure that). Reset wherever the tabs
  // stop being the thing on screen: another landing, the mark's place,
  // or the last tab closing.
  const [switchingTabs, setSwitchingTabs] = useState(false);
  useEffect(() => {
    if (openTabs.length === 0) setSwitchingTabs(false);
  }, [openTabs.length]);
  /** The open ids that have actually been seen on the list — see the
      effect below for why "open" alone is not enough. */
  const wasListed = useRef<Set<string>>(new Set());
  // The open machine, kept apart from the open session for the same
  // reason each pane keeps its own query: they are two selections in two
  // places, and one variable would have opening a machine close the
  // session you were reading.
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
  // …and what the files landing has open, which is no longer *a*
  // machine at all: it is a strip of tabs, each a machine and a path,
  // kept on this device (`useBrowseTabs`). It lives here rather than
  // inside the landing so that opening a directory from somewhere else
  // — the machine card's 看它的文件 — is a tab appearing rather than a
  // landing being told to remember something on arrival.
  const dirTabs = useBrowseTabs(FILES.kind);
  // …and the browser landing's own tabs. **A second strip, not a
  // second mechanism** (浏览器接壳批): both browsing landings are the
  // same shell with a different kind, and the two stores are kept apart
  // by that kind alone (`useBrowseTabs` keys on it), so a page can
  // never turn up where a directory belongs.
  const pageTabs = useBrowseTabs(PAGES.kind);
  // Narrow face only: which single screen is up. Wide ignores it.
  const [screen, setScreen] = useState<"list" | "detail">("list");
  const [stale, setStale] = useState(false);
  // One query per pane: switching landings is not a way of clearing a
  // search, and a search that follows you across panes filters the wrong
  // list on arrival.
  const [queries, setQueries] = useState<Record<Landing, string>>(NO_QUERIES);
  // Per pane and remembered, unlike the query above: a search box is a
  // thing you are doing right now, a filter is a way you have decided to
  // look at this list.
  const [words, setWords] = useState<Record<Landing, string[]>>(storedWordsAll);
  const [arrangeBy, setArrangeBy] = useState<string>(storedArrange);
  const [sheet, setSheet] = useState<Sheet>(null);
  // Settings sit behind the rail's last glyph, one level down and not on
  // any first screen (docs/UX.md 设置: 升成了也藏深). Its own state
  // rather than another `Sheet`: those three are the "+" menu's, and
  // this one does not belong to any pane.
  const [settings, setSettings] = useState(false);
  // Whether the mark's place is showing. See `Mark`.
  const [mark, setMark] = useState<Mark>(false);
  // **What the agents have cost is no longer asked for here at all.**
  // This used to hold a `usage` answer beside the quota — every day
  // there has ever been, eighteen seconds for the first pass on the
  // machine answering it (`khor_node::usage::Meters::tally`). Its last
  // reader was the network's-now 今天 line, removed 2026-08-29, and a
  // poll nobody reads is not free: it is that tally, on a timer, for a
  // number nothing draws. `khor usage` is where that question lives.
  const [quota, setQuota] = useState<QuotaAnswer | null>(null);

  // This machine's hooks, polled with everything else rather than read
  // once when the card opens: `khor hooks install` in a terminal is the
  // other way this changes, and a button that only learns on mount would
  // sit there offering to do something already done.
  const [hooks, setHooks] = useState<HooksState | null>(null);
  const [hooksFailed, setHooksFailed] = useState(false);

  // Nobody looking, nobody paying: this is three calls every two
  // seconds, and it is the app's whole heartbeat.
  const hidden = useHidden();

  useEffect(() => {
    let live = true;
    const tick = () => {
      Promise.all([fetchSessions(arrangeBy), fetchDevices(), fetchHooks()])
        .then(([s, d, h]) => {
          if (!live) return;
          setRows(s);
          setDevices(d);
          setHooks(h);
          setStale(false);
        })
        .catch(() => live && setStale(true));
    };
    tick();
    const t = setInterval(tick, hidden ? HIDDEN_MS : POLL_MS);
    return () => {
      live = false;
      clearInterval(t);
    };
    // Re-subscribed when the arrangement changes: the new order comes
    // from the node, so switching modes is a fetch, never a re-sort.
    //
    // **And when the window is looked at again**, which is the same
    // shape: this effect already asks once before it sets its interval,
    // so coming back is a fresh answer immediately rather than after a
    // wait — no extra wiring, and nothing to remember to clear.
  }, [arrangeBy, hidden]);

  // The subscription's arcs, on a beat of their own.
  //
  // **One reader now**: the 额度 landing's foot (`QuotaLanding`). They
  // were on both machine surfaces until 2026-09-02 —「机器页面不需要有
  // token usage的环」pulled them off — so the gate narrowed with the
  // readers. The beat still costs the endpoint nothing: one fetch
  // answers for five minutes and the whole machine shares it
  // (`khor_node::quota`), so this reaches a cache and stops.
  useEffect(() => {
    if (landing !== "quota") return;
    let live = true;
    const tick = () => {
      fetchQuota()
        .then((q) => live && setQuota(q))
        .catch(() => {});
    };
    tick();
    // Left alone by the hidden pace on purpose: its beat already **is**
    // the hidden beat, and it only runs while a surface that can show it
    // is open.
    const t = setInterval(tick, USAGE_POLL_MS);
    return () => {
      live = false;
      clearInterval(t);
    };
  }, [mark, landing]);

  const onSelect = useCallback((row: SessionRow) => {
    sessionTabs.open(row.id);
    if (row.unread > 0) {
      // Looking at it is the seen semantics; the watermark replicates,
      // so this clears the badge on every device.
      markSeen(row.id).catch(() => {});
    }
    setScreen("detail");
    // `open` is a stable callback; the object around it is not.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionTabs.open]);

  const onSelectDevice = useCallback((row: DeviceRow) => {
    setSelectedDevice(row.id);
    setScreen("detail");
  }, []);

  const setQuery = useCallback(
    (q: string) => setQueries((was) => ({ ...was, [landing]: q })),
    [landing],
  );

  // Pinning: ask the node, then refresh from it. **The rows are not
  // reordered here** — the order is the node's answer, and guessing it
  // locally would mean the screen briefly shows an order the library
  // never produced (and permanently shows it if the call fails).
  const refresh = useCallback(async () => {
    const [s, d] = await Promise.all([fetchSessions(arrangeBy), fetchDevices()]);
    setRows(s);
    setDevices(d);
  }, [arrangeBy]);

  // Two-argument `then` for the same reason the pins use it: the
  // rejection handler must see *this* call failing and nothing else. And
  // the answer carries the state afterwards, so the button never spends
  // a poll interval showing the job it just finished.
  const onHooks = useCallback((run: () => Promise<HooksState>) => {
    run().then(
      (state) => {
        setHooks(state);
        setHooksFailed(false);
      },
      () => setHooksFailed(true),
    );
  }, []);

  const onInstallHooks = useCallback(() => onHooks(installHooks), [onHooks]);
  const onUninstallHooks = useCallback(() => onHooks(uninstallHooks), [onHooks]);

  /**
   * Rows whose last pin attempt did not take, by row id.
   *
   * **A failed pin used to look exactly like a missed click** — the row
   * simply did not move — and docs/UX.md forbids 做了但没变化 and 失败
   * wearing one face. The race that made this common is fixed at the
   * source (a shared temp filename in the block store, measured at 17 of
   * 40 pins failing before and 0 of 40 after), so what is left is the
   * rare disk error; rare is a reason for the face to be small, not a
   * reason for it to be absent.
   *
   * **It is said on the button that was pressed**, and no notification
   * surface is introduced to carry it. There is nowhere in this app that
   * collects messages, and building one for the least frequent thing
   * that happens here would be a whole mechanism whose first and only
   * customer is a disk error.
   *
   * Session ids and device ids cannot collide (`kind/leaf` against hex),
   * so one set serves both lists.
   */
  const [pinFailed, setPinFailed] = useState<Set<string>>(() => new Set());

  const markPin = useCallback((id: string, failed: boolean) => {
    setPinFailed((was) => {
      if (was.has(id) === failed) return was;
      const next = new Set(was);
      if (failed) next.add(id);
      else next.delete(id);
      return next;
    });
  }, []);

  // Two-argument `then`, not `.then().catch()`: the rejection handler
  // must see the pin call failing and nothing else. Chained, a refresh
  // that threw after a pin that worked would paint the failure face on a
  // pin that actually took.
  //
  // **The returned promise says whether the pin took**, which is what
  // the row's menu reads to decide between closing and staying open
  // (`RowMenu`). Two things about the shape are load-bearing and neither
  // is style: the rejection handler still **rethrows** rather than
  // swallowing, so the failure that painted the face is the same event
  // that keeps the menu open; and the refresh is *not* returned into the
  // chain — awaiting it would put a whole poll between a pin that
  // already worked and the menu closing, and a refresh that failed
  // afterwards would then read as a pin that failed, which is the very
  // confusion the two-argument form exists to prevent.
  const onPinSession = useCallback(
    (row: SessionRow) =>
      pinSession(row.id, !row.pinned).then(
        () => {
          markPin(row.id, false);
          refresh().catch(() => {});
        },
        (e) => {
          markPin(row.id, true);
          throw e;
        },
      ),
    [refresh, markPin],
  );

  /**
   * Ending a session from its row's menu.
   *
   * **The refusal is the point.** Whether a close is possible is
   * `Node::close`'s judgment — a discovered session is not khor's to
   * end — and it comes back as a whole sentence written for a person
   * ("不是 khor 起的,关不了它;去它自己的窗口里退"). Re-deriving that
   * rule here would be this face guessing at a backend judgment and
   * going stale the day one of those paths changes, which is the same
   * reasoning `DetailPane`'s own close carries.
   *
   * Kept by row id, the way `pinFailed` is, and cleared the moment a
   * later attempt on that row gets anywhere: a sentence about a refusal
   * that has since been answered is worse than no sentence.
   */
  const [closeRefused, setCloseRefused] = useState<Map<string, string>>(() => new Map());
  const onCloseSession = useCallback(
    (row: SessionRow) => {
      setCloseRefused((was) => {
        if (!was.has(row.id)) return was;
        const next = new Map(was);
        next.delete(row.id);
        return next;
      });
      return closeSession(row.id).then(
        () => {
          // Nothing to paint: the row leaves the list on the next poll,
          // and the selection guard below takes the pane with it.
          refresh().catch(() => {});
        },
        (e) => {
          setCloseRefused((was) =>
            new Map(was).set(row.id, String(e instanceof Error ? e.message : e)),
          );
          throw e;
        },
      );
    },
    [refresh],
  );

  // The session pin's shape, now that a machine row's pin is a menu
  // item too (2026-09-06): the promise says whether it took, the
  // rejection rethrows so the menu stays open on the row where the
  // press was made, and the refresh is not in the chain.
  const onPinDevice = useCallback(
    (row: DeviceRow) =>
      pinDevice(row.name, !row.pinned).then(
        () => {
          markPin(row.id, false);
          refresh().catch(() => {});
        },
        (e) => {
          markPin(row.id, true);
          throw e;
        },
      ),
    [refresh, markPin],
  );

  /**
   * Showing a machine the door from its row's menu — `khor forget`.
   *
   * The refusal is `Node::forget_device`'s, in its own sentence (this
   * machine cannot forget itself; a name the table does not hold), and
   * it is kept by row the way `closeRefused` is: the menu prints it
   * under the item, and it is cleared the moment a later attempt on
   * that row gets anywhere. Nothing to paint on success — the row
   * leaves the list on the next poll, and if its card was open,
   * `openDevice` falls to null with it and the landing shows the map.
   */
  const [forgetRefused, setForgetRefused] = useState<Map<string, string>>(() => new Map());
  const onForgetDevice = useCallback(
    (row: DeviceRow) => {
      setForgetRefused((was) => {
        if (!was.has(row.id)) return was;
        const next = new Map(was);
        next.delete(row.id);
        return next;
      });
      return forgetDevice(row.name).then(
        () => {
          refresh().catch(() => {});
        },
        (e) => {
          setForgetRefused((was) =>
            new Map(was).set(row.id, String(e instanceof Error ? e.message : e)),
          );
          throw e;
        },
      );
    },
    [refresh],
  );

  // What is happening that nobody is watching — see the hook for the
  // two rules about what may appear here.
  const status = useStatusBar(rows);
  const selectedRow = rows.find((r) => r.id === selected) ?? null;
  // The open row's workspace, in the same words the list's chips and
  // headings use (`placeLabels` over every row, so two checkouts that
  // share a leaf stay told apart). Resolved here because only the
  // shell holds every row; the detail header just prints it.
  const placeWord = useMemo(() => {
    if (!selectedRow?.place) return null;
    const names = placeLabels(rows.map((r) => `at:${r.place ?? ""}`));
    return names.get(`at:${selectedRow.place}`) ?? groupLabel(`at:${selectedRow.place}`);
  }, [rows, selectedRow]);
  // A session that is gone stops being the selected one, and on the
  // narrow face the screen goes back to the list.
  //
  // **Without this the close leaves a lie standing**: `selectedRow`
  // falls to null on its own, so the wide face already empties — but the
  // *selection* would still name a dead id, and the narrow face would sit
  // on a detail screen with nothing in it and no list to see, which is a
  // dead end reachable by pressing 关闭.
  //
  // **Only a selection that was actually on the list gets cleared**, and
  // that is not belt-and-braces: the wizard selects the session it just
  // opened, and the row for it does not exist until the next poll — a
  // rule that read "selected but not in the list" as "gone" threw that
  // selection away a second after making it, and the fresh conversation
  // came up empty. Measured, on this very run.
  //
  // An empty list is likewise not evidence: a poll that answered with
  // nothing is a backend blink, not everything closing at once.
  //
  // The same rule for every open tab now, not only the front one: a tab
  // whose row was listed and is listed no more closes; one never yet
  // listed (a session the wizard just opened) stays until it appears.
  useEffect(() => {
    if (rows.length === 0 || openTabs.length === 0) return;
    const listed = new Set(rows.map((r) => r.id));
    for (const id of openTabs) if (listed.has(id)) wasListed.current.add(id);
    const gone = openTabs.filter((id) => !listed.has(id) && wasListed.current.has(id));
    if (gone.length === 0) return;
    for (const id of gone) wasListed.current.delete(id);
    dropTabs(gone);
    if (selected !== null && gone.includes(selected)) setScreen("list");
  }, [rows, openTabs, dropTabs, selected]);
  // **Rows, not events.** This added the two conditions, so a session
  // that was both waiting on an answer and unread counted twice and the
  // badge could read 4 over two rows — a number that matches nothing the
  // person can go and look at. What the glyph is for is "how many things
  // want you", and a thing wants you once however many reasons it has.
  const blockedOrUnread = rows.reduce(
    (n, r) => n + (r.word === "blocked" || r.unread > 0 ? 1 : 0),
    0,
  );

  // The filter offers the state keys the node actually sent, in the order
  // it sent them — which is its ranking, so the choices come out in the
  // order that matters. Enumerating the six words here instead would be
  // this layer deciding what states exist, and it does not get to
  // (docs/UX.md 状态呈现). Keys already ticked stay on the list even after
  // their last row leaves, or a tick becomes impossible to undo — which
  // now also covers a tick restored from storage before any row has
  // arrived, the one case where the ticked key is guaranteed to be
  // missing from `rows`.
  //
  // **Three axes now, and they are the node's own** (`khor_node::list`
  // groups by exactly these). A ticked key is a group key, so a filter
  // and a heading are the same string and cannot drift apart; the axis
  // heading is what `PaneBar` starts a section on.
  //
  // The machine axis reads `home` rather than `source`: `source` is the
  // offline axis and is absent on rows that live here, so an axis built
  // on it would be missing this machine — the one people reach for
  // first (`SessionRow::home` says the rest).
  const wordOptions = useMemo(() => {
    // The place chips, told apart where two checkouts share a name —
    // the same reader the list's headings use, so a chip and the
    // heading it filters to always print the same words.
    const placeNames = placeLabels(rows.map((r) => `at:${r.place ?? ""}`));
    const seen = (pick: (r: SessionRow) => string, axis: string, label: (v: string) => string) => {
      const values: string[] = [];
      for (const r of rows) {
        const v = pick(r);
        if (!values.includes(v)) values.push(v);
      }
      // A ticked key stays on the list after its last row leaves, or the
      // tick becomes impossible to undo — which also covers a tick
      // restored from storage before any row has arrived, the one case
      // where the ticked key is guaranteed to be missing from `rows`.
      for (const key of words.sessions) {
        if (axisOf(key) === axis && !values.includes(valueOf(key))) values.push(valueOf(key));
      }
      return values.map((v) => ({ key: `${axis}${v}`, label: label(v) }));
    };
    return [
      ...seen((r) => r.word, "state:", word).map((o) => ({ ...o, axis: gui.axis_state })),
      ...seen((r) => r.home, "dev:", (v) => v).map((o) => ({ ...o, axis: gui.devices_tab })),
      ...seen((r) => r.category ?? "", "cat:", (v) => groupLabel(`cat:${v}`)).map((o) => ({
        ...o,
        axis: gui.axis_category,
      })),
      // The 工作区 axis, gathered the same way as the other three — the
      // values are whatever places the rows on screen actually carry,
      // so the list is never longer than the work in front of you.
      ...seen((r) => r.place ?? "", "at:", (v) => placeNames.get(`at:${v}`) ?? groupLabel(`at:${v}`)).map((o) => ({
        ...o,
        axis: gui.axis_place,
      })),
    ];
  }, [rows, words]);

  // The same options, grouped into the axes the search box offers.
  // **Derived from `wordOptions` rather than gathered again**: the chips
  // and the menu ticks are then not merely kept in step, they are one
  // list read two ways, and there is no second gathering that can come
  // back with something else.
  const sessionAxes = useMemo<OmniAxis[]>(() => {
    const byAxis = new Map<string, OmniAxis>();
    for (const o of wordOptions) {
      const axis = axisOf(o.key);
      const seat = byAxis.get(axis) ?? { key: axis, label: o.axis, candidates: [] };
      seat.candidates.push({
        key: o.key,
        label: o.label,
        // A machine wears its face here as it does everywhere else; the
        // other two axes have no face to wear, and an invented one is
        // worse than none (Avatar.tsx's rule).
        face: axis === "dev:" ? (devices.find((d) => d.name === valueOf(o.key))?.face ?? null) : undefined,
      });
      byAxis.set(axis, seat);
    }
    return [...byAxis.values()];
  }, [wordOptions, devices]);

  // (The files pane's box used to be assembled here — a machine chip
  // and a path axis, for a pane whose subject was one machine. It moved
  // into `AddressBar` with the landing's shape: the box now belongs to
  // a tab rather than to a pane, and a landing with no list pane has no
  // `PaneBar` to hang one on.)

  // (The browser pane's box was assembled here too, for the same kind of
  // pane, and left for the same reason. Both browsing landings put their
  // box in `AddressBar` now, where it belongs to a tab.)

  const chooseArrange = useCallback((key: string) => {
    setArrangeBy(key);
    // Kept even if storage refuses (private mode, quota): the choice
    // still applies to this session, it just will not outlive it.
    try {
      window.localStorage.setItem(ARRANGE_KEY, key);
    } catch {
      /* the list still arranges; only the memory of it is lost */
    }
  }, []);

  const toggleWord = useCallback(
    (key: string) =>
      setWords((was) => {
        const had = was[landing];
        const next = had.includes(key) ? had.filter((w) => w !== key) : [...had, key];
        // Kept even if storage refuses, same as the arrangement: the
        // filter still applies, it just will not outlive this session.
        try {
          window.localStorage.setItem(filterKey(landing), JSON.stringify(next));
        } catch {
          /* the list still filters; only the memory of it is lost */
        }
        return { ...was, [landing]: next };
      }),
    [landing],
  );

  const sessionActions: PaneAction[] = [
    // First, because opening a session is the pane's first act
    // (会话身份批B: 建 session 是第一动作).
    { key: "new", label: gui.new_session, onSelect: () => setSheet("new") },
    { key: "tell", label: gui.tell_machine, onSelect: () => setSheet("tell") },
  ];
  const deviceActions: PaneAction[] = [
    { key: "invite", label: gui.make_a_ticket, onSelect: () => setSheet("invite") },
    { key: "join", label: gui.join_with_a_ticket, onSelect: () => setSheet("join") },
  ];

  // Whether the open landing is one the narrow bar folded away — the
  // 「…」trigger lights for it, because the rail must always answer
  // 我在哪一页, even for a page whose own glyph is off the bar.
  const foldedOn = !mark && LANDINGS.slice(NARROW_SHOWN).some((l) => l.key === landing);
  const rail = (
    <nav
      className={cn(
        "flex items-center select-none",
        // Two gaps, one per face. The column rail holds its items a
        // `--gap-row` apart: at `--gap-in` the four landings read as a
        // single clump (that is the distance for the *parts* of one
        // thing), while a whole `--gap-group` centered them into a
        // free-floating cluster that read as layout gone wrong — this
        // rail is a list, and row-step is the list's own step. The narrow
        // bar keeps `gap-in`: `justify-around` is already spending the
        // row's width, and a phone's rail has no room to give
        // (`scripts/check-rail.mjs` measures what overflow does there).
        // `gap-1` on the narrow bar, not `gap-in`: eight items at the
        // finger floor are 352 wide before any gap, and the sixth
        // landing (额度) put `gap-in`'s eight-per-gap over a 390 phone by
        // twelve — the bar overflowed and the whole page scrolled
        // sideways (check-tabs found it). `justify-around` still spreads
        // whatever room is left, so the group reads as one.
        narrow ? "flex-row justify-around gap-1 border-t py-1" : "w-rail flex-col gap-row border-r py-3",
      )}
    >
      {(narrow ? LANDINGS.slice(0, NARROW_SHOWN) : LANDINGS).map((l) => (
        <RailItem
          key={l.key}
          label={l.name}
          tab={l.key}
          // **Not lit while the mark's place is up.** A landing is still
          // remembered — coming back out of the mesh returns to it — but
          // remembering it and being the thing on screen are two facts,
          // and painting both at once puts two selected items in one
          // rail with the wrong one lit.
          on={!mark && landing === l.key}
          narrow={narrow}
          // Only the sessions glyph counts anything: the badge rule is
          // that it must be able to reach zero (docs/UX.md 状态呈现),
          // and machines do not go away when you have looked at them.
          badge={l.key === "sessions" ? blockedOrUnread : undefined}
          onClick={() => {
            // The open landing's glyph pressed again puts its list away
            // and brings it back (2026-09-02:「再次点击侧边栏icon,应该
            // 收起这个列表,再次点击打开。初次点击默认是打开的」) — the
            // activity-bar gesture, and a second handle on the same
            // fold the seam's drag owns. Only where a list exists to
            // put away (the two inbox landings) and only wide: narrow
            // shows one screen, so there is nothing beside the list to
            // give its room to.
            const again = !mark && landing === l.key;
            setMark(false);
            setLanding(l.key);
            setScreen("list");
            setSwitchingTabs(false);
            if (again && !narrow && (l.key === "sessions" || l.key === "devices")) {
              listPane.toggle();
            } else {
              // Choosing a landing reopens a collapsed list: the rail is
              // the anchor a hidden pane comes back from (the seam's own
              // handle is the other way), and it comes back at the width
              // it had, not at the default.
              listPane.expand();
            }
          }}
        >
          {l.glyph}
        </RailItem>
      ))}
      {!narrow && <span className="flex-1" />}
      {narrow ? (
        // The rest of the rail, folded (2026-09-02:「窄屏的时候,下面的
        // 侧边栏最多只显示四个icon,多余的折叠为...」): the landings past
        // the fourth, and 设置 — which already answered to this same
        // glyph, so the two「…」s merged instead of standing side by
        // side. The trigger's classes mirror `RailItem`'s button (one
        // look for every rail seat; the reasons live there).
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              aria-label={gui.rail_more}
              data-rail-item
              data-rail-more
              data-on={foldedOn}
              className={cn(
                "group relative h-auto min-h-touch min-w-touch flex-col justify-center gap-0 rounded-md px-2 py-2 shrink-0 text-rail-ink",
                "hover:text-rail-ink",
                foldedOn && "text-[var(--rail-on-ink)] hover:text-[var(--rail-on-ink)]",
              )}
            >
              <IconMore />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent side="top" data-rail-more-menu>
            {LANDINGS.slice(NARROW_SHOWN).map((l) => (
              <DropdownMenuItem
                key={l.key}
                data-more-landing={l.key}
                data-on={!mark && landing === l.key}
                className="min-h-touch gap-2"
                onSelect={() => {
                  setMark(false);
                  setLanding(l.key);
                  setScreen("list");
                  setSwitchingTabs(false);
                  listPane.expand();
                }}
              >
                {l.glyph}
                {l.name}
              </DropdownMenuItem>
            ))}
            <DropdownMenuItem
              data-more-settings
              className="min-h-touch gap-2"
              onSelect={() => setSettings(true)}
            >
              <IconMore />
              {gui.settings}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      ) : (
        <RailItem label={gui.settings} narrow={narrow} onClick={() => setSettings(true)}>
          <IconMore />
        </RailItem>
      )}
      {/* This machine, at the foot of the rail — **and it is the way
          into the mesh now**.
          「不要右上角的 icon 了吧。取而代之,用左下角的点击作为原来的
          khor 页面」 — the user, 2026-08-29. The app's mark used to sit
          at the head of this rail and open the mark's place; the face
          sat down here as a fact about this machine that was
          deliberately **not** a button.

          Both halves of that swapped, and the second half is the one
          worth writing down: the old comment here argued that a face
          parked at a control's distance from a control reads as one
          more thing to press, and used spacing to say it was not one.
          **It is one now, so the argument is not patched, it is
          spent** — what is left is the spacing, which now says the same
          thing the mark's `mb-group` used to say at the other end: this
          is not a fifth landing.

          It reads better than the mark did, and that is why this is not
          merely obedience: the mark's place is this machine in the
          middle of the mesh, so pressing **this machine's own face** to
          get there says where you are going. A logo says nothing about
          where it leads — it was the app's identity being asked to
          double as navigation.

          Drawn on both faces, unlike before. The face used to be
          wide-only because it was decoration and a phone's rail has no
          room for decoration; it is the only door to the mesh now, and
          a door cannot be wide-only. */}
      <RailItem
        label={gui.mesh_map}
        on={mark}
        narrow={narrow}
        onClick={() => {
          setMark(true);
          setScreen("list");
          setSwitchingTabs(false);
        }}
        // Whitespace is the whole separator, and it is deliberately not
        // a line. A rule would have to be drawn twice to say one thing —
        // horizontal in the column rail, vertical in the narrow bottom
        // bar — so it is said in the one form that does not change shape
        // with the rail.
        className={narrow ? "ms-group" : "mt-group"}
      >
        {/* `data-rail-mark` names **the way into the mark's place**, not
            the artwork that used to wear it — that is what every gate
            and the smoke reach for, and what they are checking did not
            change when the picture did. Same derivation as this
            machine's row in the device list, so the two are the same
            picture; that is the whole promise, and it is checkable on
            one screen. */}
        <span data-rail-mark className="block size-avatar">
          <MachineAvatar face={devices.find((d) => d.me)?.face ?? null} className="size-full" />
        </span>
      </RailItem>
    </nav>
  );

  const sessions = landing === "sessions";
  /** Whether this landing browses (docs/UX.md 浏览型) — the two that do
      share one shell and have no list pane at all. */
  const browses = landing === "files" || landing === "browser";
  // The 额度 page is one region like the browsing landings, for the
  // reason its rail entry gives.
  const quotas = landing === "quota";
  // Every row's last day, on one beat, for the band under the open
  // session's header (`DetailPane`). Sized to the pane the band is drawn
  // in — measured, never assumed (`useStripWidth`) — and asked for every
  // row rather than the front one, so switching tabs costs no fetch.
  const detailBox = useRef<HTMLDivElement>(null);
  const days = useDayStrips(useStripWidth(detailBox, sessions));
  // The open sessions as the strip paints them: each tab wears its
  // row's title and the row's own identity assembly (`SessionFace` —
  // face, state halo, kind mark), the same facts in the same clothes. A
  // tab whose row has not been listed yet (just opened) wears its id.
  const tabFaces: TabFace[] = useMemo(
    () =>
      openTabs.map((id) => {
        const r = rows.find((row) => row.id === id);
        return r
          ? {
              id,
              label: r.title,
              face: r.face,
              path: r.place ?? "",
              session: { word: r.word, kind: r.kind, category: r.category },
            }
          : { id, label: id, face: null, path: "" };
      }),
    [openTabs, rows],
  );
  // The open tabs as rows to pick from — one switcher, written once,
  // whichever face asked for it (`BrowseLanding`'s rule). No `onNew`:
  // a session tab *is* its session, so there is no blank tab to mint —
  // sessions are made by the wizard, and the switcher's foot stays off.
  const tabSwitcher = (
    <TabSwitcher
      faces={tabFaces}
      active={selected ?? ""}
      onFocus={(id) => {
        sessionTabs.focus(id);
        setSwitchingTabs(false);
        setScreen("detail");
      }}
      onClose={sessionTabs.close}
    />
  );
  const paneName = LANDINGS.find((l) => l.key === landing)!.name;

  // No pane wears its name where you can see it (docs/UX.md). The name
  // rides the region's aria-label — read aloud, taking no room — and the
  // strip where a title would have sat holds the work instead.
  const list = (
    <section
      data-list
      aria-label={paneName}
      className={cn(
        "flex h-full min-h-0 flex-col overflow-hidden bg-card",
        !narrow && !listPane.collapsed && "border-r",
      )}
      // Wide, the width is the viewer's own, dragged at the seam and
      // remembered (`useListWidth`); collapsed it is zero, not unmounted
      // — what the pane holds (a typed search, a scroll) is state worth
      // keeping through a collapse. Narrow keeps the class-driven flow.
      style={narrow ? undefined : { width: listPane.collapsed ? 0 : listPane.width }}
    >
      <PaneBar
        // Files and browser search machines, because machines are what
        // they list — so they borrow the machine pane's label rather
        // than growing one that names a thing the box cannot find.
        searchLabel={sessions ? gui.search_sessions : gui.search_devices}
        query={queries[landing]}
        onQuery={setQuery}
        filterLabel={sessions ? gui.filter : undefined}
        // Only the sessions pane narrows a list now: the two browsing
        // landings have no list pane at all, and their box belongs to a
        // tab (`AddressBar`).
        filter={
          sessions ? { options: wordOptions, chosen: words[landing], onToggle: toggleWord } : undefined
        }
        axes={sessions ? sessionAxes : undefined}
        arrange={
          sessions
            ? {
                label: gui.arrange,
                options: ARRANGE.map((a) => ({ key: a.key, label: a.label })),
                chosen: arrangeBy,
                onChoose: chooseArrange,
              }
            : undefined
        }
        actionsLabel={gui.new_item}
        // Nothing gets created from the files or browser panes yet, and
        // an empty "+" is worse than no "+": it is a menu that teaches
        // people not to open menus.
        actions={sessions ? sessionActions : landing === "devices" ? deviceActions : undefined}
      />
      <div className="min-h-0 flex-1 overflow-y-auto">
        {stale && <div className="p-4 text-sm text-muted-foreground">{gui.backend_unreachable}</div>}
        {sessions ? (
          <SessionsList
            rows={rows}
            query={queries.sessions}
            words={words.sessions}
            selected={selected}
            onSelect={onSelect}
            onPin={onPinSession}
            onClose={onCloseSession}
            pinFailed={pinFailed}
            closeRefused={closeRefused}
          />
        ) : (
          <DevicesList
            rows={devices}
            query={queries[landing]}
            onPin={onPinDevice}
            onForget={onForgetDevice}
            forgetRefused={forgetRefused}
            // One question left: the devices pane opens a machine's
            // card. Files and browser were the other two, and each
            // stopped being one when it stopped having a machine list
            // (`DevicesList` says why onOpen is a prop).
            onOpen={onSelectDevice}
            selected={selectedDevice}
            pinFailed={pinFailed}
          />
        )}
      </div>
    </section>
  );

  // Every pane has something to open now, and each gets the component
  // that can say something true about what it opened.
  //
  // **The devices pane with nothing picked is the exception, and it is
  // not a counter-example.** What used to sit there was an empty card,
  // for exactly the reason above: there was nothing true to say. There is
  // now — the whole mesh, which is what that pane is about — so the space
  // holds the map rather than a sentence about being empty.
  const openDevice = devices.find((d) => d.id === selectedDevice) ?? null;
  // (Both browsing landings' starting points moved in with the shell,
  // which reads the device table itself — `BrowseLanding`.)
  const detail = sessions ? (
    <DetailPane
      row={selectedRow}
      narrow={narrow}
      days={selectedRow ? days[selectedRow.id] : undefined}
      onBack={() => setScreen("list")}
      onNew={() => setSheet("new")}
      placeWord={placeWord}
      // The way to the other open tabs on the face that has no strip:
      // the count is the button (`TabCount`), and pressing it swaps
      // this pane for the switcher. Wide has the strip above the pane,
      // so it hands nothing in.
      tabs={
        narrow && openTabs.length > 0 ? (
          <TabCount n={openTabs.length} onPress={() => setSwitchingTabs(true)} />
        ) : undefined
      }
      // The network's now, painted from the shell's own polled state —
      // rows and devices, both already on their beats. It asked for a
      // third thing until 2026-08-29 (the spending tally, for the 今天
      // line) and asks for nothing of its own now.
      empty={
        <MeshNow
          rows={rows}
          devices={devices}
          onPick={onSelect}
          onDevice={(d) => {
            setLanding("devices");
            onSelectDevice(d);
          }}
          onNew={() => setSheet("new")}
        />
      }
    />
  ) : landing === "devices" && !openDevice ? (
    <MandalaMap rows={devices} onOpen={onSelectDevice} selected={selectedDevice} />
  ) : landing === "devices" ? (
    <MachineCard
      row={openDevice}
      narrow={narrow}
      onBack={() => setScreen("list")}
      hooks={hooks}
      hooksFailed={hooksFailed}
      onInstallHooks={onInstallHooks}
      onUninstallHooks={onUninstallHooks}
      // The fresh terminal is what the person came for: land on its row
      // the way the wizard does (`NewSessionDialog.onOpened`), trusting
      // the same guard that keeps a not-yet-polled selection alive. A
      // refusal is rethrown so the card's button can wear it.
      onOpenTerm={async (row) => {
        const id = await openTermOn(row.name);
        setMark(false);
        setLanding("sessions");
        sessionTabs.open(id);
        setScreen("detail");
      }}
      // This machine's disk, in a **new** tab. Not the open one: the
      // request came from somewhere else entirely, so whatever that tab
      // was showing is not part of it — replacing it would throw away a
      // place the person did not ask to leave (`useBrowseTabs.open` says
      // the same from the other side).
      onBrowseFiles={(row) => {
        setMark(false);
        setLanding("files");
        dirTabs.open(row.id, "");
        setScreen("detail");
      }}
      // This machine's network, in a new tab of the browser landing —
      // the same judgment as the disk above it, and the address bar is
      // what an empty place opens to (docs/NET.md 借网).
      onBorrowWeb={(row) => {
        setMark(false);
        setLanding("browser");
        pageTabs.open(row.id, "");
        setScreen("detail");
      }}
    />
  ) : null;

  const sheets = (
    <>
      <NewSessionDialog
        open={sheet === "new"}
        onOpenChange={(o) => setSheet(o ? "new" : null)}
        // The freshly-opened session is what the person came to see.
        onOpened={(id) => sessionTabs.open(id)}
      />
      <TellDialog
        open={sheet === "tell"}
        onOpenChange={(o) => setSheet(o ? "tell" : null)}
        devices={devices}
      />
      <InviteDialog open={sheet === "invite"} onOpenChange={(o) => setSheet(o ? "invite" : null)} />
      <JoinDialog open={sheet === "join"} onOpenChange={(o) => setSheet(o ? "join" : null)} />
      {/* A restyle repaints this machine everywhere at once, because
          `refresh` fetches rows and devices together and every face on
          screen — the rail's, the device row's, each session row's —
          is painted from that one answer. There is no path by which
          two of them could be showing different styles. */}
      <FaceSettings
        open={settings}
        onOpenChange={setSettings}
        onChanged={() => {
          refresh().catch(() => {});
        }}
      />
    </>
  );

  // Its own file: this surface and the session list get reshaped by
  // different people at the same time, and while it lived here those
  // were one file's changes (`MarkPlace` says the rest).
  const markPlace = (
    <MarkPlace
      rows={rows}
      devices={devices}
      onPickWaiting={(row) => {
        setMark(false);
        setLanding("sessions");
        onSelect(row);
      }}
    />
  );

  /**
   * The two browsing landings, which are neither half of the
   * list-and-detail pair either — for a different reason from the
   * mark's, and the two reasons are worth keeping apart.
   *
   * The mark's place is not a landing at all. These are: they are
   * simply **browsing** landings rather than **inbox** ones
   * (docs/UX.md 姿势 — 收件箱型 / 浏览型). 会话 and 机器 are lists a
   * person keeps an eye on, so the list is furniture and stays on
   * screen. A place is not watched, it is walked: which machine is a
   * step on the way, and a permanent column of machines is a column
   * spent on a question that was answered the moment the tab opened.
   * What replaces it is the browser's own answer — tabs, one per
   * place, and an address bar.
   *
   * Same on both faces, which is what the wide/narrow rule already
   * says: the strip is every tab at once where there is room, a count
   * and a switcher where there is not (`TabStrip`).
   *
   * **One component, two kinds, and nothing else differs** — which is
   * the whole claim docs/DESIGN.md 浏览型外壳 makes, now with a second
   * customer to test it against rather than one.
   */
  const browsing =
    landing === "files" ? { kind: FILES, tabs: dirTabs } : { kind: PAGES, tabs: pageTabs };
  const browsingLanding = (
    <BrowseLanding
      // **Keyed by the kind, and that is load-bearing.** A kind carries
      // a hook (`BrowseKind.useSecond`), so two kinds under one mounted
      // shell would change which hooks run — the one thing React cannot
      // survive. There really are two now, and switching landings is
      // exactly the swap that would do it; the key makes it a remount.
      key={browsing.kind.kind}
      kind={browsing.kind}
      devices={devices}
      tabs={browsing.tabs}
      narrow={narrow}
    />
  );

  // One shell, two width classes: wide is rail|list|detail all at once,
  // narrow is one screen at a time with the rail as a bottom bar. The
  // mark's place is neither half of that pair, so it takes the whole of
  // what is left of the window beside the rail (see `Mark`).
  return (
    <TooltipProvider delayDuration={0}>
      {narrow ? (
        <div className="flex h-dvh flex-col">
          {/* `key` is what makes the arrival visible: a CSS animation
              runs when an element is inserted, and without a changing
              key React would reuse this box and the new screen would
              simply appear inside it. The key is the screen, not the
              row — moving between two sessions is not a place change,
              and re-keying on the row would replay the slide (and
              remount a live terminal) every time the list is clicked.
              `data-screen` is what app.css keys the direction off. */}
          <div
            key={mark ? "mark" : browses ? landing : screen}
            data-screen={mark ? "mark" : browses ? landing : screen}
            ref={detailBox}
            className="min-h-0 flex-1 overflow-hidden"
          >
            {/* A browsing landing is one screen either way — it has no
                list screen to be the other half of a pair, so `screen`
                says nothing about it and it is keyed on its own. */}
            {mark
              ? markPlace
              : quotas
                ? <QuotaLanding narrow quota={quota} />
                : browses
                  ? browsingLanding
                  : switchingTabs && openTabs.length > 0
                    ? tabSwitcher
                    : screen === "detail" && detail
                      ? detail
                      : list}
          </div>
          {rail}
          {sheets}
          <StatusBar items={status.items} onDismiss={status.dismiss} />
        </div>
      ) : (
        <div className="flex h-dvh">
          {rail}
          {mark ? (
            <div className="min-w-0 flex-1">{markPlace}</div>
          ) : quotas ? (
            <div className="min-w-0 flex-1">
              <QuotaLanding narrow={false} quota={quota} />
            </div>
          ) : browses ? (
            // The whole width beside the rail, and that width is the
            // point: what the machine column used to spend is now the
            // content's (see `browsingLanding`).
            <div className="min-w-0 flex-1">{browsingLanding}</div>
          ) : (
            <>
              {list}
              {/* The seam between list and detail, and the whole width
                  control: drag to size, drag under the snap line to put
                  the list away, double-click for the default. It takes
                  no room — a hit margin overhanging the pane's border —
                  so collapsing paints it as the one visible strip left,
                  which is also the way to pull the pane back out. */}
              <div
                data-list-seam
                role="separator"
                aria-orientation="vertical"
                // **Reachable, and that is the whole change here.** It
                // had the role and the pointer gestures and nothing
                // else: no focus, no key, so on a keyboard the list
                // could not be put away or brought back at all. Arrow
                // keys walk its stops, Home restores the default
                // (`useListWidth` says why stops rather than nudges).
                tabIndex={narrow ? undefined : 0}
                {...listPane.aria}
                className={cn(
                  "relative z-10 -mx-1 w-2 shrink-0 cursor-col-resize transition-colors hover:bg-border",
                  // Focus has to be visible on a strip that is mostly
                  // not there: the ring is the whole seam lighting up,
                  // because there is no glyph to put a ring around.
                  "outline-none focus-visible:bg-ring",
                  listPane.collapsed && "bg-border",
                )}
                {...listPane.handleProps}
              />
              {/* The open sessions as tabs over the pane (iTerm's
                  arrangement), only on the sessions landing and only
                  when something is open — an empty strip is a band
                  saying nothing. The strip closes a *view*; the header's
                  关闭 ends the session, and the gone-row rule then
                  closes the tab. `ref` is the band's width source. */}
              <div ref={detailBox} className="flex min-w-0 flex-1 flex-col">
                {sessions && openTabs.length > 0 && (
                  <div data-session-tabs aria-label={gui.session_tabs}>
                    <TabStrip
                      faces={tabFaces}
                      active={selected ?? ""}
                      onFocus={(id) => {
                        sessionTabs.focus(id);
                        setSwitchingTabs(false);
                        setScreen("detail");
                      }}
                      onClose={sessionTabs.close}
                      // The strip decides *whether* to offer the way to
                      // the switcher — only it can measure whether it
                      // has stopped showing every tab. This side only
                      // says what pressing it does.
                      onSwitch={() => setSwitchingTabs(true)}
                    />
                  </div>
                )}
                <div className="min-h-0 flex-1">
                  {sessions && switchingTabs && openTabs.length > 0 ? tabSwitcher : detail}
                </div>
              </div>
            </>
          )}
          {sheets}
          <StatusBar items={status.items} onDismiss={status.dismiss} />
        </div>
      )}
    </TooltipProvider>
  );
}
