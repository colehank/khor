// 网络的状态 — the readings 网络 carried until UX-16 (本机, 已连接的机器,
// 直连占比) and its 连接 group, machine by machine: they are readings,
// not settings, so they stand on 诊断 now, under the checks. The map in
// 我的 khor says the same links as a picture; these say them as rows a
// person can screenshot.
import { useEffect, useState } from "react";

import {
  fetchAbout,
  fetchRoadShare,
  type About,
  type DeviceRow,
  type RoadShare,
} from "@/api";
import { gui } from "@/gen/catalog";
import { hopWord } from "@/words";
import { ConnectionsSection } from "@/views/ConnectionsSection";
import { MoreFact, MoreSection } from "@/views/more/MorePage";

/** Between two facts on one line — the separator `ImportDialog` and
    额度's rows put between theirs. */
const DOT = " · ";

export function NetStatusSections({ devices }: { devices: DeviceRow[] }) {
  // 暂停 is read off this answer rather than off the device table's
  // `paused`: the file is the thing `khor pause` writes.
  const [about, setAbout] = useState<About | null>(null);
  useEffect(() => {
    void fetchAbout().then(setAbout, () => setAbout(null));
  }, []);
  // 直连占比 (docs/CONNECT.md 度量与验收): the resident's own memory of
  // its dials. A failed ask leaves the row out.
  const [share, setShare] = useState<RoadShare | null>(null);
  useEffect(() => {
    void fetchRoadShare().then(setShare, () => setShare(null));
  }, []);
  const me = devices.find((d) => d.me) ?? null;
  const others = devices.filter((d) => !d.me);
  const alone = others.length === 0;
  // Connected is a road, and the road is the node's word (`hop`).
  const roads = (["direct", "relay"] as const)
    .map((k) => ({ k, n: others.filter((d) => d.hop === k).length }))
    .filter((r) => r.n > 0);
  const connected = roads.reduce((a, r) => a + r.n, 0);
  const noServe = !alone && others.every((d) => d.hop === "no-serve");
  const state = about?.paused
    ? gui.machine_paused
    : noServe
      ? hopWord("no-serve")
      : gui.net_online;
  return (
    <>
      <MoreSection name="status" label={gui.network}>
        {/* 本机: its name and whether it is talking — one fact row, the
            two halves the way the machines list says them (name · word). */}
        <MoreFact name="net-me" label={gui.machine_here}>
          {me ? (
            <>
              {me.shown}
              {about && (
                <>
                  {DOT}
                  <span
                    data-net-state={
                      about.paused ? "paused" : noServe ? "no-serve" : "online"
                    }
                  >
                    {state}
                  </span>
                </>
              )}
            </>
          ) : null}
        </MoreFact>
        {/* A machine that has paired with nobody gets the sentence the
            page has always said then: 「怎么再加一台」 is the only
            question there is, and the two buttons in the head answer it. */}
        <MoreFact name="net-connected" label={gui.net_connected}>
          {alone ? (
            <span data-net-alone>{gui.identity_alone}</span>
          ) : roads.length > 0 ? (
            gui.net_connected_n(
              connected,
              roads.map((r) => gui.net_via(r.n, hopWord(r.k))).join(DOT),
            )
          ) : (
            gui.net_connected_bare(0)
          )}
        </MoreFact>
        {/* 直连占比: of the dials this machine's serve made lately that
            settled on a road, how many went direct — the answer to
            「这张网还要不要中继」 over time, where 已连接的机器 above is
            the answer right now. Out of how many is part of the fact:
            100% of one dial is not 100% of twenty. No reading says
            why — serve 未运行 in the word the row above uses for it. */}
        {!alone && share && (
          <MoreFact name="net-direct-share" label={gui.roads_share_label}>
            <span data-net-direct-share={share.unread ?? share.settled}>
              {share.unread === "no-serve"
                ? hopWord("no-serve")
                : share.unread
                  ? gui.roads_unanswered
                  : share.settled === 0
                    ? gui.roads_none
                    : gui.roads_share(
                        Math.round((100 * share.direct) / share.settled),
                        share.settled,
                      )}
            </span>
          </MoreFact>
        )}
      </MoreSection>
      {/* Machine by machine (round NET-2). Not on a machine in no
          network: there is nothing to be connected to. */}
      {!alone && <ConnectionsSection />}
    </>
  );
}
