import { useCallback, useEffect, useMemo, useState } from "react";

import {
  fetchDevices,
  fetchSessions,
  markSeen,
  type DeviceRow,
  type SessionRow,
} from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { IconDevices, IconMore, IconSessions } from "@/components/icons";
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

type Landing = "sessions" | "devices";
/** Which of the "+" dialogs is up, if any. */
type Sheet = "tell" | "invite" | "join" | null;

const POLL_MS = 2000;

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
  const [sessionQuery, setSessionQuery] = useState("");
  const [deviceQuery, setDeviceQuery] = useState("");
  const [words, setWords] = useState<string[]>([]);
  const [sheet, setSheet] = useState<Sheet>(null);

  useEffect(() => {
    let live = true;
    const tick = () => {
      Promise.all([fetchSessions(), fetchDevices()])
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
  }, []);

  const onSelect = useCallback((row: SessionRow) => {
    setSelected(row.id);
    if (row.unread > 0) {
      // Looking at it is the seen semantics; the watermark replicates,
      // so this clears the badge on every device.
      markSeen(row.id).catch(() => {});
    }
    setScreen("detail");
  }, []);

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
      <RailItem
        label={gui.sessions_tab}
        tab="sessions"
        on={landing === "sessions"}
        narrow={narrow}
        badge={blockedOrUnread}
        onClick={() => {
          setLanding("sessions");
          setScreen("list");
        }}
      >
        <IconSessions />
      </RailItem>
      <RailItem
        label={gui.devices_tab}
        tab="devices"
        on={landing === "devices"}
        narrow={narrow}
        onClick={() => {
          setLanding("devices");
          setScreen("list");
        }}
      >
        <IconDevices />
      </RailItem>
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

  // No pane wears its name where you can see it (docs/UX.md). The name
  // rides the region's aria-label — read aloud, taking no room — and the
  // strip where a title would have sat holds the work instead.
  const list = (
    <section
      data-list
      aria-label={sessions ? gui.sessions_tab : gui.devices_tab}
      className={cn("flex h-full min-h-0 flex-col overflow-hidden bg-card", !narrow && "w-list border-r")}
    >
      <PaneBar
        searchLabel={sessions ? gui.search_sessions : gui.search_devices}
        query={sessions ? sessionQuery : deviceQuery}
        onQuery={sessions ? setSessionQuery : setDeviceQuery}
        filterLabel={gui.filter}
        filter={
          sessions
            ? { options: wordOptions, chosen: words, onToggle: toggleWord }
            : undefined
        }
        actionsLabel={gui.new_item}
        actions={sessions ? sessionActions : deviceActions}
      />
      <div className="min-h-0 flex-1 overflow-y-auto">
        {stale && <div className="p-4 text-sm text-muted-foreground">{gui.backend_unreachable}</div>}
        {sessions ? (
          <SessionsList
            rows={rows}
            query={sessionQuery}
            words={words}
            selected={selected}
            onSelect={onSelect}
          />
        ) : (
          <DevicesList rows={devices} query={deviceQuery} />
        )}
      </div>
    </section>
  );

  const detail = (
    <DetailPane row={selectedRow} narrow={narrow} onBack={() => setScreen("list")} />
  );

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
            {screen === "detail" && landing === "sessions" ? detail : list}
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
