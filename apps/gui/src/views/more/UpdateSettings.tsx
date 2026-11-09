import { useEffect, useRef, useState } from "react";
import { checkAppUpdate, downloadAppUpdate, fetchAppUpdateStatus, inApp, installAppUpdate, onAppUpdate, selectUpdateCompanions, setAppUpdatePreferences, type AppUpdateStatus, type UpdatePreferences } from "@/api";
import { ActionRow } from "@/components/settings/ActionRow";
import { PopupRow } from "@/components/settings/PopupRow";
import { SwitchRow } from "@/components/settings/SwitchRow";
import { gui } from "@/gen/catalog";
import { MoreFact, MoreFold, MoreSection } from "./MorePage";

function stateLabel(status: AppUpdateStatus): string {
  switch (status.phase) {
    case "checking": return gui.update_checking;
    case "available": return gui.update_available;
    case "downloading": return gui.update_downloading(status.total ? Math.min(100, Math.floor(Number(status.received) * 100 / Number(status.total))) : 0);
    case "verifying": return gui.update_verifying;
    case "ready": return gui.update_ready;
    case "installing": case "awaiting-health": return gui.update_installing;
    case "complete": return gui.update_complete;
    case "failed": return status.retry === "check" ? gui.update_check_failed
      : status.retry === "download" ? gui.update_download_failed
      : status.retry === "install" ? gui.update_install_failed : gui.update_failed;
    case "idle": return status.checked_at ? gui.update_app_current : gui.update_label;
  }
}

type UpdateWork = () => Promise<AppUpdateStatus>;
type OperationFailure = { message: string; retry: UpdateWork };

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function FailureDetails({ message }: { message: string }) {
  return <div data-update-failure>
    <MoreFold name="update-error" label={gui.update_error_details}>
      <p className="select-text whitespace-pre-wrap px-row text-callout text-muted-foreground [overflow-wrap:anywhere]">{message}</p>
    </MoreFold>
  </div>;
}

export function UpdateSettings() {
  const [status, setStatus] = useState<AppUpdateStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<OperationFailure | null>(null);
  const running = useRef(false);
  const alive = useRef(false);
  useEffect(() => {
    if (!inApp) return;
    alive.current = true;
    const receive = (next: AppUpdateStatus) => { if (alive.current) { setStatus(next); } };
    const stop = onAppUpdate(receive);
    void fetchAppUpdateStatus().then(receive).catch(error => { if (alive.current) setFailure({ message: errorMessage(error), retry: fetchAppUpdateStatus }); });
    return () => { alive.current = false; stop(); };
  }, []);
  if (!inApp) return null;
  const run = (work: UpdateWork) => {
    if (running.current) return;
    running.current = true;
    setBusy(true); setFailure(null);
    void work().then(next => { if (alive.current) setStatus(next); }).catch(error => {
      if (alive.current) setFailure({ message: errorMessage(error), retry: work });
    }).finally(() => {
      running.current = false;
      if (alive.current) setBusy(false);
    });
  };
  const working = busy || (status !== null && ["checking", "downloading", "verifying", "installing", "awaiting-health"].includes(status.phase));
  const operationFailure = failure && <div data-update-operation-failure>
    <ActionRow name="update-retry" label={gui.update_failed} actionLabel={gui.region_retry}
      busy={working} onPress={() => run(failure.retry)} />
    <FailureDetails message={failure.message} />
  </div>;
  if (!status) return <MoreSection name="app-update">{operationFailure ??
    <ActionRow name="update-loading" label={gui.update_checking} actionLabel={gui.update_check} busy onPress={() => {}} />
  }</MoreSection>;
  if (status.source !== "direct") {
    const hint = status.source === "store" ? gui.update_store : status.source === "development" ? gui.update_development : gui.update_external;
    return <MoreSection name="app-update"><MoreFact name="update-source" label={gui.update_label}>{hint}</MoreFact></MoreSection>;
  }

  const retry = status.phase === "failed";
  const download = status.phase === "available";
  // A failed check may retain an older offer; only the backend knows which
  // operation failed. Older backends safely refresh instead of guessing.
  const retryWork = status.retry === "install" ? installAppUpdate
    : status.retry === "download" ? downloadAppUpdate : checkAppUpdate;
  const ready = status.phase === "ready";
  const blocked = status.blocked.map(reason => ({"unsaved-files":gui.update_save_files,"attachments":gui.update_finish_attachments,"unconfirmed-sends":gui.update_finish_sends,"draft-storage":gui.update_draft_storage,"navigation-storage":gui.update_draft_storage,"window-not-ready":gui.update_window_not_ready,"input-in-progress":gui.update_finish_input,"user-active":gui.update_later})[reason]).join(" · ");
  const preferences = (next: Partial<UpdatePreferences>) => run(() => setAppUpdatePreferences({ ...status.preferences, ...next }));
  return (
    <>
    <MoreSection name="app-update">
      <MoreFact name="update-current-version" label={gui.update_current_version}>{status.current}</MoreFact>
      <ActionRow name="update-check" label={stateLabel(status)} value={status.available ?? undefined}
        hint={blocked || undefined} actionLabel={retry ? gui.region_retry : ready ? (status.restart_required ? gui.update_restart : gui.update_apply_selected) : download ? gui.update_download : gui.update_check} busy={working} onPress={() => run(retry ? retryWork : ready ? installAppUpdate : download ? downloadAppUpdate : checkAppUpdate)} />
      {status.phase === "failed" && status.failure && <FailureDetails message={status.failure} />}
      {operationFailure}
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
    {status.companions.map((companion, index) => (
      <MoreSection key={companion.id} name={`update-companion-${index}`} label={gui.update_companion}>
        <MoreFact name={`update-cli-version-${index}`} label={gui.identity_version} note={companion.path}>{companion.current}</MoreFact>
        <SwitchRow name={`update-cli-selected-${index}`} label={gui.update_include_cli}
          hint={companion.available ?? undefined} on={companion.selected} onToggle={() => {
            if (working) return;
            const selected = status.companions.filter(item => item.selected && item.id !== companion.id).map(item => item.id);
            if (!companion.selected) selected.push(companion.id);
            run(() => selectUpdateCompanions(selected));
          }} />
        <MoreFact name={`update-cli-result-${index}`} label={gui.axis_state}>
          {companion.phase === "complete" ? gui.update_cli_complete : companion.phase === "failed" ? gui.update_failed : companion.phase === "installing" ? gui.update_installing : companion.available ? gui.update_available : gui.update_component_current}
        </MoreFact>
        {companion.failure && <FailureDetails message={companion.failure} />}
      </MoreSection>
    ))}
    </>
  );
}
