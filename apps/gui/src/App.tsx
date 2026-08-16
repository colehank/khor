import { useCallback, useEffect, useState } from "react";

import {
  fetchDevices,
  fetchSessions,
  markSeen,
  type DeviceRow,
  type SessionRow,
} from "@/api";
import { IconDevices, IconMore, IconSessions } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { gui } from "@/gen/catalog";
import { useNarrow } from "@/hooks/use-narrow";
import { cn } from "@/lib/utils";
import { DetailPane } from "@/views/DetailPane";
import { DevicesList } from "@/views/DevicesList";
import { SessionsList } from "@/views/SessionsList";

type Landing = "sessions" | "devices";

const POLL_MS = 2000;

function RailItem({
  label,
  on,
  narrow,
  badge,
  onClick,
  children,
}: {
  label: string;
  on?: boolean;
  narrow: boolean;
  badge?: number;
  onClick?: () => void;
  children: React.ReactNode;
}) {
  return (
    <Button
      variant="ghost"
      aria-label={label}
      data-rail-item
      data-on={on}
      onClick={onClick}
      className={cn(
        "group relative h-auto flex-col gap-0 rounded-md px-2 py-2 text-muted-foreground",
        on && "text-primary hover:text-primary",
      )}
    >
      {children}
      {/* The name shows itself on hover — a real element, instantly,
          not the OS title tooltip and its 1.5s delay. Narrow has no
          hover; the glyphs stand alone there. */}
      {!narrow && (
        <span className="pointer-events-none invisible absolute left-full top-1/2 z-10 ml-2 -translate-y-1/2 whitespace-nowrap rounded-md border bg-popover px-2 py-1 text-xs text-popover-foreground opacity-0 shadow-md transition-opacity duration-120 group-hover:visible group-hover:opacity-100 group-focus-visible:visible group-focus-visible:opacity-100">
          {label}
        </span>
      )}
      {badge !== undefined && badge > 0 && (
        <span className="absolute right-1 top-1 min-w-4 rounded-full bg-badge px-1 text-center text-xs leading-4 text-badge-foreground">
          {badge}
        </span>
      )}
    </Button>
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

  const rail = (
    <nav
      className={cn(
        "flex items-center gap-1 select-none",
        narrow ? "flex-row justify-around border-t py-1" : "w-rail flex-col border-r py-3",
      )}
    >
      <RailItem
        label={gui.sessions_tab}
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
      {!narrow && <span aria-hidden="true" className="size-avatar flex-none rounded-full bg-muted" />}
    </nav>
  );

  const list = (
    <section
      data-list
      className={cn("flex h-full min-h-0 flex-col overflow-hidden bg-card", !narrow && "w-list border-r")}
    >
      <header className="flex h-ctl-lg flex-none select-none items-center px-4 text-lg font-semibold">
        {landing === "sessions" ? gui.sessions_tab : gui.devices_tab}
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {stale && <div className="p-4 text-sm text-muted-foreground">{gui.backend_unreachable}</div>}
        {landing === "sessions" ? (
          <SessionsList rows={rows} selected={selected} onSelect={onSelect} />
        ) : (
          <DevicesList rows={devices} />
        )}
      </div>
    </section>
  );

  const detail = (
    <DetailPane row={selectedRow} narrow={narrow} onBack={() => setScreen("list")} />
  );

  // One shell, two width classes: wide is rail|list|detail all at once,
  // narrow is one screen at a time with the rail as a bottom bar.
  return narrow ? (
    <div className="flex h-dvh flex-col">
      <div className="min-h-0 flex-1 overflow-hidden">
        {screen === "detail" && landing === "sessions" ? detail : list}
      </div>
      {rail}
    </div>
  ) : (
    <div className="flex h-dvh">
      {rail}
      {list}
      <div className="min-w-0 flex-1">{detail}</div>
    </div>
  );
}
