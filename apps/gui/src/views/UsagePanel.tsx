import type { QuotaAnswer, QuotaWindow, Usage } from "@/api";
import { Ring } from "@/components/Ring";
import { cli, gui } from "@/gen/catalog";
import { gaugeColor } from "@/lib/vitals";
import { ago, until } from "@/words";

/**
 * What the agents have cost, by day.
 *
 * # Numbers, and nothing explaining them
 *
 * No total, no bar, no percentage. **A total would be almost entirely
 * cache reads** — measured on the development machine, 16.2 billion
 * cached input against 32 million output — so it would move with how
 * often a conversation was resumed rather than with how much work was
 * done (`khor_core::Tokens` carries the judgment). A bar needs a
 * denominator, and there is none: nobody has told khor what an allowance
 * is. So the four numbers stand as they are, in the order and the words
 * `khor usage` prints them, from the same catalog keys — one fact, one
 * word, on both faces.
 *
 * # What is *not* said here, and where it is said instead
 *
 * These are every machine's numbers added together, and this panel does
 * not name the machines. It does not have to: the map is beside it, and
 * a machine khor has never got an answer out of says so on its own face
 * there. Two halves of one screen, neither captioning the other.
 *
 * # No window
 *
 * Every day khor has, newest first, in a scroll. `khor usage` takes
 * `--days` because a terminal prints once and scrolls away; a screen
 * scrolls on its own, so a control here would be a setting answering a
 * question the scrollbar already answers (docs/UX.md 设置).
 *
 * # The subscription above them, and why *it* gets rings
 *
 * The paragraph above says a bar needs a denominator and there is none.
 * That is still true of the token counts — nobody has told khor what an
 * allowance is — and it is exactly **not** true of the subscription
 * windows, which are nothing but a denominator and a share of it. So one
 * panel carries both, drawn differently on purpose: the windows as
 * rings, the spending as numbers, and the difference between them says
 * which one has a limit to be near.
 */
export function UsagePanel({ usage, quota }: { usage: Usage | null; quota: QuotaAnswer | null }) {
  // Nothing at all until the first answer is in. A panel that said "no
  // spending" while the answer was still coming would be telling a
  // machine's whole history wrong for as long as it took to arrive.
  if (!usage) return null;
  // Newest first here, oldest first on the wire: the order a list is
  // read in is this screen's business, and reversing the library's one
  // answer is not a second sort.
  const days = [...usage.days].reverse();
  return (
    <section data-usage className="flex h-full min-w-0 flex-col overflow-y-auto p-4">
      <Subscription answer={quota} />
      {days.length === 0 && <div className="text-sm text-muted-foreground">{cli.usage_none}</div>}
      {days.map((row, i) => (
        <div key={`${row.day}/${row.category}`} data-usage-day={row.day}>
          {/* The date leads its own rows and is not repeated on each of
              them — the same shape the session list's group headings
              take, and the reason is the same: a heading says what the
              rows under it have in common. */}
          {(i === 0 || days[i - 1].day !== row.day) && (
            <div data-usage-date className="pt-3 text-sm text-muted-foreground first:pt-0">
              {row.day}
            </div>
          )}
          <div data-usage-row={row.category} className="flex flex-wrap items-baseline gap-x-3 py-1">
            {/* The vendor's own name, printed as it stands. A proper
                noun translated is a proper noun nobody can look up
                (docs/handoff 借来的色板留自己的名字). */}
            <span data-usage-category className="text-sm">
              {row.category}
            </span>
            <span className="text-sm text-muted-foreground">
              {cli.tokens_input(count(row.tokens.input))}
            </span>
            <span className="text-sm text-muted-foreground">
              {cli.tokens_output(count(row.tokens.output))}
            </span>
            <span className="text-sm text-muted-foreground">
              {cli.tokens_cached(count(row.tokens.cached_input))}
            </span>
            <span className="text-sm text-muted-foreground">
              {cli.tokens_cache_write(count(row.tokens.cache_write))}
            </span>
          </div>
        </div>
      ))}
      {/* Said only when it is not zero. A vendor reshaped a file khor
          reads, so the numbers above are missing something — and silence
          plus a zero mean the same thing while silence plus a count does
          not. */}
      {usage.unreadable > 0 && (
        <div data-usage-unreadable className="pt-3 text-sm text-muted-foreground">
          {cli.usage_unreadable(usage.unreadable)}
        </div>
      )}
    </section>
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
function count(n: number): string {
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
    <div data-quota className="flex flex-col gap-in pb-pane">
      <div className="flex flex-wrap gap-row">
        {q.windows.map((w) => (
          <div key={w.kind} data-quota-window={w.kind} className="flex flex-col items-center gap-in">
            <Ring fraction={w.used_pct / 100} color={gaugeColor(null)}>
              {`${Math.round(w.used_pct)}%`}
            </Ring>
            <span className="text-aux text-muted-foreground">{windowName(w)}</span>
            {/* Only when there is one. A window past its reset has none —
                the moment it names is behind it, and the reading beside
                it has already gone back to zero. */}
            {w.resets_at !== null && (
              <span data-quota-resets className="text-aux text-muted-foreground">
                {gui.quota_resets(until(w.resets_at * 1000))}
              </span>
            )}
          </div>
        ))}
      </div>
      <div data-quota-via className="text-aux text-muted-foreground">
        {gui.quota_via}
      </div>
      {/* The same sentence a machine's readings use, for the same reason:
          a number with no age beside it is read as the present, and this
          one may be up to five minutes old by design. */}
      {q.as_of !== null && (
        <div data-quota-age className="text-aux text-muted-foreground">
          {cli.vitals_taken(ago(q.as_of * 1000))}
        </div>
      )}
    </div>
  );
}
