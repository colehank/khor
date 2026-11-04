import { useEffect, useRef, useState } from "react";
import { checkAppUpdate, downloadAppUpdate, fetchAppUpdateStatus, inApp, installAppUpdate, onAppUpdate, setAppUpdatePreferences, type AppUpdateStatus, type UpdatePreferences } from "@/api";
import { ActionRow } from "@/components/settings/ActionRow";
import { PopupRow } from "@/components/settings/PopupRow";
import { SwitchRow } from "@/components/settings/SwitchRow";
import { gui } from "@/gen/catalog";
import { MoreFact, MoreSection } from "./MorePage";

function stateLabel(status: AppUpdateStatus): string {
  switch (status.phase) {
    case "checking": return gui.update_checking;
    case "available": return gui.update_available;
    case "downloading": return gui.update_downloading(status.total ? Math.min(100, Math.floor(Number(status.received) * 100 / Number(status.total))) : 0);
    case "verifying": return gui.update_verifying;
    case "ready": return gui.update_ready;
    case "installing": case "awaiting-health": return gui.update_installing;
    case "complete": return gui.update_complete;
    case "failed": return gui.update_failed;
    case "idle": return status.checked_at ? gui.update_current : gui.update_label;
  }
}

export function UpdateSettings() {
  const [status, setStatus] = useState<AppUpdateStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const alive = useRef(false);
  useEffect(() => {
    if (!inApp) return;
    alive.current = true;
    const receive = (next: AppUpdateStatus) => { if (alive.current) { setStatus(next); setFailed(false); } };
    const stop = onAppUpdate(receive);
    void fetchAppUpdateStatus().then(receive).catch(() => { if (alive.current) setFailed(true); });
    return () => { alive.current = false; stop(); };
  }, []);
  if (!inApp) return null;
  const run = (work: () => Promise<AppUpdateStatus>) => {
    if (busy) return;
    setBusy(true); setFailed(false);
    void work().then(next => { if (alive.current) setStatus(next); }).catch(() => { if (alive.current) setFailed(true); })
      .finally(() => { if (alive.current) setBusy(false); });
  };
  if (!status) return failed ? (
    <MoreSection name="app-update"><ActionRow name="update-retry" label={gui.update_failed} actionLabel={gui.region_retry} busy={busy} onPress={() => run(fetchAppUpdateStatus)} /></MoreSection>
  ) : null;
  if (status.source !== "direct") {
    const hint = status.source === "store" ? gui.update_store : status.source === "development" ? gui.update_development : gui.update_external;
    return <MoreSection name="app-update"><MoreFact name="update-source" label={gui.update_label}>{hint}</MoreFact></MoreSection>;
  }
  const working = busy || ["checking", "downloading", "verifying", "installing", "awaiting-health"].includes(status.phase);
  const download = status.phase === "available" || (status.phase === "failed" && status.available !== null);
  const ready = status.phase === "ready";
  const blocked = status.blocked.map(reason => ({"unsaved-files":gui.update_save_files,"attachments":gui.update_finish_attachments,"unconfirmed-sends":gui.update_finish_sends,"draft-storage":gui.update_draft_storage,"navigation-storage":gui.update_draft_storage,"window-not-ready":gui.update_window_not_ready,"input-in-progress":gui.update_finish_input,"user-active":gui.update_later})[reason]).join(" · ");
  const preferences = (next: Partial<UpdatePreferences>) => run(() => setAppUpdatePreferences({ ...status.preferences, ...next }));
  return (
    <MoreSection name="app-update">
      <ActionRow name="update-check" label={failed ? gui.update_failed : stateLabel(status)} value={status.available ?? undefined}
        hint={blocked || undefined} actionLabel={ready ? gui.update_restart : download ? gui.update_download : gui.update_check} busy={working} onPress={() => run(ready ? installAppUpdate : download ? downloadAppUpdate : checkAppUpdate)} />
      <PopupRow name="update-channel" label={gui.update_channel} value={status.preferences.channel}
        choices={[{ value: "stable", label: gui.update_stable }, { value: "beta", label: gui.update_beta }]}
        onPick={value => { if (!working && (value === "stable" || value === "beta")) preferences({ channel: value }); }} />
      <SwitchRow name="update-automatic-check" label={gui.update_automatic_check} on={status.preferences.automatic_check}
        onToggle={() => { if (!working) preferences({ automatic_check: !status.preferences.automatic_check }); }} />
      <SwitchRow name="update-automatic-download" label={gui.update_automatic_download} on={status.preferences.automatic_download}
        onToggle={() => { if (!working) preferences({ automatic_download: !status.preferences.automatic_download }); }} />
      <SwitchRow name="update-install-on-start" label={gui.update_install_on_start} hint={gui.update_install_on_start_hint} on={status.preferences.install_on_start}
        onToggle={() => { if (!working) preferences({ install_on_start: !status.preferences.install_on_start }); }} />
    </MoreSection>
  );
}
