import { useState, type ReactNode } from "react";

import {
  importNetworkProfile, refreshNetworkProfile, setNetworkPolicy,
  type NetworkPolicy, type NetworkSettings,
} from "@/api";
import { ActionRow } from "@/components/settings/ActionRow";
import { NavRow } from "@/components/settings/NavRow";
import { PopupRow } from "@/components/settings/PopupRow";
import { SwitchRow } from "@/components/settings/SwitchRow";
import { Button } from "@/components/ui/button";
import { Dialog, DialogHeader, DialogTitle, TaskActions, TaskOverlay } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { gui } from "@/gen/catalog";
import { MoreFact, MoreSection } from "@/views/more/MorePage";

const automatic = {
  khor_relays: true,
  n0_relays: true,
  khor_lookup: true,
  n0_lookup: true,
  n0_publish: true,
  mdns: true,
  peer_relay: true,
  advertised_relays: true,
  publish_direct: false,
  public_dns_fallback: true,
};
const privateOnly = {
  ...automatic,
  khor_relays: false,
  n0_relays: false,
  khor_lookup: false,
  n0_lookup: false,
  n0_publish: false,
  peer_relay: false,
  advertised_relays: false,
  public_dns_fallback: false,
};
type Toggle = keyof typeof automatic;
type Form = "import" | "export" | "lookup";

function matches(policy: NetworkPolicy, preset: typeof automatic) {
  return (Object.keys(preset) as Toggle[]).every((key) => policy[key] === preset[key]);
}

export type NetworkDetail = "relays" | "addressing" | "profile" | "advanced";

export function networkDetailTitle(detail: NetworkDetail) {
  return {
    relays: gui.net_relays,
    addressing: gui.net_addressing,
    profile: gui.net_profile_files,
    advanced: gui.advanced,
  }[detail];
}

