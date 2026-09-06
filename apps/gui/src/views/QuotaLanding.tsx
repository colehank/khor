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
  type QuotaCredits,
  type QuotaWindow,
} from "@/api";
import { IconBack } from "@/components/icons";
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
 * How long each span khor can order is, in minutes — **and the table
 * having exactly these two entries is the rule about the ring**.
 *
 * ⚠️ **The ring draws one layer per *length*.** A window limited to one
 * model is a share of a length, not another length, so it never goes on
 * the ring: three concentric arcs where two of them are the same seven
 * days is precisely the shape removed on 2026-09-02 (`Ring`'s
 * tombstone). Those live in the detail, where each window is its own
 * single ring and nothing has to be told apart by radius.
 *
 * **This used to be a table with `null` in it** — `seven_day_sonnet:
 * null` meaning「not a length, do not ring it」— which encoded the rule
 * as an exception list that every new window had to be added to. The
 * type now says it: a span khor can order is a *string*, and one it
 * cannot is `{ other }`, so `typeof span === "string"` is the whole
 * test and a length nobody has seen yet is handled without an entry
 * here.
 *
 * The day the endpoint reports a thirty-day window, this table grows a
 * row and the ring becomes three layers, still ordered by length.
 */
const SPAN_MINUTES: Record<string, number> = {
  five_hour: 5 * 60,
  seven_day: 7 * 24 * 60,
};

/** One window's identity, for a React key and for the gate's selector:
    the span, plus the model it is limited to where there is one. The
    same pair the node merges on (`QuotaWindow::scope`). */
function windowId(w: QuotaWindow): string {
  const span = typeof w.span === "string" ? w.span : `other:${w.span.other}`;
  return w.scope ? `${span}:${w.scope}` : span;
}

/** The windows that go on the ring, **outermost first — longest
    first**, which is the one-sentence order `NestedRing` requires its
    caller to state: further out is a longer window. */
