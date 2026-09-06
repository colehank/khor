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
import { Fragment, useEffect, useMemo, useState } from "react";

import {
  fetchQuotaBoard,
  registerAccount,
  removeAccount,
  type AccountRing,
  type QuotaAnswer,
  type QuotaWindow,
} from "@/api";
import { type OmniAxis } from "@/components/Omnibox";
import { PaneBar, type FilterOption } from "@/components/PaneBar";
import { NestedRing, Ring, RING_SIZE, RING_SIZE_ROW } from "@/components/Ring";
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

import { QUOTA_AXES, axisOf, lasting, until, valueOf } from "@/words";

/** How often the board is asked for. The node caches every reading for
    five minutes and cools a failed account for ten (`accounts` module
    head), so this beat costs the providers nothing — it is the beat at
    which a fresh reading from another machine reaches this screen. */
const POLL_MS = 10_000;

/** The words `khor_sync::accounts::Provider::parse` accepts, in the
    order the form offers them. A reading can carry a word that is not
    here — an account registered by a newer khor — and shows it as
    itself (`providerWord`'s default); the form only offers what this
    build can register. */
const PROVIDERS = ["newapi", "claude", "openai", "deepseek", "moonshot", "moonshot-cn", "openrouter"] as const;
type Provider = (typeof PROVIDERS)[number];

/** Which providers a registered budget means anything for: those that
    report a spend (the vendors' cost reports, OpenRouter's monthly
    usage). A prepaid wallet reports what is left, and a budget over a
    balance would be a fraction of two different things. */
const BUDGETED: readonly string[] = ["claude", "openai", "openrouter"];
/** Both vendors' usage reports are organisation-level admin calls. */
const NEEDS_ADMIN: readonly string[] = ["claude", "openai"];

function providerWord(p: string): string {
  switch (p) {
    case "newapi":
      return gui.accounts_provider_newapi;
    case "claude":
      return gui.accounts_provider_claude;
    case "openai":
      return gui.accounts_provider_openai;
    case "deepseek":
      return gui.accounts_provider_deepseek;
    case "moonshot":
      return gui.accounts_provider_moonshot;
    case "moonshot-cn":
      return gui.accounts_provider_moonshot_cn;
    case "openrouter":
      return gui.accounts_provider_openrouter;
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
    case "unknown-provider":
      return gui.accounts_trouble_unknown_provider;
    default:
      return key;
  }
}

function money(n: number, currency: string | null): string {
  const figure = n >= 100 ? n.toFixed(0) : n.toFixed(2);
  return currency ? `${figure} ${currency}` : figure;
}

/**
 * How long each subscription window is, and **which ones may go on the
 * ring at all**.
 *
 * ⚠️ **The ring draws one layer per *duration*; a window split by model
 * never goes on it.** `seven_day_sonnet` and `seven_day_opus` are parts
 * of the seven-day window, not another length of time — putting them on
 * would make four concentric arcs, which is precisely the shape that
 * was removed on 2026-09-02 (`Ring`'s tombstone). They live in the
 * detail, where each window is its own single ring and nothing has to
 * be told apart by radius.
 *
 * **This is a rule about meaning, not a cap on count.** The day a
 * vendor adds a thirty-day window the ring becomes three layers and is
 * still ordered by length and still readable; the day it splits a
 * window by model again, the ring does not grow at all.
 *
 * `null` = a split, detail only. An unknown kind is treated as a split
 * for the same reason: khor does not know its length, so it cannot put
 * it in order, and an arc out of order is an arc that lies.
 */
const WINDOW_MINUTES: Record<string, number | null> = {
  five_hour: 5 * 60,
  seven_day: 7 * 24 * 60,
  seven_day_sonnet: null,
  seven_day_opus: null,
};

/** The windows that go on the ring, **outermost first — longest
    first**, which is the one-sentence order `NestedRing` requires its
    caller to state: further out is a longer window. */
function ringWindows(ws: QuotaWindow[]): QuotaWindow[] {
  return ws
    .filter((w) => WINDOW_MINUTES[w.kind] != null)
    .sort((a, b) => (WINDOW_MINUTES[b.kind] ?? 0) - (WINDOW_MINUTES[a.kind] ?? 0));
}

/** The window a reader is closest to running out of — what the nested
    ring's centre shows, and what the row's right-hand summary names.
    **Chosen by a rule, not picked**: it is the one that will stop you
    first. */