export function NetworkPolicySettings({
  settings, onChange, detail, onOpenDetail, renderRelays, relayBusy, advancedRows,
}: {
  settings: NetworkSettings;
  detail: NetworkDetail | null;
  onOpenDetail: (detail: NetworkDetail) => void;
  renderRelays: (sources: ReactNode, busy: boolean) => ReactNode;
  relayBusy: boolean;
  advancedRows: ReactNode;
  onChange: (settings: NetworkSettings) => Promise<void>;
}) {
  const { profile, source } = settings;
  const { policy } = profile;
  const [form, setForm] = useState<Form | null>(null);
  const [input, setInput] = useState("");
  const [key, setKey] = useState("");
  const [pkarr, setPkarr] = useState("");
  const [dns, setDns] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const mode = matches(policy, automatic) ? "automatic" : matches(policy, privateOnly) ? "private" : "custom";
  const status = settings.invalid ? gui.net_detail_invalid
    : settings.pending_restart ? gui.net_profile_restart
    : settings.resident ? gui.net_profile_active : gui.net_profile_stopped;
  const title = form === "import" ? gui.net_profile_import
    : form === "lookup" ? gui.net_lookup_private : gui.net_profile_export;

  const run = async (request: () => Promise<NetworkSettings>, close = false) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await onChange(await request());
      if (close) setForm(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const toggle = (key: Toggle, label: string, hint?: string, disabled = false) =>
    <SwitchRow
      name={key}
      label={label}
      on={policy[key]}
      hint={hint}
      disabled={disabled || busy}
      onToggle={() => void run(() => setNetworkPolicy({ ...policy, [key]: !policy[key] }))}
    />;

  const open = (next: Form) => {
    setError(null);
    setInput("");
    setKey("");
    setPkarr(policy.pkarr.join("\n"));
    setDns(policy.dns_origins.join("\n"));
    setForm(next);
  };
  const lines = (value: string) => value.split("\n").map((line) => line.trim()).filter(Boolean);
  const submit = () => {
    if (form === "import") void run(() => importNetworkProfile(input, key.trim() || null), true);
    if (form === "lookup") void run(() => setNetworkPolicy({
      ...policy, pkarr: lines(pkarr), dns_origins: lines(dns),
    }), true);
  };

  const relaySummary = [
    (policy.khor_relays || policy.n0_relays) && gui.net_summary_public_infrastructure,
    profile.relays.length > 0 && gui.net_summary_custom_services,
    policy.advertised_relays && gui.net_summary_advertised,
    policy.peer_relay && policy.advertised_relays && gui.net_summary_peer,
  ].filter(Boolean).join(" · ") || gui.net_summary_no_sources;
  const addressingSummary = [
    (policy.khor_lookup || policy.n0_lookup || policy.n0_publish) && gui.net_summary_public_infrastructure,
    (policy.pkarr.length > 0 || policy.dns_origins.length > 0) && gui.net_summary_custom_services,
    policy.mdns && gui.net_summary_local,
  ].filter(Boolean).join(" · ") || gui.net_summary_no_sources;
  return <>
    {settings.invalid && <p role="alert" className="text-callout text-destructive">{gui.net_profile_invalid}</p>}
    {error && !form && <p role="alert" className="text-callout text-destructive">{error}</p>}
    {detail === "addressing" && <MoreSection name="network-detail-status">
      <MoreFact name="network-config-state" label={gui.net_detail_config_status}>
        <span role="status">{status}</span>
      </MoreFact>
      <MoreFact name="network-config-sources" label={gui.net_detail_config_sources}>
        {addressingSummary}
      </MoreFact>
    </MoreSection>}
    {(detail === "profile" || detail === "advanced") && settings.pending_restart && <p role="status" className="px-row text-callout text-muted-foreground">{status}</p>}
    {!detail && <>
      <MoreSection name="network-configuration">
        <PopupRow
          name="network-preset"
          label={gui.net_profile_mode}
          hint={status}
          value={mode}
          choices={[
            { value: "automatic", label: gui.net_profile_auto },
            { value: "private", label: gui.net_profile_private },
            ...(mode === "custom" ? [{ value: "custom", label: gui.net_profile_custom }] : []),
          ]}
          onPick={(value) => {
            if (value === mode || value === "custom") return;
            const preset = value === "private" ? privateOnly : automatic;
            void run(() => setNetworkPolicy({ ...policy, ...preset }));
          }}
        />
        <NavRow name="network-open-relays" label={gui.net_relays}
          value={relaySummary} onOpen={() => onOpenDetail("relays")} />
        <NavRow name="network-open-addressing" label={gui.net_addressing}
          value={addressingSummary} onOpen={() => onOpenDetail("addressing")} />
        <NavRow name="network-open-profile" label={gui.net_profile_files}
          value={gui.net_summary_profile_actions} onOpen={() => onOpenDetail("profile")} />
      </MoreSection>
      <MoreSection name="network-more">
        <NavRow name="network-open-advanced" label={gui.advanced} onOpen={() => onOpenDetail("advanced")} />
      </MoreSection>
    </>}
    {detail === "relays" && <>
      {renderRelays(<>
        {toggle("khor_relays", gui.net_relay_khor, undefined, relayBusy)}
        {toggle("n0_relays", gui.net_relay_n0, undefined, relayBusy)}
      </>, busy)}
      <MoreSection name="relays" label={gui.net_relay_forwarding} mark={{ "data-net-settings": "" }}>
        {toggle("advertised_relays", gui.net_relay_advertised, gui.net_relay_advertised_note, relayBusy)}
        {toggle("peer_relay", gui.net_relay_peers, gui.net_relay_peers_note, relayBusy || !policy.advertised_relays)}
      </MoreSection>
    </>}
    {detail === "addressing" && <>
      <MoreSection name="network-public-infrastructure" label={gui.net_summary_public_infrastructure}>
        {toggle("khor_lookup", gui.net_lookup_khor)}
        {toggle("n0_lookup", gui.net_lookup_n0)}
        {toggle("n0_publish", gui.net_publish_n0)}
      </MoreSection>
      <MoreSection name="network-addressing">
        <ActionRow name="network-lookup-custom" label={gui.net_lookup_private}
          actionLabel={gui.net_lookup_edit} onPress={() => open("lookup")} />
        {toggle("mdns", gui.net_lookup_mdns)}
      </MoreSection>
    </>}
    {detail === "profile" && <MoreSection name="network-profile-files">
      <ActionRow name="network-import" label={gui.net_profile_input}
        actionLabel={gui.net_profile_import} onPress={() => open("import")} />
      <ActionRow name="network-export" label={gui.net_profile_export}
        actionLabel={gui.net_profile_export_action} onPress={() => open("export")} />
      {source && <ActionRow name="network-source" label={gui.net_profile_source}
        hint={<span className="break-all">{source.url}</span>} actionLabel={gui.net_profile_refresh}
        onPress={() => void run(refreshNetworkProfile)} />}
    </MoreSection>}
    {detail === "advanced" && <>
      <MoreSection name="network-advanced">
        {toggle("publish_direct", gui.net_publish_direct, gui.net_publish_direct_note)}
        {toggle("public_dns_fallback", gui.net_dns_fallback, gui.net_dns_fallback_note)}
      </MoreSection>
      {advancedRows}
    </>}
    <Dialog open={form !== null} onOpenChange={(open) => !open && !busy && setForm(null)}>
      <TaskOverlay purpose="edit">
        <DialogHeader><DialogTitle>{title}</DialogTitle></DialogHeader>
        <form id="network-policy-form" className="flex flex-col gap-in" onSubmit={(e) => {
          e.preventDefault();
          submit();
        }}>
          {form === "import" && <>
            <label className="flex flex-col gap-in text-body">{gui.net_profile_input}
              <Textarea value={input} onChange={(e) => setInput(e.target.value)} spellCheck={false} required />
            </label>
            <label className="flex flex-col gap-in text-body">{gui.net_profile_key}
              <Input value={key} onChange={(e) => setKey(e.target.value)} spellCheck={false} />
            </label>
            <p className="text-callout text-muted-foreground">{gui.net_profile_key_note}</p>
          </>}
          {form === "lookup" && <>
            <p className="text-callout text-muted-foreground">{gui.net_lookup_lines}</p>
            <label className="flex flex-col gap-in text-body">{gui.net_lookup_pkarr}
              <Textarea value={pkarr} onChange={(e) => setPkarr(e.target.value)} spellCheck={false} />
            </label>
            <label className="flex flex-col gap-in text-body">{gui.net_lookup_dns}
              <Textarea value={dns} onChange={(e) => setDns(e.target.value)} spellCheck={false} />
            </label>
          </>}
          {form === "export" && <Textarea aria-label={title} value={JSON.stringify(profile, null, 2)}
            className="font-mono" rows={12} readOnly />}
          {error && form && <p role="alert" className="text-callout text-destructive">{error}</p>}
        </form>
        <TaskActions
          busy={busy}
          dirty={form === "import" ? !!(input || key) : form === "lookup"
            && (pkarr !== policy.pkarr.join("\n") || dns !== policy.dns_origins.join("\n"))}
          cancel={<Button onClick={() => setForm(null)}>{gui.delete_keep}</Button>}
          primary={form !== "export" ? <Button variant="primary" form="network-policy-form" type="submit">
            {form === "import" ? gui.net_profile_import : gui.watch_save}
          </Button> : undefined}
        />
      </TaskOverlay>
    </Dialog>
  </>;
}
