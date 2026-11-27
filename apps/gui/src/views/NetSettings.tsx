// Relay endpoints and their local credentials on the connection settings page.
import { useEffect, useState, type ReactNode } from "react";

import { fetchNetworkSettings, fetchRelays, setPreferredRelay, setRelays, type DeviceRow, type NetworkSettings, type RelayChoices, type RelayWant } from "@/api";
import { NetworkPolicySettings, type NetworkDetail } from "./NetworkPolicySettings";
import { ActionRow } from "@/components/settings/ActionRow";
import { IconCheck } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Dialog, TaskOverlay, TaskActions, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { gui } from "@/gen/catalog";
import { MoreSection } from "@/views/more/MorePage";
import { RegionState, Waiting } from "@/components/RegionState";

/** A relay to keep as it is stored — its key, if any, stays put. */
const kept = (url: string): RelayWant => ({ url, token: null });

function relayAddress(url: string) {
  try {
    return new URL(url);
  } catch {
    return null;
  }
}

const relayKey = (url: string) => relayAddress(url)?.href ?? url;

function RelayPaths({ choices, devices, settings, busy, onPick, onRemove }: {
  choices: RelayChoices;
  devices: DeviceRow[];
  settings: NetworkSettings;
  busy: boolean;
  onPick?: (url: string) => void;
  onRemove: (url: string) => void;
}) {
  // A relay's live reading is host and non-default port. Only a unique
  // match can identify a configured URL, which also carries its scheme.
  const selectable = new Map(choices.in_use.map((url) => [relayKey(url), url]));
  const configured = new Map([...selectable, ...choices.mine.map((url) => [relayKey(url), url] as const)]);
  const configuredByHost = new Map<string, string[]>();
  for (const url of configured.values()) {
    const host = relayAddress(url)?.host;
    if (!host) continue;
    const entries = configuredByHost.get(host) ?? [];
    entries.push(url);
    configuredByHost.set(host, entries);
  }

  const used = new Set<string>();
  const otherRelays = new Map<string | null, string>();
  const forwarding = new Set<string | null>();
  for (const device of devices) {
    if (device.me) continue;
    if (device.hop === "forwarded") forwarding.add(device.via);
    if (device.hop !== "relay") continue;
    const matches = device.relay ? configuredByHost.get(device.relay.toLowerCase()) ?? [] : [];
    if (matches.length === 1) {
      used.add(matches[0]);
      continue;
    }
    const advertised = device.relay && matches.length === 0
      && !settings.pending_restart && !settings.invalid && settings.profile.policy.advertised_relays;
    otherRelays.set(device.relay, advertised ? gui.net_summary_advertised : gui.net_relay_unidentified);
  }

  const preferred = choices.preferred && relayKey(choices.preferred);
  return <>
    {[...configured].map(([key, url]) => {
      const mine = choices.mine.includes(url);
      const candidate = selectable.get(key);
      return <RelayPathRow key={url} label={url} kind="configured"
        source={mine ? gui.net_summary_custom_services : gui.net_relays_configured}
        active={used.has(url)} plainHttp={url.startsWith("http://")}
        keyed={choices.keyed.includes(url)} preferred={key === preferred} busy={busy}
        pick={onPick && candidate ? () => onPick(candidate) : undefined}
        remove={mine ? () => onRemove(url) : undefined} />;
    })}
    {[...otherRelays].map(([host, source]) => (
      <RelayPathRow key={host ?? "unnamed-relay"} label={host ?? gui.net_relay_unknown_server}
        kind="relay" source={source} active />
    ))}
    {[...forwarding].map((via) => (
      <RelayPathRow key={`via:${via ?? ""}`} label={via ?? gui.net_relay_unknown_device}
        kind="forwarding" source={gui.net_summary_peer} active />
    ))}
    {configured.size === 0 && otherRelays.size === 0 && forwarding.size === 0 && (
      <p className="text-callout text-muted-foreground">{gui.net_detail_no_relay_routes}</p>
    )}
  </>;
}

function RelayPathRow({ label, kind, source, active, plainHttp, keyed, preferred, busy, pick, remove }: {
  label: string;
  kind: "configured" | "relay" | "forwarding";
  source: string;
  active: boolean;
  plainHttp?: boolean;
  keyed?: boolean;
  preferred?: boolean;
  busy?: boolean;
  pick?: () => void;
  remove?: () => void;
}) {
  const Row = pick ? "button" : "div";
  return (
    <div data-more-row data-relay-path={kind} data-relay-active={active}
      data-relay-mine={remove ? label : undefined} className="flex min-w-0 items-center gap-row">
      <Row className={`flex min-w-0 flex-1 items-center gap-row text-left ${pick ? "group/relay min-h-touch rounded-sm focus-visible:outline-2 focus-visible:outline-ring" : ""}`}
        {...(pick ? { type: "button" as const, onClick: pick, disabled: busy, "aria-pressed": !!preferred, "data-relay-pick": "" } : {})}>
        <span className="flex min-w-0 flex-1 flex-col">
          <span data-selectable={pick ? undefined : ""} className={`text-body [overflow-wrap:anywhere] ${pick && !preferred && !busy ? "underline-offset-2 group-hover/relay:underline" : ""}`}>{label}</span>
          <span className="text-callout text-muted-foreground">
            {source}
            {plainHttp && <> · {gui.relays_scheme_http}</>}
            {keyed && <span data-relay-keyed> · {gui.relays_keyed}</span>}
            {preferred && <> · <span data-relay-preferred className="text-foreground">{gui.relays_preferred}</span></>}
          </span>
        </span>
        {active && <span data-relay-path-status className="flex flex-none items-center gap-xxs text-callout text-foreground">
          <IconCheck className="size-affordance" />
          {gui.relays_in_use}
        </span>}
      </Row>
      {remove && <Button onClick={remove} disabled={busy} data-setting-trail className="min-h-touch min-w-touch flex-none">{gui.remove}</Button>}
    </div>
  );
}

