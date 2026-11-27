// Relay endpoints and their local credentials on the connection settings page.
import { useEffect, useState, type ReactNode } from "react";

import { fetchNetworkSettings, fetchRelays, setPreferredRelay, setRelays, type DeviceRow, type NetworkSettings, type RelayChoices, type RelayWant } from "@/api";
import { NetworkPolicySettings, type NetworkDetail } from "./NetworkPolicySettings";
import { ActionRow } from "@/components/settings/ActionRow";
import { PopupRow } from "@/components/settings/PopupRow";
import { IconCheck } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Dialog, TaskOverlay, TaskActions, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { MoreSection } from "@/views/more/MorePage";
import { RegionState, Waiting } from "@/components/RegionState";

/** A relay to keep as it is stored — its key, if any, stays put. */
const kept = (url: string): RelayWant => ({ url, token: null });

/** **How a relay is reached, said in words** (round NET-7): HTTPS, or
    HTTP in the clear — the Beijing relay is both for a while, the same
    box over two schemes, and the address alone left the difference to
    a reader who knows what the letters mean. */
const scheme = (url: string) => (url.startsWith("https://") ? gui.relays_scheme_https : gui.relays_scheme_http);

function relayAddress(url: string) {
  try {
    return new URL(url);
  } catch {
    return null;
  }
}

function RelayPaths({ choices, devices, settings, onRemove }: {
  choices: RelayChoices;
  devices: DeviceRow[];
  settings: NetworkSettings;
  onRemove: (url: string) => void;
}) {
  // A relay's live reading is host and non-default port. Only a unique
  // match can identify a configured URL, which also carries its scheme.
  const configured = new Map<string, string>();
  for (const url of [...choices.in_use, ...choices.mine]) {
    configured.set(relayAddress(url)?.href ?? url, url);
  }
  const configuredByHost = new Map<string, string[]>();
  for (const url of configured.values()) {
    const host = relayAddress(url)?.host;
    if (!host) continue;
    const entries = configuredByHost.get(host) ?? [];
    entries.push(url);
    configuredByHost.set(host, entries);
  }

  const used = new Set<string>();
  const ambiguous = new Set<string>();
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
    for (const url of matches) ambiguous.add(url);
    const advertised = device.relay && matches.length === 0
      && !settings.pending_restart && !settings.invalid && settings.profile.policy.advertised_relays;
    otherRelays.set(device.relay, advertised ? gui.net_summary_advertised : gui.net_relay_unidentified);
  }

  const unresolved = otherRelays.has(null) || !devices.some((device) => device.me);
  return <>
    {[...configured.values()].map((url) => {
      const mine = choices.mine.includes(url);
      const active = used.has(url);
      const status = active ? gui.relays_in_use : !settings.resident ? gui.net_relay_node_stopped
        : unresolved || ambiguous.has(url) ? gui.net_relay_unconfirmed : gui.net_relay_idle;
      return <RelayPathRow key={url} label={url} kind="configured"
        source={mine ? gui.net_summary_custom_services : gui.net_relays_configured}
        status={status} active={active} keyed={choices.keyed.includes(url)}
        remove={mine ? () => onRemove(url) : undefined} />;
    })}
    {[...otherRelays].map(([host, source]) => (
      <RelayPathRow key={host ?? "unnamed-relay"} label={host ?? gui.net_relay_unknown_server}
        kind="relay" source={source} status={gui.relays_in_use} active />
    ))}
    {[...forwarding].map((via) => (
      <RelayPathRow key={`via:${via ?? ""}`} label={via ?? gui.net_relay_unknown_device}
        kind="forwarding" source={gui.net_summary_peer} status={gui.relays_in_use} active />
    ))}
    {configured.size === 0 && otherRelays.size === 0 && forwarding.size === 0 && (
      <p className="text-callout text-muted-foreground">{gui.net_detail_no_relay_routes}</p>
    )}
  </>;
}

