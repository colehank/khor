import { useCallback, useEffect, useState } from "react";
import {
  fetchDevices,
  fetchSessions,
  markSeen,
  type DeviceRow,
  type SessionRow,
} from "./api";
import { gui } from "./gen/catalog";
import { UiButton } from "./ui/Button";
import { IconDevices, IconMore, IconSessions } from "./ui/Icons";
import { useNarrow } from "./ui/layout";
import { DetailPane } from "./views/DetailPane";
import { DevicesList } from "./views/DevicesList";
import { SessionsList } from "./views/SessionsList";

type Landing = "sessions" | "devices";

const POLL_MS = 2000;

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
    <nav className="rail">
      <UiButton
        label={gui.sessions_tab}
        className={`rail-item${landing === "sessions" ? " on" : ""}`}
        onClick={() => {
          setLanding("sessions");
          setScreen("list");
        }}
      >
        <IconSessions />
        <span>{gui.sessions_tab}</span>
        {blockedOrUnread > 0 && <span className="rail-badge">{blockedOrUnread}</span>}
      </UiButton>
      <UiButton
        label={gui.devices_tab}
        className={`rail-item${landing === "devices" ? " on" : ""}`}
        onClick={() => {
          setLanding("devices");
          setScreen("list");
        }}
      >
        <IconDevices />
        <span>{gui.devices_tab}</span>
      </UiButton>
      <span className="rail-spacer" />
      <UiButton label={gui.settings} className="rail-item">
        <IconMore />
      </UiButton>
      <span className="avatar-blank" aria-hidden="true" />
    </nav>
  );

  const list = (
    <section className="list">
      <header className="list-header">
        {landing === "sessions" ? gui.sessions_tab : gui.devices_tab}
      </header>
      <div className="list-body">
        {stale && <div className="notice">{gui.backend_unreachable}</div>}
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
  return (
    <div className="app" data-narrow={narrow}>
      {narrow ? (
        <>
          {screen === "detail" && landing === "sessions" ? detail : list}
          {rail}
        </>
      ) : (
        <>
          {rail}
          {list}
          {detail}
        </>
      )}
    </div>
  );
}
