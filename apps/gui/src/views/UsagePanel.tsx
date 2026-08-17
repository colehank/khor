import type { Usage } from "@/api";
import { cli } from "@/gen/catalog";

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
export function UsagePanel({ usage }: { usage: Usage | null }) {
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
