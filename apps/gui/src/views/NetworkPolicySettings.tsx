import { useState, type ReactNode } from "react";

import {
  importNetworkProfile, refreshNetworkProfile, setNetworkPolicy,
  type NetworkPolicy, type NetworkSettings,
} from "@/api";
import { ActionRow } from "@/components/settings/ActionRow";
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

export function NetworkPolicySettings({
  settings, onChange, relayRows,
}: {
  settings: NetworkSettings;
  relayRows: ReactNode;
  onChange: (settings: NetworkSettings) => void;
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
  const status = settings.pending_restart ? gui.net_profile_restart
    : settings.resident ? gui.net_profile_active : gui.net_profile_stopped;
  const title = form === "import" ? gui.net_profile_import
    : form === "lookup" ? gui.net_lookup_private : gui.net_profile_export;

  const run = async (request: () => Promise<NetworkSettings>, close = false) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      onChange(await request());
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

  return <>
    <MoreSection name="network-configuration" label={gui.net_profile}>
      <PopupRow
        name="network-preset"
        label={gui.net_profile_mode}
        hint={mode === "private" ? gui.net_profile_private_note : gui.net_profile_auto_note}
        value={mode}
        choices={[
          { value: "automatic", label: gui.net_profile_auto },
          { value: "private", label: gui.net_profile_private },
          ...(mode === "custom" ? [{ value: "custom", label: gui.net_profile_custom }] : []),
        ]}
        onPick={(value) => {
          if (value === "custom") return;
          const preset = value === "private" ? privateOnly : automatic;
          void run(() => setNetworkPolicy({ ...policy, ...preset }));
        }}
      />
      <MoreFact name="network-resident" label={gui.net_profile_state}>{status}</MoreFact>
      {settings.invalid && <p className="px-row py-in text-callout text-destructive">{gui.net_profile_invalid}</p>}
      {error && !form && <p role="alert" className="px-row py-in text-callout text-destructive">{error}</p>}
      <ActionRow name="network-import" label={gui.net_profile_import}
        actionLabel={gui.net_profile_import} onPress={() => open("import")} />
      <ActionRow name="network-export" label={gui.net_profile_export}
        actionLabel={gui.net_profile_export_action} onPress={() => open("export")} />
      {source && <ActionRow name="network-source" label={gui.net_profile_source}
        hint={<span className="break-all">{source.url}</span>} actionLabel={gui.net_profile_refresh}
        onPress={() => void run(refreshNetworkProfile)} />}
    </MoreSection>
    <MoreSection name="relays" label={gui.net_relays} mark={{ "data-net-settings": "" }}>
      {toggle("khor_relays", gui.net_relay_khor)}
      {toggle("n0_relays", gui.net_relay_n0)}
      {toggle("advertised_relays", gui.net_relay_advertised, gui.net_relay_advertised_note)}
      {relayRows}
    </MoreSection>
    <MoreSection name="network-addressing" label={gui.net_addressing}>
      {toggle("khor_lookup", gui.net_lookup_khor)}
      {toggle("n0_lookup", gui.net_lookup_n0)}
      {toggle("n0_publish", gui.net_publish_n0)}
      <ActionRow name="network-lookup-custom" label={gui.net_lookup_private}
        actionLabel={gui.net_lookup_edit} onPress={() => open("lookup")} />
      {toggle("publish_direct", gui.net_publish_direct, gui.net_publish_direct_note)}
      {toggle("public_dns_fallback", gui.net_dns_fallback, gui.net_dns_fallback_note)}
    </MoreSection>
    <MoreSection name="network-device-paths" label={gui.net_device_paths}>
      {toggle("mdns", gui.net_lookup_mdns)}
      {toggle("peer_relay", gui.net_relay_peers, gui.net_relay_peers_note, !policy.advertised_relays)}
    </MoreSection>
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
