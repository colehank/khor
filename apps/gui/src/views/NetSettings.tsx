// Relay endpoints and their local credentials on the connection settings page.
import { useEffect, useState, type ReactNode } from "react";

import { fetchNetworkSettings, fetchRelays, setPreferredRelay, setRelays, type DeviceRow, type NetworkSettings, type RelayChoices, type RelayWant } from "@/api";
import { NetworkPolicySettings, type NetworkDetail } from "./NetworkPolicySettings";
import { ActionRow } from "@/components/settings/ActionRow";
import { SectionHeading } from "@/components/settings/SectionHeading";
import { IconCheck, IconChevronRight } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";
import { Dialog, TaskOverlay, TaskActions, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { feel } from "@/lib/haptics";
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

function CurrentRelayPaths({ devices, read, since }: { devices: DeviceRow[]; read: boolean; since: number }) {
  // The observed paths come from the device table alone. A saved relay
  // configuration can change before the running endpoint picks it up.
  const routes = new Map<string, { label: string; forwarding: boolean; devices: Set<string> }>();
  for (const device of devices) {
    if (device.me || (device.hop !== "relay" && device.hop !== "forwarded")) continue;
    const forwarding = device.hop === "forwarded";
    const label = forwarding
      ? device.via || gui.net_relay_unknown_device
      : device.relay || gui.net_relay_unknown_server;
    const key = `${device.hop}:${label}`;
    const route = routes.get(key) ?? { label, forwarding, devices: new Set<string>() };
    route.devices.add(device.shown);
    routes.set(key, route);
  }

  return (
    <section data-settings-subgroup data-relay-current className="py-in">
      <SectionHeading className="px-row">{gui.relays_in_use}</SectionHeading>
      <div data-settings-card>
        {!read ? <Waiting since={since} /> : !routes.size ? (
          <p className="text-callout text-muted-foreground">{gui.net_detail_no_relay_routes}</p>
        ) : [...routes].sort(([a], [b]) => a.localeCompare(b)).map(([key, route]) => (
          <div key={key} data-more-row data-relay-path={route.forwarding ? "forwarding" : "relay"}
            className="flex flex-col justify-center">
            <span data-selectable className="text-body [overflow-wrap:anywhere]">{route.label}</span>
            <span data-relay-devices className="text-callout text-muted-foreground [overflow-wrap:anywhere]">
              {[...(route.forwarding ? [gui.net_summary_peer] : []), ...[...route.devices].sort()].join(" · ")}
            </span>
          </div>
        ))}
      </div>
    </section>
  );
}

function RelayDisclosure({ section, label, value, status, children }: {
  section: "preferred" | "pool";
  label: string;
  value?: string;
  status?: string;
  children: ReactNode;
}) {
  return (
    <Collapsible data-settings-subgroup data-relay-section={section}>
      <CollapsibleTrigger type="button" data-relay-disclosure
        className="group flex min-h-touch w-full items-center gap-row px-row py-in text-left text-body hover:bg-fill focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring">
        <span className="flex-none">{label}</span>
        <span data-relay-value title={value} className="min-w-0 flex-1 truncate text-right text-muted-foreground">{value}</span>
        <IconChevronRight className="size-affordance flex-none text-muted-foreground transition-transform group-data-[state=open]:rotate-90" />
      </CollapsibleTrigger>
      {status && <p role="status" data-relay-config-status className="px-row pb-in text-callout text-muted-foreground">{status}</p>}
      <CollapsibleContent data-relay-disclosure-content>
        <div data-settings-card>{children}</div>
      </CollapsibleContent>
    </Collapsible>
  );
}

function RelayServerRow({ url, mine, keyed, preferred, busy, pick, remove }: {
  url: string;
  mine: boolean;
  keyed: boolean;
  preferred?: boolean;
  busy: boolean;
  pick?: () => void;
  remove?: () => void;
}) {
  const Row = pick ? "button" : "div";
  const note = [mine && gui.net_summary_custom_services, url.startsWith("http://") && gui.relays_scheme_http,
    keyed && gui.relays_keyed].filter(Boolean).join(" · ");
  return (
    <div data-more-row data-relay-path="configured" data-relay-mine={remove ? url : undefined}
      className="flex min-w-0 items-center gap-row">
      <Row className={`flex min-w-0 flex-1 items-center gap-row text-left ${pick ? "group/relay -my-in min-h-touch rounded-sm py-in focus-visible:outline-2 focus-visible:outline-ring" : ""}`}
        {...(pick ? {
          type: "button" as const,
          onClick: () => {
            feel("selection");
            pick();
          },
          disabled: busy,
          title: preferred ? gui.net_relay_clear_preferred : gui.net_relay_set_preferred,
          "aria-pressed": !!preferred,
          "data-relay-pick": "",
        } : {})}>
        <span className="flex min-w-0 flex-1 flex-col">
          <span data-selectable={pick ? undefined : ""} className="text-body [overflow-wrap:anywhere]">{url}</span>
          {note && <span data-relay-keyed={keyed ? "" : undefined} className="text-callout text-muted-foreground">{note}</span>}
        </span>
        {pick && (
          <span aria-hidden="true" data-setting-trail data-relay-selection-icon={preferred ? "selected" : "unselected"}
            className={`grid size-affordance flex-none place-items-center rounded-full border ${preferred ? "border-foreground text-foreground" : "border-muted-foreground group-hover/relay:border-foreground"}`}>
            {preferred && <IconCheck className="size-full" />}
          </span>
        )}
      </Row>
      {remove && <Button onClick={remove} disabled={busy} data-setting-trail className="min-h-touch min-w-touch flex-none">{gui.remove}</Button>}
    </div>
  );
}

/** Mounting is reading: the page is drawn only while it is on screen. */
export function NetSettings({ detail, onOpenDetail, devices, devicesRead, advancedRows }: {
  detail: NetworkDetail | null;
  onOpenDetail: (detail: NetworkDetail) => void;
  devices: DeviceRow[];
  devicesRead: boolean;
  advancedRows: ReactNode;
}) {
  const [choices, setChoices] = useState<RelayChoices | null>(null);
  const [network, setNetwork] = useState<NetworkSettings | null>(null);
  /** When the first asking began (`Waiting`, below). */
  const [since] = useState(() => Date.now());
  const [error, setError] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
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

  const savePreferred = (url: string | null) => {
    if (url === (choices?.preferred ?? null)) return;
    void write(async () => {
      await setPreferredRelay(url);
      setChoices((before) => before && { ...before, preferred: url });
    });
  };

  const retry = () => {
    setError(null);
    setReadAttempt((attempt) => attempt + 1);
  };

  if (!network) {
    return error ? <RegionState state="error" message={error} onRetry={retry} /> : <Waiting since={since} />;
  }

  const renderRelays = (sources: ReactNode, policyBusy: boolean) => {
    const busy = writingRelay || policyBusy;
    const candidates = new Map(choices?.in_use.map((url) => [relayKey(url), url]));
    const own = new Set(choices?.mine.map(relayKey));
    const keyed = new Set(choices?.keyed.map(relayKey));
    const preferred = choices?.preferred ? relayKey(choices.preferred) : null;
    return <>
      <MoreSection name="relay-paths">
        <CurrentRelayPaths devices={devices} read={devicesRead} since={since} />
        {choices ? <>
          {error && <p data-relays-error role="alert" className="text-callout text-destructive">{error}</p>}
          <RelayDisclosure section="preferred" label={gui.relays_preferred}
            value={choices.preferred || gui.net_relay_select_auto}
            status={!network.invalid && network.pending_restart ? gui.net_profile_restart : undefined}>
            {candidates.size > 0 ? <>
              <p className="text-callout text-muted-foreground">{gui.relays_preferred_note}</p>
              {[...candidates].map(([key, url]) => (
                <RelayServerRow key={key} url={url} mine={own.has(key)} keyed={keyed.has(key)}
                  preferred={key === preferred} busy={busy} pick={() => savePreferred(key === preferred ? null : url)} />
              ))}
            </> : <p className="text-callout text-muted-foreground">{gui.net_relay_no_candidates}</p>}
          </RelayDisclosure>
          <RelayDisclosure section="pool" label={gui.net_relay_manage_pool}>
            <div data-more-section="network-public-infrastructure" data-settings-subgroup className="pt-in">
              <SectionHeading className="px-row text-muted-foreground">{gui.net_summary_public_infrastructure}</SectionHeading>
              <div data-settings-card>{sources}</div>
            </div>
            {choices.mine.map((url) => (
              <RelayServerRow key={url} url={url} mine keyed={keyed.has(relayKey(url))} busy={busy} remove={() => {
                void write(async () => {
                  const next = await setRelays(choices.mine.filter((entry) => entry !== url).map(kept), choices.only_mine);
                  setChoices(next);
                });
              }} />
            ))}
            <ActionRow name="relay-add" label={gui.relays_mine} actionLabel={gui.relays_add}
              onPress={() => setAdding(true)} busy={busy} mark={{ "data-relay-add": "" }} />
          </RelayDisclosure>
        </> : error ? <RegionState state="error" message={error} onRetry={retry} /> : <Waiting since={since} />}
      </MoreSection>
      {choices && adding && <RelayAddDialog
        open={adding}
        onClose={() => setAdding(false)}
        onWritten={(next) => {
          setChoices(next);
          void fetchNetworkSettings().then(setNetwork).catch((e: unknown) => {
            setError(e instanceof Error ? e.message : String(e));
          });
        }}
      />}
    </>;
  };

  return <NetworkPolicySettings
    settings={network}
    detail={detail}
    onOpenDetail={onOpenDetail}
    renderRelays={renderRelays}
    relayBusy={writingRelay}
    advancedRows={advancedRows}
    onChange={async (settings) => {
      setNetwork(settings);
      setError(null);
      try {
        setChoices(await fetchRelays());
      } catch (e: unknown) {
        setError(e instanceof Error ? e.message : String(e));
      }
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
