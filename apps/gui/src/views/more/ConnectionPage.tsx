// 连接, 设置's fifth page (UX-16, the owner 2026-10-05: 中继, 首选中继,
// 暂停同步). Settings only: the readings that stood on 网络 beside them
// (本机, 已连接的机器, 直连占比, the per-machine 连接 rows) are on 诊断
// now (`NetStatusSections`), and the links themselves on 我的 khor's map.
//
// ⚠️ 墓碑 (UX-16): this was 网络 (`network`), a page of status, settings
// and 离开网络; 离开网络 is on 我的 khor with the identity's other acts.
import { useEffect, useState } from "react";

import { fetchAbout, setPaused, type About, type DeviceRow } from "@/api";
import { SwitchRow } from "@/components/settings/SwitchRow";
import { gui } from "@/gen/catalog";
import { NetSettings } from "@/views/NetSettings";
import { ConfigSections } from "@/views/more/ConfigRows";
import { MoreFrame, MoreSection } from "@/views/more/MorePage";

export function ConnectionPage({
  devices,
  onClose,
  onChanged,
}: {
  /** The shell's own polled table: only whether this machine is in a
      network is read off it. */
  devices: DeviceRow[];
  onClose: () => void;
  /** The device table changed (暂停 paints on every machine's row). */
  onChanged: () => void;
}) {
  // 暂停 is read off this answer rather than off the device table's
  // `paused`: the file is the thing `khor pause` writes and the table is
  // a report about it.
  const [about, setAbout] = useState<About | null>(null);
  useEffect(() => {
    void fetchAbout().then(setAbout, () => setAbout(null));
  }, []);
  const [error, setError] = useState<string | null>(null);
  const alone = devices.every((d) => d.me);

  return (
    <MoreFrame page="connection" onClose={onClose}>
      {error && (
        <p data-net-error className="text-callout text-destructive">
          {error}
        </p>
      )}
      {/* 中继 and 首选中继 (and 只使用自己的中继): see `NetSettings` for
          why it is read on arrival. */}
      <NetSettings />
      {/* 通过 SSH 转连 and 通过其他机器转发 (`khor config`'s
          `connection` section, rounds ROAD-2 and ROAD-3): drawn from the
          node's own rows like every other page's settings. */}
      <ConfigSections page="connection" onChanged={onChanged} />
      {!alone && about && (
        <MoreSection name="sync">
          {/* 暂停同步: **a switch** — one state, taken at once, undone by the
              same press. Not drawn on a machine that is in no network:
              `khor pause` there is an act with no audience. The fresh
              `About` is the answer: the switch shows what the file now
              says, never what the press assumed. **The rail paints off
              the device table, not off this page**, so the shell re-reads
              it too (the rail's face and this machine's row say 已暂停). */}
          {!alone && about && (
            <SwitchRow
              name="net-pause"
              label={gui.net_pause_sync}
              hint={gui.net_pause_note}
              on={about.paused}
              mark={
                {
                  "data-net-pause": "",
                  "data-paused": about.paused ? "" : undefined,
                } as Record<string, string>
              }
              onToggle={() => {
                setError(null);
                setPaused(!about.paused).then(
                  (now) => {
                    setAbout(now);
                    onChanged();
                  },
                  (e: unknown) =>
                    setError(String(e instanceof Error ? e.message : e)),
                );
              }}
            />
          )}
        </MoreSection>
      )}
    </MoreFrame>
  );
}
