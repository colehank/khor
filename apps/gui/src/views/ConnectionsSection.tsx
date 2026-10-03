// 连接 — the 网络 page's group that says how this machine is connected
// (round NET-2, the owner: 「做成一页，手机上能截图」).
//
// The doctor's connection rows, in the rows every 「…」 page is made of:
// where this machine is homed (家中继), how far each relay is — **one
// round trip, timed on a warm connection** (`doctor::relay_round_trip`),
// not the first request's handshakes, which is what read 2040 ms for the
// Hong Kong relay — and per machine its road, its round trip, how long
// ago it was last heard and the last thing that went wrong, in khor's
// own words. Asked on arrival and every few seconds while the page is
// up (each ask times a round trip to every relay, so a hidden window
// asks at the hidden beat); nothing here is a control.
import { useEffect, useState } from "react";

import { fetchConnections, type Connections } from "@/api";
import { Waiting } from "@/components/RegionState";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";
import { gui } from "@/gen/catalog";
import { MoreFact, MoreSection } from "@/views/more/MorePage";
import { ago, hopWord, linkWord } from "@/words";

/** How often the group asks again while it is on screen. */
const AGAIN_MS = 5000;

/** A relay's address as a row names it: its host, no scheme. */
const hostOf = (url: string) => {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
};

export function ConnectionsSection() {
  const [conn, setConn] = useState<Connections | null>(null);
  /** When the first asking began (`Waiting`, below). */
  const [since] = useState(() => Date.now());
  const hidden = useHidden();
  useEffect(() => {
    let alive = true;
    let asking = false;
    const ask = () => {
      if (asking) return;
      asking = true;
      fetchConnections()
        .then((c) => {
          if (alive) setConn(c);
        })
        // The last answer stays: a page that blanked on a slow question
        // would say less than it knew.
        .catch(() => {})
        .finally(() => {
          asking = false;
        });
    };
    ask();
    const t = window.setInterval(ask, hidden ? HIDDEN_MS : AGAIN_MS);
    return () => {
      alive = false;
      window.clearInterval(t);
    };
  }, [hidden]);
  // **Loading until the first answer, under its own heading** (UX-4; the
  // owner's rule of 2026-10-04): the probes take seconds, and a section
  // that appeared only then pushed the page down under the reader.
  if (!conn) {
    return (
      <MoreSection name="connections" label={gui.card_connection}>
        <Waiting since={since} />
      </MoreSection>
    );
  }
  if (conn.unread) {
    return (
      <MoreSection name="connections" label={gui.card_connection}>
        <MoreFact name="conn-unread" label={gui.conn_home}>
          <span data-conn-unread={conn.unread}>
            {conn.unread === "no-serve" ? hopWord("no-serve") : gui.conn_unanswered}
          </span>
        </MoreFact>
      </MoreSection>
    );
  }
  const now = Date.now();
  return (
    <MoreSection name="connections" label={gui.card_connection} mark={{ "data-connections": "" }}>
      <MoreFact
        name="conn-home"
        label={gui.conn_home}
        // Why the home relay cannot be reached, under the row (UX-4).
        note={conn.home_relay && !conn.home_connected && conn.home_error ? conn.home_error : undefined}
      >
        <span data-conn-home={conn.home_relay ?? ""} className="font-mono">
          {conn.home_relay
            ? conn.home_connected
              ? gui.conn_home_ok(hostOf(conn.home_relay))
              : gui.conn_home_down(hostOf(conn.home_relay))
            : gui.conn_home_none}
        </span>
      </MoreFact>
      {conn.relays.map((r) => (
        <MoreFact key={r.url} name="conn-relay" label={hostOf(r.url)}>
          <span data-conn-relay={r.url} className="tabular-nums">
            {r.rtt_ms === null ? gui.conn_relay_quiet : gui.conn_relay_ms(r.rtt_ms)}
          </span>
        </MoreFact>
      ))}
      {conn.peers.map((p) => (
        <MoreFact
          key={p.id}
          name="conn-peer"
          label={p.shown}
          // The last error is a sentence about the row, on its own line
          // under the name (UX-4): beside the value it wrapped into a
          // ragged right column.
          note={
            p.last_failure && p.failure_ago_ms !== null ? (
              <span data-conn-failure>{gui.conn_peer_failed(ago(now - p.failure_ago_ms), p.last_failure)}</span>
            ) : undefined
          }
        >
          <span data-conn-peer={p.id} data-hop={p.hop} className="tabular-nums">
            {linkWord(p.hop, p.heard_ms)}
            {p.rtt_ms !== null && ` · ${gui.conn_peer_rtt(p.rtt_ms)}`}
          </span>
        </MoreFact>
      ))}
    </MoreSection>
  );
}
