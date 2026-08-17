import type { CodexQuota, Usage } from "@/api";
import { cli, gui } from "@/gen/catalog";
import { ago } from "@/words";

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
 */
export function UsagePanel({ usage, quota }: { usage: Usage | null; quota: CodexQuota | null }) {
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
      <Quota quota={quota} />
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
 * The codex rate windows, labelled by who answered them.
 *
 * # The label is the point (user ruling 2026-08-17)
 *
 * A rollout's `rate_limits` is the window of whatever backend served
 * that session — the official subscription, or a relay's own counter
 * (new-api and kin). 官方 and 中转 must never wear one face, so the
 * word comes from the snapshot's own provider field, and a relay keeps
 * its proper name untranslated beside it.
 *
 * # Two absences, two different sentences
 *
 * No snapshot at all → no line (khor read nothing; a line would claim
 * it did). A snapshot whose windows are null → the line says 没报窗口:
 * this machine's relay answers the key with nulls, and silence there
 * would read as "no quota exists" (`khor_core::CodexQuota`).
 */
function Quota({ quota }: { quota: CodexQuota | null }) {
  if (!quota) return null;
  const label = quota.provider === "openai" ? gui.quota_official : labelRelay(quota.provider);
  return (
    <div data-codex-quota data-provider={quota.provider ?? undefined} className="flex flex-wrap items-baseline gap-x-3 pb-3">
      <span className="text-sm">{gui.quota_of("codex")}</span>
      <span data-quota-label className="text-sm">
        {label}
      </span>
      {quota.primary || quota.secondary ? (
        [quota.primary, quota.secondary].map(
          (w, i) =>
            w && (
              <span key={i} data-quota-window={i === 0 ? "primary" : "secondary"} className="text-sm text-muted-foreground">
                {gui.quota_window_used(span(w.window_minutes), w.used_percent)}
              </span>
            ),
        )
      ) : (
        <span data-quota-no-windows className="text-sm text-muted-foreground">
          {gui.quota_no_windows}
        </span>
      )}
      <span data-quota-age className="text-sm text-muted-foreground">
        {cli.vitals_taken(ago(quota.at_ms))}
      </span>
    </div>
  );
}

/** 中转, with the relay's proper name when the rollout wrote one. */
function labelRelay(provider: string | null): string {
  return provider ? `${gui.quota_relay} ${provider}` : gui.quota_relay;
}

/** A window's span as a person says it: 300 → 5 小时, 10080 → 7 天. */
function span(minutes: number): string {
  if (minutes % 1440 === 0) return gui.quota_days(minutes / 1440);
  if (minutes % 60 === 0) return gui.quota_hours(minutes / 60);
  return gui.quota_minutes(minutes);
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