function ringWindows(ws: QuotaWindow[]): QuotaWindow[] {
  return ws
    .filter((w) => typeof w.span === "string" && w.scope === null && SPAN_MINUTES[w.span] != null)
    .sort((a, b) => (SPAN_MINUTES[String(b.span)] ?? 0) - (SPAN_MINUTES[String(a.span)] ?? 0));
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

/**
 * A window's name — the same string on the row and in the detail. No
 * short form: a second spelling of one window is the drift this page's
 * whole vocabulary is built to avoid.
 *
 * **Three sources, in this order, and only the first is khor's.** A
 * window limited to a model wears that model's name exactly as the
 * endpoint spelled it (「Fable」) — khor has no word for a model it may
 * not have heard of, and inventing one is how a page ends up naming
 * something the vendor renamed last week. A whole-plan window of a
 * length khor can order gets the catalog's word. A length khor cannot
 * order wears the endpoint's own group word, which may not be a human
 * sentence — that is the endpoint talking, not khor claiming.
 */
function windowWord(w: QuotaWindow): string {
  if (w.scope) return w.scope;
  if (typeof w.span !== "string") return w.span.other;
  return w.span === "five_hour" ? gui.quota_five_hour : gui.quota_seven_day;
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
  /** The plan behind this row, where the row has one —「Max 20x」 for
      the subscription, nothing for a registered account. Read from the
      stored credential by the node (`Quota::plan`), which is why it is
      a product name rather than a catalog word. */
  plan: string | null;
  account: AccountRing | null;
  windows: QuotaWindow[];
  credits: QuotaCredits | null;
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
/** The line under a row's name, and the same line under the detail's
    title. **One function, because they are one sentence** — the row and
    its detail disagreeing about what a row *is* was the shape this page
    kept drifting into. The plan sits second where there is one: it
    qualifies 「订阅」, and reads as part of it. */
function subLine(r: Row): string {
  return [kindWord(r.kind), r.plan, providerWord(r.provider), fromWord(r.from)]
    .filter(Boolean)
    .join(" · ");
}

/** The `disabled_reason` key's sentence. A key this build never heard
    of is shown as itself — same bargain `troubleWord` makes. */
function creditWord(key: string): string {
  return key === "out_of_credits" ? gui.quota_credits_out : key;
}

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
    const said = [r.name, r.plan ?? "", ...QUOTA_AXES.map((a) => valueWord(a, on[a] ?? ""))].join(" ");
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
  screen = "list",
  onOpen,
  onBack,
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
  /** **The narrow face's half of `App`'s one-screen-at-a-time pair**,
      and it is the shell's state rather than this file's on purpose:
      the entrance animation is keyed off `screen` up there
      (`data-screen`), and a second copy down here would be a landing
      that slides on its own schedule. Wide passes none of the three —
      there the list never leaves, so there is nothing to go back
      from, which is the same rule `DetailPane`'s back button follows. */
  screen?: "list" | "detail";
  onOpen?: () => void;
  onBack?: () => void;
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
            plan: quota.quota.plan,
            account: null,
            windows: quota.quota.windows,
            credits: quota.quota.credits,
          },
        ]
      : []),
    ...(board ?? []).map((a) => ({
      id: a.reading.account,
      name: a.reading.name,
      kind: "bill" as const,
      provider: a.reading.provider,
      from: "",
      plan: null,
      account: a,
      windows: [] as QuotaWindow[],
      credits: null,
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

  // **A detail screen with nothing to show is a dead end**: narrowing
  // the list can take the picked row away (search, a tick), and the
  // shell would still be on the detail half with no row and no way but
  // the back button. Same shape as `App`'s own rule for a session that
  // has gone.
  useEffect(() => {
    if (screen === "detail" && !current) onBack?.();
  }, [screen, current, onBack]);

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
            onClick={() => {
              setPicked(r.id);
              onOpen?.();
            }}
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
                tracks={layers.map((w) => ({ id: windowId(w), fraction: w.used_pct / 100 }))}
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
                {subLine(r)}
              </span>
            </span>
            <span className="flex-none text-xs tabular-nums text-muted-foreground">
              {r.account ? centre(r.account) : bad ? `${windowWord(bad)} ${Math.round(bad.used_pct)}%` : ""}
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
        <span className="text-sm text-muted-foreground">{subLine(current)}</span>
      </div>
      <div className="flex flex-wrap gap-6">
        {current.windows.map((w) => (
          <div key={windowId(w)} data-quota-window={windowId(w)} className="flex items-center gap-row">
            <Ring fraction={w.used_pct / 100} color={gaugeColor(null)} centerClass="text-lg">
              {`${Math.round(w.used_pct)}%`}
            </Ring>
            <span className="flex min-w-0 flex-col">
              <span className="text-fact">{windowWord(w)}</span>
              {w.resets_at !== null && (
                <span className="text-aux text-muted-foreground">
                  {gui.quota_resets(until(w.resets_at * 1000))}
                </span>
              )}
            </span>
          </div>
        ))}
      </div>
      {/* **The credits, and they are not a window.** Windows are a
          fraction of a period that refills by itself; this is money,
          it does not refill and it has no reset. So it is a list of
          figures rather than a ring, it is not in `ringWindows`, and
          the row's right-hand summary never names it — three separate
          places where treating it as a window would have said 「how
          full am I」 with two different kinds of thing.
          The dl is the account detail's, reused: one shape for
          「figures about an account」 on this page, not a second. */}
      {current.credits && (
        <dl
          data-quota-credits
          className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm"
        >
          <dt className="text-muted-foreground">{gui.quota_credits}</dt>
          <dd data-quota-credits-on={current.credits.enabled}>
            {current.credits.enabled ? gui.quota_credits_on : gui.quota_credits_off}
            {/* The reason only where there is one to give, and it is
                the endpoint's key turned into a sentence — a key khor
                has never heard of is printed as itself rather than
                swallowed, because a person can act on a reason they
                can read. */}
            {!current.credits.enabled && current.credits.disabled_reason
              ? ` · ${creditWord(current.credits.disabled_reason)}`
              : ""}
          </dd>
          {/* **`!= null`, not truthiness**: a balance of exactly 0 is a
              fact worth drawing, and `0 &&` would hide precisely the
              reader who most needs to see it. */}
          {current.credits.used != null && (
            <>
              <dt className="text-muted-foreground">{gui.accounts_spent}</dt>
              <dd data-quota-credits-used className="tabular-nums">
                {money(current.credits.used, current.credits.currency)}
              </dd>
            </>
          )}
          {current.credits.balance != null && (
            <>
              <dt className="text-muted-foreground">{gui.accounts_remaining}</dt>
              <dd data-quota-credits-balance className="tabular-nums">
                {money(current.credits.balance, current.credits.currency)}
              </dd>
            </>
          )}
          {current.credits.monthly_limit != null && (
            <>
              <dt className="text-muted-foreground">{gui.quota_credits_monthly}</dt>
              <dd className="tabular-nums">
                {money(current.credits.monthly_limit, current.credits.currency)}
              </dd>
            </>
          )}
        </dl>
      )}
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
        // **One screen at a time, with a way back** — the same pair the
        // sessions landing draws (2026-09-06: 额度 became an inbox, so
        // it takes the inbox posture). It was a list and a detail
        // stacked in one scroller, which reads as「everything at once」
        // on a phone: no exit, and the thing you pressed opens below
        // the fold you are already looking at.
        screen === "detail" && current ? (
          <div className="flex min-h-0 flex-1 flex-col">
            <header
              data-detail-header
              className="flex min-h-ctl-lg flex-none items-center gap-2 border-b px-3"
            >
              {/* **The only exit from this screen**, so it carries the
                  finger floor outright — the rule `DetailPane`'s back
                  button was rewritten for. */}
              <Button
                variant="ghost"
                size="icon"
                aria-label={gui.back}
                data-back
                onClick={onBack}
                className="min-h-touch min-w-touch"
              >
                <IconBack />
              </Button>
              {/* **And nothing else — the name is not repeated here.**
                  It was, for one screenshot: the bar said the row's
                  name and the body's first line said it again, one
                  above the other. That is the same duplication
                  2026-09-02 took out of the session header
                  (「都和标签内容重复了」), and the answer is the same —
                  the content titles itself, so the bar is the way out
                  and only that. `DetailPane` keeps an identity in its
                  bar for the opposite reason: a terminal body has no
                  title of its own to read. */}
            </header>
            <div className="min-h-0 flex-1 overflow-y-auto">
              {detail}
              {subDetail}
            </div>
          </div>
        ) : (
          <div className="flex min-h-0 flex-1 flex-col">
            {bar}
            <div className="min-h-0 flex-1 overflow-y-auto">{list}</div>
          </div>
        )
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
