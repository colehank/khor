import type { QuotaAnswer, QuotaWindow, Usage } from "@/api";
import type { UsageDay } from "@/gen/bindings/UsageDay";
import { useState } from "react";
import { Bar } from "@/components/Gauge";
import { Ring } from "@/components/Ring";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { gaugeColor } from "@/lib/vitals";
import { ago, until } from "@/words";

/**
 * What the agents have cost, by day.
 *
 * # One line a day, and the bar compares it to the busiest one
 *
 * **This overturns the head this file used to carry**, which read「No
 * total, no bar, no percentage」and gave one reason for all three: a bar
 * needs a denominator and khor has none. Two of the three still stand —
 * there is still no total (**it would be almost entirely cache reads**:
 * measured on the development machine, 16.2 billion cached input against
 * 32 million output, so it would move with how often a conversation was
 * resumed rather than with how much work was done, `khor_core::Tokens`
 * carries that judgment) and still no percentage-of-an-allowance,
 * because nobody has told khor what an allowance is.
 *
 * The bar fell because **the missing denominator was only missing for
 * one question**. "How much of what you are allowed" has none. "Busy day
 * or quiet one" has an obvious one — *the other days*, which this panel
 * is already holding. So each day gets a bar against the busiest day
 * khor knows, and the bar claims nothing about a limit.
 *
 * What the bar is drawn from is **output**, not a sum, for the reason
 * the vanished total was refused: output is the tokens the models
 * actually produced. The other three ride behind a press.
 *
 * Measured before this, on the mark place at 1120×1079: **537 pieces of
 * text, 4243 characters, 535 of them at one size and 446 of those at one
 * colour and one weight.** Every day looked exactly like every other
 * day, so nothing could be found by looking — only by reading, and by
 * comparing two rows a screen apart digit by digit. The user's word for
 * it was「一堆字表达什么不清楚,看着费劲」.
 *
 * # What is *not* said here, and where it is said instead
 *
 * These are every machine's numbers added together, and this panel does
 * not name the machines. It does not have to: the map is beside it, and
 * a machine khor has never got an answer out of says so on its own face
 * there. Two halves of one screen, neither captioning the other.
 *
 * # Seven days, then 更早
 *
 * Newest first, and the recent week is what opens. **This is not the
 * setting the old head refused** — it stated that a screen scrolls on
 * its own and a control here would answer a question the scrollbar
 * already answers, and that reasoning was sound about a *setting*, which
 * remembers a preference and has to be maintained. 更早 remembers
 * nothing: it is one press that appends the rest for as long as the
 * panel stays open. What it buys is that the first thing on the screen
 * is this week rather than half a month, and half a month of identical
 * rows is the thing that made today hard to find.
 *
 * # The subscription above them, and why *it* gets rings
 *
 * A window is nothing but a denominator and a share of it, so it gets
 * the shape that says "how full": a closed arc. The days get open bars.
 * **The difference between the two shapes is the difference between the
 * two questions** — an arc has an end and being near it means something,
 * a bar only says which of these is longer. That was true when the days
 * were bare numbers and it is why they did not get arcs now that they
 * are drawn at all.
 */
