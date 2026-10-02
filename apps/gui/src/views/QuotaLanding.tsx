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
// arc is **the window that speaks for the whole plan** (the node's
// `whole_plan_fill`) and
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
import { useEffect, useMemo, useState } from "react";

import {
  fetchQuotaBoard,
  fetchUsage,
  pinAccount,
  registerAccount,
  removeAccount,
  updateAccount,
  type AccountRing,
  type QuotaCredits,
  type QuotaWindow,
  type Usage,
} from "@/api";
import { EmptyState, RegionState } from "@/components/RegionState";
import { Bar } from "@/components/Bar";
import { IconBack } from "@/components/icons";
import { GlassCircle } from "@/components/Glass";
import { ListPane } from "@/components/ListPane";
import { type OmniAxis } from "@/components/Omnibox";
import { PaneBar, type FilterOption } from "@/components/PaneBar";
import { PageAvatar, PageSubtitle, Region } from "@/components/Region";
import { FlushInk } from "@/hooks/use-flush-ink";
import { NarrowHead } from "@/components/SideHead";
import { useKept } from "@/lib/kept";
import { type ListAxis, listAxes, listOptions, sieve } from "@/lib/list-filter";
import { type RowAction } from "@/components/RowMenu";
import { sessionMark } from "@/components/BrandIcons";
import { EditableName } from "@/components/EditableName";
import { SideList, type SideRow } from "@/components/SideList";
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
import { PopupButton } from "@/components/ui/popup-button";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { FormGrid, FormLabel } from "@/components/ui/form-grid";
import { gui } from "@/gen/catalog";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";
import type { PaneWidth } from "@/hooks/use-list-width";
import { cn } from "@/lib/utils";
import { UsageHalf, fromReading, fromUsage } from "@/views/UsageHalf";

import { QUOTA_AXES, groupLabel, lasting, until } from "@/words";

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
const PROVIDERS = ["newapi", "claude", "openai", "deepseek", "moonshot", "moonshot-cn", "openrouter", "codex"] as const;
type Provider = (typeof PROVIDERS)[number];

/** Which providers a registered budget means anything for: those that
    report a spend (the vendors' cost reports, OpenRouter's monthly
    usage). A prepaid wallet reports what is left, and a budget over a
    balance would be a fraction of two different things. */
const BUDGETED: readonly string[] = ["claude", "openai", "openrouter"];
/** Both vendors' usage reports are organisation-level admin calls. */
const NEEDS_ADMIN: readonly string[] = ["claude", "openai"];
/** Whose subscription khor can read — the vendors whose own login it
    knows how to find (`khor_node::quota`, one description per vendor).
    The node refuses anything else by name, so this list only decides
    what the form *offers*. */
const SUBSCRIBED: readonly string[] = ["claude", "codex"];
/** A plan read through a login and nothing else: offered only as a
    subscription (the node refuses a billed one by name). */
const SUB_ONLY: readonly string[] = ["codex"];

/** The tool whose login a subscription reads, by provider word, and the
    command that logs it in again — the two blanks in the sentences that
    send a person to fix a login (khor never renews one). */
function vendorOf(p: string): { name: string; command: string } {
  switch (p) {
    case "codex":
      return { name: gui.accounts_provider_codex, command: "codex" };
    default:
      return { name: gui.vendor_claude, command: "claude" };
  }
}

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
    case "codex":
      return gui.accounts_provider_codex;
    default:
      return p;
  }
}

/** The trouble key's sentence (`khor_node::accounts::trouble`). A key
    this face has never heard of is shown as itself — a reason is still
    a reason, and hiding it would paint 问不出来 with no why. */