function worst(ws: QuotaWindow[]): QuotaWindow | null {
  return ws.reduce<QuotaWindow | null>((m, w) => (m === null || w.used_pct > m.used_pct ? w : m), null);
}

/** 订阅 or 计费. */
function kindWord(k: "sub" | "bill"): string {
  return k === "sub" ? gui.quota_kind_sub : gui.quota_kind_bill;
}

/** The machine a subscription was read on, or 「khor 登记」 for an
    account. **The machine's own name, not 「本机」** — 2026-09-02:
    「本机不用写一个括号在机器名旁边」. */
function fromWord(from: string): string {
  return from === "" ? gui.quota_from_khor : from;
}

/** A window's name — the catalog's own, the same string on the row and
    in the detail. No short form: a second spelling of one window is the
    drift this page's whole vocabulary is built to avoid. */
function windowWord(kind: string): string {
  switch (kind) {
    case "five_hour":
      return gui.quota_five_hour;
    case "seven_day":
      return gui.quota_seven_day;
    case "seven_day_sonnet":
      return gui.quota_seven_day_sonnet;
    case "seven_day_opus":
      return gui.quota_seven_day_opus;
    default:
      return kind;
  }
}

/**
 * One line of the 額度 list, whichever of the two things it is.
 *
 * **The subscription is a row, not a band at the foot** (2026-09-06:
 * 「不要有现在下方单独一个订阅的横条」). It had a strip of its own
 * because it is the one reading khor does not get from a registered
 * key — but that is a fact about *where it comes from*, which is now
 * what the 来源 axis says, and a page that lists what you have should
 * list all of it.
 */
type Row = {
  id: string;
  name: string;
  /** `sub` or `bill` — see `zh.toml`'s note: it answers both 「按什么计」
      and 「这一行怎么来的」. */
  kind: "sub" | "bill";
  provider: string;
  /** The machine whose Claude Code was read, or `""` for 「khor 登记」 —
      **not** the machine holding an account's key, which is a property
      of the account and lives in its detail. */
  from: string;
  account: AccountRing | null;
  windows: QuotaWindow[];
};

/** What sits in the ring's centre: the share when there is one, the
    spend when there is a number but no denominator, what is left when
    the provider reports a wallet rather than a spend, nothing else. */
function centre(r: AccountRing): string {
  const { reading } = r;
  if (reading.trouble) return "";
  if (reading.fill != null) return `${Math.round(reading.fill * 100)}%`;
  if (reading.spent != null) return money(reading.spent, null);
  if (reading.remaining != null) return money(reading.remaining, null);
  return "";
}

/** The id the subscription row wears. Not an account id and cannot
    collide with one: those are the provider's, this one is khor's. */
const SUB_ID = "sub:local";

/**
 * What a row answers on each of the three axes, **in the axis's own
 * terms** — the same strings the filter ticks and the group headings
 * are made of.
 *
 * There is exactly one of these, and everything that narrows or groups
 * this list reads it. The alternative (a `switch` in the filter and
 * another in the grouper) is what `words.ts` calls the drift: two
 * places deciding what 来源 means, agreeing until the day one of them
 * is edited.
 *
 * **One value per axis, never a list.** 来源 is where this row's
 * reading came from — a subscription's machine, or 「khor 登记」 for
 * an account you filled a key in for. It is deliberately *not* the
 * machine holding the key: that is a property of the account, it lives
 * in the detail, and a row that belonged to two 来源 groups at once
 * would make「按来源分组」 draw the same row twice.
 */
function answersOf(r: Row): Record<string, string> {
  return { "kind:": r.kind, "by:": r.provider, "from:": r.from };
}

/** The word for one axis value. **The filter's tick and the group's
    heading print this same call**, so a value cannot be spelled two
    ways on one screen. */
function valueWord(axis: string, value: string): string {
  switch (axis) {
    case "kind:":
      return kindWord(value === "sub" ? "sub" : "bill");
    case "by:":
      return providerWord(value);
    case "from:":
      return fromWord(value);
    default:
      return value;
  }
}

/** The axis's own name, for the heading over its ticks and over its
    group. */
function axisWord(axis: string): string {
  switch (axis) {
    case "kind:":
      return gui.quota_kind;
    case "by:":
      return gui.quota_by;
    default:
      return gui.quota_from;
  }
}

