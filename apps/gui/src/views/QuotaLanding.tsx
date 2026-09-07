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
// arc is **the window that speaks for the whole plan** (`headline`) and
// every window is a pip on the rim, one for one with the lines listed
// beside it; an account has one reading and no parts, so it has no
// pips.
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
import { Bar } from "@/components/Bar";
import { IconBack } from "@/components/icons";
import { type OmniAxis } from "@/components/Omnibox";
import { PaneBar, type FilterOption } from "@/components/PaneBar";
import { RowMenu, type RowAction } from "@/components/RowMenu";
import {
  DIAL_CARD_RATIO,
  DIAL_CARD_SIZE,
  DIAL_CARD_STROKE,
  DIAL_ROW_RATIO,
  DIAL_ROW_SIZE,
  DIAL_ROW_STROKE,
  Face,
  type Pip,
} from "@/components/Face";
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

/**
 * How many characters the middle of a ring may hold.
 *
 * 2026-09-07:「保持在 4 位字符以内吧。比如,百分比最长是 `100%`。」
 *
 * **A count, not a width, and that is what makes it a rule.** The gate
 * that came before this measured the string against the ring's inner
 * diameter — true, but it drifts: change the face, the size or the
 * stroke and yesterday's safe string overflows with nothing to say so.
 * **A character count does not drift.** The width check stays as well,
 * because four *wide* characters can still overflow — the count is the
 * rule and the width is the outcome.
 */
const RING_CHARS = 4;

/**
 * The currency's sign, for the ring where there is no room for a code.
 *
 * ⚠️ **Not a catalog word, and the reason is the precedent this file
 * already set for `·`**: punctuation is not copy. `$` is notation — it
 * is the same mark in every language this app might speak, and it is
 * not khor's to translate.
 *
 * **What khor may not do is invent one.** `currency` is the endpoint's
 * own answer, carried and not authored (`AccountReading::currency`), so
 * a code with no sign khor knows gets **no sign** rather than a guessed
 * one — a figure with the wrong mark on it is worse than a figure with
 * none. Two codes reach this today: `USD` from every gateway and
 * vendor, `CNY` from Moonshot's China site (`khor_node::accounts`).
 */
function sign(currency: string | null): string {
  switch (currency) {
    case "USD":
      return "$";
    case "CNY":
      return "¥";
    default:
      return "";
  }
}

/**
 * A reading as it goes **inside a ring**: at most [`RING_CHARS`].
 *
 * - a share is already short — `100%` is the longest one there is;
 * - money is rounded to whole units and wears its sign: `124$`;
 * - and past what fits, `1.2k` / `1.2m`.
 *
 * ⚠️ **The rounding is not new here.** `money` has always dropped the
 * decimals above 100 (`123.75` has been drawing as `124` since before
 * this rule), so what this adds is the sign and the k/m step — and the
 * only band it changes is 10–99.99, which used to draw `12.34` and now
 * draws `12$`.
 */