function troubleWord(key: string, provider: string): string {
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
    // A subscription's two, and they are two because the remedies are:
    // log in over there, or run the vendor's command once over there.
    case "no-login":
      return gui.accounts_trouble_no_login(vendorOf(provider).name);
    case "login-stale":
      return gui.accounts_trouble_login_stale(vendorOf(provider).name, vendorOf(provider).command);
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

/* ⚠️ **Tombstone: `sign()` is gone, 2026-09-07 — a table from currency
   codes to marks (`USD→$`, `CNY|RMB→¥`), one day old when it was
   pulled.**

   **It is not that the table was incomplete. It is that no table can be
   complete, and that its input was never trustworthy.** The user's
   words on both halves:「不要去识别,不然各种货币符号我们识别不完,
   用户也可能说的不对」——

   - **there is no closed set to recognise.** Every entry khor adds is
     a promise about a world that keeps adding currencies, and the
     branch that catches the rest cannot say anything true. Yesterday's
     shape answered an unknown code with no mark at all, which is
     honest and also useless to the person reading it;
   - **and the code being translated is itself a guess.** The gateways
     report a code the account may not be billed in — that is why
     [`AccountRing.unit`] exists at all. A perfect table over a wrong
     input produces a confident wrong mark.

   So khor stopped reading codes. **The mark is the characters the
   person typed**, carried to the screen untranslated, and where they
   have typed nothing it is [`DEFAULT_MARK`]. Nothing in this file maps
   one string to another one, and nothing should be added that does. */

/**
 * What a figure wears when nobody has said otherwise.
 *
 * 2026-09-07:「默认是 $」.
 *
 * ⚠️ **It has a cost, and the cost was accepted knowingly**: an account
 * really billed in renminbi — Moonshot's China site is the one khor
 * has met — reads `$` until somebody opens 编辑 and types `¥`. That is
 * a change of judge, not an oversight. Yesterday khor decided from the
 * endpoint's code and drew nothing when it could not tell; today the
 * person decides, and **a default they can see and change is worth
 * more than a guess khor makes on their behalf** — the guess is the
 * thing they cannot argue with.
 *
 * ⚠️ **Applied when drawing, never when storing.** An account with no
 * override stores `null`, not `"$"`. Writing it down would turn a
 * default into a decision nobody made — the same reason the edit box
 * shows this as a placeholder rather than a value, and the same reason
 * the node still carries the endpoint's own answer untouched
 * (`AccountReading::currency`).
 */
const DEFAULT_MARK = "$";

/** The marks 编辑 offers as one press. **A shortcut row, not a set of
    allowed values** — see where it is drawn. `$` leads because it is
    [`DEFAULT_MARK`]; the other three are the currencies an account
    reachable from here is plausibly billed in, and the length is
    bounded by the one line they have to sit on. */
const UNIT_PICKS = ["$", "¥", "€", "£"];

/** The mark an account's figures wear: what its owner typed, else
    [`DEFAULT_MARK`]. **One source for the ring and the column** — they
    used to reach for `sign(currency)` and `currency`, which is how one
    of them could say `124$` while the other said `124 USDT`. */
function markOf(unit: string | null): string {
  // `??` alone would let an empty string through as「no mark at all」.
  // The node already stores blank as absent, so this only ever fires on
  // a wire that broke that promise — and the one thing the default
  // exists to prevent is a figure standing there with no unit.
  return unit === null || unit.trim() === "" ? DEFAULT_MARK : unit;
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
function ringWord(n: number, mark: string): string {
  const whole = Math.round(n);
  const fits = (s: string) => s.length <= RING_CHARS;
  if (fits(`${whole}${mark}`)) return `${whole}${mark}`;
  // Not enough room: buy some by shortening the **magnitude**, which is
  // what `1.1k` was asked for.
  const short = whole < 1_000_000 ? `${Math.round(n / 1_000)}k` : `${Math.round(n / 1_000_000)}m`;
  if (fits(`${short}${mark}`)) return `${short}${mark}`;
  // ⚠️ **And when even that does not fit, the mark is what goes — not
  // the digits.** Since 2026-09-07 the mark is free text (「直接输入
  // 符号」), so it can be longer than the whole budget, and a step down
  // in magnitude cannot pay for it: `124` under a `USDT` would become
  // `0kUSDT`, then `0k`, and **a reading of 124 would be drawn as
  // zero**. The magnitude is the part a person is reading, so it keeps
  // its own full form and the column beside it spells the unit out.
  return fits(`${whole}`) ? `${whole}` : short;
}

/**
 * The figure a column shows, with its unit spelled out in full.
 *
 * **`unit` is whatever string its caller holds, and this function does
 * not interpret it**: for an account that is the mark its owner typed
 * (or [`DEFAULT_MARK`]); for the subscription's credits it is the code
 * the endpoint reported. Nothing here maps one to the other — see the
 * tombstone above `DEFAULT_MARK` for why khor stopped doing that.
 *
 * ⚠️ **Why credits are not defaulted to `$` like an account is.** The
 * default exists because a person can open 编辑 and correct it. The
 * subscription has no such box, so a `$` there would be a mark nobody
 * could argue with — the exact thing the default was chosen over. Its
 * figures wear the endpoint's own code, or nothing.
 *
 * # The space, and why it is conditional
 *
 * 2026-09-07:「右栏要读起来像钱」. A mark that is a **symbol** sits
 * against its figure (`124$`, `1235¥`); one that starts with a
 * **letter** is a word and takes a word gap (`124 USDT`). Measured on
 * the page before choosing: `124 $` reads as a typo and `124USDT` as
 * one token.
 *
 * **This is typography, not recognition.** It does not decide *what*
 * the unit is — that is the thing khor stopped doing — only whether
 * the two things beside each other need air between them. A person who
 * types `£` and a person who types `GBP` both get their own string back.
 *
 * ⚠️ **This and [`ringWord`] round differently on purpose**: above 100
 * both drop the decimals, below it a ring says `12$` where this says
 * `12.34$` — the ring has four characters, this line has as many as it
 * needs. What they may **never** differ on is the unit itself.
 *
 * ⚠️ **The unit is a label, never a rate.** An account may carry a
 * `unit` its owner typed, because a gateway can stamp 「USD」 on
 * renminbi (「他说 usd,但可能是 rmb。同样的数字,单位不同而已」) — the
 * node swaps the *name* before the board is sent and touches no digit.
 * **Nothing here may grow into a conversion**: no rates, no sources,
 * no arithmetic. That is a different feature and a different ruling.
 */
function money(n: number, unit: string | null): string {
  const figure = n >= 100 ? n.toFixed(0) : n.toFixed(2);
  if (!unit) return figure;
  return /^\p{L}/u.test(unit) ? `${figure} ${unit}` : `${figure}${unit}`;
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
function kindWord(k: string): string {
  switch (k) {
    case "sub":
      return gui.quota_kind_sub;
    case "bill":
      return gui.quota_kind_bill;
    // A kind from a newer khor, shown as itself — `providerWord`'s
    // default and the same bargain. 计费 over a row that is not billed
    // would be khor claiming; the other khor's own word is visibly not
    // khor's.
    default:
      return k;
  }
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
  kind: string;
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
   * How full this row is, `0..=1`, or `null` for 「no fraction」 — the
   * arc of its dial and the number in the middle.
   *
   * **Always the node's** (`Quota::fill` probed, `AccountReading::fill`
   * registered), and that is the point of the field. A subscription is
   * drawable two ways now — probed and registered — and if each worked
   * its own share out of the windows, 「the two draw the same arc」
   * would be a property somebody had to keep true. One number makes it
   * a property nothing can break.
   *
   * ⚠️ **Tombstone: `headline()` decided this here until 2026-09-07.**
   * The rule is unchanged and it was right; it lives in
   * `khor_node::quota::whole_plan_fill` now, with its reasoning.
   */
  fill: number | null;
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

/* ⚠️ **Tombstone: `headline()` lived here until 2026-09-07.**

   It answered 「which window speaks for the whole plan」 — the longest
   unscoped one — and it answered it correctly. **What changed is not
   the rule, it is how many places could ask.** A subscription used to
   reach this file exactly one way, probed off this machine; it can now
   also arrive as a registered account, whose reading comes back over
   the mesh with a share already in it. Two sources for one picture, and
   「the probed row and the registered row draw the same arc」 becomes a
   property somebody has to keep true — a shape this page has paid for
   before.

   So the rule moved **whole**, its reasoning with it, to
   `khor_node::quota::whole_plan_fill`: why the longest *unscoped*
   window rather than the fullest, why `is_active` is not a trustworthy
   signal, why nothing is drawn when no window covers the plan. Both
   readers carry the node's one number in `Row.fill` now, and the two
   rows cannot disagree because there is nothing left to disagree with.

   `subDials` stayed here on purpose: its pips are the windows
   themselves, one mark per reading, and no choosing happens in it. */

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
  // **`r.unit`, not `reading.currency`.** The reading carries the
  // endpoint's own code, which is not a mark and is not khor's to
  // translate into one; the mark is the person's, defaulted here and
  // nowhere upstream.
  const mark = markOf(r.unit);
  if (reading.spent != null) return ringWord(reading.spent, mark);
  if (reading.remaining != null) return ringWord(reading.remaining, mark);
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
function arcOf(r: Row): number | null {
  if (r.account?.reading.trouble) return null;
  // ⚠️ **A share that is neither a number nor `null` is left to draw
  // `NaN`, and that is a decision.** 2026-09-07 the replica stopped
  // supplying `fill` and this drew `A57.6 57.6 0 0 0 NaN NaN`. A guard
  // here would have turned the gate's message — which carries that
  // `NaN` and so names its own cause — into a generic 「no reading」:
  // **a failure that says what happened, traded for one that does
  // not.**
  //
  // ⚠️ **It is not invisible, which is the opposite of what this
  // comment first said.** The first draft argued 「the browser ignores
  // a NaN path, so the screen is identical either way」. **A screenshot
  // that day showed the subscription's ring drawn as a broken arc with
  // stray strokes, while every account's ring beside it was whole** —
  // and the geometry gate was **green** through it, because NaN
  // segments never enter a bounding box. So the true cost of this
  // choice is not 「a worse message」, it is 「a visibly broken ring」,
  // and it is only ever paid where the reading comes from a replica.
  //
  // ⚠️ **It rests on one thing, and that thing is a rule somebody could
  // break next week: the node always sends `fill`.** `khor_node::proto`
  // bans `skip_serializing_if` and gives tail fields `serde default`, so
  // an `Option` crosses as `null` and never as an absent key —
  // `undefined` is a state only a hand-written replica can reach. **Put
  // `skip_serializing_if` on `Quota::fill` or `AccountReading::fill` and
  // this stops being gate-only evidence and starts drawing NaN in front
  // of people**, with nothing anywhere to say so.
  // **A subscription with no whole-plan window draws bare track, never
  // a full ring.** A full ring says 「nothing caps this」 about an
  // account; about a subscription it would say 「your week is gone」.
  // The two things a full ring can mean are opposite here, which is why
  // the `?? 1` below is deliberately not shared with this branch.
  if (r.kind === "sub") return r.fill;
  return r.fill ?? 1;
}

/** The middle of a row's dial. A registered row says what its reading
    says (`centre`); a probed subscription has no reading object, so its
    share is spelled here — the same rounding, on the same number. */
function centreOf(r: Row): string {
  if (r.account) return centre(r.account);
  return r.fill == null ? "" : `${Math.round(r.fill * 100)}%`;
}

// ⚠️ 墓碑: `SUB_ID` (「sub:local」), the probed subscription row's id,
// retired 2026-09-22 with the row (docs/SHELL.md 第七轮 第 3 条).

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
 * What a row answers on one axis, as the filter reads it — `answersOf`,
 * one axis at a time. Never `null`: every row answers all three.
 */
const quotaPick =
  (axis: string) =>
  (r: Row): string =>
    answersOf(r)[axis] ?? "";
/** The three axes as the sieve reads them: keys and answers, no words. */
const QUOTA_SIEVE = QUOTA_AXES.map((key) => ({ key, pick: quotaPick(key) }));
/** …and with their headings and words, for the menu and the box. A
    function, not a constant: the words are catalog reads (批 D3). The
    tick and the group heading print the same `valueWord`. */
const quotaAxes = (): ListAxis<Row>[] =>
  QUOTA_AXES.map((key) => ({
    key,
    heading: axisWord(key),
    pick: quotaPick(key),
    label: (v) => valueWord(key, v),
  }));

/**
 * Which rows the bar's search and filter leave standing — the same
 * shape `visibleSessions` has, and the same two decisions:
 *
 * **AND across axes, OR within one.** Ticking 订阅 and 计费 means
 * 「either」; ticking 订阅 and a provider means 「the subscription, and
 * from that provider」. That half is `lib/list-filter`'s `sieve`, the
 * one every list with a filter uses (moved there 2026-09-29; this page
 * carried its own copy of the same loop until then).
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
  return sieve(rows, ticked, QUOTA_SIEVE, QUOTA_AXES).filter((r) => {
    if (q === "") return true;
    const on = answersOf(r);
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
  pane,
  query,
  onQuery,
  words,
  onToggleWord,
  screen = "list",
  onOpen,
  onBack,
  stale = false,
}: {
  /** The shell cannot reach khor: what is on the board is the last
      answer, and R0's caption says so (components/RegionState). */
  stale?: boolean;
  narrow: boolean;
  /**
   * The list column's width, **the shell's and not this landing's**
   * (2026-09-20 第 6 条 / 第 4 条).
   *
   * It was `w-72`: a fixed number, no seam, no way to put it away, and
   * — the thing the user actually saw — no ground, so this one column
   * sat on `--background` while every other pane in the app sat on
   * `--card`. 「这看起来没有用统一的逻辑进行编写」. All four of those
   * are `ListPane`'s now, and this prop is the one of them a landing
   * cannot supply for itself: the rail is what puts a list away, so the
   * rail has to be holding the same object.
   */
  pane: PaneWidth | null;
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
}) {
  // **Both outlive a remount.** The narrow face keys its screen box on
  // `screen` (`App`, for the entrance slide), so opening a row mounts
  // this landing afresh: with the board back at null the dead-end guard
  // below saw no rows and sent the detail screen straight back, and the
  // pick was gone besides — pressing any row drew nothing. The probed
  // subscription row hid it, being drawn from `App`'s own state; it went
  // on 2026-09-22 (第七轮 第 3 条) and every row on a phone stopped
  // opening.
  //
  // Kept in `lib/kept` since 第十六轮 (it was a module-level object
  // here), beside the machine card's answers and this page's details.
  const [board, setBoard] = useKept<AccountRing[] | null>("quota.board", null);
  const [picked, setPicked] = useKept<string | null>("quota.picked", null);
  const [groupBy, setGroupBy] = useState<string>(storedGroup);
  // The landing's one act, and its one home: ＋ in L0, pressed straight
  // through (2026-09-30, the owner: 「订阅页右上角不要有登记按钮，这个功能
  // 属于搜索框旁边的 ＋」; ⚠️ 墓碑: it stood at R0's right end as well).
  // It opens a dialog, so it wears the ellipsis (components/Button).
  const quotaActions = [{ key: "register", label: gui.accounts_register_ask, onSelect: () => setRegistering(true) }];
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
  /** This machine's own token spend, or `null` for 「not asked yet」.
      **Fetched when a subscription's detail is opened, never on a
      beat** — `api`'s `fetchUsage` carries why, and the short of it is
      that the first pass over the transcripts is 18.5 s. `null` is the
      state `UsageHalf` draws nothing at all for. */
  // Kept across a remount (`lib/kept`, 第十六轮): coming back to a
  // subscription's detail draws the last tally at once and asks again
  // in place, instead of an empty chart for the length of the walk.
  const [usage, setUsage] = useKept<Usage | null>("quota.usage", null);
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

  // **Every row is a registration** (docs/SHELL.md 第七轮 第 3 条,
  // 2026-09-22:「这些自动读取的一视同仁. 只是自动初始化了, 但一切操作与
  // 别的list无异」). ⚠️ 墓碑: a probed row (`SUB_ID`, 「sub:local」) stood
  // here for this machine's Claude Code login — no menu, because nothing
  // had been registered — and stood down when a registered subscription
  // was homed here. The node now registers that login itself the first
  // time it reads the board (`Node::adopt_subscriptions`), once: a row a
  // person deleted stays deleted, which a probe drawn from the login
  // could never do.
  const rows: Row[] = [
    ...(board ?? []).map((a) => ({
      id: a.reading.account,
      name: a.reading.name,
      kind: a.kind,
      provider: a.reading.provider,
      // **来源 is where the reading came from.** A subscription is read
      // through a login on its home machine, so it names that machine —
      // the same answer the probed row gives, which is what lets one
      // replace the other. A billed account's reading comes from a key
      // somebody filled in, which is 「khor 登记」 and not a machine.
      from: a.kind === "sub" ? a.home_name : "",
      plan: a.reading.plan,
      account: a,
      windows: a.reading.windows,
      credits: a.reading.credits,
      fill: a.reading.fill,
      // The node answers 0 for a reading this machine took itself.
      //
      // ⚠️ **A subscription read here is the exception, because it is
      // replacing a row that already told the truth.** The probed row
      // shows its reading's real age (`Quota::as_of`), and that reading
      // comes out of a machine-wide cache up to five minutes deep — so
      // a registered row claiming 「刚刚」 would be a step backwards on
      // the one row a person can compare against yesterday. Its own
      // `at_ms` is that same age, carried by the node.
      //
      // **A billed account read here has the same cache and is left
      // alone**: nothing on that row has ever said otherwise, and
      // changing it is a ruling about the age axis, not about
      // subscriptions.
      age_ms:
        a.mine && a.kind === "sub" ? Date.now() - a.reading.at_ms : a.mine ? null : a.age_ms,
    })),
  ];
  const shown = visibleQuota(rows, query, words);
  // **Grouped by re-ordering, not by nesting.** The rows stay one flat
  // list and a heading is drawn where the group changes between
  // neighbours — the same mechanism `SessionsList` uses, and for the
  // same reason: a group that no row falls into cannot draw a heading
  // over nothing, with no special case to remember.
  const grouped = useMemo(() => {
    // **Pinned first, under their own heading, in every grouping** — the
    // pinned group every sidebar has (docs/SHELL.md 第四轮 第 1 条); the
    // chosen axis groups the rest.
    const pinned = shown.filter((r) => r.account?.pinned);
    const rest = shown.filter((r) => !r.account?.pinned);
    if (groupBy === GROUP_NONE) return [...pinned, ...rest];
    const order: string[] = [];
    const bins = new Map<string, Row[]>();
    for (const r of rest) {
      const g = answersOf(r)[groupBy] ?? "";
      if (!bins.has(g)) {
        bins.set(g, []);
        order.push(g);
      }
      bins.get(g)!.push(r);
    }
    return [...pinned, ...order.flatMap((g) => bins.get(g)!)];
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
    // Only once the board has answered: before that there is nothing
    // to judge by, and 「no row」 would be a guess.
    if (screen === "detail" && board !== null && !current) onBack?.();
  }, [screen, board, current, onBack]);

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
  // from storage before the first board has arrived). Both halves are
  // `listOptions`', the one gathering every list uses.
  const options = useMemo<FilterOption[]>(
    () => listOptions(rows, words, quotaAxes(), QUOTA_AXES),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [rows, words],
  );

  // The same options read as the search box's axes — derived from the
  // one list above rather than gathered a second time, so a chip and a
  // tick cannot come back with different answers (`listAxes`). No
  // faces: none of the three values is a machine.
  const axes = useMemo<OmniAxis[]>(() => listAxes(options, QUOTA_AXES), [options]);

  /**
   * The row's own actions — **the same gesture the machine and session
   * lists answer** (2026-09-07:「右键列表里的条目弹 置顶 / 编辑 / 删除」).
   *
   * They were a button on the card until now, which is a control a
   * phone has no room for and a card has no business carrying: the card
   * is the reading, the row is the thing. `RowMenu` gives one mechanism
   * for right-click, long-press and the keyboard's Menu key.
   *
   * **Only a registration has these**, which since 2026-09-07 includes
   * a registered subscription: it has a name somebody chose, a place in
   * the order and a row to remove, so it gets the same three. Since
   * 2026-09-22 every row is a registration — the probed row that had no
   * menu is retired (docs/SHELL.md 第七轮 第 3 条) — so every row has
   * all three.
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
      { key: "edit", label: gui.edit_ask, run: () => setEditing(r) },
      // 删除 last, behind a line — every sidebar's menu opens on 置顶 and
      // closes on 删除 (2026-09-28 audit 第 16 条, Apple HIG). It was the
      // second item from docs/SHELL.md 第二轮 until then, and 撤掉 and
      // third before that.
      {
        key: "remove",
        label: gui.delete,
        askLabel: gui.delete_ask,
        destructive: true,
        ask: gui.accounts_remove_confirm,
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

  // **The rows are `SideList`'s** (docs/SHELL.md 第四轮): the heading
  // where the group changes, its fold, the face with its corner. The
  // box's rule stays `visibleQuota` above, because the pick has to
  // follow what is on screen (`current`) and it is read off the same
  // filtered list.
  const groupOf = (r: Row): string =>
    r.account?.pinned ? "pin" : groupBy === GROUP_NONE ? "" : `${groupBy}${answersOf(r)[groupBy] ?? ""}`;
  const side: SideRow[] = grouped.map((r) => {
    const on = current?.id === r.id;
    // The pips are the windows themselves, so **every** subscription row
    // has them — probed or registered.
    const dials = r.kind === "sub" ? subDials(r) : [];
    const M = sessionMark("agent", r.provider);
    return {
      id: r.id,
      group: groupOf(r),
      pinned: !!r.account?.pinned,
      // **The machine card's dial, one size down** — the same geometry,
      // milestones and rim (2026-09-07:「环全部和机器卡对齐」); the
      // reading is inside the ring.
      // Not a machine's face, so `art`, and no mark on it.
      face: {
        art: (
          <Face
            size={DIAL_ROW_SIZE}
            stroke={DIAL_ROW_STROKE}
            ratio={DIAL_ROW_RATIO}
            crop
            fraction={arcOf(r)}
            strain={r.account?.strain ?? null}
            pips={dials.map((d): Pip => ({ word: d.word, v: d.pct }))}
          >
            {centreOf(r)}
          </Face>
        ),
      },
      // **The kind's mark opens the title** (2026-09-30, the owner: 「额度
      // 页 list 里，把类型的头像做成标题的开头，如 f"{icon} Claude Code
      // 订阅"」): the provider's glyph, then the name, on one line and one
      // baseline — what this reading is *of*, said where the eye starts.
      // The glyph's box stands on the baseline (`items-baseline`) and is
      // let down 0.15em, so it sits across the line the way an SF Symbol
      // does beside text — centred on the caps, not perched above them
      // (bottom-on-baseline read as a superscript at 14 beside 13).
      //
      // ⚠️ 墓碑 (2026-09-28 → 09-30): the same glyph stood in a corner
      // *beside* the dial (`FaceMark` `beside`), because on the dial it
      // sat on the reading's last glyph (「39%」「124$」, audit 第 8 条).
      // Beside, it widened every face by a corner and still read as the
      // dial's, not the row's; in the title it is neither on the number
      // nor in its way, and `beside` retired with it.
      //
      // **The title is somebody's own word** (「标题是用户可以自己起的」),
      // so the line under it carries what the row *is*: 类型 · 提供商 —
      // and it renames the session row's way: double-click, type, Enter
      // (`EditableName`, `nested` inside the button that opens the row).
      title: (
        // ⚠️ The audit fix plan (F2.1) put the mark after the name, so every
        // list's words start on one x; the owner's own ruling above has it
        // open the title, and the ruling stands.
        <span data-quota-name className="flex min-w-0 items-baseline gap-xxs">
          <M className="size-kind-mark-glyph flex-none translate-y-[0.15em]" />
          <span className="block min-w-0 flex-1">
            <EditableName
              kind="account"
              id={r.id}
              title={r.name}
              className="block"
              inputClassName="w-full text-body"
              nested
            />
          </span>
        </span>
      ),
      name: r.name,
      sub: <span data-quota-sub-line>{subLine(r)}</span>,
      // **The right end is when this was read**, not what it says.
      trailing: (
        <span data-quota-age-cell className="flex-none text-caption tabular-nums text-muted-foreground">
          {r.age_ms === null ? gui.accounts_just_now : gui.card_age(lasting(r.age_ms))}
        </span>
      ),
      find: `${r.name} ${subLine(r)}`,
      selected: on,
      menu: actionsFor(r),
      onOpen: () => {
        setPicked(r.id);
        onOpen?.();
      },
      openData: {
        "data-quota-row": r.id,
        "data-on": on,
        "data-provider": r.provider,
        "data-quota-kind": r.kind,
        "data-quota-from": r.from || "khor",
        "data-quota-pinned": r.account ? r.account.pinned : undefined,
        "data-pinned": r.account ? r.account.pinned : undefined,
        "data-trouble": r.account?.reading.trouble ?? undefined,
      },
    };
  });
  const list = (
    <SideList
      rows={side}
      // Not read yet: the list's skeleton under its own title row.
      loading={board === null}
      mark={{ "data-quota-list": "" }}
      label={(g) => (g === "pin" ? groupLabel(g) : valueWord(groupBy, g.slice(groupBy.length)))}
      headData={(g) => (g === "pin" ? {} : { "data-quota-group": g })}
      // An empty list is a filter — said the same way either road —
      // unless there is nothing at all (a machine with no login read and
      // nothing registered, 2026-10-01): then the wide face's list is
      // blank and its board says so, and the narrow face's list screen,
      // which has no board, says it here with the way to the ＋ above.
      empty={
        rows.length > 0 ? (
          <span data-quota-none>{gui.no_matches}</span>
        ) : narrow ? (
          <>
            <span data-quota-empty>{gui.accounts_empty}</span>
            <span data-quota-empty-how className="mt-xxs block text-callout">
              {gui.accounts_empty_how}
            </span>
          </>
        ) : null
      }
      none={<span data-quota-none>{gui.no_matches}</span>}
    />
  );

  // **The strip where a title would have sat.** This landing used to
  // print 「额度」 across the top, which no other pane does
  // (docs/UX.md): the pane's name rides its `aria-label`, and the room
  // holds the work — search, the three axes, the grouping, and 登记.
  const bar = (
    <PaneBar
      seat="side"
      title={gui.accounts_tab}
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
      actions={quotaActions}
    />
  );

  // **Two details, because there are two kinds of row.** The
  // subscription's leads with the same dial its row wears, one size up:
  // the row answers 「什么最先卡住我」 at a glance and the card answers
  // 「每个窗口各是多少」 in words beside it — one drawing, read twice,
  // instead of a second vocabulary on the second screen.
  const acc = current?.account ?? null;
  const isSubRow = current?.kind === "sub";
  // **Asked once, when a subscription card is on screen.** Not on the
  // page's poll: a tally of every transcript on the machine is not a
  // thing to run every two seconds for a chart nobody is looking at,
  // which is exactly what got this call deleted once already.
  //
  // Keyed on the row's id rather than on 「is a sub open」 so that
  // hopping between two subscriptions re-asks; the node's own cache
  // makes the second ask a directory walk (`Meters::tally`).
  useEffect(() => {
    if (!isSubRow) return;
    let live = true;
    fetchUsage()
      .then((u) => live && setUsage(u))
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [isSubRow, current?.id]);
  // **By kind, and that is the whole change here.** 「Has an account」
  // used to mean 「is not a subscription」; a registered subscription
  // has one, so asking the old question would draw *both* cards for it
  // — the windows card and the money card, one under the other.
  const isSub = current?.kind === "sub";
  const dials = current && isSub ? subDials(current) : [];
  const subDetail = current && isSub && (
    <div data-quota-detail={current.id} className="flex flex-col gap-row px-page py-row">
      {/* No head: the name is R0's, the plan and the age are R0's
          caption (`ReadCaption`). */}
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
      {/* **Narrow: the windows under the dial, full width** (the audit fix
          plan F4.4; N09): beside it, the names were cut to 「5…」「7..」. */}
      <div
        data-quota-lead
        className={cn("flex gap-x-group gap-y-row", narrow ? "flex-col items-stretch" : "flex-wrap items-center")}
      >
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
        {/* On the page's left line (`FlushInk`): the dial's first mark,
            not its box (the redesign, 2026-10-02). */}
        <FlushInk>
        <Face
          size={DIAL_CARD_SIZE}
          stroke={DIAL_CARD_STROKE}
          ratio={DIAL_CARD_RATIO}
          crop
          fraction={arcOf(current)}
          pips={dials.map((d): Pip => ({ word: d.word, v: d.pct }))}
        >
          {centreOf(current)}
        </Face>
        </FlushInk>
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
          className="grid min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-row gap-y-1.5 text-callout"
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
              <dt className="flex min-w-0 flex-wrap items-baseline gap-x-2 text-body">
                <span data-quota-window-name className="whitespace-nowrap">{d.word}</span>
                {d.resets_at !== null && (
                  <span className="flex-none text-callout text-muted-foreground">
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
              <dd data-quota-window-pct className="min-w-[4ch] text-right font-title tabular-nums">
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
          className="grid grid-cols-[auto_1fr] gap-x-row gap-y-1 text-callout"
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
      {/* **The lower half — the same subscription in tokens.** The card
          above is money and windows; this is what was spent to fill
          them. It draws nothing while the answer is out and a sentence
          when there is none, so the seam below never becomes an empty
          chart (`UsageHalf`). */}
      {(() => {
        const u = fromUsage(usage, current.provider);
        return <UsageHalf rows={u.rows} why={u.why} today={u.today} cacheKnown={u.cacheKnown} source="transcript" />;
      })()}
    </div>
  );

  const detail = acc && current && !isSub && (
    <div data-quota-detail={acc.reading.account} className="flex flex-col gap-row px-page py-row">
      {/* No head: the name is R0's, the plan and the age are R0's
          caption (`ReadCaption`). */}
      {/* **The same two columns the subscription's card has**: the dial
          on the left, this reading's own figures on the right
          (2026-09-07:「首节表盘 | 事实两栏,单读数的卡右边不许空」).
          Stacked, the figures sat under a dial with half a card of white
          space beside it — and a card whose right half is empty reads as
          a card that failed to load something.
          **No pips**: an account is one reading and has no parts, and a
          rim with no marks is the honest drawing of that. */}
      <div data-quota-lead className="flex flex-wrap items-center gap-x-group gap-y-row">
        <FlushInk>
        <Face
          size={DIAL_CARD_SIZE}
          stroke={DIAL_CARD_STROKE}
          ratio={DIAL_CARD_RATIO}
          crop
          fraction={arcOf(current)}
          strain={acc.strain}
        >
          {centre(acc)}
        </Face>
        </FlushInk>
        <dl data-quota-figures className="grid min-w-0 grid-cols-[auto_1fr] gap-x-6 gap-y-1 text-callout">
        {acc.reading.trouble ? (
          <>
            <dt className="text-muted-foreground">{gui.accounts_state}</dt>
            <dd data-quota-trouble={acc.reading.trouble}>{troubleWord(acc.reading.trouble, acc.reading.provider)}</dd>
          </>
        ) : (
          <>
            {acc.reading.spent != null && (
              <>
                <dt className="text-muted-foreground">{gui.accounts_spent}</dt>
                <dd data-quota-spent className="tabular-nums">
                  {acc.reading.limit != null
                    ? gui.accounts_of(
                        money(acc.reading.spent, null),
                        money(acc.reading.limit, markOf(acc.unit)),
                      )
                    : money(acc.reading.spent, markOf(acc.unit))}
                  {acc.reading.unlimited ? ` · ${gui.accounts_unlimited}` : ""}
                </dd>
              </>
            )}
            {acc.reading.remaining != null && (
              <>
                <dt className="text-muted-foreground">{gui.accounts_remaining}</dt>
                <dd data-quota-remaining className="tabular-nums">
                  {money(acc.reading.remaining, markOf(acc.unit))}
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
                    <span key={m} className="rounded bg-secondary px-1.5 font-mono text-caption">
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
                  {money(acc.reading.spent_this_month, markOf(acc.unit))}
                </dd>
                <dt className="text-muted-foreground">{gui.accounts_all_time}</dt>
                <dd data-quota-all-time className="tabular-nums">
                  {money(acc.reading.spent_all_time, markOf(acc.unit))}
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
      {/* **What the provider billed, in tokens.** Most accounts have no
          such report and say so rather than drawing an empty chart; see
          `fromReading`.

          ⚠️ **Tombstone: the by-day *spend* chart left this card on
          2026-09-10** (「取消现在那个多出的最近1天那的bar图.就绘制花掉的
          token」). Two bar charts of the same shape sat here, one above
          the other — money per day, then tokens per day — and the card
          read as two products however carefully the second was matched
          to the first. The money is not lost: the dial above is money
          and the row's own column is money.

          **The cut-log line stayed, and keeping it here is the point.**
          `days` and `model_tokens` come out of the *same*
          `/api/log/token` answer (`crates/node/src/accounts.rs`), so a
          log the provider truncated truncates **this** chart too.
          Dropping the sentence along with the chart it used to sit
          under would leave a reader believing a picture of one day is a
          picture of the window — 「看图的人不能以为自己看的是 7 天」,
          2026-09-07. */}
      {(() => {
        const u = fromReading(acc.reading);
        return (
          <>
            {acc.reading.log_capped != null && u.rows !== null && (
              <div data-quota-days-capped className="text-caption text-muted-foreground">
                {gui.accounts_log_capped(acc.reading.log_capped)}
              </div>
            )}
            <UsageHalf rows={u.rows} why={u.why} today={null} cacheKnown={u.cacheKnown} source="vendor" />
          </>
        );
      })()}
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

  // Never read yet: the skeleton, not a blank (components/RegionState).
  // Read and empty: one sentence and its one button (components/EmptyState).
  // **R0's right end** (components/RegionState): stale — the shell
  // cannot reach khor, or this landing's own poll was refused over a
  // board already read — says so; otherwise the open account's plan and
  // the age of its reading (`ReadCaption`).
  /** 过期 / 离线: the last board stays and R0's right end says so. */
  const staleSay =
    stale || (error && board !== null) ? (
      <span data-quota-stale title={error ?? undefined}>
        {stale ? gui.backend_unreachable : error}
      </span>
    ) : undefined;
  /** The narrow face's head has one line, so it keeps both at its end. */
  const caption = staleSay ?? (current ? <ReadCaption row={current} /> : undefined);
  /** **R0's second line on the wide face** (2026-09-30, the A pages
      「和工作区看齐」): the plan and when the reading was taken, under
      the account's name — moved from R0's right end, which keeps only
      the stale note above. Nothing with no account on the board: an
      empty landing has no reading to date. */
  // **R0's avatar** (2026-10-01, the owner: 「额度页就是厂商的 icon 呀」):
  // the vendor's mark — the one this account's row opens its title with
  // (`sessionMark`), drawn itself at `--avatar` 32, two lines tall, on
  // nothing — no disc, no ring (`PageAvatar`); an account has no face.
  // Never the dial. Nothing with no account open: the landing's name alone
  // heads an empty board. The board under it opens on the dial: the
  // account's name, plan and vendor are R0's, said once.
  const VendorMark = current ? sessionMark("agent", current.provider) : null;
  const subtitle = current && VendorMark && (
    <>
    <PageAvatar kind={current.provider}>
      <VendorMark className="block size-head-ctl" />
    </PageAvatar>
    <PageSubtitle>
      <span
        data-quota-head-sub
        className="min-w-0 truncate text-callout leading-(--text-caption--line-height) text-muted-foreground tabular-nums"
      >
        <ReadCaption row={current} />
      </span>
    </PageSubtitle>
    </>
  );

  // Never answered and refused: the machine's own sentence where the
  // empty state sits, and 重试 (components/RegionState `error`).
  const empty =
    board === null && error ? (
      <RegionState
        state="error"
        message={error}
        onRetry={() => {
          fetchQuotaBoard()
            .then((rows) => {
              setBoard(rows);
              setError(null);
            })
            .catch((e) => setError(String(e instanceof Error ? e.message : e)));
        }}
      />
    ) : board === null ? (
      <RegionState state="loading-never" />
    ) : (
      // **The sentence, and where the one act lives — no button**
      // (2026-10-01, the owner: 「空状态不加添加工作区和登记」). 登记… has
      // one home, the ＋ beside the search box (10-layout D9); the empty
      // board points at it rather than being a second door.
      // ⚠️ 墓碑 (2026-09-28 → 10-01): 「登记…」 stood here as the empty
      // state's one button (`data-quota-register`), D9's named exception.
      <EmptyState data-quota-empty-state hint={<span data-quota-empty-how>{gui.accounts_empty_how}</span>}>
        <span data-quota-empty>{gui.accounts_empty}</span>
      </EmptyState>
    );

  return (
    <section data-quota-landing aria-label={gui.accounts_tab} className="flex h-full min-h-0 min-w-0 flex-col">
      {/* ⚠️ 墓碑 (2026-09-27): a red strip (`data-quota-error`) stood
          here over the whole landing whenever a poll was refused. With
          a board already read that is 过期 — the last answer stays and
          R0's caption says so (`caption` below); with none it is the
          error state in the board (`empty`). Never a banner
          (components/RegionState). */}
      {rows.length === 0 && (board === null || !narrow) ? (
        // Never read (the skeleton, or the refusal and 重试), or read and
        // empty on the wide face. **The list column and its bar stay**
        // (2026-10-01): the empty state has no button any more, so the
        // ＋ beside the search box is the one way to register the first
        // account, and it has to be there to be pointed at. ⚠️ 墓碑
        // (→ 10-01): an empty landing drew no list column and no bar —
        // 「a search box over an empty list is a control with nothing to
        // do」 — because the board's button was the door. **Still a page
        // with an R0** on the wide face (第十五轮 第 5 条).
        // **A loading list keeps its title row** (the audit fix plan F3.4;
        // W06): the bar is there before the first answer, so nothing
        // jumps when it lands. ⚠️ 墓碑: the column (and on the narrow face
        // the screen's head) was drawn only once the board had been read.
        narrow ? (
          board === null && !error ? (
            <ListPane mark="data-quota-pane" name={gui.accounts_tab} pane={null} narrow seat="side" head={bar}>
              {list}
            </ListPane>
          ) : (
            empty
          )
        ) : (
          <div className="flex min-h-0 flex-1 flex-col">
            {(board !== null || !error) && (
              <ListPane
                mark="data-quota-pane"
                name={gui.accounts_tab}
                pane={pane}
                narrow={false}
                seam={{ "data-list-seam": "" }}
                seat="side"
                head={bar}
              >
                {list}
              </ListPane>
            )}
            <Region kind="detail" title={gui.accounts_tab} caption={stale ? gui.backend_unreachable : undefined}>
              {empty}
            </Region>
          </div>
        )
      ) : narrow ? (
        // **One screen at a time, with a way back** — the same pair the
        // sessions landing draws (2026-09-06: 额度 became an inbox, so
        // it takes the inbox posture). It was a list and a detail
        // stacked in one scroller, which reads as「everything at once」
        // on a phone: no exit, and the thing you pressed opens below
        // the fold you are already looking at.
        screen === "detail" && current ? (
          <div className="flex min-h-0 flex-1 flex-col">
            {/* In the window's first row, not a second bar under it
                (`NarrowHead`, 2026-09-28). */}
            <NarrowHead>
            {(seated) => (
            <header
              data-detail-header
              className={seated ? "flex min-w-0 flex-1 items-center gap-2 self-stretch" : "flex min-h-ctl-lg flex-none items-center gap-2 border-b px-3"}
            >
              {/* **The only exit from this screen**, so it carries the
                  finger floor outright — the rule `DetailPane`'s back
                  button was rewritten for. */}
              <GlassCircle size="head" aria-label={gui.back} data-back onClick={onBack}>
                <IconBack />
              </GlassCircle>
              {/* The head the machine page has (the redesign): the
                  vendor's mark at the head's size, the name over the
                  reading's caption. */}
              {VendorMark && <VendorMark className="block size-head-ctl flex-none" />}
              {/* **The name, said once, here** — the narrow face's R0.
                  ⚠️ Rewritten 2026-09-27: this bar used to hold the way
                  back and nothing else, because the card's first line
                  titled the card and a name here was the same word
                  twice. The card lost that line (「身份只出现一次」,
                  components/Toolbar), so the one place left to say it
                  is this bar, as wide R0 does — with the reading's
                  caption at its right end. */}
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="min-w-0 truncate text-title-md font-title">{current.name}</span>
                <span data-region-caption className="min-w-0 truncate text-callout text-muted-foreground tabular-nums">
                  {caption}
                </span>
              </span>
            </header>
            )}
            </NarrowHead>
            <div className="min-h-0 flex-1 overflow-y-auto">
              {detail}
              {subDetail}
            </div>
          </div>
        ) : (
          <ListPane mark="data-quota-pane" name={gui.accounts_tab} pane={null} narrow seat="side" head={bar}>
            {list}
          </ListPane>
        )
      ) : (
        <div className="flex min-h-0 flex-1 flex-col">
          {/* The bar belongs to the list column, not to the landing:
              what it narrows is the list, and a bar spanning the detail
              too would say it narrows that as well. Same seat it has on
              the sessions pane — **and since 2026-09-20 it is literally
              the same component** (第 6 条): the ground, the head's
              height, the scroller and the seam all come from
              `ListPane`, and what is left here is the bar and the rows.
              **Both are drawn in the left block** (第十三轮), written
              here: the column beside the rail, the bar in the block's
              first row. What stays on this side is the card, on the
              region's board. */}
          <ListPane
            mark="data-quota-pane"
            name={gui.accounts_tab}
            pane={pane}
            narrow={false}
            seam={{ "data-list-seam": "" }}
            seat="side"
            head={bar}
          >
            {list}
          </ListPane>
          {/* 详情 (A): R0 names the account on the board, the board
              reads it. */}
          <Region
            kind="detail"
            title={current?.name ?? gui.accounts_tab}
            caption={staleSay}
            className="overflow-y-auto"
          >
            {subtitle}
            {detail}
            {subDetail}
          </Region>
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
 * What R0 says under the account's name: its plan, and **when it was
 * read** (`PageSubtitle` on the wide face since 2026-09-30; before that
 * R0's right end, which the narrow face's one-line head still is).
 *
 * # Why the age is here, and why the tombstone that removed it stands
 *
 * `zh.toml` carries a tombstone from 2026-08-28: the line naming whose
 * login the reading came through was taken off, and 「同一刀去掉的还有
 * 读数年龄」. **That judgment was true of the thing it was about** — a
 * small grey sentence explaining khor's own plumbing, under a number.
 * The age came back on 2026-09-06 because the machine card said how old
 * its reading was in the same seat, and two cards disagreeing about
 * whether age is a fact worth having is not restraint. What the two
 * share is the seat and the age, not the pair of keys — khor's version
 * says nothing about a vendor's reading, so it is not copied.
 *
 * ⚠️ 墓碑 (2026-09-27): **the seat moved from the card's head to R0.**
 * The card had a first line of its own — the account's name in
 * `title-md`, the plan as a bordered chip, the age at the far end —
 * while R0 above already carried the name. 「身份只出现一次… 板里不许再
 * 有第二个标题」 (components/Toolbar, Board): the name went, and what it
 * carried went to R0's right end, where a reading's age is said on every
 * page (「3 分钟前的读数」, components/RegionState). The plan rides
 * with it because nobody chose it — it comes off the login — so it is a
 * fact about the reading, not something the list row already said.
 *
 * The words are the ones the figures list was already using
 * (`accounts-read-here` / `accounts-read-from`), moved, not rewritten.
 */
function ReadCaption({ row }: { row: Row }) {
  // Who took it: the machine holding an account's key, or the machine
  // whose login the subscription was read through.
  const reader = row.account ? row.account.home_name : fromWord(row.from);
  return (
    <>
      {row.plan && (
        <>
          <span data-quota-plan>{row.plan}</span>
          <span aria-hidden="true"> · </span>
        </>
      )}
      <span data-quota-age={row.age_ms ?? 0}>
        {row.age_ms === null ? gui.accounts_read_here : gui.accounts_read_from(reader, lasting(row.age_ms))}
      </span>
    </>
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
  // What a subscription has to edit is its name; the boxes below are
  // for things it does not have, and the node refuses them (its
  // `register_subscription`). **The 单位 box is deliberately left
  // standing** — it is another lane's work this week, and it is inert
  // here rather than wrong: a subscription draws no money figure for a
  // unit to be the label of.
  const isSub = row?.kind === "sub";
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [key, setKey] = useState("");
  const [budget, setBudget] = useState("");
  const [unit, setUnit] = useState("");
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
    // **The override, not the effective unit.** `reading.currency` has
    // the override already applied, so seeding from it would fill this
    // box on every account that has ever been read — the thing the
    // note above refuses.
    setUnit(acc.unit ?? "");
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
      // Blank travels as absent: 「use what the endpoint said」 is the
      // absence of an override, not an override to nothing.
      unit: unit.trim() === "" ? undefined : unit.trim(),
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
          <label className="flex min-w-0 flex-col gap-1 text-callout">
            <span className="text-muted-foreground">{gui.accounts_name}</span>
            <Input data-quota-edit-name value={name} onChange={(e) => setName(e.target.value)} />
          </label>
          {!isSub && acc.reading.provider === "newapi" && (
            <label className="flex min-w-0 flex-col gap-1 text-callout">
              <span className="text-muted-foreground">{gui.accounts_address}</span>
              <Input
                data-quota-edit-address
                value={baseUrl}
                placeholder="https://"
                onChange={(e) => setBaseUrl(e.target.value)}
              />
            </label>
          )}
          {/* **Not drawn where the key is not** (`acc.mine`, the node's
              own answer to 「is this reading mine to take」), so a secret
              is not asked for and then refused. ⚠️ 墓碑 (the audit fix
              plan F1.9): drawn shut and greyed. */}
          {!isSub && acc.mine && (
          <label className="flex min-w-0 flex-col gap-1 text-callout">
            <span className="text-muted-foreground">{gui.accounts_key}</span>
            <Input
              data-quota-edit-key
              type="password"
              autoComplete="off"
              value={key}
              onChange={(e) => setKey(e.target.value)}
            />
            <span data-quota-edit-key-note className="text-caption text-muted-foreground">
              {gui.accounts_key_keep}
            </span>
          </label>
          )}
          {/* **单位:人打进去的那串字符,khor 不认它。** 2026-09-07:
              「直接输入符号。比如 $,不要去识别,不然各种货币符号我们
              识别不完,用户也可能说的不对。」 The box holds a *mark*,
              not a currency code, and whatever is in it is drawn as it
              stands — see the tombstone over `DEFAULT_MARK`.

              ⚠️ **The placeholder is `$` — the default — and no longer
              the endpoint's code.** It used to show what the gateway
              claimed, which made sense while khor read codes; it now
              reads as an invitation to type one, and a typed `USD`
              would land on the ring as three characters of a
              four-character budget. What an empty box actually draws
              is `$`, and that is what stands in it.

              ⚠️ **Still a placeholder, never a value.** Prefilled, a
              person who came to fix a *name* and pressed 保存 would
              leave with a unit override they never chose — **a control
              that changes state while nobody touches it is not an
              option, it is a decision taken on their behalf.** (The key
              box above refuses the mirror image: it may not show what
              is stored. Same rule, opposite direction.)

              **Nothing here converts anything**; the figure is
              untouched, and this must not grow into a rate feature. */}
          <label className="flex min-w-0 flex-col gap-1 text-callout">
            <span className="text-muted-foreground">{gui.accounts_unit}</span>
            <Input
              data-quota-edit-unit
              value={unit}
              placeholder={DEFAULT_MARK}
              onChange={(e) => setUnit(e.target.value)}
            />
            {/* **Shortcuts, not an enumeration.** 2026-09-07:「给几个
                推荐选项,默认是 $」. Pressing one fills the box and
                leaves it editable — the free field above is still the
                whole answer, and a mark that is not on this row (a
                person typing `₩`, or `USDT`) saves exactly the same
                way. **The row must not read as a set of allowed
                values**, which is why these are chips beside an open
                field rather than a picker in front of one.

                Four, and the count is set by 「一行放得下」: they sit
                on one line at the dialog's width with room to spare,
                and a fifth buys a currency khor has never seen an
                account in. `$` is first because it is the default.

                **The one in force is marked**, so a person opening 编辑
                can see what this account reads as today — including
                when the box is empty and the answer is the default. The
                mark is a real difference in ink (border and ground),
                not a `data-` label about one. */}
            <span data-quota-unit-picks className="flex flex-wrap items-center gap-1">
              {UNIT_PICKS.map((m) => {
                const on = m === markOf(unit === "" ? null : unit);
                return (
                  <button
                    key={m}
                    type="button"
                    data-quota-unit-pick={m}
                    onClick={() => setUnit(m)}
                    className={cn(
                      "flex-none rounded-full border px-2 py-0.5 text-caption",
                      // The one in force is chosen: `fill-strong`, not
                      // the brand wash (README 颜色, 4K).
                      on ? "border-foreground/40 bg-fill-strong text-foreground" : "text-muted-foreground",
                    )}
                  >
                    {m}
                  </button>
                );
              })}
            </span>
            <span data-quota-edit-unit-note className="text-caption text-muted-foreground">
              {gui.accounts_unit_blank(DEFAULT_MARK)}
            </span>
          </label>
          {!isSub && BUDGETED.includes(acc.reading.provider) && (
            <label className="flex min-w-0 flex-col gap-1 text-callout">
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
            <p data-quota-edit-error className="text-callout text-destructive">
              {error}
            </p>
          )}
        </div>
        <DialogFooter>
          <Button variant="primary" data-quota-save disabled={busy} aria-busy={busy || undefined} onClick={save}>
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
  const [kind, setKind] = useState("bill");
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [key, setKey] = useState("");
  const [budget, setBudget] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) return;
    setProvider("newapi");
    setKind("bill");
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
      kind,
      base_url: kind === "sub" || provider !== "newapi" ? undefined : baseUrl,
      // **Empty rather than omitted, and the node refuses a key on a
      // subscription rather than dropping one.** Between them there is
      // no path where a secret is accepted and quietly thrown away.
      key: kind === "sub" ? "" : key,
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
      <DialogContent form data-quota-dialog>
        <DialogHeader>
          <DialogTitle>{gui.accounts_register}</DialogTitle>
        </DialogHeader>
        <FormGrid data-quota-form>
          {/* **类型 first, because it decides what the rest of this form
              even is** — a subscription has no key, no address and no
              budget, and a form that asked for them and then refused
              them would be asking somebody to type a secret it was
              always going to send back.

              The same two words the list's 类型 axis is made of, so a
              person picks here what they will read there. Picking 订阅
              narrows 哪家 to the vendors whose login khor knows how
              to find, and carries the choice over rather than leaving a
              provider selected that the kind cannot use. */}
          {/* **Two fixed choices → a segmented control; a vendor list
              that is eight long → the system's menu** (components/
              SegmentedControl, PopupButton; 2026-09-28 — both were rows
              of grey pills). */}
          <FormLabel>{gui.quota_kind}</FormLabel>
          <div className="flex min-w-0">
            <SegmentedControl
              label={gui.quota_kind}
              value={kind}
              onChange={(k) => {
                setKind(k);
                if (k === "sub" && !SUBSCRIBED.includes(provider)) {
                  setProvider(SUBSCRIBED[0] as Provider);
                }
                if (k !== "sub" && SUB_ONLY.includes(provider)) setProvider("newapi");
              }}
              options={["bill", "sub"].map((k) => ({
                key: k,
                label: kindWord(k),
                mark: { "data-quota-kind-pick": k },
              }))}
            />
          </div>
          <FormLabel>{gui.accounts_provider}</FormLabel>
          <div className="flex min-w-0">
            <PopupButton
              look="field"
              label={gui.accounts_provider}
              value={provider}
              onPick={(p) => setProvider(p as Provider)}
              mark={{ "data-quota-provider-select": "" }}
              choices={PROVIDERS.filter((p) => (kind === "sub" ? SUBSCRIBED.includes(p) : !SUB_ONLY.includes(p))).map((p) => ({
                value: p,
                label: providerWord(p),
                mark: { "data-quota-provider": p },
              }))}
            />
          </div>
          <FormLabel htmlFor="khor-quota-name">{gui.accounts_name}</FormLabel>
          <Input id="khor-quota-name" data-quota-name value={name} onChange={(e) => setName(e.target.value)} />
          {kind !== "sub" && provider === "newapi" && (
            <>
              <FormLabel htmlFor="khor-quota-address">{gui.accounts_address}</FormLabel>
              <Input
                id="khor-quota-address"
                data-quota-address
                value={baseUrl}
                placeholder="https://"
                onChange={(e) => setBaseUrl(e.target.value)}
              />
            </>
          )}
          {kind === "sub" ? (
            /* **The note is the key box's replacement, not decoration.**
               Somebody who came here to register something and finds no
               box to put a secret in is owed the reason — it is the
               same debt the two lines under the key box pay, in the
               other direction: there is nothing to paste because what
               reads this is the login already on this machine. */
            <p data-quota-sub-note className="col-start-2 text-caption text-muted-foreground">
              {gui.accounts_sub_note(vendorOf(provider).name)}
            </p>
          ) : (
          <>
            <FormLabel htmlFor="khor-quota-key">{gui.accounts_key}</FormLabel>
            <Input
              id="khor-quota-key"
              data-quota-key
              type="password"
              autoComplete="off"
              value={key}
              onChange={(e) => setKey(e.target.value)}
            />
            {/* The two facts the form owes a person before they paste a
                secret: where it goes, and which kind of key can read
                usage at all. Under the box, in its column. */}
            <span className="col-start-2 text-caption text-muted-foreground">
              {NEEDS_ADMIN.includes(provider) ? `${gui.accounts_key_stays} ${gui.accounts_needs_admin}` : gui.accounts_key_stays}
            </span>
          </>
          )}
          {kind !== "sub" && BUDGETED.includes(provider) && (
            <>
              <FormLabel htmlFor="khor-quota-budget">{gui.accounts_budget}</FormLabel>
              <Input
                id="khor-quota-budget"
                data-quota-budget
                inputMode="decimal"
                value={budget}
                placeholder={gui.accounts_budget_hint}
                onChange={(e) => setBudget(e.target.value)}
              />
            </>
          )}
          {error && (
            <p data-quota-dialog-error className="col-span-2 text-callout text-destructive">
              {error}
            </p>
          )}
        </FormGrid>
        <DialogFooter>
          <Button data-dialog-cancel onClick={() => onOpenChange(false)}>
            {gui.delete_keep}
          </Button>
          <Button variant="primary" data-quota-create disabled={busy} aria-busy={busy || undefined} onClick={create}>
            {gui.accounts_register}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
