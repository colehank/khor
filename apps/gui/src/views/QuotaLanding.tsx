// 额度: the API accounts a person registered, each as a ring.
//
// **One ring is one account, and the account is on one machine.** The
// key never leaves the machine it was registered on; that machine reads
// the provider and every other machine shows what it last read, with
// how long ago (`khor_node::accounts`). So a ring here is never "this
// screen asked the provider" — it is "the home machine last read this",
// and the age beside it is part of the reading, not decoration.
//
// **The fraction and the ink are the node's.** `fill` and `strain`
// arrive decided (a new-api token fills against its own quota, the two
// vendors against the registered budget, no budget is no fraction);
// this page draws a track where there is no fraction and paints the
// arc in the strain ink the disks wear (`gaugeColor`), never a ramp of
// its own.
//
// Registration happens here too, and the form says the two things a
// person cannot guess: the key stays on this machine, and the two
// vendors' usage is an **admin** key's to read.
import { useEffect, useState } from "react";

import {
  fetchQuotaBoard,
  registerAccount,
  removeAccount,
  type AccountRing,
  type QuotaAnswer,
} from "@/api";
import { Ring, RING_SIZE, RING_SIZE_ROW } from "@/components/Ring";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { gui } from "@/gen/catalog";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";
import { cn } from "@/lib/utils";
import { gaugeColor } from "@/lib/vitals";
import { QuotaRings } from "@/views/QuotaRings";
import { lasting, until } from "@/words";

/** How often the board is asked for. The node caches every reading for
    five minutes and cools a failed account for ten (`accounts` module
    head), so this beat costs the providers nothing — it is the beat at
    which a fresh reading from another machine reaches this screen. */
const POLL_MS = 10_000;

const PROVIDERS = ["newapi", "claude", "openai"] as const;
type Provider = (typeof PROVIDERS)[number];

function providerWord(p: string): string {
  switch (p) {
    case "newapi":
      return gui.accounts_provider_newapi;
    case "claude":
      return gui.accounts_provider_claude;
    case "openai":
      return gui.accounts_provider_openai;
    default:
      return p;
  }
}

/** The trouble key's sentence (`khor_node::accounts::trouble`). A key
    this face has never heard of is shown as itself — a reason is still
    a reason, and hiding it would paint 问不出来 with no why. */
function troubleWord(key: string): string {
  switch (key) {
    case "key-refused":
      return gui.accounts_trouble_key_refused;
    case "unreachable":
      return gui.accounts_trouble_unreachable;
    case "unreadable":
      return gui.accounts_trouble_unreadable;
    case "cooling":
      return gui.accounts_trouble_cooling;
    case "no-key":
      return gui.accounts_trouble_no_key;
    case "no-reading":
      return gui.accounts_trouble_no_reading;
    case "gateway-too-old":
      return gui.accounts_trouble_gateway_too_old;
    default:
      return key;
  }
}

function money(n: number, currency: string | null): string {
  const figure = n >= 100 ? n.toFixed(0) : n.toFixed(2);
  return currency ? `${figure} ${currency}` : figure;
}

/** What sits in the ring's centre: the share when there is one, the
    spend when there is a number but no denominator, nothing else. */
function centre(r: AccountRing): string {
  const { reading } = r;
  if (reading.trouble) return "";
  if (reading.fill != null) return `${Math.round(reading.fill * 100)}%`;
  if (reading.spent != null) return money(reading.spent, null);
  return "";
}