export function UsagePanel({ usage, quota }: { usage: Usage | null; quota: QuotaAnswer | null }) {
  // Before the early return: 更早 is state, and a hook cannot sit behind
  // a condition.
  const [all, setAll] = useState(false);
  const [open, setOpen] = useState<ReadonlySet<string>>(() => new Set());
  // Nothing at all until the first answer is in. A panel that said "no
  // spending" while the answer was still coming would be telling a
  // machine's whole history wrong for as long as it took to arrive.
  if (!usage) return null;
  // Newest first here, oldest first on the wire: the order a list is
  // read in is this screen's business, and reversing the library's one
  // answer is not a second sort.
  const groups = byDay([...usage.days].reverse());
  const shown = all ? groups : groups.slice(0, RECENT_DAYS);
  // **The peak is taken over the days on screen, and the first version
  // took it over every day khor had.** The argument for that was that
  // pressing 更早 must not redraw bars the person had already read — a
  // scale that moves while you look at it. It is a real cost and it lost
  // to a worse one, measured on the live panel the day it shipped: one
  // outlier a fortnight back (1.9M against a week whose busiest day was
  // 586.3k) left **all seven visible bars between 2% and 12%**, which is
  // a row of empty tracks. A bar that cannot be compared to its
  // neighbours is not doing the one thing a bar does, and the scale is
  // stated in words right underneath them (`usage_vs_peak`), so the
  // rescale is legible when it happens and is something the person
  // asked for.
  const peak = Math.max(1, ...shown.flatMap((g) => g.rows.map((r) => r.tokens.output)));
  const toggle = (key: string) =>
    setOpen((was) => {
      const next = new Set(was);
      if (!next.delete(key)) next.add(key);
      return next;
    });
  return (
    <section data-usage className="flex h-full min-w-0 flex-col overflow-y-auto p-4">
      <Subscription answer={quota} />
      {groups.length === 0 && <div className="text-aux text-muted-foreground">{cli.usage_none}</div>}
      {/* One grid for the whole ledger, rows spanning it through subgrid,
          exactly as the network's now seats its machines: **the bars all
          start on one left edge**, which is the only way a length means
          anything. Per-row grids would let a vendor with a longer name
          push its own bar right and quietly make its day look shorter. */}
      {groups.length > 0 && (
        <div data-usage-grid className="grid grid-cols-[max-content_1fr_max-content] gap-x-row">
          {shown.map((g) => (
            <div key={g.day} data-usage-day={g.day} className="col-span-3 grid grid-cols-subgrid">
              {/* The date leads its own rows and is not repeated on each
                  of them — the same shape the session list's group
                  headings take, and the reason is the same: a heading
                  says what the rows under it have in common. */}
              <div
                data-usage-date
                className="col-span-3 pt-group text-aux text-muted-foreground first:pt-0"
              >
                {g.day}
              </div>
              {g.rows.map((row) => (
                <SpendRow
                  key={row.category}
                  row={row}
                  peak={peak}
                  open={open.has(`${g.day}/${row.category}`)}
                  onToggle={() => toggle(`${g.day}/${row.category}`)}
                />
              ))}
            </div>
          ))}
        </div>
      )}
      {/* What the bars are measured against, said once under them rather
          than beside each one. */}
      {groups.length > 0 && (
        <div data-usage-scale className="pt-in text-aux text-muted-foreground">
          {gui.usage_vs_peak}
        </div>
      )}
      {!all && groups.length > shown.length && (
        <button
          type="button"
          data-usage-earlier
          onClick={() => setAll(true)}
          className="mt-group self-start rounded-md px-2 py-1 text-aux text-muted-foreground hover:bg-accent"
        >
          {gui.usage_earlier}
        </button>
      )}
      {/* Said only when it is not zero. A vendor reshaped a file khor
          reads, so the numbers above are missing something — and silence
          plus a zero mean the same thing while silence plus a count does
          not. */}
      {usage.unreadable > 0 && (
        <div data-usage-unreadable className="pt-group text-aux text-muted-foreground">
          {cli.usage_unreadable(usage.unreadable)}
        </div>
      )}
    </section>
  );
}

/** How many days open without asking (the head: seven, then 更早). */
const RECENT_DAYS = 7;

/**
 * The library's rows regrouped by the day they fall on.
 *
 * The wire carries one row per (day, vendor) and a day with two vendors
 * is two rows; the screen wants the day once with its rows under it.
 * Adjacent-run grouping rather than a map, because the input is already
 * sorted by day — and if it ever stops being, this draws the same day
 * twice, which is visible, instead of silently merging rows that were
 * far apart.
 */
