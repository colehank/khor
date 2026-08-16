import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";

import {
  fetchDevices,
  fetchSessions,
  markSeen,
  pinDevice,
  pinSession,
  type DeviceRow,
  type SessionRow,
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
import { word } from "@/words";
import { DetailPane } from "@/views/DetailPane";
import { DevicesList } from "@/views/DevicesList";
import { InviteDialog, JoinDialog } from "@/views/PairDialogs";
import { SessionsList } from "@/views/SessionsList";
import { TellDialog } from "@/views/TellDialog";

/**
 * The four landings, all present from the first day.
 *
 * Files and browser have no content of their own yet, and they are here
 * anyway because both of them *begin* with the same question the devices
 * pane answers — which machine — so listing machines is not a placeholder
 * standing in for the real thing, it is the real thing's first step.
 * What they are missing is the second step (a machine's files, a
 * machine's network), and that is what their batches add.
 */
type Landing = "sessions" | "devices" | "files" | "browser";

/** Which of the "+" dialogs is up, if any. */
type Sheet = "tell" | "invite" | "join" | null;

const POLL_MS = 2000;

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
  children,
}: {
  label: string;
  /** Which landing this glyph opens, when it opens one. */
  tab?: Landing;
  on?: boolean;
  narrow: boolean;
  badge?: number;
  onClick?: () => void;
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
  // Narrow face only: which single screen is up. Wide ignores it.
  const [screen, setScreen] = useState<"list" | "detail">("list");
  const [stale, setStale] = useState(false);
  // One query per pane: switching landings is not a way of clearing a
  // search, and a search that follows you across panes filters the wrong
  // list on arrival.
  const [queries, setQueries] = useState<Record<Landing, string>>(NO_QUERIES);
  const [words, setWords] = useState<string[]>([]);
  const [arrangeBy, setArrangeBy] = useState<string>(storedArrange);
  const [sheet, setSheet] = useState<Sheet>(null);

  useEffect(() => {
    let live = true;
    const tick = () => {
      Promise.all([fetchSessions(arrangeBy), fetchDevices()])
        .then(([s, d]) => {
          if (!live) return;
          setRows(s);
          setDevices(d);
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

  const onSelect = useCallback((row: SessionRow) => {
    setSelected(row.id);
    if (row.unread > 0) {
      // Looking at it is the seen semantics; the watermark replicates,
      // so this clears the badge on every device.
      markSeen(row.id).catch(() => {});
    }
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

  // A failed pin currently shows as nothing happening, which is what
  // "I missed the button" also looks like (docs/UX.md: 做了但没变化 and
  // 失败 must not wear one face). The race that made this reachable is
  // fixed at the source — a shared temp filename in the block store,
  // measured at 17 of 40 pins failing before and 0 of 40 after — so what
  // is left is the rare disk error, and saying it properly needs a place
  // to say it that this screen does not have yet. On the ledger.
  const onPinSession = useCallback(
    (row: SessionRow) => {
      pinSession(row.id, !row.pinned).then(refresh).catch(() => {});
    },
    [refresh],
  );

  const onPinDevice = useCallback(
    (row: DeviceRow) => {
      pinDevice(row.name, !row.pinned).then(refresh).catch(() => {});
    },
    [refresh],
  );

  const selectedRow = rows.find((r) => r.id === selected) ?? null;
  const blockedOrUnread = rows.reduce(
    (n, r) => n + (r.word === "blocked" ? 1 : 0) + (r.unread > 0 ? 1 : 0),
    0,
  );

  // The filter offers the state keys the node actually sent, in the order
  // it sent them — which is its ranking, so the choices come out in the
  // order that matters. Enumerating the six words here instead would be
  // this layer deciding what states exist, and it does not get to
  // (docs/UX.md 状态呈现). Keys already ticked stay on the list even after
  // their last row leaves, or a tick becomes impossible to undo.
  const wordOptions = useMemo(() => {
    const keys: string[] = [];
    for (const r of rows) if (!keys.includes(r.word)) keys.push(r.word);
    for (const w of words) if (!keys.includes(w)) keys.push(w);
    return keys.map((key) => ({ key, label: word(key) }));
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
      setWords((was) => (was.includes(key) ? was.filter((w) => w !== key) : [...was, key])),
    [],
  );

  const sessionActions: PaneAction[] = [
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
      {/* The app's mark, wide only. On the narrow face this rail is a
          bottom bar of places to go, and the mark is not one of them —
          it would be the only thing there that does not answer a tap,
          in the row where every tap is expected to. */}
      {!narrow && <KhorMark className="mb-2" />}
      {LANDINGS.map((l) => (
        <RailItem
          key={l.key}
          label={l.name}
          tab={l.key}
          on={landing === l.key}
          narrow={narrow}
          // Only the sessions glyph counts anything: the badge rule is
          // that it must be able to reach zero (docs/UX.md 状态呈现),
          // and machines do not go away when you have looked at them.
          badge={l.key === "sessions" ? blockedOrUnread : undefined}
          onClick={() => {
            setLanding(l.key);
            setScreen("list");
          }}
        >
          {l.glyph}
        </RailItem>
      ))}
      {!narrow && <span className="flex-1" />}
      <RailItem label={gui.settings} narrow={narrow}>
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
            ? { options: wordOptions, chosen: words, onToggle: toggleWord }
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
            words={words}
            selected={selected}
            onSelect={onSelect}
            onPin={onPinSession}
          />
        ) : (
          <DevicesList rows={devices} query={queries[landing]} onPin={onPinDevice} />
        )}
      </div>
    </section>
  );

  // Only the sessions pane has anything to select, so only it gets a
  // detail. Over a list of machines the same component would print
  // `gui.pick_a_session` — an instruction to do something this screen
  // cannot do. Machine cards land in their own batch and this is where
  // they go; until then the space stays empty rather than filled with
  // copy about the emptiness (docs/UX.md 文案).
  const detail = sessions ? (
    <DetailPane row={selectedRow} narrow={narrow} onBack={() => setScreen("list")} />
  ) : null;

  const sheets = (
    <>
      <TellDialog
        open={sheet === "tell"}
        onOpenChange={(o) => setSheet(o ? "tell" : null)}
        devices={devices}
      />
      <InviteDialog open={sheet === "invite"} onOpenChange={(o) => setSheet(o ? "invite" : null)} />
      <JoinDialog open={sheet === "join"} onOpenChange={(o) => setSheet(o ? "join" : null)} />
    </>
  );

  // One shell, two width classes: wide is rail|list|detail all at once,
  // narrow is one screen at a time with the rail as a bottom bar.
  return (
    <TooltipProvider delayDuration={0}>
      {narrow ? (
        <div className="flex h-dvh flex-col">
          <div className="min-h-0 flex-1 overflow-hidden">
            {screen === "detail" && detail ? detail : list}
          </div>
          {rail}
          {sheets}
        </div>
      ) : (
        <div className="flex h-dvh">
          {rail}
          {list}
          <div className="min-w-0 flex-1">{detail}</div>
          {sheets}
        </div>
      )}
    </TooltipProvider>
  );
}