export function QuotaLanding({
  narrow,
  quota,
}: {
  narrow: boolean;
  /** This machine's Claude Code subscription, polled by the shell on
      its own beat (`App`'s quota effect) — drawn as a band at the foot
      when there is a reading. A shell that has not asked yet passes
      null, and the band simply is not there. */
  quota?: QuotaAnswer | null;
}) {
  const [board, setBoard] = useState<AccountRing[] | null>(null);
  const [picked, setPicked] = useState<string | null>(null);
  const [registering, setRegistering] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const hidden = useHidden();

  useEffect(() => {
    let live = true;
    const tick = () => {
      fetchQuotaBoard()
        .then((rows) => {
          if (!live) return;
          setBoard(rows);
          setError(null);
        })
        .catch((e) => live && setError(String(e instanceof Error ? e.message : e)));
    };
    tick();
    const t = setInterval(tick, hidden ? HIDDEN_MS : POLL_MS);
    return () => {
      live = false;
      clearInterval(t);
    };
  }, [hidden]);

  const rows = board ?? [];
  const current = rows.find((r) => r.reading.account === picked) ?? rows[0] ?? null;

  const remove = (id: string) => {
    removeAccount(id)
      .then(() => {
        setBoard((was) => (was ? was.filter((r) => r.reading.account !== id) : was));
        if (picked === id) setPicked(null);
      })
      .catch((e) => setError(String(e instanceof Error ? e.message : e)));
  };

  const list = (
    <div data-quota-list className="flex min-w-0 flex-col">
      {rows.map((r) => {
        const on = current?.reading.account === r.reading.account;
        return (
          <button
            key={r.reading.account}
            type="button"
            data-quota-row={r.reading.account}
            data-on={on}
            data-provider={r.reading.provider}
            data-trouble={r.reading.trouble ?? undefined}
            onClick={() => setPicked(r.reading.account)}
            className={cn(
              "flex min-h-touch w-full items-center gap-3 px-3 py-2 text-left hover:bg-secondary",
              on && "bg-accent hover:bg-accent",
            )}
          >
            <Ring
              fraction={r.reading.trouble ? null : r.reading.fill}
              color={gaugeColor(r.strain)}
              size={RING_SIZE_ROW}
            />
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="truncate text-fact">{r.reading.name}</span>
              <span className="truncate text-xs text-muted-foreground">
                {providerWord(r.reading.provider)} · {r.home_name}
              </span>
            </span>
            <span className="flex-none text-xs tabular-nums text-muted-foreground">{centre(r)}</span>
          </button>
        );
      })}
    </div>
  );

  const detail = current && (
    <div data-quota-detail={current.reading.account} className="flex flex-col gap-4 p-4">
      <div className="flex items-center gap-4">
        <Ring
          fraction={current.reading.trouble ? null : current.reading.fill}
          color={gaugeColor(current.strain)}
          size={RING_SIZE}
        >
          {centre(current)}
        </Ring>
        <div className="flex min-w-0 flex-col">
          <span className="text-title">{current.reading.name}</span>
          <span className="text-sm text-muted-foreground">
            {providerWord(current.reading.provider)}
            {current.base_url ? ` · ${current.base_url}` : ""}
          </span>
        </div>
      </div>
      <dl data-quota-figures className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        {current.reading.trouble ? (
          <>
            <dt className="text-muted-foreground">{gui.accounts_state}</dt>
            <dd data-quota-trouble={current.reading.trouble}>{troubleWord(current.reading.trouble)}</dd>
          </>
        ) : (
          <>
            {current.reading.spent != null && (
              <>
                <dt className="text-muted-foreground">{gui.accounts_spent}</dt>
                <dd className="tabular-nums">
                  {current.reading.limit != null
                    ? gui.accounts_of(
                        money(current.reading.spent, null),
                        money(current.reading.limit, current.reading.currency),
                      )
                    : money(current.reading.spent, current.reading.currency)}
                  {current.reading.unlimited ? ` · ${gui.accounts_unlimited}` : ""}
                </dd>
              </>
            )}
            {current.reading.limit == null && !current.reading.unlimited && (
              <>
                <dt className="text-muted-foreground">{gui.accounts_budget}</dt>
                <dd className="text-muted-foreground">{gui.accounts_no_budget}</dd>
              </>
            )}
            {current.reading.resets_at != null && (
              <>
                <dt className="text-muted-foreground">{gui.accounts_resets}</dt>
                <dd>{until(current.reading.resets_at * 1000)}</dd>
              </>
            )}
          </>
        )}
        <dt className="text-muted-foreground">{gui.accounts_read_by}</dt>
        <dd data-quota-age={current.age_ms}>
          {current.mine
            ? gui.accounts_read_here
            : gui.accounts_read_from(current.home_name, lasting(current.age_ms))}
        </dd>
      </dl>
      <div>
        <Button
          size="sm"
          variant="outline"
          data-quota-remove={current.reading.account}
          className="min-h-touch"
          onClick={() => remove(current.reading.account)}
        >
          {gui.accounts_remove}
        </Button>
      </div>
    </div>
  );

  const empty = (
    <div className="flex flex-col items-start gap-3 p-4 text-sm text-muted-foreground">
      <span data-quota-empty>{board === null ? "" : gui.accounts_empty}</span>
      <Button size="sm" data-quota-register className="min-h-touch" onClick={() => setRegistering(true)}>
        {gui.accounts_register}
      </Button>
    </div>
  );

  return (
    <section data-quota-landing className="flex h-full min-w-0 flex-col">
      <header className="flex min-h-ctl-lg flex-none items-center gap-2 border-b px-3">
        <span className="text-title">{gui.accounts_tab}</span>
        <span className="min-w-0 flex-1" />
        {rows.length > 0 && (
          <Button
            size="sm"
            variant="ghost"
            data-quota-register
            // The finger floor (tokens.css `--touch`): the `sm` look
            // stays, the box grows, the ghost wash paints the box.
            className="min-h-touch"
            onClick={() => setRegistering(true)}
          >
            {gui.accounts_register}
          </Button>
        )}
      </header>
      {error && (
        <div data-quota-error className="border-b px-3 py-1.5 text-sm text-destructive">
          {error}
        </div>
      )}
      {rows.length === 0 ? (
        empty
      ) : narrow ? (
        <div className="min-h-0 flex-1 overflow-y-auto">
          {list}
          {detail}
        </div>
      ) : (
        <div className="flex min-h-0 flex-1">
          <div className="w-72 flex-none overflow-y-auto border-r">{list}</div>
          <div className="min-w-0 flex-1 overflow-y-auto">{detail}</div>
        </div>
      )}
      {/* This machine's Claude Code login, the two windows as arcs —
          and since 2026-09-02 the only place they are drawn:「机器页面
          不需要有token usage的环」took them off both machine surfaces,
          and the page whose name is 额度 is the home (`QuotaRings`'
          head carries the three moves). **Only with a reading**: a
          no-login / stale sentence here would be a warning about a
          product nobody registered — absent says it better. And only
          this machine's: no frame carries another machine's
          subscription, so there is nothing else this band could draw
          without inventing it. */}
      {quota?.quota && quota.quota.windows.length > 0 && (
        <footer data-quota-sub className="flex flex-none flex-col gap-2 border-t px-3 py-2">
          <span className="text-aux text-muted-foreground">{gui.accounts_subscription}</span>
          <QuotaRings quota={quota} />
        </footer>
      )}
      <RegisterDialog
        open={registering}
        onOpenChange={setRegistering}
        onRegistered={(id) => {
          setPicked(id);
          fetchQuotaBoard().then(setBoard).catch(() => {});
        }}
      />
    </section>
  );
}