function byDay(rows: UsageDay[]): { day: string; rows: UsageDay[] }[] {
  const out: { day: string; rows: UsageDay[] }[] = [];
  for (const r of rows) {
    const last = out[out.length - 1];
    if (last && last.day === r.day) last.rows.push(r);
    else out.push({ day: r.day, rows: [r] });
  }
  return out;
}

/**
 * One vendor's day: what it produced, how that compares, and — on a
 * press — the three numbers behind it.
 *
 * **Output leads and the other three hide** because they answer
 * different questions and only one of them is asked at a glance. 读缓存
 * is the largest number on this screen by three orders of magnitude and
 * the least informative: it says how often a conversation was resumed.
 * Printed beside 输入 748 at the same size and colour, the two read as
 * comparable quantities, which they are not.
 *
 * The row is a button because it does something, and it says so with
 * `aria-expanded` rather than with a chevron: the bar and the number are
 * already the row's whole width, and a glyph whose only job is to
 * announce that a press is possible is a glyph on every row forever.
 */
function SpendRow({
  row,
  peak,
  open,
  onToggle,
}: {
  row: UsageDay;
  peak: number;
  open: boolean;
  onToggle: () => void;
}) {
  return (
    <>
      <button
        type="button"
        data-usage-row={row.category}
        aria-expanded={open}
        onClick={onToggle}
        className="col-span-3 grid grid-cols-subgrid items-center rounded-md px-2 py-1 text-left hover:bg-accent"
      >
        {/* The vendor's own name, printed as it stands. A proper noun
            translated is a proper noun nobody can look up (docs/handoff
            借来的色板留自己的名字). */}
        <span data-usage-category className="text-aux text-muted-foreground">
          {row.category}
        </span>
        <Bar anchor="spend" fraction={row.tokens.output / peak} len="100%" className="min-w-0" />
        <span data-usage-lead className="text-fact tabular-nums">
          {cli.tokens_output(count(row.tokens.output))}
        </span>
      </button>
      {open && (
        <div
          data-usage-detail={row.category}
          className={cn(
            "col-span-3 flex flex-wrap gap-x-row px-2 pb-1",
            "text-aux tabular-nums text-muted-foreground",
          )}
        >
          <span>{cli.tokens_input(count(row.tokens.input))}</span>
          <span>{cli.tokens_cached(count(row.tokens.cached_input))}</span>
          <span>{cli.tokens_cache_write(count(row.tokens.cache_write))}</span>
        </div>
      )}
    </>
  );
}

/**
 * A count as a person reads it: 1000-based, because tokens are counted,
 * not stored.
 *
 * The CLI has its own copy (`crates/cli` `count`), deliberately —
 * formatting is painting, not judgment, and nothing depends on the two
 * producing identical characters. **Not the same function as the byte
 * one either**: 1024 is a property of memory, and a thousand tokens is a
 * thousand tokens.
 */
export function count(n: number): string {
  const units = ["", "k", "M", "G"];
  let v = n;
  let u = 0;
  while (v >= 1000 && u + 1 < units.length) {
    v /= 1000;
    u += 1;
  }
  return u === 0 ? `${n}` : `${v.toFixed(1)}${units[u]}`;
}

/** The name of a window, from its key — the words are the catalog's. */
function windowName(w: QuotaWindow): string {
  switch (w.kind) {
    case "five_hour":
      return gui.quota_five_hour;
    case "seven_day":
      return gui.quota_seven_day;
    case "seven_day_sonnet":
      return gui.quota_seven_day_sonnet;
    case "seven_day_opus":
      return gui.quota_seven_day_opus;
  }
}

/** The sentence for a reason there is no reading. */
function troubleWord(answer: QuotaAnswer): string | null {
  const t = answer.trouble;
  if (!t) return null;
  if (t === "no_login") return gui.quota_no_login;
  if (t === "stale") return gui.quota_stale;
  if (t === "unreadable") return gui.quota_unreadable;
  if (t === "unreachable") return gui.quota_unreachable;
  return gui.quota_cooling(t.cooling.minutes);
}