/**
 * Which rows the bar's search and filter leave standing — the same
 * shape `visibleSessions` has, and the same two decisions:
 *
 * **AND across axes, OR within one.** Ticking 订阅 and 计费 means
 * 「either」; ticking 订阅 and a provider means 「the subscription, and
 * from that provider」.
 *
 * **Search reads what the row shows.** The row prints its name and,
 * under it, 类型 · 提供商 · 来源 — so those four are what the box
 * matches, in the words on the glass rather than the keys behind them.
 * Typing 「turing」 finds that machine's subscription because the row
 * says 「turing」; typing 「claude」 finds it because the row says the
 * provider. A field the row does not print is not searchable here, and
 * that is the rule, not a gap: a hit you cannot see on the row it
 * matched reads as the search being broken.
 */
export function visibleQuota(rows: Row[], query: string, ticked: string[]): Row[] {
  const q = query.trim().toLowerCase();
  const wanted = new Map<string, Set<string>>();
  for (const key of ticked) {
    const axis = axisOf(key, QUOTA_AXES);
    const set = wanted.get(axis) ?? new Set<string>();
    set.add(valueOf(key, QUOTA_AXES));
    wanted.set(axis, set);
  }
  return rows.filter((r) => {
    const on = answersOf(r);
    for (const [axis, set] of wanted) if (!set.has(on[axis] ?? "")) return false;
    if (q === "") return true;
    const said = [r.name, ...QUOTA_AXES.map((a) => valueWord(a, on[a] ?? ""))].join(" ");
    return said.toLowerCase().includes(q);
  });
}

/** How the list is cut into groups. `none` is a real choice, not a
    missing one: three axes and no way back to a flat list would be a
    control you cannot undo. */
const GROUP_NONE = "none";
const GROUP_KEY = "khor.quota.group";

function storedGroup(): string {
  try {
    const raw = window.localStorage.getItem(GROUP_KEY);
    return raw && (raw === GROUP_NONE || QUOTA_AXES.includes(raw as never)) ? raw : GROUP_NONE;
  } catch {
    return GROUP_NONE;
  }
}