function RelayPathRow({ label, kind, source, status, active, keyed, remove }: {
  label: string;
  kind: "configured" | "relay" | "forwarding";
  source: string;
  status: string;
  active: boolean;
  keyed?: boolean;
  remove?: () => void;
}) {
  return (
    <div data-more-row data-relay-path={kind} data-relay-active={active}
      data-relay-mine={remove ? label : undefined} className="flex min-w-0 items-center gap-row">
      <div className="min-w-0 flex-1">
        <div data-selectable className="text-body [overflow-wrap:anywhere]">{label}</div>
        <div className="text-callout text-muted-foreground">
          {source}{keyed && <span data-relay-keyed> · {gui.relays_keyed}</span>}
        </div>
      </div>
      <span data-relay-path-status className={`flex flex-none items-center gap-xxs text-callout ${active ? "text-foreground" : "text-muted-foreground"}`}>
        {active && <IconCheck className="size-affordance" />}
        {status}
      </span>
      {remove && <Button onClick={remove} data-setting-trail className="min-h-touch min-w-touch flex-none">{gui.remove}</Button>}
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

  // One door for every change — add and remove both send the whole list,
  // because that is the shape the node takes (`Node::set_relays`
  // replaces). The node's own sentence names the entry it refused.
  const write = async (next: RelayWant[], onlyMine: boolean) => {
    setError(null);
    try {
      setChoices(await setRelays(next, onlyMine));
      void fetchNetworkSettings().then(setNetwork, (e: unknown) => {
        setError(e instanceof Error ? e.message : String(e));
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const retry = () => {
    setError(null);
    setReadAttempt((attempt) => attempt + 1);
  };

  const relayPaths = detail === "relays" && network && (
    <MoreSection name="relay-paths" label={gui.net_relay_path_list}>
      {choices ? <>
        <RelayPaths choices={choices} devices={devices} settings={network} onRemove={(url) => {
          void write(choices.mine.filter((entry) => entry !== url).map(kept), choices.only_mine);
        }} />
        <ActionRow name="relay-add" label={gui.relays_mine} actionLabel={gui.relays_add}
          onPress={() => setAdding(true)} mark={{ "data-relay-add": "" }} />
      </> : error ? <RegionState state="error" message={error} onRetry={retry} /> : <Waiting since={since} />}
    </MoreSection>
  );

  const relayRows = (
    <>
      {choices && error && (
        <div data-relays-error className="text-callout text-destructive">
          {error}
        </div>
      )}
      {choices && (
        <>
          {/* 首选中继 (round NET-2): which of the relays in use this
              machine homes on whenever it is reachable — 自动 is iroh's
              own choice, the nearest. The system's menu, like every
              choice of one among a few. Drawn only with more than one
              relay in use: with one there is nothing to prefer. */}
          {choices.in_use.length > 1 && (
            <PopupRow
              name="relays-preferred"
              label={gui.relays_preferred}
              hint={gui.relays_preferred_note}
              value={choices.preferred ?? ""}
              choices={[
                { value: "", label: gui.relays_preferred_auto },
                ...choices.in_use.map((url) => ({ value: url, label: `${url}（${scheme(url)}）` })),
              ]}
              onPick={(v) => {
                setError(null);
                setPreferredRelay(v || null)
                  .then(fetchRelays)
                  .then(setChoices)
                  .then(fetchNetworkSettings)
                  .then(setNetwork)
                  .catch((e: unknown) => setError(String(e instanceof Error ? e.message : e)));
              }}
              mark={{ "data-relays-preferred": choices.preferred ?? "" }}
            />
          )}

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
        </>
      )}
    </>
  );

  if (!network) {
    return error ? <RegionState state="error" message={error} onRetry={retry} /> : <Waiting since={since} />;
  }

  return <NetworkPolicySettings
    settings={network}
    detail={detail}
    onOpenDetail={onOpenDetail}
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
