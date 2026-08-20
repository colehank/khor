import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import {
  fetchDevices,
  fetchHooks,
  fetchSessions,
  fetchUsage,
  installHooks,
  markSeen,
  pinDevice,
  pinSession,
  uninstallHooks,
  type DeviceRow,
  type HooksState,
  type SessionRow,
  type Usage,
} from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import {
  IconBrowser,
  IconDevices,
  IconFiles,
  IconMore,
  IconSessions,
} from "@/components/icons";
import { KhorMark } from "@/components/KhorMark";
import { PaneBar, type PaneAction } from "@/components/PaneBar";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { gui } from "@/gen/catalog";
import { useNarrow } from "@/hooks/use-narrow";
import { cn } from "@/lib/utils";
import { axisOf, groupLabel, valueOf, word } from "@/words";
import { DetailPane } from "@/views/DetailPane";
import { FilesPane } from "@/views/FilesPane";
import { PinnedDirs } from "@/views/PinnedDirs";
import { BrowserPane } from "@/views/BrowserPane";
import { PinnedWebs } from "@/views/PinnedWebs";
import { DevicesList } from "@/views/DevicesList";
import { FaceSettings } from "@/views/FaceSettings";
import { MachineCard } from "@/views/MachineCard";
import { MandalaMap } from "@/views/MandalaMap";
import { UsagePanel } from "@/views/UsagePanel";
import { InviteDialog, JoinDialog } from "@/views/PairDialogs";
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
type Landing = "sessions" | "devices" | "files" | "browser";

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
/** How often the spending answer is re-asked while it is on screen. See
    where it is used for why it is not the rows' beat. */
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
  };
}

const LANDINGS: { key: Landing; name: string; glyph: ReactNode }[] = [
  { key: "sessions", name: gui.sessions_tab, glyph: <IconSessions /> },
  { key: "devices", name: gui.devices_tab, glyph: <IconDevices /> },
  { key: "files", name: gui.files_tab, glyph: <IconFiles /> },
  { key: "browser", name: gui.browser_tab, glyph: <IconBrowser /> },
];

