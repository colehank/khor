import type { QuotaAnswer, QuotaWindow } from "@/api";
import { Ring } from "@/components/Ring";
import { gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { gaugeColor } from "@/lib/vitals";
import { until } from "@/words";

/**
 * What the Claude subscription has left — the two windows, as arcs.
 *
 * # It lives at the 额度 page's foot, and the road here had three stops
 *
 * Under the mandala first; then 「额度都是机器卡里的。这里包括 khor
 * 页面和设备页面里点击设备后显示的机器都要画」 (the user, 2026-08-28)
 * put it on both machine cards; then 「机器页面不需要有token usage的环」
 * (the same person, 2026-09-02) took it off them — by then the 额度
 * landing existed, and a page whose name is what these arcs answer is
 * the home the cards never quite were. `QuotaLanding`'s foot is the one
 * caller left.
 *
 * **This machine's login only**, wherever it is drawn: khor reads this
 * from the Claude Code login stored **here** — `khor_node::quota` has
 * two sources and both are local files — and no protocol frame carries
 * a quota, so what another machine has left is something khor does not
 * know. Painting it under another machine's name was the one thing a
 * card was never allowed to do, and the rule survives the move.
 *
 * That is also why it is not a per-machine reading in the way CPU is.
 * `gui_core::quota` says it outright: it takes no `root`, because it is
 * a fact about an *account* rather than about an installation — the
 * same account seen from two machines is one quota, not two. A machine
 * logged into a **different** account would have a genuinely different
 * number, and khor has no way to ask it; that is the batch this file
 * does not pretend to have done.
 *
 * # Why these get arcs when the map's machines get nothing
 *
 * A window is nothing but a denominator and a share of it, so it gets
 * the shape that says 「how full」: a closed arc that carries its own
 * 「满」. That is the same property the star map's bars lacked — a bar
 * says how full something is without saying what, and the user's word
 * for those was 看不懂. Here the denominator is real, it is stated
 * (「5 小时窗口」), and the number is written in the middle.
 *
 * **Neutral rings, no ramp.** A window at 80% is a fact, not a warning,
 * and khor has no threshold to say otherwise: `Strain` exists for memory
 * and disk because the node decides those, and nothing decides this one.
 * Grading it here would be the frontend inventing a judgment — the same
 * thing this app already refused when it left mandala's `severityColor`
 * behind.
 *
 * # The grey line under the arcs is gone, and it was not an ordinary cut
 *
 * It read 「借你登录的 Claude Code · N 分钟前的读数」 and the head of
 * this file used to call it 「唯一非画不可的字」: the credential was
 * stored so `claude` could run, khor reading it to answer a second
 * question widens what it was given for, and the person agreed to that
 * widening **on the condition it stayed visible** (2026-08-21).
 * 「那个小字就不要了」 — the same person, 2026-08-28, withdrawing the
 * condition they set. It is their credential and their call; what is
 * not theirs to lose is the record, so the reasoning stays written here
 * and in the catalog's tombstone rather than leaving with the line.
 *
 * The age went with it: on a card that already prints how old its
 * readings are, a second age under a second set of rings was two
 * clocks on one small card. What it cost is real and worth knowing —
 * a quota is cached for five minutes, so this can be up to that old and
 * nothing on the glass says so any more.
 */
/**
 * **Side by side, packed rather than spread.** The stacked version this
 * replaces was measured and its measurement was real: inside a 448-wide
 * column the two came out 74 and 64 with 12 between them, and their
 * reset lines — same size, same colour, same baseline, 14 apart — read
 * as one sentence,「4 小时后重置 2 天后重置」. What fixed that is each
 * window taking the width its own words need, not a track list
 * stretching them to the pane's edges: a grid of `1fr` columns put
 * these two a third of a metre apart on the machine card while the
 * readings right above them sat packed to the left, and two rows of
 * rings arranged by different rules read as two unrelated things.
 *
 * **And the old objection does not come back at this gap, which was
 * measured rather than assumed**: the two reset lines end up 100 apart,
 * because what sits between them is not the 24 of `--gap-pane` — it is
 * the gap plus the next window's whole ring. A ring between two
 * sentences is a wall the 26 that widening bought never was.
 *
 * Wrapping is the narrow answer, and it arrives by measurement rather
 * than by a breakpoint: past the point where two windows do not fit,
 * the second goes under the first — which is the old layout.
 */
const ROW = "flex flex-wrap gap-pane";

export function QuotaRings({
  quota,
  className,
}: {
  quota: QuotaAnswer | null;
  /** The frame this sits in — the two cards space their sections
      differently, and neither spacing belongs to this file. Safe to
      merge because nothing here sets spacing of its own (the `cn()`
      trap needs two claims on the same axis). */
  className?: string;
}) {
  // Nothing at all until the first answer is in: neither a reading nor a
  // reason has happened yet, and drawing either would describe a state
  // that does not exist.
  if (!quota) return null;
  const trouble = troubleWord(quota);
  if (trouble) {
    return (
      <div data-quota-panel className={cn(ROW, className)}>
        <span data-quota-trouble className="text-aux text-muted-foreground">
          {trouble}
        </span>
      </div>
    );
  }
  const q = quota.quota;
  if (!q || q.windows.length === 0) return null;
  return (
    <div data-quota-panel data-quota className={cn(ROW, className)}>
      {q.windows.map((w) => (
        <div
          key={w.kind}
          data-quota-window={w.kind}
          className="flex items-center gap-row"
        >
          {/* The percentage is what this panel exists to say, so it
              reads at a size of its own — the tier tokens.css describes
              as being for「a quota arc's centre」and which had never
              been used at one. */}
          <Ring fraction={w.used_pct / 100} color={gaugeColor(null)} centerClass="text-lg">
            {`${Math.round(w.used_pct)}%`}
          </Ring>
          <span className="flex min-w-0 flex-col">
            {/* 「5 小时窗口」, not 「5 小时」: with the reset line right
                under it there were two durations touching, one the
                length of the window and one what is left of it, and
                nothing on the glass said which was which. */}
            <span className="text-fact">{windowName(w)}</span>
            {/* Only when there is one. A window past its reset has none
                — the moment it names is behind it, and the reading
                beside it has already gone back to zero. */}
            {w.resets_at !== null && (
              <span data-quota-resets className="text-aux text-muted-foreground">
                {gui.quota_resets(until(w.resets_at * 1000))}
              </span>
            )}
          </span>
        </div>
      ))}
    </div>
  );
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
