import { useRef, useState } from "react";

import type { DayRun } from "@/api";
import { StatusWord } from "@/components/StatusWord";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cli, gui } from "@/gen/catalog";
import { stretch, word } from "@/words";

/**
 * One session's last 24 hours, as a band across the top of its pane.
 *
 * **Called a day strip and not a strip**, because `SessionsList` already
 * uses that word for a row — "the row is a strip, not a button itself".
 * A second meaning for it in the same pane is how the next person greps
 * for one and reads the other.
 *
 * # Why it spans the pane
 *
 * A strip is a time axis, and axes only compare if they line up. Every
 * session's band is drawn across the same pane at the same inset, so
 * reading one session's day after another's — *was this one blocked
 * while that one was working?* — compares like with like.
 *
 * # One axis, three tiers — and the third is the absence of ink
 *
 * The six words are painted in their own tokens and nothing else is
 * (docs/UX.md 状态呈现: 六词之外无第七词). The two nothings
 * (`khor_core::DayCell`) have to be told apart from the words *and from
 * each other* without inventing a seventh hue:
 *
 * - a word — a pill the full height of the band, in its own colour.
 * - 不知道 (`unwatched`, nobody was sampling) — a pill of the same
 *   height and shape, in a quiet tint of the page's own ink. No hue,
 *   because it is not a state.
 * - 那时没有这一行 (`absent`, khor was sampling and this session was not
 *   there) — **no element at all**, the row's own ground.
 *
 * # Why this is not the thing this comment used to forbid
 *
 * It used to say "not one tint at three opacities" and drew 不知道 as a
 * **hairline** to make sure. That defended the right thing — the two
 * nothings must not differ only by how light they are — and it is still
 * defended here, because **one of them has no ink at all**. What the
 * hairline bought on top of that was not distinction, it was a second
 * kind of object: a user reading a real strip said 「左边有一条细线,
 * 右边才是 bar」 — a line, and a bar. Two nouns for one time axis, and
 * the axis is the thing the strip exists to be.
 *
 * So the geometry went back to one, and the separation moved into
 * weight, where there is room for it: 不知道 is ×2.9 lighter than the
 * faintest word (`--day-unwatched` carries the measurements). The pair
 * that has to stay apart is now 不知道 and **空闲** — the one word left
 * without a hue — and the gate measures exactly that gap rather than
 * trusting this paragraph.
 *
 * # Round, like everything else on the board (2026-09-29)
 *
 * 「session的状态条也弄成圆角的bar，而非现在的直角，符合app的一致性」.
 * The band was the one square thing left on a board where nothing is
 * square but a hairline (README 形状). Each stretch is now a pill —
 * `radius-full` on the 4 px band — and neighbours stand **2 px apart**,
 * the gap carved out of each stretch's own share of the axis (a pixel
 * off either end) so every pill still starts at its own time.
 *
 * **A stretch never gets thinner than a dot, and that is not decided
 * here.** A pill 4 tall and under 4 wide is a sliver the rounding eats;
 * floored at a dot instead, it overlaps its neighbours. So the strip
 * asks the node for cells at least 6 px wide — a dot and its gap
 * (`use-day-strips` `CELL_WIDTH`) — and the node's own merge
 * (`khor_core::DayCell::rank`, biased toward 待批) decides what a
 * coarser cell shows. Merging short stretches here would be a second copy
 * of that judgment, which the last section of this comment refuses.
 *
 * The band stands `gap-in` off the pane's left and right edges and
 * `gap-xxs` below its top, clear of the board's rounded top corners:
 * pressed against a 12 px corner, the first and last pills were cut on a
 * curve. (`use-day-strips` counts cells for that inset axis, not for the
 * pane.)
 *
 * # Which end is now, said without a word of explanation
 *
 * The strip runs oldest to newest, and nothing on it says so. What says
 * it is free: **the rightmost stretch is painted in the same colour as
 * the state word the session wears**, so the live end of the band and
 * the session's current word are the same colour at the same moment, and
 * the eye joins them without being told.
 *
 * That cue was there all along and was invisible for a while, because
 * the 不知道 ink was invisible: a young khor has recorded only a few
 * hours, most of the band was 不知道, and a band that is blank on the
 * left and coloured on the right reads as time running right-to-left —
 * which is what a user concluded. **The blank was doing the arguing, not
 * the axis.** So there is deliberately no legend here (docs/UX.md
 * 装饰性说明归零); if the question comes back now that the line is
 * visible, that judgment was wrong and what it earns is a one-time hint,
 * never a permanent key.
 *
 * # What a stretch says: pointed at, pressed, tapped, or reached by keys
 *
 * 「鼠标悬停在bar上要显示时间与状态。点击bar要有一个弹窗显示详情，照顾
 * 手机没有悬停」 (2026-09-29). **One stretch, one sentence, three ways
 * to it**, and all three say the same facts:
 *
 * - **Pointed at** — the readout, a Tooltip (`components/Tooltip`):
 *   `10:05–11:55 · 待批 · 1 小时 50 分钟`. Immediate (the provider's
 *   delay is 0; the first spelling of this, 2026-09-01, went through the
 *   OS `title` and its 1.5 s wait read as no hover at all). It hangs
 *   **below** the band, the side a toolbar's tooltip takes, and is held
 *   inside the pane by the pane itself as its collision boundary.
 * - **Pressed or tapped** — a popover (Radix, `ui/popover`) anchored to
 *   the stretch, on the one floating language: the word with its dot,
 *   the two clocks, the duration, and for 不知道 the sentence that says
 *   what it is. A phone has no hover, and this is its way in. A press
 *   outside, Esc, or the same stretch again closes it. One float at a
 *   time: the readout stands down while a popover is up.
 * - **Keys** — the band is one Tab stop (`role="toolbar"`), ← → Home End
 *   walk the stretches with the `ring` round the focused pill, and the
 *   readout follows focus. Enter or Space opens the popover; Esc closes
 *   it and focus lands back on the stretch.
 *
 * **The span is the stretch's, not the minute under the pointer.** The
 * readout of 2026-09-02 printed the clock at the cursor, and it was the
 * right answer to a delay, not to the question: the state is the same
 * all along a stretch, so a moving clock adds a *position* and no fact —
 * it is a time axis, which this band deliberately does not draw (README
 * 不加刻度、不加时间轴) — and a finger could never get it, while the
 * popover exists precisely so a finger gets what a pointer gets.
 *
 * **No 24-hour totals in the popover.** They would be sums of cells, and
 * a cell shows the word that *most wanted you* at some point inside it,
 * not the one that held it longest — so 「待批 2 小时」 summed here could
 * be twelve cells with a minute of 待批 each. An honest total has to be
 * counted in the node, at the record's minute resolution, and nobody has
 * asked for it yet.
 *
 * **The clock is read at render, not at fetch.** The strip's right edge
 * is "now" as of the last poll, and the poll is a few seconds — a reading
 * inside one poll is inside one minute, and the readout prints minutes.
 *
 * The facts are chrome, not text to take away (nothing here is a value
 * anybody pastes), so none of it is `data-selectable`; and the pointer
 * over a stretch is the arrow every control wears (30-behavior 选字与手).
 *
 * # Its hit area is its own row, and covers nothing else
 *
 * The band is 4 px; a pointer, let alone a finger, needs more. Each
 * stretch's hit box reaches `gap-xxs` above and below the band — exactly
 * the row's own padding, so the row the band sits in (12 px, the band in
 * the middle) is the whole target and **nothing outside it** is. It used
 * to reach 12 px *up*, behind the tab strip — where it was harmless only
 * by accident: the board is `overflow: hidden`, which clips hit-testing
 * as well as paint, so the reach-up was cut off at the board's edge and
 * the band's real target was its 4 px. Now that a press opens something,
 * the target is laid out rather than left to a clip.
 *
 * # Hidden until asked for (2026-09-29)
 *
 * 「把这个状态bar设置为，点击标签后显示吧。悬停的时候加一个提示，点击查看
 * 近24小时状态」. The band is not drawn until the tab in front is pressed —
 * pressed again it goes — and the choice is one switch for this face,
 * remembered (`use-day-shown`). The narrow face has no tabs, and its
 * detail screen's head stands in for the front tab (`DetailPane`).
 *
 * That also ended the blank band the owner reported the same morning:
 * this component used to hold its 12 px row open whenever it had no runs
 * — while the day loaded, and **for good** on a session with no record —
 * so that the session under it would not jump when the day arrived. Two
 * things replace that one empty row, and neither is empty:
 *
 * - **on its way** (`null`): a skeleton in the band's own geometry — one
 *   pill the width of the axis on `fill`, the app's skeleton language
 *   (`RegionState`) — so the band lands where its skeleton stood and
 *   the row is never blank;
 * - **no record** (not one stretch wears a word — `dayHasRecord`): no row
 *   at all, and the front tab's hover says so (「近 24 小时没有记录」)
 *   instead of offering to open it. A band of 不知道 and nothing is not a
 *   day to look at: `day-unwatched-why` itself says 「khor 没有记录」 of
 *   every minute of it.
 *
 * # It floats over the session (2026-09-29)
 *
 * 「下方多了一个空间…挤占了session的内容呀。悬浮内容悬浮在session内容上
 * 即可」. Once the day became something a press brings down, the row it
 * stood in read as room made for a float — and it was room taken from the
 * session: 12 px of push each time, and under a terminal a resize, which
 * tmux answers by redrawing the screen. So the row now hangs over the top
 * of the session from a seat of no height (`DetailPane`), fading out of
 * the surface beneath it, and passes every press but a stretch's through
 * to the session. The row's geometry is unchanged — the band, its
 * padding, its hit boxes — it just no longer costs the session anything.
 *
 * # It paints, it does not decide
 *
 * The runs arrive merged to the cell count this pane asked for, and
 * which cell won a merge was `khor_core::DayCell::rank`'s answer. There
 * is no ordering, no bucketing and no fallback word in this file — a
 * second copy of that judgment here is how a strip and the row above it
 * would come to disagree about the same session.
 */