/**
 * What the Claude subscription has left.
 *
 * **The line naming whose login this came through is not a caption, it
 * is the point.** That credential was stored so `claude` could run;
 * khor reading it to answer a second question widens what it was given
 * for, and the person agreed to that widening knowing it. Numbers that
 * appeared with no account behind them would read as khor's own
 * knowledge, which they are not — so the line goes wherever the numbers
 * go, and it is the first thing written here rather than the last.
 *
 * **Neutral rings, no ramp.** A window at 80% is a fact, not a warning,
 * and khor has no threshold to say otherwise: `Strain` exists for memory
 * and disk because the node decides those, and nothing decides this one.
 * Grading it here would be the frontend inventing a judgment — the same
 * thing this app already refused when it left mandala's `severityColor`
 * behind.
 */
function Subscription({ answer }: { answer: QuotaAnswer | null }) {
  // Nothing at all until the first answer is in: neither a reading nor a
  // reason has happened yet, and drawing either would describe a state
  // that does not exist.
  if (!answer) return null;
  const trouble = troubleWord(answer);
  if (trouble) {
    return (
      <div data-quota-trouble className="pb-pane text-aux text-muted-foreground">
        {trouble}
      </div>
    );
  }
  const q = answer.quota;
  if (!q || q.windows.length === 0) return null;
  return (
    <div data-quota className="flex flex-col gap-row pb-pane">
      {/* **One window per row, ring then sentence** — not two columns
          side by side. Measured on the columns: they were 74 and 64 wide
          with 12 between them, and their reset lines — same size, same
          colour, same baseline, 14 apart — read as one sentence,
          「4 小时后重置 2 天后重置」. Widening the gutter to `--gap-pane`
          bought 26, which is two characters: better, and still two
          separate facts sitting on one line of the page. Stacked, each
          window's words are the only words beside its own arc and no
          gutter has to carry the meaning. Same height either way — two
          rings tall is what two rings cost. */}
      {q.windows.map((w) => (
        <div key={w.kind} data-quota-window={w.kind} className="flex items-center gap-row">
          {/* The percentage is what this panel exists to say, so it reads
              at a size of its own — the tier tokens.css describes as
              being for「a quota arc's centre」and which had never been
              used at one. */}
          <Ring fraction={w.used_pct / 100} color={gaugeColor(null)} centerClass="text-lg">
            {`${Math.round(w.used_pct)}%`}
          </Ring>
          <span className="flex min-w-0 flex-col">
            {/* 「5 小时窗口」, not 「5 小时」: with the reset line right
                under it there were two durations touching, one the length
                of the window and one what is left of it, and nothing on
                the glass said which was which. */}
            <span className="text-fact">{windowName(w)}</span>
            {/* Only when there is one. A window past its reset has none —
                the moment it names is behind it, and the reading beside
                it has already gone back to zero. */}
            {w.resets_at !== null && (
              <span data-quota-resets className="text-aux text-muted-foreground">
                {gui.quota_resets(until(w.resets_at * 1000))}
              </span>
            )}
          </span>
        </div>
      ))}
      {/* Whose login this came through, and how old the reading is —
          **one line, because they are one kind of remark**: both qualify
          the arcs above and neither is a number. As two lines they were
          two thirds of the grey text in this corner of the panel. The age
          is here for the reason a machine's readings carry one: a number
          with no age beside it is read as the present, and this one may
          be up to five minutes old by design. */}
      <div data-quota-via className="text-aux text-muted-foreground">
        {q.as_of !== null
          ? `${gui.quota_via} · ${cli.vitals_taken(ago(q.as_of * 1000))}`
          : gui.quota_via}
      </div>
    </div>
  );
}
