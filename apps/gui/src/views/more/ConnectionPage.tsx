import { useEffect, useState } from "react";

import { fetchAbout, setPaused, type About, type DeviceRow } from "@/api";
import { SwitchRow } from "@/components/settings/SwitchRow";
import { gui } from "@/gen/catalog";
import { NetSettings } from "@/views/NetSettings";
import { networkDetailTitle, type NetworkDetail } from "@/views/NetworkPolicySettings";
import { ConfigSections } from "@/views/more/ConfigRows";
import { MoreFrame, MoreSection } from "@/views/more/MorePage";

export function ConnectionPage({
  devices,
  devicesRead,
  onClose,
  onChanged,
}: {
  /** The shell's polled table supplies active relay paths and network membership. */
  devices: DeviceRow[];
  devicesRead: boolean;
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
  const [detail, setDetail] = useState<NetworkDetail | null>(null);
  const alone = devices.every((d) => d.me);

  const advancedRows = <>
    {error && <p data-net-error role="alert" className="text-callout text-destructive">{error}</p>}
    <ConfigSections page="connection" omit={["peer-hop"]} onChanged={onChanged} />
    {!alone && about && (
      <MoreSection name="sync">
        <SwitchRow
          name="net-pause"
          label={gui.net_pause_sync}
          hint={gui.net_pause_note}
          on={about.paused}
          mark={{ "data-net-pause": "", ...(about.paused ? { "data-paused": "" } : {}) }}
          onToggle={() => {
            setError(null);
            setPaused(!about.paused).then(
              (now) => {
                setAbout(now);
                onChanged();
              },
              (e: unknown) => setError(e instanceof Error ? e.message : String(e)),
            );
          }}
        />
      </MoreSection>
    )}
  </>;

  return (
    <MoreFrame
      page="connection"
      detailTitle={detail ? networkDetailTitle(detail) : undefined}
      onClose={() => detail ? setDetail(null) : onClose()}
    >
      <NetSettings detail={detail} onOpenDetail={setDetail} devices={devices} devicesRead={devicesRead} advancedRows={advancedRows} />
    </MoreFrame>
  );
}