/**
 * One session's day as a pane holds it: `null` while the first answer is
 * on its way, then its runs — an empty list for a row that answer did not
 * include (a poll a tick behind the list, or a row the node has no day
 * for; either way there is nothing to draw yet).
 */
export type Day = DayRun[] | null;

/** Whether a day has anything for the band to say: at least one stretch
    wearing a word. A day of 不知道 and 那时没有这一行 alone is 「没有记录」
    (the strip's own sentence for 不知道), and draws no row. */
export function dayHasRecord(runs: DayRun[]): boolean {
  return runs.some((r) => typeof r.mark === "object");
}

/** The row the band stands in — what the front tab's `aria-controls`
    names. One at a time: only the open session's pane draws a day. */
export const DAY_ROW_ID = "khor-session-day";

/**
 * What the switch for the day says and does — the front tab's on the
 * wide face, the head's on the narrow one; one answer, so the two cannot
 * come to disagree about a session.
 *
 * `hint` is the tab's hover. `expanded` and `toggle` are absent for a day
 * with no record: a press there opens nothing, so the control is not a
 * switch and does not say it is one. While the day is on its way it is
 * still a switch — what arrives might be a record, and pressing shows the
 * skeleton until it does.
 */
export function daySwitch(
  day: Day,
  shown: boolean,
  toggle: () => void,
): { hint: string; expanded?: boolean; toggle?: () => void } {
  if (day !== null && !dayHasRecord(day)) return { hint: gui.day_none };
  return { hint: shown ? gui.day_hide : gui.day_show, expanded: shown, toggle };
}

