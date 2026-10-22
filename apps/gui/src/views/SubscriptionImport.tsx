import { useEffect, useRef, useState } from "react";
import { fetchSubscriptionCandidates, importSubscription, type DeviceRow } from "@/api";
import type { SubscriptionCandidate } from "@/gen/bindings/SubscriptionCandidate";
import { sessionMark } from "@/components/BrandIcons";
import { ImportCalm, ImportMachinePicker, ImportResult, ImportSearch, importMachineAway, importMachineInScope, toggleImportMachine } from "@/components/ImportPicker";
import { RegionState } from "@/components/RegionState";
import { Button } from "@/components/ui/button";
import { DialogBody, DialogFooter } from "@/components/ui/dialog";
import { gui } from "@/gen/catalog";
import { imeKey } from "@/lib/ime";
import { connectionKeyWord } from "@/words";

type Found = { rows: SubscriptionCandidate[] | null; error: string | null };
type Row = SubscriptionCandidate & { device: DeviceRow };
const rowKey = (row: Row) => `${row.device.id}/${row.provider}`;

/** The same search → machine filter → tap-to-import flow as session import. */
export function SubscriptionImport({ devices, registered, onImported, onCancel }: {
  devices: DeviceRow[];
  registered: string[];
  onImported: (id: string) => void;
  onCancel: () => void;
}) {
  const machines = devices.filter((d) => d.hosts && !d.sponsor).sort((a, b) =>
    Number(importMachineAway(a) !== null) - Number(importMachineAway(b) !== null)
    || Number(b.me) - Number(a.me) || a.shown.localeCompare(b.shown));
  const machineKey = machines.map((d) => `${d.id}:${d.paused}`).sort().join(",");
  const [found, setFound] = useState<Record<string, Found>>({});
  const [q, setQ] = useState("");
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [active, setActive] = useState(0);
  const [going, setGoing] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const generation = useRef(0);
  const alive = useRef(true);
  const list = useRef<HTMLDivElement>(null);

  useEffect(() => {
    alive.current = true;
    return () => { alive.current = false; };
  }, []);

  const discover = (machine: string, turn: number) => {
    setFound((held) => ({ ...held, [machine]: { rows: null, error: null } }));
    fetchSubscriptionCandidates(machine).then(
      (rows) => {
        if (generation.current === turn) setFound((held) => ({ ...held, [machine]: { rows, error: null } }));
      },
      (reason) => {
        if (generation.current === turn) setFound((held) => ({ ...held, [machine]: { rows: null, error: String(reason instanceof Error ? reason.message : reason) } }));
      },
    );
  };
  useEffect(() => {
    const turn = ++generation.current;
    setFound({});
    setPicked((held) => new Set([...held].filter((id) => machines.some((d) => d.id === id))));
    for (const d of machines) {
      if (d.paused) setFound((held) => ({ ...held, [d.id]: { rows: null, error: connectionKeyWord("paused") } }));
      else discover(d.id, turn);
    }
    return () => { generation.current++; };
    // Ordinary device readings must not restart every network request.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [machineKey]);

  const words = q.toLocaleLowerCase().trim().split(/\s+/).filter(Boolean);
  const scoped = machines.filter((d) => importMachineInScope(d.id, picked, d));
  const rows: Row[] = scoped.flatMap((device) => (found[device.id]?.rows ?? [])
    .filter((r) => r.account === null || !registered.includes(r.account))
    .map((r) => ({ ...r, device })));
  const shown = rows.filter((r) => words.every((word) =>
    [r.vendor, r.provider, r.device.shown, r.device.name].some((text) => text.toLocaleLowerCase().includes(word))));
  const pending = scoped.some((d) => !found[d.id] || (found[d.id].rows === null && !found[d.id].error));
  const failed = scoped.filter((d) => found[d.id]?.error);

  useEffect(() => setActive(0), [q, picked]);
  useEffect(() => {
    list.current?.querySelector<HTMLElement>("[data-active]")?.scrollIntoView({ block: "nearest" });
  }, [active]);

  const go = (row: Row) => {
    if (going) return;
    setGoing(rowKey(row));
    setError(null);
    importSubscription(row.device.id, row.provider).then(
      (id) => { if (alive.current) onImported(id); },
      (reason) => { if (alive.current) setError(String(reason instanceof Error ? reason.message : reason)); },
    ).finally(() => { if (alive.current) setGoing(null); });
  };

  return <>
    <DialogBody data-subscription-body>
      <ImportSearch label={gui.subscriptions_search} value={q} onChange={setQ} onKeyDown={(e) => {
        if (imeKey(e)) return;
        if (e.key === "ArrowDown") { e.preventDefault(); setActive((i) => Math.max(0, Math.min(shown.length - 1, i + 1))); }
        else if (e.key === "ArrowUp") { e.preventDefault(); setActive((i) => Math.max(0, i - 1)); }
        else if (e.key === "Enter" && shown[active]) { e.preventDefault(); go(shown[active]); }
      }} />
      {machines.length > 0 && <div data-subscription-machines className="flex-none">
        <ImportMachinePicker machines={machines.map((device) => ({ id: device.id, name: device.shown, device }))}
          picked={picked} onPick={(id) => setPicked((held) => toggleImportMachine(held, id))} disabled={going !== null} />
      </div>}
      <div ref={list} data-subscription-choices className="-mx-in flex min-w-0 flex-none flex-col px-in" aria-busy={pending || undefined}>
        {shown.map((row, index) => {
          const Glyph = sessionMark("agent", row.provider);
          return <ImportResult key={rowKey(row)} data-subscription-machine={row.device.id} data-subscription-provider={row.provider}
            glyph={<Glyph className="size-5" />} title={row.vendor} detail={row.device.shown}
            active={q !== "" && index === active} busy={going === rowKey(row)} onClick={() => go(row)} />;
        })}
        {shown.length === 0 && (pending ? <RegionState state="loading-never" /> : failed.length === 0 &&
          <ImportCalm sentence={machines.length === 0 ? gui.subscriptions_no_computers : q ? gui.no_matches : gui.subscriptions_empty} mark={{ "data-subscription-empty": "" }} />)}
      </div>
      {failed.map((d) => <div key={d.id} data-subscription-status={d.id} className="flex-none [&>[data-empty-state]]:h-auto [&>[data-empty-state]]:p-3">
        <RegionState state="error" message={`${d.shown} · ${found[d.id].error}`}
          onRetry={d.paused ? undefined : () => discover(d.id, generation.current)} />
      </div>)}
      {error && <span role="alert" data-subscription-error className="flex-none text-callout text-destructive">{error}</span>}
      {rows.length > 0 && <p className="flex-none text-caption text-muted-foreground">{gui.subscriptions_note}</p>}
    </DialogBody>
    <DialogFooter>
      <Button data-dialog-cancel className="min-h-touch" onClick={onCancel}>{gui.delete_keep}</Button>
    </DialogFooter>
  </>;
}