/** Mounting is reading: the page is drawn only while it is on screen. */
export function NetSettings({ detail, onOpenDetail, devices, advancedRows }: {
  detail: NetworkDetail | null;
  onOpenDetail: (detail: NetworkDetail) => void;
  devices: DeviceRow[];
  advancedRows: ReactNode;
}) {
  const [choices, setChoices] = useState<RelayChoices | null>(null);
  const [network, setNetwork] = useState<NetworkSettings | null>(null);
  /** When the first asking began (`Waiting`, below). */
  const [since] = useState(() => Date.now());
  const [error, setError] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [choosingPreferred, setChoosingPreferred] = useState(false);
  const [writingRelay, setWritingRelay] = useState(false);
  const [readAttempt, setReadAttempt] = useState(0);

  useEffect(() => {
    let alive = true;
    fetchNetworkSettings().then(
      (settings) => { if (alive) setNetwork(settings); },
      (e: unknown) => { if (alive) setError(e instanceof Error ? e.message : String(e)); },
    );
    fetchRelays()
      .then((r) => {
        if (alive) setChoices(r);
      })
      .catch((e: unknown) => {
        if (alive) setError(String(e instanceof Error ? e.message : e));
      });
    return () => {
      alive = false;
    };
  }, [readAttempt]);

  const write = async (request: () => Promise<void>) => {
    if (writingRelay) return;
    setWritingRelay(true);
    setError(null);
    try {
      await request();
      setNetwork(await fetchNetworkSettings());
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setWritingRelay(false);
    }
  };

  const pickPreferred = (url: string | null) => {
    if (url === (choices?.preferred ?? null)) return;
    void write(async () => {
      await setPreferredRelay(url);
      setChoices((before) => before && { ...before, preferred: url });
      setChoosingPreferred(false);
    });
  };

  const retry = () => {
    setError(null);
    setReadAttempt((attempt) => attempt + 1);
  };

  const canChoose = !!choices?.in_use.length;
  const manual = !!choices?.preferred || (choosingPreferred && canChoose);
  const relayPaths = detail === "relays" && network && (
    <MoreSection name="relay-paths" label={gui.net_relay_path_list}>
      {choices ? <>
        <div className="flex flex-wrap items-center justify-between gap-x-row gap-y-in">
          <span className="text-body">{gui.net_relay_selection}</span>
          <SegmentedControl
            label={gui.net_relay_selection}
            value={manual ? "manual" : "auto"}
            options={[
              { key: "auto", label: gui.net_relay_select_auto, disabled: writingRelay },
              { key: "manual", label: gui.net_relay_select_manual, disabled: writingRelay || !canChoose },
            ]}
            onChange={(mode) => {
              if (mode === "manual") setChoosingPreferred(true);
              else if (choices.preferred) pickPreferred(null);
              else setChoosingPreferred(false);
            }}
            mark={{ "data-relay-selection": "" }}
          />
          {manual && <p className="w-full text-callout text-muted-foreground">
            {choices.preferred ? gui.relays_preferred_note : gui.net_relay_pick_prompt}
          </p>}
        </div>
        {error && <p data-relays-error role="alert" className="text-callout text-destructive">{error}</p>}
        <RelayPaths choices={choices} devices={devices} settings={network} busy={writingRelay}
          onPick={manual ? pickPreferred : undefined} onRemove={(url) => {
          void write(async () => {
            const next = await setRelays(choices.mine.filter((entry) => entry !== url).map(kept), choices.only_mine);
            setChoices(next);
            if (!next.in_use.length) setChoosingPreferred(false);
          });
        }} />
        <ActionRow name="relay-add" label={gui.relays_mine} actionLabel={gui.relays_add}
          onPress={() => setAdding(true)} busy={writingRelay} mark={{ "data-relay-add": "" }} />
      </> : error ? <RegionState state="error" message={error} onRetry={retry} /> : <Waiting since={since} />}
    </MoreSection>
  );

  const relayRows = choices && (
    <RelayAddDialog
      open={adding}
      onClose={() => setAdding(false)}
      onWritten={(next) => {
        setChoices(next);
        void fetchNetworkSettings().then(setNetwork).catch((e: unknown) => {
          setError(e instanceof Error ? e.message : String(e));
        });
      }}
    />
  );

  if (!network) {
    return error ? <RegionState state="error" message={error} onRetry={retry} /> : <Waiting since={since} />;
  }

  return <NetworkPolicySettings
    settings={network}
    detail={detail}
    onOpenDetail={(next) => {
      setChoosingPreferred(false);
      onOpenDetail(next);
    }}
    relayPaths={relayPaths}
    relayRows={relayRows}
    advancedRows={advancedRows}
    onChange={(settings) => {
      setNetwork(settings);
      void fetchRelays().then(setChoices).catch((e: unknown) => {
        setError(e instanceof Error ? e.message : String(e));
      });
    }}
  />;
}