/** The strip's whole span: the node's `Day` is a rolling day
    (`khor_node::day`), and every run is a share of it. */
const DAY_MS = 24 * 60 * 60_000;

/** A clock reading, `HH:MM`. */
export function clock(ms: number): string {
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/** One run placed on the day: where it starts and ends, as of `now`. */
export type Stretch = {
  i: number;
  run: DayRun;
  /** The word it says; `null` for 那时没有这一行, which says nothing. */
  label: string | null;
  from: number;
  to: number;
  width: string;
};

export function labelOf(mark: DayRun["mark"]): string | null {
  if (mark === "absent") return null;
  return mark === "unwatched" ? gui.day_unwatched : word(mark.word);
}

/** The runs as stretches of the day ending `now`, oldest first. */
export function lay(runs: DayRun[], total: number, now: number): Stretch[] {
  let before = 0;
  return runs.map((run, i) => {
    const from = now - (1 - before / total) * DAY_MS;
    before += run.cells;
    const to = now - (1 - before / total) * DAY_MS;
    return { i, run, label: labelOf(run.mark), from, to, width: `${(run.cells / total) * 100}%` };
  });
}

/** What a stretch says in one line — the readout, and the name a screen
    reader gives the stretch. The ` · ` and the `–` are punctuation; every
    word either side is the catalog's. */
export function sentence(s: Stretch): string {
  return `${clock(s.from)}–${clock(s.to)} · ${s.label} · ${stretch(s.to - s.from)}`;
}

/** How far a float keeps off the pane's edges: the inset the band
    itself keeps (`gap-in`), read from the token. */
function inset(): number {
  return parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--gap-in")) || 0;
}

export function DayStrip({ runs }: { runs: Day }) {
  const box = useRef<HTMLDivElement>(null);
  /** The stretch whose readout is up. */
  const [peek, setPeek] = useState<number | null>(null);
  /** **A moment**, not an index: the popover belongs to the stretch that
      holds it. Runs have no identity (below), and an index would move to
      the next stretch when the oldest one slides off the day under an
      open popover; the moment stays inside the stretch it was taken
      from, and falls off the day with it. */
  const [openAt, setOpenAt] = useState<number | null>(null);
  /** The stretch that holds the band's one Tab stop. */
  const [stop, setStop] = useState<number | null>(null);
  if (runs === null) {
    // On its way: one pill the width of the axis, on `fill` (the head's
    // 「Hidden until asked for」) — the band's row, padding and height, so
    // the band lands where this stood. Not a band: nothing here says
    // `data-day-strip`, which is how a gate tells the two apart.
    return (
      <div data-day-skeleton aria-busy="true" aria-label={cli.day_title} className="flex px-in py-xxs">
        <span data-skeleton className="h-1 w-full rounded-full bg-fill" />
      </div>
    );
  }
  // No record draws nothing: the pane does not ask for this row then
  // (`DetailPane`), and if it ever did, an empty row is the blank band.
  if (!dayHasRecord(runs)) return null;
  const total = runs.reduce((n, r) => n + r.cells, 0);
  const laid = lay(runs, total, Date.now());
  const inked = laid.filter((s) => s.label !== null);
  const tabStop = inked.some((s) => s.i === stop) ? stop : (inked[inked.length - 1]?.i ?? null);
  const bound = box.current?.closest("[data-detail-pane]") ?? null;
  const pad = inset();
  const holds = (s: Stretch, at: number | null) => at !== null && at >= s.from && at < s.to;

  // ← → Home End, from a stretch only: the popover is portaled but React
  // still bubbles its keys through here, and they are not the band's.
  const walk = (e: React.KeyboardEvent) => {
    const at = inked.findIndex((s) => String(s.i) === (e.target as HTMLElement).dataset?.dayRun);
    if (at < 0) return;
    const to =
      e.key === "ArrowLeft" ? at - 1 : e.key === "ArrowRight" ? at + 1 : e.key === "Home" ? 0 : e.key === "End" ? inked.length - 1 : null;
    if (to === null) return;
    e.preventDefault();
    const next = inked[Math.max(0, Math.min(inked.length - 1, to))];
    setStop(next.i);
    box.current?.querySelector<HTMLElement>(`[data-day-run="${next.i}"]`)?.focus();
  };

  return (
    <div
      ref={box}
      data-day-strip
      role="toolbar"
      aria-label={cli.day_title}
      aria-orientation="horizontal"
      onKeyDown={walk}
      // The row: the band in the middle, `gap-xxs` over and under it —
      // which is exactly how far each stretch's hit box reaches.
      className="flex px-in py-xxs"
    >
      {laid.map((s) => {
        // Runs have no identity of their own — they are a slice of a
        // timeline, and the slice at position i is the same thing frame
        // to frame even when its width changes.
        if (s.label === null) {
          // Sampled, and this session was not on the list: the band's own
          // ground, and deliberately no ink and no target — "nothing
          // here" is the one of the three that should cost nothing.
          return (
            <span
              key={s.i}
              data-day-mark="absent"
              data-day-cells={s.run.cells}
              aria-hidden
              className="h-1 flex-none"
              style={{ width: s.width }}
            />
          );
        }
        const text = sentence(s);
        return (
          <Tooltip
            key={s.i}
            open={peek === s.i && openAt === null}
            onOpenChange={(o) => setPeek((cur) => (o ? s.i : cur === s.i ? null : cur))}
            // The readout is a name, never a place to go.
            disableHoverableContent
          >
            <Popover
              open={holds(s, openAt)}
              // Closing clears the moment only if it is still this
              // stretch's: on a touch screen the tap that opens the next
              // stretch lands before the last one hears it was left.
              onOpenChange={(o) => setOpenAt((cur) => (o ? (s.from + s.to) / 2 : holds(s, cur) ? null : cur))}
            >
              <TooltipTrigger asChild>
                <PopoverTrigger asChild>
                  <button
                    type="button"
                    data-day-mark={typeof s.run.mark === "string" ? s.run.mark : s.run.mark.word}
                    data-day-run={s.i}
                    // How many cells it is: what lets a gate say a cell
                    // is wide enough for a dot and its gap.
                    data-day-cells={s.run.cells}
                    tabIndex={s.i === tabStop ? 0 : -1}
                    aria-label={text}
                    onFocus={() => setStop(s.i)}
                    // The slot is this stretch's share of the axis; a pixel
                    // of it at each end is the gap. `before:` is the hit
                    // box, the row's padding over and under the band.
                    // `pointer-events-auto` because the row it floats in
                    // passes every other press to the session beneath
                    // (`DetailPane`).
                    className="group pointer-events-auto relative flex h-1 flex-none justify-center px-px outline-none before:absolute before:inset-x-0 before:-inset-y-1 before:content-['']"
                    style={{ width: s.width }}
                  >
                    <Ink mark={s.run.mark} />
                  </button>
                </PopoverTrigger>
              </TooltipTrigger>
              <TooltipContent data-day-peek side="bottom" collisionBoundary={bound} collisionPadding={pad} className="tabular-nums">
                {text}
              </TooltipContent>
              <PopoverContent
                data-day-detail
                side="bottom"
                collisionBoundary={bound}
                collisionPadding={pad}
                aria-label={s.label}
              >
                <Detail s={s} />
              </PopoverContent>
            </Popover>
          </Tooltip>
        );
      })}
    </div>
  );
}

/**
 * The ink of one stretch: a pill, in its word's token or 不知道's tint.
 *
 * The union is discriminated in the type (`DayMark` is
 * `{ word } | "unwatched" | "absent"`), so a word can only be reached
 * through the branch that has one — which is the point of it having that
 * shape. Spelled as one flat string, `var(--state-unwatched)` would
 * resolve to nothing and paint transparent, folding 不知道 into
 * 那时没有这一行 in CSS after the node worked to keep them apart.
 *
 * Read through an inline `var()` rather than a Tailwind colour utility:
 * that would need a second entry in `@theme inline` naming the same fact.
 * `min-w-1` is a floor for the one case the cell width cannot promise —
 * a pane not measured yet — and never draws in a measured one.
 */
function Ink({ mark }: { mark: DayRun["mark"] }) {
  // 那时没有这一行 is drawn by the band's caller as an empty slot and
  // never reaches here; if it did, it would still cost no ink.
  if (mark === "absent") return null;
  return (
    <span
      className="h-full w-full min-w-1 flex-none rounded-full group-focus-visible:outline-2 group-focus-visible:outline-offset-1 group-focus-visible:outline-ring"
      style={{ backgroundColor: mark === "unwatched" ? "var(--day-unwatched)" : `var(--state-${mark.word})` }}
    />
  );
}

/**
 * The popover's facts, in reading order: what it was (the word and its
 * dot, `StatusWord`'s one look — or 不知道's own tint and word, which is
 * not a state and does not wear one), when, and for how long. 不知道
 * adds the one sentence that says what it means.
 */
function Detail({ s }: { s: Stretch }) {
  const mark = s.run.mark;
  return (
    <div className="flex flex-col gap-xxs">
      {mark === "unwatched" || mark === "absent" ? (
        <span className="inline-flex items-center gap-xxs text-caption text-muted-foreground">
          <span aria-hidden className="size-1.5 flex-none rounded-full" style={{ backgroundColor: "var(--day-unwatched)" }} />
          {gui.day_unwatched}
        </span>
      ) : (
        <StatusWord word={mark.word} />
      )}
      <span className="text-body tabular-nums">
        {clock(s.from)}–{clock(s.to)}
      </span>
      <span className="text-callout text-muted-foreground tabular-nums">{stretch(s.to - s.from)}</span>
      {mark === "unwatched" && <span className="text-callout text-muted-foreground">{gui.day_unwatched_why}</span>}
    </div>
  );
}