const NO_QUERIES: Record<Landing, string> = {
  sessions: "",
  devices: "",
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
   * Extra styling for one item. **Exists for the mark**, whose glyph is
   * an image: the selected look below is a text colour, which tints a
   * drawn glyph and does nothing at all to a picture — so that one item
   * has to say it a second way.
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
        "group relative h-auto flex-col gap-0 rounded-md px-2 py-2 text-muted-foreground",
        on && "text-primary hover:text-primary",
        className,
      )}
    >
      {children}
      {badge !== undefined && badge > 0 && (
        <span className="absolute right-1 top-1 min-w-4 rounded-full bg-badge px-1 text-center text-xs leading-4 text-badge-foreground">
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
  const [landing, setLanding] = useState<Landing>("sessions");
  const [rows, setRows] = useState<SessionRow[]>([]);
  const [devices, setDevices] = useState<DeviceRow[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  /** The selected id, once it has actually been seen on the list — see
      the effect below for why "selected" alone is not enough. */
  const wasListed = useRef<string | null>(null);
  // The open machine, kept apart from the open session for the same
  // reason each pane keeps its own query: they are two selections in two
  // places, and one variable would have opening a machine close the
  // session you were reading.
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
  // …and the machine whose files are open, apart from both: the files
  // landing asks a different question of a machine than the devices
  // landing does, and sharing the variable would have browsing a disk
  // flip which machine card the devices pane shows. The path rides
  // along so a pinned shortcut can open straight where it points.
  const [browse, setBrowse] = useState<{ device: string; path: string } | null>(null);
  // …and the exit machine whose network the browser landing surfs,
  // apart from all the above for the same reason: browsing through a
  // machine is a third question, and a shared variable would tangle it
  // with the machine card and the disk. `url` rides along so a pinned
  // page opens straight through on arrival.
  const [surf, setSurf] = useState<{ device: string; url: string | null } | null>(null);
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
  // What the agents have cost. **Asked for only while the place that
  // shows it is open**, and not on the two-second poll the rows ride:
  // the answer is every day there has ever been, and on the machine
  // answering it a first pass is eighteen seconds
  // (`khor_node::usage::Meters::tally`). Every one after that is cheap,
  // so a slower beat of its own is enough for a number that changes as
  // fast as somebody can type.
  const [usage, setUsage] = useState<Usage | null>(null);

  // This machine's hooks, polled with everything else rather than read
  // once when the card opens: `khor hooks install` in a terminal is the
  // other way this changes, and a button that only learns on mount would
  // sit there offering to do something already done.
  const [hooks, setHooks] = useState<HooksState | null>(null);
  const [hooksFailed, setHooksFailed] = useState(false);

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
    const t = setInterval(tick, POLL_MS);
    return () => {
      live = false;
      clearInterval(t);
    };
    // Re-subscribed when the arrangement changes: the new order comes
    // from the node, so switching modes is a fetch, never a re-sort.
  }, [arrangeBy]);

  useEffect(() => {
    if (!mark) return;
    let live = true;
    const tick = () => {
      fetchUsage()
        .then((u) => live && setUsage(u))
        .catch(() => {});
    };
    tick();
    const t = setInterval(tick, USAGE_POLL_MS);
    return () => {
      live = false;
      clearInterval(t);
    };
  }, [mark]);

  const onSelect = useCallback((row: SessionRow) => {
    setSelected(row.id);
    if (row.unread > 0) {
      // Looking at it is the seen semantics; the watermark replicates,
      // so this clears the badge on every device.
      markSeen(row.id).catch(() => {});
    }
    setScreen("detail");
  }, []);

  const onSelectDevice = useCallback((row: DeviceRow) => {
    setSelectedDevice(row.id);
    setScreen("detail");
  }, []);

  const onBrowseDevice = useCallback((row: DeviceRow) => {
    setBrowse({ device: row.id, path: "" });
    setScreen("detail");
  }, []);

  const onSurfDevice = useCallback((row: DeviceRow) => {
    setSurf({ device: row.id, url: null });
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
  const onPinSession = useCallback(
    (row: SessionRow) => {
      pinSession(row.id, !row.pinned).then(
        () => {
          markPin(row.id, false);
          refresh().catch(() => {});
        },
        () => markPin(row.id, true),
      );
    },
    [refresh, markPin],
  );

  const onPinDevice = useCallback(
    (row: DeviceRow) => {
      pinDevice(row.name, !row.pinned).then(
        () => {
          markPin(row.id, false);
          refresh().catch(() => {});
        },
        () => markPin(row.id, true),
      );
    },
    [refresh, markPin],
  );

  const selectedRow = rows.find((r) => r.id === selected) ?? null;
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
  useEffect(() => {
    if (!selected || rows.length === 0) return;
    if (rows.some((r) => r.id === selected)) {
      wasListed.current = selected;
      return;
    }
    if (wasListed.current !== selected) return;
    wasListed.current = null;
    setSelected(null);
    setScreen("list");
  }, [rows, selected]);
  const blockedOrUnread = rows.reduce(
    (n, r) => n + (r.word === "blocked" ? 1 : 0) + (r.unread > 0 ? 1 : 0),
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
    ];
  }, [rows, words]);

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

  const rail = (
    <nav
      className={cn(
        "flex items-center gap-1 select-none",
        narrow ? "flex-row justify-around border-t py-1" : "w-rail flex-col border-r py-3",
      )}
    >
      {/* The app's mark, on both faces now. It used to be wide-only,
          because the narrow rail is a row of places to go and this was
          not one of them; it is now, so the reason went with the
          inertness (see `KhorMark`). */}
      <RailItem
        label={gui.mesh_map}
        on={mark}
        narrow={narrow}
        onClick={() => {
          setMark(true);
          setScreen("list");
        }}
        // The one item whose glyph is a picture, so being the open one
        // has to be said with something a picture can wear.
        className="data-[on=true]:bg-accent"
      >
        <KhorMark />
      </RailItem>
      {LANDINGS.map((l) => (
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
            setMark(false);
            setLanding(l.key);
            setScreen("list");
          }}
        >
          {l.glyph}
        </RailItem>
      ))}
      {!narrow && <span className="flex-1" />}
      <RailItem label={gui.settings} narrow={narrow} onClick={() => setSettings(true)}>
        <IconMore />
      </RailItem>
      {/* This machine, at the foot of the rail. Same derivation as its
          row in the device list, so the two are the same picture — that
          is the whole promise, and it is checkable on one screen. */}
      {!narrow && <MachineAvatar face={devices.find((d) => d.me)?.face ?? null} className="size-avatar" />}
    </nav>
  );

  const sessions = landing === "sessions";
  const paneName = LANDINGS.find((l) => l.key === landing)!.name;

  // No pane wears its name where you can see it (docs/UX.md). The name
  // rides the region's aria-label — read aloud, taking no room — and the
  // strip where a title would have sat holds the work instead.
  const list = (
    <section
      data-list
      aria-label={paneName}
      className={cn("flex h-full min-h-0 flex-col overflow-hidden bg-card", !narrow && "w-list border-r")}
    >
      <PaneBar
        // Files and browser search machines, because machines are what
        // they list — so they borrow the machine pane's label rather
        // than growing one that names a thing the box cannot find.
        searchLabel={sessions ? gui.search_sessions : gui.search_devices}
        query={queries[landing]}
        onQuery={setQuery}
        filterLabel={gui.filter}
        filter={
          sessions
            ? { options: wordOptions, chosen: words[landing], onToggle: toggleWord }
            : undefined
        }
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
            pinFailed={pinFailed}
          />
        ) : (
          <DevicesList
            rows={devices}
            query={queries[landing]}
            onPin={onPinDevice}
            // Each landing asks the machine a different question:
            // devices opens the card, files opens the disk, browser
            // borrows the network. Three selections, three variables
            // (`DevicesList` says why onOpen is a prop).
            onOpen={
              landing === "devices"
                ? onSelectDevice
                : landing === "files"
                  ? onBrowseDevice
                  : onSurfDevice
            }
            selected={
              landing === "files"
                ? (browse?.device ?? null)
                : landing === "browser"
                  ? (surf?.device ?? null)
                  : selectedDevice
            }
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
  const browsed = browse ? (devices.find((d) => d.id === browse.device) ?? null) : null;
  const surfed = surf ? (devices.find((d) => d.id === surf.device) ?? null) : null;
  const detail = sessions ? (
    <DetailPane row={selectedRow} narrow={narrow} onBack={() => setScreen("list")} />
  ) : landing === "files" && browse && browsed ? (
    // Keyed so switching machines or shortcuts restarts the browse
    // where it points — a path from one disk means nothing on another.
    <FilesPane
      key={`${browse.device}:${browse.path}`}
      machine={browsed.name}
      device={browsed.id}
      initialPath={browse.path}
      narrow={narrow}
      onBack={() => setScreen("list")}
    />
  ) : landing === "files" ? (
    // Before a machine is picked: the pinned shortcuts, each a jump
    // straight to a directory somebody chose to keep close.
    <PinnedDirs
      onOpen={(device, path) => {
        setBrowse({ device, path });
        setScreen("detail");
      }}
    />
  ) : landing === "browser" && surf && surfed ? (
    // Keyed by exit and page so a shortcut opening a different machine's
    // page restarts the pane pointed at it.
    <BrowserPane
      key={`${surf.device}:${surf.url ?? ""}`}
      machine={surfed.name}
      device={surfed.id}
      initialUrl={surf.url}
      narrow={narrow}
      onBack={() => setScreen("list")}
    />
  ) : landing === "browser" ? (
    // Before an exit is picked: the pinned pages, each a jump straight to
    // opening it through the machine it was pinned against.
    <PinnedWebs
      onOpen={(device, url) => {
        setSurf({ device, url });
        setScreen("detail");
      }}
    />
  ) : landing === "devices" && !openDevice ? (
    <MandalaMap rows={devices} onOpen={onSelectDevice} selected={selectedDevice} />
  ) : landing === "devices" ? (
    <MachineCard
      row={openDevice}
      narrow={narrow}
      onBack={() => setScreen("list")}
      onPin={onPinDevice}
      pinFailed={pinFailed}
      hooks={hooks}
      hooksFailed={hooksFailed}
      onInstallHooks={onInstallHooks}
      onUninstallHooks={onUninstallHooks}
    />
  ) : null;

  const sheets = (
    <>
      <NewSessionDialog
        open={sheet === "new"}
        onOpenChange={(o) => setSheet(o ? "new" : null)}
        // The freshly-opened session is what the person came to see.
        onOpened={(id) => setSelected(id)}
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

  /**
   * The mark's place: the mesh, and what it cost.
   *
   * **Side by side wide, stacked narrow, and the two are one argument
   * rather than two panels that happen to share a screen.** The picture
   * names the machines and says which of them khor has heard from; the
   * numbers are every machine's added together and name none. Each is
   * the caption the other would otherwise have needed printed under it.
   *
   * The map is handed no `onOpen` here: there is no machine card in this
   * space to open into, and an affordance that answers nothing is what
   * the mark itself was forbidden from being until this batch
   * (`MandalaMap` says why that is a prop).
   */
  const markPlace = (
    <div
      data-mark-place
      className={cn("flex h-full min-h-0 min-w-0", narrow ? "flex-col" : "flex-row")}
    >
      <div className={cn("min-h-0 min-w-0", narrow ? "flex-1" : "flex-[3]")}>
        <MandalaMap rows={devices} />
      </div>
      <div
        className={cn(
          "min-h-0 min-w-0",
          narrow ? "max-h-1/2 flex-none border-t" : "w-list flex-none border-l",
        )}
      >
        <UsagePanel usage={usage} />
      </div>
    </div>
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
            key={mark ? "mark" : screen}
            data-screen={mark ? "mark" : screen}
            className="min-h-0 flex-1 overflow-hidden"
          >
            {mark ? markPlace : screen === "detail" && detail ? detail : list}
          </div>
          {rail}
          {sheets}
        </div>
      ) : (
        <div className="flex h-dvh">
          {rail}
          {mark ? (
            <div className="min-w-0 flex-1">{markPlace}</div>
          ) : (
            <>
              {list}
              <div className="min-w-0 flex-1">{detail}</div>
            </>
          )}
          {sheets}
        </div>
      )}
    </TooltipProvider>
  );
}