/**
 * **The one window that adds a relay** (2026-09-30 docs/NET.md 中继; moved
 * out of the group on 2026-10-06 so the 我的 khor dial's add menu opens
 * the same one): the address and, when its operator set one, the shared
 * key — masked, the one moment it is on screen at all. It reads the
 * relays as they stand when it opens (`fetchRelays`, a local read), so a
 * caller need not hold them; the whole list goes back in one write
 * (`Node::set_relays` replaces), and the node's own sentence names an
 * entry it refused — the address stays in the box next to it, so the
 * fix is an edit and not a retype. Closed only on success.

 */
export function RelayAddDialog({
  open,
  onClose,
  onWritten,
}: {
  open: boolean;
  onClose: () => void;
  /** The node's answer after the write: both halves of the relay list. */
  onWritten?: (choices: RelayChoices) => void;
}) {
  const [saving,setSaving]=useState(false);
  const [typed, setTyped] = useState("");
  const [typedKey, setTypedKey] = useState("");
  const [qad, setQad] = useState("");
  const [refused, setRefused] = useState<string | null>(null);
  const [was, setWas] = useState<RelayChoices | null>(null);
  useEffect(() => {
    if (!open) return;
    setWas(null);
    setTyped("");
    setTypedKey("");
    setQad("");
    setRefused(null);
    let alive = true;
    fetchRelays().then(
      (r) => alive && setWas(r),
      (e: unknown) => alive && setRefused(String(e instanceof Error ? e.message : e)),
    );
    return () => {
      alive = false;
    };
  }, [open]);
  return (
    <Dialog open={open} onOpenChange={(o) => !o && onClose()}>
      <TaskOverlay purpose="edit" data-relay-add-dialog>
        <DialogHeader>
          <DialogTitle>{gui.relays_add}</DialogTitle>
        </DialogHeader>
        <form
          id="relay-add"
          className="flex flex-col gap-in"
          onSubmit={async (e) => {
            e.preventDefault();
            if (saving || !was) return;
            const url = typed.trim();
            // Always pressable (`components/Why`): with no address,
            // say so and put the caret in its box.
            if (!url) {
              setRefused(gui.need_relay);
              document.querySelector<HTMLElement>("[data-relay-add-url]")?.focus();
              return;
            }
            const token = typedKey.trim() || null;
            const mine = was.mine;
            setSaving(true);
            try {
              const next = await setRelays([...mine.filter((u) => u !== url).map(kept), { url, token, qad: qad ? Number(qad) : undefined }], was.only_mine);
              onWritten?.(next);
              onClose();
            } catch (err) {
              setRefused(String(err instanceof Error ? err.message : err));
            } finally {setSaving(false);}
          }}
        >
          <Input
            data-relay-add-url
            autoComplete="off"
            aria-label={gui.relays_add}
            placeholder={gui.relays_hint}
            value={typed}
            onChange={(e) => setTyped(e.target.value)}
            className="min-h-touch w-full font-mono"
          />
          {/* The relay's shared key, when its operator set one
              (`khor relay --token`, iroh-relay's shared_token).
              Masked: it is a secret, and this is the one moment
              it is on screen at all. */}
          <Input
            data-relay-add-key
            type="password"
            autoComplete="off"
            aria-label={gui.relays_key}
            placeholder={gui.relays_key_hint}
            value={typedKey}
            onChange={(e) => setTypedKey(e.target.value)}
            className="min-h-touch w-full font-mono"
          />
          <Input
            type="number"
            min={1}
            max={65535}
            aria-label={gui.net_qad_port}
            placeholder={gui.net_qad_port}
            value={qad}
            onChange={(e) => setQad(e.target.value)}
            className="min-h-touch w-full font-mono"
          />
        </form>
        {refused && (
          <p data-relays-refused className="text-callout text-destructive">
            {refused}
          </p>
        )}
        <TaskActions busy={saving} dirty={!!(typed || typedKey || qad)} cancel={<Button onClick={onClose}>{gui.delete_keep}</Button>} primary={<Button type="submit" form="relay-add" variant="primary" disabled={!was} data-relay-add-do>
            {gui.watch_add}
          </Button>}/>
      </TaskOverlay>
    </Dialog>
  );
}