export function QuotaLanding({
  narrow,
  quota,
  me,
  query,
  onQuery,
  words,
  onToggleWord,
}: {
  narrow: boolean;
  /** The bar's search text and its ticked filter keys. **Held by the
      shell, like every other pane's** (`App`'s `queries` / `words`,
      keyed by landing): the query is per pane so switching landings is
      not a way of clearing a search, and the ticks are remembered on
      this device under `khor.filter.quota`. This landing draws its own
      bar because it is its own region — but the state it drives is the
      same state, in the same place, spelled the same way. */
  query: string;
  onQuery: (q: string) => void;
  words: string[];
  onToggleWord: (key: string) => void;
  /** This machine's name — the 来源 of its own subscription row. Passed
      in rather than guessed from an account's `home_name`: a machine
      with no registered key still has a subscription, and 「来源」 with
      no name is the one thing this row must not be. */
  me: string;
  /** This machine's Claude Code subscription, polled by the shell on
      its own beat (`App`'s quota effect) — drawn as a band at the foot
      when there is a reading. A shell that has not asked yet passes
      null, and the band simply is not there. */
  quota?: QuotaAnswer | null;
}) {
  const [board, setBoard] = useState<AccountRing[] | null>(null);
  const [picked, setPicked] = useState<string | null>(null);
  const [groupBy, setGroupBy] = useState<string>(storedGroup);
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

  // **One list, two kinds of thing.** The subscription is first because
  // it is the reading you did not ask for — everything below it you
  // registered yourself.
  const rows: Row[] = [
    ...(quota?.quota && quota.quota.windows.length > 0
      ? [
          {
            id: SUB_ID,
            name: gui.accounts_subscription,
            kind: "sub" as const,
            provider: "claude",
            from: me,
            account: null,
            windows: quota.quota.windows,
          },
        ]
      : []),
    ...(board ?? []).map((a) => ({
      id: a.reading.account,
      name: a.reading.name,
      kind: "bill" as const,
      provider: a.reading.provider,
      from: "",
      account: a,
      windows: [] as QuotaWindow[],
    })),
  ];
  const shown = visibleQuota(rows, query, words);
  // **Grouped by re-ordering, not by nesting.** The rows stay one flat
  // list and a heading is drawn where the group changes between
  // neighbours — the same mechanism `SessionsList` uses, and for the
  // same reason: a group that no row falls into cannot draw a heading
  // over nothing, with no special case to remember.
  const grouped = useMemo(() => {
    if (groupBy === GROUP_NONE) return shown;
    const order: string[] = [];
    const bins = new Map<string, Row[]>();
    for (const r of shown) {
      const g = answersOf(r)[groupBy] ?? "";
      if (!bins.has(g)) {
        bins.set(g, []);
        order.push(g);
      }
      bins.get(g)!.push(r);
    }
    return order.flatMap((g) => bins.get(g)!);
    // `shown` is rebuilt every render (it is a filter over `rows`), so
    // the deps that matter are the three inputs behind it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows, query, words, groupBy]);
  // **The pick follows what is on screen.** A row filtered away must
  // not keep the detail: the pane would be showing an account the list
  // no longer offers, with no row lit to say which one it is.
  const current = grouped.find((r) => r.id === picked) ?? grouped[0] ?? null;

  const chooseGroup = (key: string) => {
    setGroupBy(key);
    try {
      window.localStorage.setItem(GROUP_KEY, key);
    } catch {
      /* the list still groups; only the memory of it is lost */
    }
  };

  // The ticks, gathered from the rows that exist — never enumerated
  // here. **Plus any key already ticked that matches nothing**, or the
  // tick becomes impossible to undo (which also covers a tick restored
  // from storage before the first board has arrived).
  const options = useMemo<FilterOption[]>(() => {
    const seen = (axis: string) => {
      const values: string[] = [];
      for (const r of rows) {
        const v = answersOf(r)[axis] ?? "";
        if (!values.includes(v)) values.push(v);
      }
      for (const key of words) {
        const v = valueOf(key, QUOTA_AXES);
        if (axisOf(key, QUOTA_AXES) === axis && !values.includes(v)) values.push(v);
      }
      return values.map((v) => ({ key: `${axis}${v}`, label: valueWord(axis, v), axis: axisWord(axis) }));
    };
    return QUOTA_AXES.flatMap(seen);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows, words]);

  // The same options read as the search box's axes — derived from the
  // one list above rather than gathered a second time, so a chip and a
  // tick cannot come back with different answers.
  const axes = useMemo<OmniAxis[]>(() => {
    const byAxis = new Map<string, OmniAxis>();
    for (const o of options) {
      const axis = axisOf(o.key, QUOTA_AXES);
      const seat = byAxis.get(axis) ?? { key: axis, label: o.axis ?? axisWord(axis), candidates: [] };
      seat.candidates.push({ key: o.key, label: o.label });
      byAxis.set(axis, seat);
    }
    return [...byAxis.values()];
  }, [options]);

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
      {grouped.length === 0 && (
        <div data-quota-none className="p-4 text-sm text-muted-foreground">
          {gui.no_matches}
        </div>
      )}
      {grouped.map((r, i) => {
        const on = current?.id === r.id;
        const layers = ringWindows(r.windows);
        const bad = worst(layers);
        // A heading starts where the group changes between neighbouring
        // rows. `GROUP_NONE` answers the empty string for every row, so
        // it draws none — no branch of its own.
        const group = groupBy === GROUP_NONE ? null : (answersOf(r)[groupBy] ?? "");
        const opens = group !== null && group !== (i > 0 ? answersOf(grouped[i - 1]!)[groupBy] : undefined);
        return (
          <Fragment key={r.id}>
            {opens && (
              <div
                data-quota-group={`${groupBy}${group}`}
                className="px-3 pt-3 pb-1 text-xs text-muted-foreground"
              >
                {valueWord(groupBy, group)}
              </div>
            )}
          <button
            type="button"
            data-quota-row={r.id}
            data-on={on}
            data-provider={r.provider}
            data-quota-kind={r.kind}
            data-quota-from={r.from || "khor"}
            data-trouble={r.account?.reading.trouble ?? undefined}
            onClick={() => setPicked(r.id)}
            className={cn(
              "flex min-h-touch w-full items-center gap-3 px-3 py-2 text-left hover:bg-secondary",
              on && "bg-accent hover:bg-accent",
            )}
          >
            {layers.length > 1 ? (
              // **Several readings of one quantity**, which is the only
              // shape `NestedRing` allows: each layer is 「how much of
              // this window is used」, in percent, on one scale. Order
              // outward is the window's length, and the centre is the
              // one that will stop you first — named in words on the
              // right, because colour alone is a puzzle.
              <NestedRing
                tracks={layers.map((w) => ({ id: w.kind, fraction: w.used_pct / 100 }))}
                color={gaugeColor(null)}
                size={RING_SIZE_ROW}
              >
                {bad ? `${Math.round(bad.used_pct)}%` : ""}
              </NestedRing>
            ) : (
              <Ring
                fraction={
                  r.account
                    ? r.account.reading.trouble
                      ? null
                      : r.account.reading.fill
                    : (bad?.used_pct ?? 0) / 100
                }
                color={gaugeColor(r.account?.strain ?? null)}
                size={RING_SIZE_ROW}
              />
            )}
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="truncate text-fact">{r.name}</span>
              {/* **Three segments, and the middle one is why.** 提供商
                  is one of the three axes this page filters and groups
                  by; an axis you can tick but cannot see is the shape
                  this repo keeps being bitten by — and it is also what
                  makes 「search reads the words on the row」 true
                  rather than a search over a hidden field. */}
              <span data-quota-sub-line className="truncate text-xs text-muted-foreground">
                {kindWord(r.kind)} · {providerWord(r.provider)} · {fromWord(r.from)}
              </span>
            </span>
            <span className="flex-none text-xs tabular-nums text-muted-foreground">
              {r.account ? centre(r.account) : bad ? `${windowWord(bad.kind)} ${Math.round(bad.used_pct)}%` : ""}
            </span>
          </button>
          </Fragment>
        );
      })}
    </div>
  );

  // **The strip where a title would have sat.** This landing used to
  // print 「额度」 across the top, which no other pane does
  // (docs/UX.md): the pane's name rides its `aria-label`, and the room
  // holds the work — search, the three axes, the grouping, and 登记.
  const bar = (
    <PaneBar
      searchLabel={gui.search_quota}
      query={query}
      onQuery={onQuery}
      filterLabel={gui.filter}
      filter={{ options, chosen: words, onToggle: onToggleWord }}
      axes={axes}
      arrange={{
        label: gui.quota_group,
        options: [
          { key: GROUP_NONE, label: gui.quota_group_none },
          ...QUOTA_AXES.map((a) => ({ key: a, label: axisWord(a) })),
        ],
        chosen: groupBy,
        onChoose: chooseGroup,
      }}
      actionsLabel={gui.new_item}
      actions={[
        { key: "register", label: gui.accounts_register, onSelect: () => setRegistering(true) },
      ]}
    />
  );

  // **Two details, because there are two kinds of row.** The account's
  // is what this page always drew; the subscription's is every window
  // as **its own single ring** — the nesting on the row answers 「什么
  //最先卡住我」 and this answers 「每个窗口各是多少」, which is the
  // split `NestedRing`'s head requires of anything that stacks arcs.
  const acc = current?.account ?? null;
  const subDetail = current && current.account === null && (
    <div data-quota-detail={current.id} className="flex flex-col gap-4 p-4">
      <div className="flex min-w-0 flex-col">
        <span className="text-title">{current.name}</span>
        <span className="text-sm text-muted-foreground">
          {kindWord(current.kind)} · {providerWord(current.provider)} · {fromWord(current.from)}
        </span>
      </div>
      <div className="flex flex-wrap gap-6">
        {current.windows.map((w) => (
          <div key={w.kind} data-quota-window={w.kind} className="flex items-center gap-row">
            <Ring fraction={w.used_pct / 100} color={gaugeColor(null)} centerClass="text-lg">
              {`${Math.round(w.used_pct)}%`}
            </Ring>
            <span className="flex min-w-0 flex-col">
              <span className="text-fact">{windowWord(w.kind)}</span>
              {w.resets_at !== null && (
                <span className="text-aux text-muted-foreground">
                  {gui.quota_resets(until(w.resets_at * 1000))}
                </span>
              )}
            </span>
          </div>
        ))}
      </div>
    </div>
  );

  const detail = acc && (
    <div data-quota-detail={acc.reading.account} className="flex flex-col gap-4 p-4">
      <div className="flex items-center gap-4">
        <Ring
          fraction={acc.reading.trouble ? null : acc.reading.fill}
          color={gaugeColor(acc.strain)}
          size={RING_SIZE}
        >
          {centre(acc)}
        </Ring>
        <div className="flex min-w-0 flex-col">
          <span className="text-title">{acc.reading.name}</span>
          <span className="text-sm text-muted-foreground">
            {providerWord(acc.reading.provider)}
            {acc.base_url ? ` · ${acc.base_url}` : ""}
          </span>
        </div>
      </div>
      <dl data-quota-figures className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
        {acc.reading.trouble ? (
          <>
            <dt className="text-muted-foreground">{gui.accounts_state}</dt>
            <dd data-quota-trouble={acc.reading.trouble}>{troubleWord(acc.reading.trouble)}</dd>
          </>
        ) : (
          <>
            {acc.reading.spent != null && (
              <>
                <dt className="text-muted-foreground">{gui.accounts_spent}</dt>
                <dd className="tabular-nums">
                  {acc.reading.limit != null
                    ? gui.accounts_of(
                        money(acc.reading.spent, null),
                        money(acc.reading.limit, acc.reading.currency),
                      )
                    : money(acc.reading.spent, acc.reading.currency)}
                  {acc.reading.unlimited ? ` · ${gui.accounts_unlimited}` : ""}
                </dd>
              </>
            )}
            {acc.reading.remaining != null && (
              <>
                <dt className="text-muted-foreground">{gui.accounts_remaining}</dt>
                <dd data-quota-remaining className="tabular-nums">
                  {money(acc.reading.remaining, acc.reading.currency)}
                </dd>
              </>
            )}
            {BUDGETED.includes(acc.reading.provider) &&
              acc.reading.limit == null &&
              !acc.reading.unlimited && (
                <>
                  <dt className="text-muted-foreground">{gui.accounts_budget}</dt>
                  <dd className="text-muted-foreground">{gui.accounts_no_budget}</dd>
                </>
              )}
            {acc.reading.resets_at != null && (
              <>
                <dt className="text-muted-foreground">{gui.accounts_resets}</dt>
                <dd>{until(acc.reading.resets_at * 1000)}</dd>
              </>
            )}
          </>
        )}
        <dt className="text-muted-foreground">{gui.accounts_read_by}</dt>
        <dd data-quota-age={acc.age_ms}>
          {acc.mine
            ? gui.accounts_read_here
            : gui.accounts_read_from(acc.home_name, lasting(acc.age_ms))}
        </dd>
      </dl>
      <div>
        <Button
          size="sm"
          variant="outline"
          data-quota-remove={acc.reading.account}
          className="min-h-touch"
          onClick={() => remove(acc.reading.account)}
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
    <section data-quota-landing aria-label={gui.accounts_tab} className="flex h-full min-w-0 flex-col">
      {error && (
        <div data-quota-error className="border-b px-3 py-1.5 text-sm text-destructive">
          {error}
        </div>
      )}
      {rows.length === 0 ? (
        // Nothing registered and no subscription read: a search box
        // over an empty list is a control with nothing to do, and the
        // one thing there is to do is the button in `empty`.
        empty
      ) : narrow ? (
        // One screen, so the bar is the screen's — the list scrolls
        // under it and the detail follows below, as it did.
        <div className="flex min-h-0 flex-1 flex-col">
          {bar}
          <div className="min-h-0 flex-1 overflow-y-auto">
            {list}
            {detail}
            {subDetail}
          </div>
        </div>
      ) : (
        <div className="flex min-h-0 flex-1">
          {/* The bar belongs to the list column, not to the landing:
              what it narrows is the list, and a bar spanning the detail
              too would say it narrows that as well. Same seat it has on
              the sessions pane. */}
          <div className="flex w-72 flex-none flex-col border-r">
            {bar}
            <div className="min-h-0 flex-1 overflow-y-auto">{list}</div>
          </div>
          <div className="min-w-0 flex-1 overflow-y-auto">
            {detail}
            {subDetail}
          </div>
        </div>
      )}
      {/* ⚠️ **Tombstone: the subscription band is gone from the foot.**
          2026-09-06:「不要有现在下方单独一个订阅的横条」. It had a strip
          of its own because it is the one reading khor does not get
          from a key you registered — but that is a fact about *where it
          comes from*, and 来源 is now an axis of the list. It is a row
          (`SUB_ID`), its windows are the nested ring on that row, and
          each window is a single ring in its detail. `QuotaRings` had
          no reader left and went with it. */}
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
              {NEEDS_ADMIN.includes(provider) ? `${gui.accounts_key_stays} ${gui.accounts_needs_admin}` : gui.accounts_key_stays}
            </span>
          </label>
          {BUDGETED.includes(provider) && (
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
