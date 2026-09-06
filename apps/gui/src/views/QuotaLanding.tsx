// 额度: the API accounts a person registered, each as a dial.
//
// **One dial is one account, and the account is on one machine.** The
// key never leaves the machine it was registered on; that machine reads
// the provider and every other machine shows what it last read, with
// how long ago (`khor_node::accounts`). So a reading here is never "this
// screen asked the provider" — it is "the home machine last read this",
// and the age beside it is part of the reading, not decoration.
//
// **The drawing is the machine card's** (2026-09-07:「环全部和机器卡对
// 齐」): the same 210° `Face`, one size down in a row. A subscription's
// arc is its fullest window and every window is a pip on the rim, one
// for one with the lines listed beside it; an account has one reading
// and no parts, so it has no pips.
//
// **The fraction and the ink are the node's.** `fill` and `strain`
// arrive decided (a new-api token fills against its own quota, the two
// vendors against the registered budget, no budget is no fraction);
// this page turns「no fraction」into a full arc or bare track by the
// rule in `arcOf`, and paints in the strain ink the disks wear, never a
// ramp of its own.
//
// Registration happens here too, and the form says the two things a
// person cannot guess: the key stays on this machine, and the two
// vendors' usage is an **admin** key's to read.
import { Fragment, useEffect, useMemo, useState } from "react";

import {
  fetchQuotaBoard,
  pinAccount,
  registerAccount,
  removeAccount,
  updateAccount,
  type AccountRing,
  type QuotaAnswer,
  type QuotaCredits,
  type QuotaWindow,
} from "@/api";
import { IconBack } from "@/components/icons";
import { type OmniAxis } from "@/components/Omnibox";
import { PaneBar, type FilterOption } from "@/components/PaneBar";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import { DIAL_ROW_RATIO, DIAL_ROW_SIZE, DIAL_ROW_STROKE, Face, type Pip } from "@/components/Face";
import { RING_SIZE_CARD, RING_STROKE_CARD } from "@/components/Ring";
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

/* ⚠️ **Tombstone: `SPAN_MINUTES` and `ringWindows` are gone,
   2026-09-07 — and with them the rule 「一个模型的份额不是第三个长度,
   所以它不上环」.**

   That rule was right about *concentric arcs*: stacking a week's
   Fable share inside the week it is a share of made two radii mean
   two different things. The drawing changed, and the rule did not
   survive the change. On a dial there are no radii to confuse — every
   window is a mark on one rim, at the angle of its own number — and
   the user's judgment is that all of them belong there:
   「你绘制的只有两个外环线段。但它有四个环。如果是两两重叠就没关系」.

   **The replacement is a counting rule, not a filter**: the rim
   carries exactly as many pips as the spread has dials, one per
   window, and two windows at the same percent are two pips on one
   spot. A window khor has no length for still gets a pip, because the
   endpoint still gave it a number — where the old code needed a
   `SPAN_MINUTES` entry to draw anything at all.

   `windowId` stayed: it is an identity, not an ordering. */

/** One window's identity, for a React key and for the gate's selector:
    the span, plus the model it is limited to where there is one. The
    same pair the node merges on (`QuotaWindow::scope`). */
function windowId(w: QuotaWindow): string {
  const span = typeof w.span === "string" ? w.span : `other:${w.span.other}`;
  return w.scope ? `${span}:${w.scope}` : span;
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
  const span = typeof w.span !== "string"
    ? w.span.other
    : w.span === "five_hour"
      ? gui.quota_five_hour
      : gui.quota_seven_day;
  // **Both halves, joined by a separator that is not copy.** 「名字从
  // payload 来」 (2026-09-06) forbids khor spelling the *model*; it does
  // not say to throw the length away, and 「Fable」 alone leaves a
  // reader with no idea what period they are 35% through. The `·` is
  // punctuation, not a word, so it stays out of `zh.toml`.
  return w.scope ? `${span} · ${w.scope}` : span;
}

/**
 * One line of the 额度 list, whichever of the two things it is.
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
  /**
   * How old this reading is, in ms, and `null` for 「taken just now」.
   *
   * ⚠️ **The subscription had no such field until 2026-09-07, and that
   * is what 2026-08-28 actually removed.** The tombstone in `zh.toml`
   * says 「同一刀去掉的还有读数年龄」 and reads as a display that was
   * dropped; what was dropped was the number ever reaching this file.
   * `Quota::as_of` has been on the wire the whole time (「`None` means
   * just now, this call」), and the cache it distinguishes is five
   * minutes deep — so the screen could be five minutes stale with
   * nothing on it saying so, which is the sentence that tombstone's own
   * last paragraph ends on.
   */
  age_ms: number | null;
};