function ringWord(n: number, currency: string | null): string {
  const mark = sign(currency);
  const whole = Math.round(n);
  if (`${whole}${mark}`.length <= RING_CHARS) return `${whole}${mark}`;
  // Past four characters the sign is what goes, because the magnitude
  // is the part a person is reading — see this batch's report: `1.2k$`
  // is five, and the user's own example (`1.1k`) carries no sign.
  const step = (v: number, suffix: string) => `${Math.round(v)}${suffix}${mark}`;
  if (whole < 1_000_000) return step(n / 1_000, "k");
  return step(n / 1_000_000, "m");
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

/**
 * The window that speaks for the subscription **as a whole** — the arc
 * the dial draws, the number in its middle, and the summary on the row.
 *
 * ⚠️ **Tombstone: it used to be 「the fullest one」** (`worst`, then
 * `fullest`), and 2026-09-07 the user overruled it:「list 页、打开后的多
 * 环,claude 显示 week_all。现在,是 fable。」
 *
 * **That rule was answering a different question and answering it
 * well.** It exists because `is_active` cannot be trusted — the
 * endpoint had it `false` on the 29% session window and `true` on the
 * weekly, so 「the one that is governing you」 has no honest source
 * (`QuotaWindow::active` carries that finding). 「The fullest」 avoids
 * that bad signal, and **avoiding a bad signal is not the same as
 * choosing the right window.**
 *
 * **Why the whole-plan week is the right one.** `7 天 · Fable` is a
 * window *scoped to one model*: how full it is says how much of that
 * model's slice is gone. 「How much of my week have I used」 is what the
 * unscoped week answers. When a line has room for exactly one number,
 * that number has to be about **this subscription**, not about one
 * slice of it — and the slices are not lost: each is a dial of its own
 * in the drawer, which is where they belong.
 *
 * # The rule, in three steps, and each keeps the same property
 *
 * 1. The **longest unscoped** window — today's readings make that the
 *    week.
 * 2. No week, but a five-hour: that one. **Still the whole plan**, just
 *    a shorter period — and the row prints the window's own word, so a
 *    reader sees 「5 小时窗口」 and knows what they are looking at.
 * 3. **No unscoped window at all: nothing.** There is no honest number
 *    for the whole plan, so none is drawn — see the dial's own note for
 *    what it does instead of inventing one.
 *
 * **Length comes from the node's order, not from a table here.**
 * `Quota::windows` arrives sorted shortest-first, whole plan before any
 * one model's share (`khor_node::quota`'s `seat`), so the last unscoped
 * entry *is* the longest one. A second opinion about which span is
 * longer is how two places come to disagree about one order.
 */
function headline(r: Row, dials: Dial[]): Dial | null {
  const whole = r.windows.reduce<QuotaWindow | null>((m, w) => (w.scope === null ? w : m), null);
  if (whole === null) return null;
  const id = windowId(whole);
  return dials.find((d) => d.id === id) ?? null;
}

/**
 * What sits in the dial's centre: the share when there is one, the
 * spend when there is a number but no denominator, what is left when
 * the provider reports a wallet rather than a spend, nothing else.
 *
 * **Everything here is at most [`RING_CHARS`]** — see `ringWord`. It is
 * the same function on the card and on the row: one seat, one rule, and
 * the card's larger ring does not get a longer string just because it
 * would fit. Two spellings of one figure on one screen is what this
 * page keeps coming back to.
 */
function centre(r: AccountRing): string {
  const { reading } = r;
  if (reading.trouble) return "";
  if (reading.fill != null) return `${Math.round(reading.fill * 100)}%`;
  if (reading.spent != null) return ringWord(reading.spent, reading.currency);
  if (reading.remaining != null) return ringWord(reading.remaining, reading.currency);
  return "";
}

/* ⚠️ **Tombstone: `share()` is gone, 2026-09-07** — it drew the card's
   dial with a percentage or **nothing**, and the nothing was wrong.

   User:「说过的没有额度的环,显示金额,你也没有做到」. A reading with no
   denominator (不限量, a prepaid wallet) now shows its **money** in the
   middle, the same thing the list row has always shown — the card and
   the row say one thing, not two.

   ⚠️ **What went here before was a note explaining why the mock was not
   followed, and that note was wrong.** It argued that the mock printed
   the money twice — under the dial and again in the facts — so the
   dial's copy was a draft's blemish. **The mock was right.** The two
   figures are not one sentence said twice: the middle of the dial is
   「what this account has spent」, and the list beside it is
   「已用 / 上限 / 重置」. One is a reading, the other is its parts.

   **This is written down rather than quietly deleted** because the old
   note told the next person that a missing line in the mock was
   deliberate. Somebody comparing the mock to this file has now been
   misled once; they should not be misled twice.

   The subscription's dial is **not** this case and keeps its empty
   middle: with no whole-plan window there is no number to put there —
   not a money figure either, because a subscription has no 「spent」.
   That one is bare track and nothing, ruled separately (`headline`). */


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
 * The line under a row's name — **类型 · 提供商**, the same pair on
 * every row.
 *
 * # Why the name cannot carry any of this
 *
 * 2026-09-07:「标题是用户可以自己起的。我们要有个统一的类型模型,订阅/
 * 计费、厂商。写在副标题。」 **The title is free text somebody typed**,
 * so it classifies nothing — and a list still has to let a reader see,
 * at a glance, what each row *is*. There is exactly one place left for
 * that, and this is it.
 *
 * **「统一的类型模型」 is the judgment, not a description.** Every row is
 * described by the same pair — kind, then provider — whichever kind of
 * thing it is. The version this replaces was `r.plan ?? provider`: a
 * subscription wore its plan and an account wore its provider, **two
 * kinds of row speaking two different languages**, which is precisely
 * what a 「unified model」 rules out.
 *
 * # ⚠️ This line has been three segments, then one, and is now two
 *
 * Each change had a reason and none of them was style:
 *
 * - **Three** (类型 · 提供商 · 来源) was the first shape. At this
 *   column's 288 the third was drawn as 「khor 登…」 — an axis you can
 *   filter by and cannot read.
 * - **One** came when 来源 moved to the group heading (the page opens
 *   grouped by it) and the plan moved to a chip beside the name. The
 *   row then said the plan, or the provider.
 * - **Two, now**, because that one word was not a *model*: it said a
 *   different kind of fact depending on the row. 来源 stays on the
 *   heading; the plan goes back to the card's head, where it has room.
 *
 * The plan is off the row entirely — it is a product name for one
 * vendor's one product, and a row's job here is to say what *kind* of
 * thing this is.
 */
function subLine(r: Row): string {
  return [kindWord(r.kind), providerWord(r.provider)].join(" · ");
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
        const lead = headline(r, dials);
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
                whole-plan window (`headline`); every window is a pip at
                its own angle,
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
            >
              {/* **The reading moved inside the ring** (2026-09-07:
                  「最左环,旁边上标题,下类型,右边是更新时间」) — the
                  right end of the row is the age now, so this is the
                  only seat left for it. */}
              {r.account ? centre(r.account) : lead ? `${Math.round(lead.pct)}%` : ""}
            </Face>
            <span className="flex min-w-0 flex-1 flex-col">
              {/* **The title, and it is somebody's own word.** The
                  user names these (「标题是用户可以自己起的」), so it
                  classifies nothing and the line under it carries the
                  whole of what this row *is*. */}
              <span data-quota-name className="truncate text-fact">{r.name}</span>
              {/* **类型 · 提供商, the same pair on every row** — see
                  `subLine` for why a unified model is the point rather
                  than a tidier line. */}
              <span data-quota-sub-line className="truncate text-xs text-muted-foreground">
                {subLine(r)}
              </span>
            </span>
            {/* **The right end is when this was read**, not what it
                says — the reading is in the ring now. An age here and a
                figure in the ring are two different questions, and the
                row answers each once.
                **Just the age**, not 「谁读的,多久以前」: that sentence
                is the *card head's* seat, and a row that repeated it
                would be saying one fact twice on one screen — measured
                too, at 288 it is 98px and cuts the row's own name off.
                `card-age` is the machine card's word for exactly this,
                reused rather than re-spelled (the catalog refuses one
                text under two names, which is the right refusal here). */}
            <span data-quota-age-cell className="flex-none text-xs tabular-nums text-muted-foreground">
              {r.age_ms === null ? gui.accounts_just_now : gui.card_age(lasting(r.age_ms))}
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
  const lead = current && current.account === null ? headline(current, dials) : null;
  const subDetail = current && current.account === null && (
    <div data-quota-detail={current.id} className="flex flex-col gap-4 p-4">
      <CardHead row={current} />
      {/* **The dial, and what it is made of beside it.** Every window
          used to be a ring of its own, side by side — which drew the
          same quantity at four radii and left 「哪个最先卡住我」 to be
          worked out by comparing four circles. One dial answers that
          (the arc and the number are the whole-plan window's —
          `headline`), and the rim carries a mark per window at its own
          angle: **as many pips as there are lines in the list beside
          it, one for one**. Two windows at the same percent are two
          marks on one spot, which is a true drawing of two windows that
          are equally full.
          Credits is in this list when it has a ceiling to fill against
          — then it is a pip like any other; when it has none it is a
          figure below and no mark, because a mark needs an angle and an
          angle needs a denominator (`creditsPct`). */}
      <div data-quota-lead className="flex flex-wrap items-center gap-x-8 gap-y-4">
        {/* ⚠️ **Tombstone: this dial was a button for about an hour,
            and before that there was a row of words under the list.
            Neither is here now, and the story is worth keeping because
            it is three judgments about three different things — not
            somebody changing their mind.**

            1. The first ruling had two sentences that disagree:「底下
               『展开』」and「和机器卡按表盘开抽屉是同一个动作」. The
               first was built. **Nobody went back to ask**, and that is
               the part to learn from.
            2. Looking at the built thing, the user ruled:「点击环本身
               展开,和机器页一样,而不是现在的一排字!」 — the words went,
               the dial became the handle (`02066e9`).
            3. Looking at *that*, the user ruled:「直接就展开,不用用户
               点击」, with the reason:「左多环右展开 bar 来铺宽度空间」.

            **Each ruling is about what was in front of them**: a mock, a
            row of words, a card with half its width empty. The third one
            makes the first two moot — a drawer that is always open needs
            no handle — so the handle is gone rather than disabled.

            What is left is the dial, drawn and not pressed. */}
        <Face
          size={DIAL_CARD_SIZE}
          stroke={DIAL_CARD_STROKE}
          ratio={DIAL_CARD_RATIO}
          fraction={lead ? lead.pct / 100 : null}
          pips={dials.map((d): Pip => ({ word: d.word, v: d.pct }))}
        >
          {lead ? `${Math.round(lead.pct)}%` : ""}
        </Face>
        {/* **The right column, and everything in it is a fact about
            this subscription** — the windows, then the credits. They
            are one column rather than two blocks stacked under the card
            because that is the half the dial does not fill
            (2026-09-07:「表盘 | 事实」).
            ⚠️ **`flex-1` is load-bearing, and its absence is invisible.**
            Without it this column is sized to its content, the bars'
            `1fr` track resolves to **zero**, and every bar measures 0px
            wide — which on a screenshot reads as 「the bars have not
            been added yet」, not as 「the layout is broken」. Measured
            2026-09-07: 0px before, and it was the measurement that
            found it, not the picture. */}
        <div className="flex min-w-0 flex-1 flex-col gap-3">
        {/* **Name, bar, number — and the bar is why this is the shape**
            (2026-09-07:「左多环右展开 bar 来铺宽度空间」,「类似机器卡片
            的环 bar 组合」). The machine card pairs a dial with rows of
            exactly this, for exactly this reason: **a bar is the one
            reading that grows sideways**, so a card with width to spare
            spends it on the readings instead of leaving it blank.
            The reset rides the name in muted ink rather than taking a
            column of its own — the same seat the machine card gives a
            volume's mount point beside its path. A column for it would
            push the bar into a gutter, and the bar is the point. */}
        <dl
          data-quota-parts
          className="grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-4 gap-y-1.5 text-sm"
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
              {/* ⚠️ **`data-quota-window-name` is on the word, not on
                  this cell.** The cell holds the reset line too, and a
                  measure that reads the cell gets 「7 天窗口23 小时后重
                  置」 — the same 「39%7」 the older comment on this block
                  warns about, re-made the moment the reset moved in
                  here. Caught by that very gate, 2026-09-07. */}
              <dt className="flex min-w-0 items-baseline gap-2 text-fact">
                <span data-quota-window-name className="truncate">{d.word}</span>
                {d.resets_at !== null && (
                  <span className="flex-none text-aux text-muted-foreground">
                    {gui.quota_resets(until(d.resets_at * 1000))}
                  </span>
                )}
              </dt>
              {/* **The machine card's bar, the same component**
                  (`components/Bar`) — 「类似机器卡片的环 bar 组合」 read
                  literally. **No `strain`**: a quota at 80% is a fact,
                  not a warning (that component's head carries why, so
                  the next reader does not take the missing argument for
                  an oversight). */}
              <Bar pct={d.pct} />
              <dd data-quota-window-pct className="text-right font-title tabular-nums">
                {Math.round(d.pct)}%
              </dd>
            </div>
          ))}
        </dl>
        {/* ⚠️ **Tombstone: the row of words under this list is gone,
            2026-09-07** —「点击环本身展开,和机器页一样,而不是现在的一排
            字!」. It was a `<button>` saying 展开每个窗口 / 收起.
            **Its two words are not gone with it**: they are the dial's
            `aria-label` now, saying the same thing to a reader who
            cannot see the arrow. What was removed is a *second
            vocabulary for one gesture* — the machine card already had
            this gesture and it is the dial there, so a card that
            answered it with a line of text was teaching the same act
            twice.
            **How this came to be built the other way is worth keeping**:
            the ruling behind it was two sentences —「底下『展开』」and
            「和机器卡按表盘开抽屉是同一个动作」— and they disagree. The
            first was implemented. That was not a mistake in the doing;
            it was a judgment with two readings that nobody went back to
            ask about. */}
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
      {/* ⚠️ **Tombstone: the row of per-window dials that used to hang
          under this card is gone, 2026-09-07** —「左多环右展开 bar 来铺
          宽度空间」. It drew each window as its own 88px dial, in a strip
          below; the same windows are the bars on the right now.

          **The reason is width, and it is a real one.** Those dials sat
          under a card whose right half was empty, so the card was tall
          and narrow while the room went unused. Bars grow sideways —
          that is what makes them the shape for「fill the width」, and it
          is the same pairing the machine card makes (a dial, and rows of
          `name | bar | number` beside it).

          **Nothing was lost with them**: every window is still a line
          over there, with its own name, its own reset and its own
          number, and still a pip on the rim of the dial to the left. */}
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
          size={DIAL_CARD_SIZE}
          stroke={DIAL_CARD_STROKE}
          ratio={DIAL_CARD_RATIO}
          fraction={arcOf(acc.reading)}
          strain={acc.strain}
        >
          {centre(acc)}
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
            {/* **到期, and only where there is one.** A token that never
                expires carries `null` here, not a zero — the two
                sentinels the endpoint uses (`0` on the wire, `-1` in its
                own table) are turned away at the parser, because either
                of them drawn straight is 1970-01-01: a date that looks
                entirely real. */}
            {acc.reading.expires_at != null && (
              <>
                <dt className="text-muted-foreground">{gui.accounts_expires}</dt>
                <dd data-quota-expires>{until(acc.reading.expires_at * 1000)}</dd>
              </>
            )}
            {/* **模型, and the list is already the answer.** Whether the
                limit is switched on was decided at the parser — the
                endpoint keeps the old names after it is switched off,
                and 「limited to these」 versus 「not limited」 are
                opposite sentences made of the same real model names. An
                empty list is 「no limit in force」 and draws nothing. */}
            {acc.reading.models.length > 0 && (
              <>
                <dt className="text-muted-foreground">{gui.accounts_models}</dt>
                <dd data-quota-models className="flex flex-wrap gap-1">
                  {acc.reading.models.map((m) => (
                    <span key={m} className="rounded bg-secondary px-1.5 font-mono text-xs">
                      {m}
                    </span>
                  ))}
                </dd>
              </>
            )}
            {/* **Two periods, each under its own word.** Which of these
                reached the screen used to depend on whether the key had
                a cap, so 「已用」 meant this month on one account and all
                time on another with nothing saying which. Drawn only
                beside each other — one of them alone is the ambiguity
                again. */}
            {acc.reading.spent_this_month != null && acc.reading.spent_all_time != null && (
              <>
                <dt className="text-muted-foreground">{gui.accounts_this_month}</dt>
                <dd data-quota-month className="tabular-nums">
                  {money(acc.reading.spent_this_month, acc.reading.currency)}
                </dd>
                <dt className="text-muted-foreground">{gui.accounts_all_time}</dt>
                <dd data-quota-all-time className="tabular-nums">
                  {money(acc.reading.spent_all_time, acc.reading.currency)}
                </dd>
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
      <SpendDays reading={acc.reading} />
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

/**
 * The card's second section: what this account spent, by day.
 *
 * # The title is the reading, not the request
 *
 * ⚠️ **It says how many days this chart *covers*, never how many were
 * asked for.** new-api's log road answers its most recent 1000 records
 * with no time window and no paging, so a busy key spends the whole
 * ceiling inside a day — and a chart labelled 「7 天」 holding half of
 * one does not error and does not look empty: **it draws a true picture
 * of the wrong stretch.** The two vendors have no such ceiling and their
 * title is just as honest, because it is the same sentence either way:
 * this is what is here.
 *
 * The cap line is the second half and only appears when the provider
 * actually cut the log (`log_capped`). **A disclaimer that is always on
 * screen is read as decoration**, and it would be false on almost every
 * account — so the title says what the chart is, and this says why it
 * is that, and both are true.
 *
 * # The bars
 *
 * One column per day the provider reported, in the order it reported
 * them, each as tall as its share of the fullest day. **Heights are
 * relative to the tallest bar, not to any budget**: this section
 * answers 「when did it go」, and the dial above already answers 「how
 * much of what I have」.
 */
function SpendDays({ reading }: { reading: AccountRing["reading"] }) {
  const days = reading.days;
  if (days.length === 0) return null;
  const most = Math.max(...days.map((d) => d.spent));
  return (
    <div data-quota-days className="flex flex-col gap-2 border-t pt-4">
      <div className="flex flex-wrap items-baseline gap-x-3 text-xs text-muted-foreground">
        <span data-quota-days-title className="text-fact">
          {gui.accounts_trend_days(days.length)}
        </span>
        {reading.log_capped != null && (
          <span data-quota-days-capped>{gui.accounts_log_capped(reading.log_capped)}</span>
        )}
      </div>
      {/* **The chart grows with the data, it does not stretch to fill.**
          Three days at `flex-1` across a card is three slabs — measured
          2026-09-07: 350px each, which reads as a diagram of something
          else. A bar has a width it stops at, and a short window simply
          takes less room. */}
      <div className="flex h-16 max-w-sm items-end gap-1">
        {days.map((d) => (
          <div
            key={d.at}
            data-quota-day={d.at}
            data-quota-day-spent={d.spent.toFixed(4)}
            title={`${money(d.spent, reading.currency)}`}
            className="min-w-1 max-w-10 flex-1 rounded-t bg-muted-foreground/60"
            // **A day with no spending still gets a column, one pixel
            // tall** — a gap would read as a day the provider did not
            // report, and those are two different facts.
            style={{ height: `${most > 0 ? Math.max(1, (d.spent / most) * 100) : 1}%` }}
          />
        ))}
      </div>
    </div>
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
 *
 * **And on a machine that does not hold the key, the box is shut.**
 * The node refuses that edit by name (`account-key-elsewhere`), which
 * is the correct answer and a late one: a person has already chosen a
 * key manager, opened it, copied a secret and pasted it before being
 * told the paste could not land. Whether this machine holds it is
 * `AccountRing.mine`, already on this row for the reading's own sake —
 * **the fact was free**, so refusing beforehand costs nothing and the
 * refusal afterwards stays as the backstop for a race.
 *
 * **The box is never filled in, on any machine.** It does not echo the
 * stored key and it does not say whether one exists. Two reasons, and
 * **the one that cannot be argued with goes first**:
 *
 * 1. **It cannot be done.** From any machine but the account's home the
 *    secret is not on this disk. There is nothing to echo, and a field
 *    showing 「••••」 there would be drawing a thing that is not here —
 *    a mask over nothing reads exactly like a mask over a key.
 * 2. **It should not be done.** A key echoed to a screen has left the
 *    machine it was promised to: a screenshot, a shared window,
 *    somebody standing behind you.
 *
 * The order is deliberate. **「should not」 is what the next person
 * overturns** when they arrive with a reason that feels good enough
 * (「just show the last four」, 「just a ✓ so they know one is set」);
 * 「cannot」 is still true when they get here. The existence tick is
 * refused by the same pair — it is a question about a secret answered
 * by a screen, and from a peer it would be answering about a file this
 * machine cannot see.
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
              // **Shut where the key is not**, so a secret is not asked
              // for and then refused. `mine` is the node's own answer to
              // 「is this reading mine to take」, and holding the key is
              // exactly what that means (`quota_board`).
              disabled={!acc.mine}
              data-quota-edit-key-shut={!acc.mine}
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
