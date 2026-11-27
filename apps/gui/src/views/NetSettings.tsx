// Relay endpoints and their local credentials on the connection settings page.
import { useEffect, useState, type ReactNode } from "react";

import { fetchNetworkSettings, fetchRelays, setPreferredRelay, setRelays, type NetworkSettings, type RelayChoices, type RelayWant } from "@/api";
import { NetworkPolicySettings, type NetworkDetail } from "./NetworkPolicySettings";
import { ActionRow } from "@/components/settings/ActionRow";
import { PopupRow } from "@/components/settings/PopupRow";
import { Button } from "@/components/ui/button";
import { Dialog, TaskOverlay, TaskActions, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { MoreFact } from "@/views/more/MorePage";
import { RegionState, Waiting } from "@/components/RegionState";

/** A relay to keep as it is stored — its key, if any, stays put. */
const kept = (url: string): RelayWant => ({ url, token: null });

/** **How a relay is reached, said in words** (round NET-7): HTTPS, or
    HTTP in the clear — the Beijing relay is both for a while, the same
    box over two schemes, and the address alone left the difference to
    a reader who knows what the letters mean. */
const scheme = (url: string) => (url.startsWith("https://") ? gui.relays_scheme_https : gui.relays_scheme_http);

/** Mounting is reading: the page is drawn only while it is on screen. */
export function NetSettings({ detail, onOpenDetail, advancedRows }: {
  detail: NetworkDetail | null;
  onOpenDetail: (detail: NetworkDetail) => void;
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
    try {
      setChoices(await setRelays(next, onlyMine));
      void fetchNetworkSettings().then(setNetwork, (e: unknown) => {
        setError(e instanceof Error ? e.message : String(e));
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const relayRows = (
    <>
      {error && (
        <div data-relays-error className="text-callout text-destructive">
          {error}
        </div>
      )}
      {/* Loading until the first answer (UX-4): the heading stood over
          nothing, and the next group's 离开网络 read as a relay. */}
      {!choices && !error && <Waiting since={since} />}
      {choices && (
        <>
          {/* 使用中: a fact, one address a line — nothing here can be
              removed, so nothing here offers to. */}
          {choices.in_use.length > 0 && <MoreFact name="relays-in-use" label={gui.net_relays_configured}>
            <span data-relays-in-use className="flex flex-col items-end font-mono">
              {choices.in_use.map((url) => (
                <span key={url} data-relay-scheme={url.startsWith("https://") ? "https" : "http"}>
                  {url}
                  <span className="ml-2 font-sans text-muted-foreground">{scheme(url)}</span>
                </span>
              ))}
            </span>
          </MoreFact>}

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

          {/* 自己的中继: the row that adds one (its act asks for the
              address, so it opens a window and wears the ellipsis —
              ⚠️ 墓碑 2026-09-28: a text box and a button stood in the
              card here, a form in a row's place), then one row per
              entry whose one act is 移除, on the spot. */}
          <ActionRow
            name="relays-mine"
            label={gui.relays_mine}
            actionLabel={gui.relays_add}
            onPress={() => setAdding(true)}
            mark={{ "data-relay-add": "" }}
          />
          {choices.mine.map((url) => (
            <ActionRow
              key={url}
              name="relay"
              rowMark={{ "data-relay-mine": url }}
              label={<span data-selectable className="block truncate font-mono">{url}</span>}
              hint={choices.keyed.includes(url) ? <span data-relay-keyed>{gui.relays_keyed}</span> : undefined}
              actionLabel={gui.remove}
              onPress={() =>
                void write(
                  choices.mine.filter((u) => u !== url).map(kept),
                  choices.only_mine,
                )
              }
            />
          ))}

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
    return error ? <RegionState state="error" message={error} onRetry={() => {
      setError(null);
      setReadAttempt((attempt) => attempt + 1);
    }} /> : <Waiting since={since} />;
  }

  return <NetworkPolicySettings
    settings={network}
    detail={detail}
    onOpenDetail={onOpenDetail}
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