/**
 * Credits as a share of something, or `null` when it is not one.
 *
 * **A dial needs a denominator and credits only sometimes has one.**
 * The endpoint reports a `monthly_limit` on some accounts and not on
 * others (the capture this was written against reports a spend and no
 * ceiling at all), so credits joins the pips **only when it has a
 * fraction of its own** — 2026-09-07:「Credits 只有在有分母时才是一个
 * 环」. With no ceiling it is a line of facts beside the dial, not an
 * empty track on it: an empty track is the picture of *nothing used*,
 * which is the opposite of what a spent-out balance means.
 *
 * `enabled` is `extra_usage.is_enabled` (`khor_node::quota`), which is
 * the switch that decides whether this money can be spent at all — off,
 * and there is no window to be a fraction of either.
 */
function creditsPct(c: QuotaCredits | null): number | null {
  if (c === null || !c.enabled) return null;
  if (c.used == null || c.monthly_limit == null || c.monthly_limit <= 0) return null;
  return Math.min(100, (c.used / c.monthly_limit) * 100);
}

/** One thing a subscription's dial is made of: a window, or credits
    when credits has a ceiling.

    **`window` is not decoration.** Credits is a part of this dial and
    it is still not a window — it is money, it does not refill, it has
    no reset — so what is drawn from this list carries the two names
    apart, and only a real window answers to 「window」. */
type Dial = { id: string; word: string; pct: number; resets_at: number | null; window: boolean };

/**
 * Everything the subscription's dial is made of — **one list, feeding
 * the rim, the row and (later) the spread**, so their counts cannot
 * disagree.
 *
 * That is the rule that replaced `ringWindows`'s filter: 「轮缘卡点数 =
 * 展开后的表盘数,一一对应」. A window this build has no length word for
 * is still a pip, because the endpoint still gave it a number.
 */
function subDials(r: Row): Dial[] {
  const out: Dial[] = r.windows.map((w) => ({
    id: windowId(w),
    word: windowWord(w),
    pct: w.used_pct,
    resets_at: w.resets_at,
    window: true,
  }));
  const credits = creditsPct(r.credits);
  // Never a reset: credits is money, it does not refill by itself.
  if (credits !== null) {
    out.push({ id: "credits", word: gui.quota_credits, pct: credits, resets_at: null, window: false });
  }
  return out;
}

/** The one a reader is closest to running out of — the arc the dial
    draws and the number in its middle. **Chosen by a rule, not
    picked**: it is the one that will stop you first. */
function fullest(dials: Dial[]): Dial | null {
  return dials.reduce<Dial | null>((m, d) => (m === null || d.pct > m.pct ? d : m), null);
}

/**
 * What sits in the dial's centre: the share when there is one, the
 * spend when there is a number but no denominator, what is left when
 * the provider reports a wallet rather than a spend, nothing else.
 *
 * **On the row. The card's dial says only the share** — see the note on
 * the card's two columns.
 */
function centre(r: AccountRing): string {
  const { reading } = r;
  if (reading.trouble) return "";
  if (reading.fill != null) return `${Math.round(reading.fill * 100)}%`;
  if (reading.spent != null) return money(reading.spent, null);
  if (reading.remaining != null) return money(reading.remaining, null);
  return "";
}

/**
 * What the **card's** dial says in its middle: the share, or nothing.
 *
 * ⚠️ **The mock contradicts itself here, and this is the one place the
 * implementation deliberately does not follow it.** In
 * `khor-usage-mock.html` the account card prints the money twice — once
 * under the dial (`12.34 / 100 USD`) and again in the facts beside it
 * (`已用 12.34` / `上限 100`). The mock is the user's own reviewed
 * draft, so **somebody reading it against this file will see a missing
 * line and take it for a drop.** It is not: the card's two halves are
 * 「表盘 | 事实」, and a number that appears in both halves makes the
 * split mean nothing. The dial says how full; the list says the
 * figures.
 *
 * **The cost, stated so it is not mistaken for an omission either**: a
 * reading with no denominator (不限量, a prepaid wallet) gets a full arc
 * and an **empty** middle, with the number on the right. Full arc +
 * nothing + 「不限量」 is a whole sentence — the mock draws it that way
 * too (`centre: ""`).
 *
 * The row keeps `centre`: a list has no second column to put a figure
 * in, so there the middle is the only place a number can go.
 */
function share(reading: AccountRing["reading"]): string {
  if (reading.trouble || reading.fill == null) return "";
  return `${Math.round(reading.fill * 100)}%`;
}

/**
 * How much arc to draw for an account.
 *
 * **A ring with no ceiling is a full ring, not an empty one**
 * (2026-09-06 user ruling). An 不限量 plan and a prepaid wallet both
 * have a reading and no denominator, and the old code drew them as bare
 * track — which is the picture of *nothing used*, the opposite of what
 * is true. There is nothing to be a fraction *of*, so the honest arc is
 * the whole circle in neutral ink: the ring says「this is fine」and the
 * centre says the number.
 *
 * **Only 问不出来 leaves track showing**, and that is the point of the
 * distinction: an empty ring now means one thing — khor could not get
 * an answer — instead of meaning that *or* an unlimited plan.
 *
 * **This is a drawing rule and it lives here**, in the face. The node's
 * `fill` stays `None`: it is the honest answer to「what fraction」, and
 * turning it into 1.0 up there would put a number on the wire that the
 * provider never reported.
 */