function RegisterDialog({
  open,
  onOpenChange,
  onRegistered,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onRegistered: (id: string) => void;
}) {
  const [provider, setProvider] = useState<Provider>("newapi");
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [key, setKey] = useState("");
  const [budget, setBudget] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setProvider("newapi");
    setName("");
    setBaseUrl("");
    setKey("");
    setBudget("");
    setError(null);
    setBusy(false);
  }, [open]);

  const create = () => {
    if (busy) return;
    setBusy(true);
    setError(null);
    const parsedBudget = budget.trim() === "" ? undefined : Number(budget);
    registerAccount({
      name,
      provider,
      base_url: provider === "newapi" ? baseUrl : undefined,
      key,
      budget: parsedBudget !== undefined && Number.isFinite(parsedBudget) ? parsedBudget : undefined,
    })
      .then((id) => {
        onOpenChange(false);
        onRegistered(id);
      })
      .catch((e) => setError(String(e instanceof Error ? e.message : e)))
      .finally(() => setBusy(false));
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-quota-dialog>
        <DialogHeader>
          <DialogTitle>{gui.accounts_register}</DialogTitle>
        </DialogHeader>
        <div className="flex min-w-0 flex-col gap-3">
          <div className="flex flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.accounts_provider}</span>
            <div className="flex flex-wrap gap-1">
              {PROVIDERS.map((p) => (
                <Button
                  key={p}
                  size="sm"
                  variant={provider === p ? "secondary" : "ghost"}
                  data-quota-provider={p}
                  data-on={provider === p}
                  onClick={() => setProvider(p)}
                >
                  {providerWord(p)}
                </Button>
              ))}
            </div>
          </div>
          <label className="flex min-w-0 flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.accounts_name}</span>
            <Input data-quota-name value={name} onChange={(e) => setName(e.target.value)} />
          </label>
          {provider === "newapi" && (
            <label className="flex min-w-0 flex-col gap-1 text-sm">
              <span className="text-muted-foreground">{gui.accounts_address}</span>
              <Input
                data-quota-address
                value={baseUrl}
                placeholder="https://"
                onChange={(e) => setBaseUrl(e.target.value)}
              />
            </label>
          )}
          <label className="flex min-w-0 flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.accounts_key}</span>
            <Input
              data-quota-key
              type="password"
              autoComplete="off"
              value={key}
              onChange={(e) => setKey(e.target.value)}
            />
            {/* The two facts the form owes a person before they paste a
                secret: where it goes, and which kind of key can read
                usage at all. */}
            <span className="text-xs text-muted-foreground">
              {provider === "newapi" ? gui.accounts_key_stays : `${gui.accounts_key_stays} ${gui.accounts_needs_admin}`}
            </span>
          </label>
          {provider !== "newapi" && (
            <label className="flex min-w-0 flex-col gap-1 text-sm">
              <span className="text-muted-foreground">{gui.accounts_budget}</span>
              <Input
                data-quota-budget
                inputMode="decimal"
                value={budget}
                placeholder={gui.accounts_budget_hint}
                onChange={(e) => setBudget(e.target.value)}
              />
            </label>
          )}
          {error && (
            <p data-quota-dialog-error className="text-sm text-destructive">
              {error}
            </p>
          )}
        </div>
        <DialogFooter>
          <Button data-quota-create disabled={busy} onClick={create}>
            {gui.accounts_register}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