function arcOf(reading: AccountRing["reading"]): number | null {
  if (reading.trouble) return null;
  return reading.fill ?? 1;
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
/**
 * The line under a row's name — **one word**.
 *
 * The plan where there is one (a subscription's 「Max 20x」), the
 * provider where there is not (「new-api 站」). 2026-09-07:「列表副行只剩
 * 档或厂商一个词(来源是组头)」.
 *
 * # What the three segments cost, and where they went
 *
 * It was 类型 · 提供商 · 来源, and at this column's 288 the third was
 * being drawn as 「khor 登…」 on the wider rows — **an axis you can tick
 * and cannot read**, which is the shape this repo keeps being bitten
 * by. Making the line longer was not available; making it shorter meant
 * choosing which axis a row wears.
 *
 * **None of the three was dropped, they moved to where there is room**:
 * 来源 is the heading over the group (which is why the page now opens
 * grouped by it — `GROUP_DEFAULT`), 提供商 is this line on every row
 * that has no plan, and 类型 is the tick and the heading you get by
 * choosing that axis. What a row shows is one of them; what the *page*
 * shows is all three.
 *
 * The plan also left the name's line, where it was a chip
 * (`f9e2062`, one day old): a chip beside the name and a word under it
 * were two spellings of one fact, and the row only has room for one.
 * The **card's** head keeps its chip — there the name is a title and
 * the plan is a badge on it, which is a different sentence.
 */
function subLine(r: Row): string {
  return r.plan ?? providerWord(r.provider);
}

/** The `disabled_reason` key as a status word — never a sentence
    (2026-09-06: 「只留数,不留句子」). A key this build never heard of
    is shown as itself, the same bargain `troubleWord` makes. */
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
 * **Search reads what the *page* shows** — which stopped being the
 * same as what the row shows on 2026-09-07, when the sub-line was cut
 * to one word.
 *
 * The rule it replaces was 「search reads the words on the row」, and
 * its reason was that a hit you cannot see on the row it matched reads
 * as the search being broken. That reason survives; what changed is
 * where the words are. 来源 is the heading over the group this page
 * opens on, 类型 and 提供商 are the ticks in the bar and the headings
 * you get by choosing those axes — **all three are on the glass, none
 * of them is on every row.** So typing 「turing」 still lands you
 * somewhere you can see why, and searching a field the page never
 * prints is still not a thing this box does.
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

/**
 * **来源 is the default, and that is not a preference about ordering.**
 *
 * 2026-09-07 cut the sub-line down to one word — the plan for a
 * subscription, the provider for an account — because three segments in
 * a 288-wide column truncate. 来源 was one of the three that left. It is
 * still an axis you can filter by and still an axis you can group by,
 * and with neither of those switched on it would be **an axis you can
 * sieve with and cannot see**: the exact shape this repo keeps being
 * bitten by (an axis you can tick but cannot read off the row).
 *
 * Grouping by it puts it back where it can be read — as the heading over
 * the rows it describes, which is also where the mock has it. The other
 * two axes are one press away and neither was ever visible on the row
 * for free.
 */
const GROUP_DEFAULT = "from:";

/**
 * ⚠️ **A new key, on purpose** (`…group` → `…group.v2`, 2026-09-07).
 *
 * The default moved from 「不分组」 to 来源, and the old key holds
 * literally `"none"` on every machine that has ever opened this page —
 * including one where nobody chose it, because the old default was
 * written down the first time the control was touched. Reading it back
 * would mean the ruling above lands on no screen that has ever seen
 * this page, which is the same as not making it.
 *
 * **The storage cannot tell 「he chose flat」 from 「he never chose」**,
 * and that is why this is a rename rather than a migration: with one
 * string there is no answer to distinguish, so the honest move is to
 * start the question over rather than infer an answer nobody gave.
 * A person who wants a flat list is one press from it and that press is
 * remembered.
 *
 * # ⚠️ This is not a general move, and here is its expiry date
 *
 * **Renaming the key throws away everybody's setting.** It is allowed
 * this once because of a fact with a date on it: the axis being
 * defaulted to was *itself* added at 18:15 on 2026-09-06 (`3e9b4d9`,
 * 「额度那栏接上搜索、三条轴的过滤和分组」), seven hours before this
 * change — and in those seven hours the only sessions that opened this
 * page on any machine were development and acceptance. **Every stored
 * value is a leftover from building it, not a choice anybody made**, so
 * 「respect the explicit choice」 is protecting something that does not
 * exist.
 *
 * The next person changing this default will very likely **not** have
 * that fact. If the axis has been in real hands for months, renaming
 * the key is throwing away other people's settings, and the honest fix
 * is the one this note keeps pointing at: **make the storage able to
 * tell 「chose flat」 from 「never chose」** — a second value, or an
 * absent key meaning absent — and then a default can move without
 * touching anybody's answer. That work is not done here; it is what
 * this shortcut is borrowed against.
 */
const GROUP_KEY = "khor.quota.group.v2";

function storedGroup(): string {
  try {
    const raw = window.localStorage.getItem(GROUP_KEY);
    return raw && (raw === GROUP_NONE || QUOTA_AXES.includes(raw as never)) ? raw : GROUP_DEFAULT;
  } catch {
    return GROUP_DEFAULT;
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
  // Which cards have their drawer open. **A set keyed by row, not one
  // boolean**: 「I opened this card」 is a fact about that card, and a
  // single flag would make walking away and back a way of closing it.
  const [spread, setSpread] = useState<Set<string>>(() => new Set());
  const [registering, setRegistering] = useState(false);
  /** Rows whose last pin attempt did not take — the same shape `App`
      keeps for the machine list, and for its reason: what failed
      outlives one opening of a menu. */
  const [pinFailed, setPinFailed] = useState<Set<string>>(() => new Set());
  /** The node's own sentence for a removal it refused, by row. Cleared
      the moment a later attempt on that row gets anywhere. */
  const [removeRefused, setRemoveRefused] = useState<Map<string, string>>(() => new Map());
  /** The account being edited, or null. The row, not the id: the dialog
      fills itself from what the list already knows, so opening it costs
      no round trip. */
  const [editing, setEditing] = useState<Row | null>(null);
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
            // `as_of` is unix seconds and `null` is 「this call」 — the
            // node's own words, kept apart from 「zero ms ago」 because
            // a served cache entry and a fresh fetch look identical
            // otherwise (`Quota::as_of`).
            age_ms: quota.quota.as_of === null ? null : Date.now() - quota.quota.as_of * 1000,
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
      // The node answers 0 for a reading this machine took itself.
      age_ms: a.mine ? null : a.age_ms,
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

  /**
   * The row's own actions — **the same gesture the machine and session
   * lists answer** (2026-09-07:「右键列表里的条目弹 置顶 / 编辑 / 删除」).
   *
   * They were a button on the card until now, which is a control a
   * phone has no room for and a card has no business carrying: the card
   * is the reading, the row is the thing. `RowMenu` gives one mechanism
   * for right-click, long-press and the keyboard's Menu key.
   *
   * **Only an account has these.** The subscription row is not a
   * registration — there is nothing to edit and nothing to remove, and
   * an item whose one answer is no is not offered (`DevicesList` makes
   * the same refusal for this machine's own row).
   *
   * **Each action rejects when it did not take**, which is how the menu
   * learns to stay open: `RowMenu` closes on a resolve and keeps the
   * sentence on screen on a reject, so a failure is reported where the
   * press was made.
   */
  const actionsFor = (r: Row): RowAction[] => {
    const acc = r.account;
    if (!acc) return [];
    const id = acc.reading.account;
    return [
      {
        key: "pin",
        // The word is what pressing it *does*, so it is the opposite of
        // the row's state — which is also what makes this item say what
        // the state is. The failure word rides the item that was
        // pressed (docs/UX.md 做了但没变化 / 失败).
        label: pinFailed.has(id)
          ? acc.pinned
            ? gui.unpin_failed
            : gui.pin_failed
          : acc.pinned
            ? gui.unpin
            : gui.pin,
        failed: pinFailed.has(id),
        run: () =>
          pinAccount(id, !acc.pinned)
            .then(() => {
              setPinFailed((was) => {
                const next = new Set(was);
                next.delete(id);
                return next;
              });
              return fetchQuotaBoard().then(setBoard);
            })
            .catch((e) => {
              setPinFailed((was) => new Set(was).add(id));
              // Rethrown, not swallowed: the menu reads the rejection
              // to decide between closing and staying open.
              throw e;
            }),
      },
      { key: "edit", label: gui.accounts_edit, run: () => setEditing(r) },
      {
        key: "remove",
        label: gui.accounts_remove,
        confirm: gui.accounts_remove_confirm,
        note: removeRefused.get(id),
        run: () => remove(id),
      },
    ];
  };

  const remove = (id: string) =>
    removeAccount(id)
      .then(() => {
        setBoard((was) => (was ? was.filter((r) => r.reading.account !== id) : was));
        if (picked === id) setPicked(null);
        setRemoveRefused((was) => {
          const next = new Map(was);
          next.delete(id);
          return next;
        });
      })
      .catch((e) => {
        const said = String(e instanceof Error ? e.message : e);
        // **On the menu, not in the error strip.** The press was made
        // there; a refusal that appeared at the top of the pane would
        // leave the person looking at an open menu that said nothing.
        setRemoveRefused((was) => new Map(was).set(id, said));
        throw e;
      });

  const list = (
    <div data-quota-list className="flex min-w-0 flex-col">
      {grouped.length === 0 && (
        <div data-quota-none className="p-4 text-sm text-muted-foreground">
          {gui.no_matches}
        </div>
      )}
      {grouped.map((r, i) => {
        const on = current?.id === r.id;
        // **One list for the rim and for the summary**, so the mark
        // count and the word on the right cannot come from two
        // different ideas of what this row is made of.
        const dials = r.account ? [] : subDials(r);
        const lead = fullest(dials);
        // A heading starts where the group changes between neighbouring
        // rows. `GROUP_NONE` answers the empty string for every row, so
        // it draws none — no branch of its own.
        const group = groupBy === GROUP_NONE ? null : (answersOf(r)[groupBy] ?? "");
        const opens = group !== null && group !== (i > 0 ? answersOf(grouped[i - 1]!)[groupBy] : undefined);
        const actions = actionsFor(r);
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
          {/* **The trigger is the row's own button.** `RowMenu` renders
              no element of its own, and the press handler it adds has
              to land on something the DOM already admits is a target —
              a handler on a bare wrapper is reachable by pointer alone,
              which `check-face`'s section 4 asserts against.
              **The subscription row gets no menu**, and the empty
              branch is the point: it is not a registration, so 编辑 and
              删除 have nothing to act on and 置顶 would need a flag no
              doc holds for it. An item whose one answer is no is not
              offered (`DevicesList` refuses this machine's own row for
              the same reason). */}
          <RowMenuMaybe items={actions}>
          <button
            type="button"
            data-quota-row={r.id}
            data-on={on}
            data-provider={r.provider}
            data-quota-kind={r.kind}
            data-quota-from={r.from || "khor"}
            data-quota-pinned={r.account ? r.account.pinned : undefined}
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
            {/* **The machine card's dial, one size down** — the same
                210° geometry, the same milestones, the same rim
                (2026-09-07:「环全部和机器卡对齐」). The arc is the
                fullest window; every window is a pip at its own angle,
                so two windows at the same percent are two marks on one
                spot and a row with one window looks like a gauge rather
                than a stack. No number inside at this size: the number
                is the word on the right, where there is room to say
                *which* window it is. */}
            <Face
              size={DIAL_ROW_SIZE}
              stroke={DIAL_ROW_STROKE}
              ratio={DIAL_ROW_RATIO}
              fraction={r.account ? arcOf(r.account.reading) : lead ? lead.pct / 100 : null}
              strain={r.account?.strain ?? null}
              pips={dials.map((d): Pip => ({ word: d.word, v: d.pct }))}
            />
            <span className="flex min-w-0 flex-1 flex-col">
              {/* ⚠️ **The plan's chip left this row on 2026-09-07**,
                  one day after it arrived (`f9e2062`). It was a chip
                  beside the name *and* the sub-line said 类型 · 提供商 ·
                  来源 underneath; the ruling cut the line to one word,
                  and that word for a subscription **is the plan** — so
                  the chip and the line had become two spellings of the
                  same fact, on a row with room for one.
                  The card's head still wears it: there the name is a
                  title and the plan is a badge on it. */}
              <span className="truncate text-fact">{r.name}</span>
              {/* **One word**, and which word depends on what this row
                  has: the plan, or the provider (`subLine`). */}
              <span data-quota-sub-line className="truncate text-xs text-muted-foreground">
                {subLine(r)}
              </span>
            </span>
            <span className="flex-none text-xs tabular-nums text-muted-foreground">
              {r.account ? centre(r.account) : lead ? `${lead.word} ${Math.round(lead.pct)}%` : ""}
            </span>
          </button>
          </RowMenuMaybe>
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

  // **Two details, because there are two kinds of row.** The
  // subscription's leads with the same dial its row wears, one size up:
  // the row answers 「什么最先卡住我」 at a glance and the card answers
  // 「每个窗口各是多少」 in words beside it — one drawing, read twice,
  // instead of a second vocabulary on the second screen.
  const acc = current?.account ?? null;
  const dials = current && current.account === null ? subDials(current) : [];
  const lead = fullest(dials);
  const subDetail = current && current.account === null && (
    <div data-quota-detail={current.id} className="flex flex-col gap-4 p-4">
      <CardHead row={current} />
      {/* **The dial, and what it is made of beside it.** Every window
          used to be a ring of its own, side by side — which drew the
          same quantity at four radii and left 「哪个最先卡住我」 to be
          worked out by comparing four circles. One dial answers that
          (the arc is the fullest window, the number in it is that
          window's), and the rim carries a mark per window at its own
          angle: **as many pips as there are lines in the list beside
          it, one for one**. Two windows at the same percent are two
          marks on one spot, which is a true drawing of two windows that
          are equally full.
          Credits is in this list when it has a ceiling to fill against
          — then it is a pip like any other; when it has none it is a
          figure below and no mark, because a mark needs an angle and an
          angle needs a denominator (`creditsPct`). */}
      <div data-quota-lead className="flex flex-wrap items-center gap-x-8 gap-y-4">
        <Face
          size={RING_SIZE_CARD}
          stroke={RING_STROKE_CARD}
          fraction={lead ? lead.pct / 100 : null}
          pips={dials.map((d): Pip => ({ word: d.word, v: d.pct }))}
        >
          {lead ? `${Math.round(lead.pct)}%` : ""}
        </Face>
        {/* **The right column, and everything in it is a fact about
            this subscription** — the windows, then the credits. They
            are one column rather than two blocks stacked under the card
            because that is the half the dial does not fill
            (2026-09-07:「表盘 | 事实」). */}
        <div className="flex min-w-0 flex-col gap-3">
        <dl
          data-quota-parts
          className="grid min-w-0 grid-cols-[auto_1fr_auto] gap-x-6 gap-y-1 text-sm"
        >
          {dials.map((d) => (
            // `contents`, so the three cells line up in the grid while
            // the window still has an element of its own to be named
            // by — a gate that reads the *name* rather than the block
            // needs somewhere to hang the name (measuring the block and
            // splitting on whitespace reads 「39%7」).
            <div
              key={d.id}
              data-quota-part={d.id}
              data-quota-window={d.window ? d.id : undefined}
              className="contents"
            >
              <dt data-quota-window-name className="text-fact">{d.word}</dt>
              <dd className="text-muted-foreground">
                {d.resets_at !== null ? gui.quota_resets(until(d.resets_at * 1000)) : ""}
              </dd>
              <dd data-quota-window-pct className="text-right font-title tabular-nums">
                {Math.round(d.pct)}%
              </dd>
            </div>
          ))}
        </dl>
        {/* **The drawer's handle, and it says what pressing it does** —
            not what the drawer is (the same rule the machine row's
            置顶 / 取消置顶 follows). Below the list rather than beside
            the dial, because what it opens is a bigger version of the
            list. */}
        {dials.length > 0 && (
          <button
            type="button"
            data-quota-spread-toggle={current.id}
            aria-expanded={spread.has(current.id)}
            onClick={() =>
              setSpread((was) => {
                const next = new Set(was);
                if (!next.delete(current.id)) next.add(current.id);
                return next;
              })
            }
            className="flex min-h-touch items-center self-start text-xs text-muted-foreground hover:text-foreground"
          >
            {spread.has(current.id) ? gui.quota_spread_close : gui.quota_spread_open}
          </button>
        )}
      {/* **The credits' figures, and credits is still not a window.**
          A window is a fraction of a period that refills by itself;
          this is money, it does not refill and it has no reset — which
          is why it has no reset line in the list above and why nothing
          here calls it one (`Dial.window`).
          ⚠️ **What changed on 2026-09-07 is narrower than it looks**:
          credits now takes a pip *when the endpoint gave it a
          ceiling*, because at that point it is a fraction like the
          others and the user asked for one mark per part. With no
          ceiling it is exactly what it always was — these figures and
          no mark (`creditsPct`).
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
      </div>
      {/* **One dial per part, and that is the same list the rim was
          drawn from** (`subDials`) — 「轮缘卡点数 = 展开后的表盘数,
          一一对应」. The lead's dial answers 「什么最先卡住我」 with one
          arc and a rim; this answers 「每个窗口各是多少」 by giving each
          one its own arc, at the size where a number can sit inside it.
          Nothing here is a second reading: it is the marks, spread out. */}
      {spread.has(current.id) && (
        <div data-quota-spread className="flex flex-wrap gap-x-8 gap-y-4 border-t pt-4">
          {dials.map((d) => (
            <div
              key={d.id}
              data-quota-spread-dial={d.id}
              className="flex min-w-0 flex-col items-center gap-0.5"
            >
              <Face size={RING_SIZE_CARD} stroke={RING_STROKE_CARD} fraction={d.pct / 100}>
                {`${Math.round(d.pct)}%`}
              </Face>
              <span className="text-fact">{d.word}</span>
              {d.resets_at !== null && (
                <span className="text-aux text-muted-foreground">
                  {gui.quota_resets(until(d.resets_at * 1000))}
                </span>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );

  const detail = acc && current && (
    <div data-quota-detail={acc.reading.account} className="flex flex-col gap-4 p-4">
      <CardHead row={current} />
      {/* **The same two columns the subscription's card has**: the dial
          on the left, this reading's own figures on the right
          (2026-09-07:「首节表盘 | 事实两栏,单读数的卡右边不许空」).
          Stacked, the figures sat under a dial with half a card of white
          space beside it — and a card whose right half is empty reads as
          a card that failed to load something.
          **No pips**: an account is one reading and has no parts, and a
          rim with no marks is the honest drawing of that. */}
      <div data-quota-lead className="flex flex-wrap items-center gap-x-8 gap-y-4">
        <Face
          size={RING_SIZE_CARD}
          stroke={RING_STROKE_CARD}
          fraction={arcOf(acc.reading)}
          strain={acc.strain}
        >
          {share(acc.reading)}
        </Face>
        <dl data-quota-figures className="grid min-w-0 grid-cols-[auto_1fr] gap-x-6 gap-y-1 text-sm">
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
                  {/* **The wallet's one line, and why it is the only
                      one.** DeepSeek and Moonshot answer a balance and
                      nothing else — no spend, no ceiling, no day
                      buckets — so this card has one figure and the
                      sentence says that is the endpoint's doing, not a
                      reading khor failed to take. **It says what the
                      provider gave, never what khor cannot draw**: the
                      second half would be explaining our own plumbing,
                      which is the family this page deleted a round of.
                      Only where there is no spend beside it — an
                      OpenRouter key reports both, and there the balance
                      is one figure among several. */}
                  {acc.reading.spent == null && (
                    <span data-quota-balance-only className="ml-2 text-muted-foreground">
                      {gui.accounts_balance_only}
                    </span>
                  )}
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
        {/* ⚠️ **The reading's age left this list on 2026-09-07** and is
            on the card's head instead, where the machine card puts the
            same fact (`card-version` · `card-age`). Two cards, one
            place. It is not gone — a figure that says nothing about
            *when* is the thing this page must never draw — and the
            `accounts-read-by` label went with it, because the head
            carries no labels. */}
        </dl>
      </div>
      {/* ⚠️ **Tombstone: the 撤掉 button left this card on 2026-09-07**
          (「卡上不放按钮(撤掉、编辑都不要):右键列表里的条目弹 置顶 /
          编辑 / 删除」). It is the row's menu now, beside 置顶 and 编辑,
          which is where the machine list and the session list already
          put the same three gestures.
          **It is not that a button is worse than a menu.** It is that
          the card is the *reading* and the row is the *thing*: an act
          on the registration belongs on the registration, and a card
          with a verb on it makes a person aim at the reading to change
          the account. The phone half decides it outright — a control
          that only appears beside a card is one a narrow face has no
          room for. */}
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
      <EditDialog
        row={editing}
        onClose={() => setEditing(null)}
        onSaved={() => {
          setEditing(null);
          fetchQuotaBoard().then(setBoard).catch(() => {});
        }}
      />
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

/** `RowMenu` where there is something to offer, and the row untouched
    where there is not. **A menu with no items is a menu that opens onto
    nothing** — worse than no menu, because the gesture answers and the
    answer is blank. */
function RowMenuMaybe({ items, children }: { items: RowAction[]; children: React.ReactNode }) {
  return items.length === 0 ? <>{children}</> : <RowMenu items={items}>{children}</RowMenu>;
}

/**
 * A card's first line: what this is, and **when it was read**.
 *
 * # Why the age is back, and why the tombstone that removed it stands
 *
 * `zh.toml` carries a tombstone from 2026-08-28: the line naming whose
 * login the reading came through was taken off, and 「同一刀去掉的还有
 * 读数年龄」. **That judgment was true of the thing it was about** — a
 * small grey sentence explaining khor's own plumbing, under a number.
 *
 * What changed is not the judgment, it is the **shape around it**: on
 * 2026-09-06 the machine card grew a head whose right end says
 * 「Khor 0.1.14 · 12 秒前」 (`card-version` / `card-age`,
 * `MachineCard`). Once one card in this app says how old its reading is
 * in that seat, a second card that does not is not restraint — it is
 * two cards disagreeing about whether age is a fact worth having. So
 * the age comes back **in that seat**, and the old tombstone is
 * superseded rather than declared wrong: it recorded a real removal,
 * and this records why the removal stopped being right.
 *
 * The version half is deliberately not copied. 「Khor 0.1.14」 is a fact
 * about a machine running khor; behind this card there is a *vendor*,
 * and khor's version says nothing about the reading. **What the two
 * cards share is the seat and the age, not the pair of keys.**
 *
 * The words are the ones the figures list was already using
 * (`accounts-read-here` / `accounts-read-from`), moved, not rewritten.
 */
function CardHead({ row }: { row: Row }) {
  // Who took it: the machine holding an account's key, or the machine
  // whose login the subscription was read through.
  const reader = row.account ? row.account.home_name : fromWord(row.from);
  return (
    <div data-quota-head className="flex min-w-0 flex-wrap items-baseline gap-x-3 gap-y-1">
      {/* **Name and plan, and nothing else** (2026-09-06: 「卡头不再写
          类型 · 厂商 · 来源 · 密钥在哪台」). Every one of those four was
          something the person had just chosen on the way in — the list
          row they pressed, the account they registered — so the card
          was spending its first line telling them what they already
          knew. The plan is the exception because nobody chose it: it
          comes off the login. */}
      <span className="min-w-0 truncate text-title">{row.name}</span>
      {row.plan && (
        <span
          data-quota-plan
          // **Bordered, not tinted.** The mock fills this chip with
          // `--accent`, and `--accent` is also what paints the selected
          // row — so a tinted chip would sit on its own colour.
          // `text-xs`, and that is the *only* spelling of this size.
          // The row's copy of this chip wrote the pixel count out as an
          // arbitrary value instead; measured 2026-09-07 on the live
          // page, both computed the same font-size to the pixel — so it
          // was never a second size, only a second name for one, which
          // is what the px and type gates refuse by name.
          className="flex-none rounded-full border px-2 py-0.5 text-xs text-muted-foreground"
        >
          {row.plan}
        </span>
      )}
      {/* Pushed to the far end by the growing gap, not by a second
          wrapper: one row, two ends. */}
      <span
        data-quota-age={row.age_ms ?? 0}
        className="ml-auto flex-none text-xs text-muted-foreground"
      >
        {row.age_ms === null
          ? gui.accounts_read_here
          : gui.accounts_read_from(reader, lasting(row.age_ms))}
      </span>
    </div>
  );
}

/**
 * 编辑 for a registered account.
 *
 * **It offers what can be changed and nothing else.** The provider is
 * absent because changing it is not editing this account — it decides
 * which endpoint answers and in what shape, so a changed provider is a
 * different account wearing an old id (`Node::update_account` says the
 * same from the other side). The home machine is absent for the reason
 * the whole design exists: the key lives on one disk and does not
 * travel.
 *
 * **The key box is empty and staying empty is an answer.** It does not
 * show the stored key — there is nothing to show, this machine may not
 * even have it — and leaving it blank means 「leave it alone」.
 *
 * **The line under the box says that outright** (`accounts-key-keep`),
 * because the other reading of an empty box is 「clear the key」, and
 * that is the expensive one: it would be read as having worked, by
 * somebody who has just pasted a secret and has no reason to doubt it.
 * It also says where a key *can* be replaced, since the node refuses it
 * anywhere but the home machine.
 */
function EditDialog({
  row,
  onClose,
  onSaved,
}: {
  row: Row | null;
  onClose: () => void;
  onSaved: () => void;
}) {
  const acc = row?.account ?? null;
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [key, setKey] = useState("");
  const [budget, setBudget] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // **Refilled from the row each time it opens**, so a dialog closed on
  // a mistake does not reopen holding it.
  useEffect(() => {
    if (!acc) return;
    setName(acc.reading.name);
    setBaseUrl(acc.base_url ?? "");
    setKey("");
    setBudget(acc.budget == null ? "" : String(acc.budget));
    setError(null);
    setBusy(false);
  }, [acc]);

  if (!acc) return null;
  const save = () => {
    if (busy) return;
    setBusy(true);
    setError(null);
    const parsed = budget.trim() === "" ? undefined : Number(budget);
    updateAccount({
      id: acc.reading.account,
      name,
      base_url: acc.reading.provider === "newapi" ? baseUrl : undefined,
      // **Absent, not empty string.** An empty box means「keep the key」
      // and the node reads absence for that; sending "" would be a
      // different sentence.
      key: key.trim() === "" ? undefined : key,
      budget: parsed !== undefined && Number.isFinite(parsed) ? parsed : undefined,
    })
      .then(onSaved)
      .catch((e) => setError(String(e instanceof Error ? e.message : e)))
      .finally(() => setBusy(false));
  };

  return (
    <Dialog open onOpenChange={(next) => !next && onClose()}>
      <DialogContent data-quota-edit={acc.reading.account}>
        <DialogHeader>
          <DialogTitle>{gui.accounts_edit}</DialogTitle>
        </DialogHeader>
        <div className="flex min-w-0 flex-col gap-3">
          <label className="flex min-w-0 flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.accounts_name}</span>
            <Input data-quota-edit-name value={name} onChange={(e) => setName(e.target.value)} />
          </label>
          {acc.reading.provider === "newapi" && (
            <label className="flex min-w-0 flex-col gap-1 text-sm">
              <span className="text-muted-foreground">{gui.accounts_address}</span>
              <Input
                data-quota-edit-address
                value={baseUrl}
                placeholder="https://"
                onChange={(e) => setBaseUrl(e.target.value)}
              />
            </label>
          )}
          <label className="flex min-w-0 flex-col gap-1 text-sm">
            <span className="text-muted-foreground">{gui.accounts_key}</span>
            <Input
              data-quota-edit-key
              type="password"
              autoComplete="off"
              value={key}
              onChange={(e) => setKey(e.target.value)}
            />
            <span data-quota-edit-key-note className="text-xs text-muted-foreground">
              {gui.accounts_key_keep}
            </span>
          </label>
          {BUDGETED.includes(acc.reading.provider) && (
            <label className="flex min-w-0 flex-col gap-1 text-sm">
              <span className="text-muted-foreground">{gui.accounts_budget}</span>
              <Input
                data-quota-edit-budget
                inputMode="decimal"
                value={budget}
                placeholder={gui.accounts_budget_hint}
                onChange={(e) => setBudget(e.target.value)}
              />
            </label>
          )}
          {error && (
            <p data-quota-edit-error className="text-sm text-destructive">
              {error}
            </p>
          )}
        </div>
        <DialogFooter>
          <Button data-quota-save disabled={busy} onClick={save}>
            {gui.accounts_edit}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
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
