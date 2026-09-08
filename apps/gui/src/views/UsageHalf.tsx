/**
 * **The lower half of an account's detail: what it spent in tokens.**
 *
 * The upper half is the dial and the bars — money, or a share of a
 * window. This half is the same account cut a different way, and it is
 * split into two zones for the same reason `MachineCard`'s `Busy` is:
 * **the trend zone answers 「when」 and the stats zone answers 「who and
 * how much」.** Two questions, two pictures, and neither one is asked
 * to do the other's job.
 *
 * # Where the numbers come from, and the one comparison nobody may make
 *
 * Two roads, and which one an account is on is not a detail:
 *
 *   - a **subscription** is read off this machine's own transcripts
 *     (`Usage::models`, `khor_node::usage`) — what the agents on this
 *     box wrote down;
 *   - an **API account** is read from the provider
 *     (`AccountReading::model_tokens`) — what they billed.
 *
 * `khor_core::ModelTokenDay`'s head says it outright and it is worth
 * repeating where the drawing happens: **the two are supposed to
 * disagree**, and a check asserting they match would be a check written
 * to go red. A provider bills per request through one key over its own
 * window; a transcript records what an agent wrote on this machine — a
 * different population on a different clock.
 *
 * ⚠️ **The provider road has never been walked with a real admin key.**
 * The shapes it decodes come from documentation, so everything drawn
 * from `model_tokens` on this page is untested against a live answer.
 *
 * # Three faces, and the empty one is never drawn
 *
 * `Busy`'s rule, and it lands harder here. An account that cannot be
 * read draws **no chart** — because an empty chart is not 「no reading」,
 * it is the picture of **an account that spent nothing**, and that is a
 * fact nobody established. There are three states and the sentences say
 * which:
 *
 *   - **still asking** — nothing at all. The first pass over a machine's
 *     transcripts is 18.5 s cold (`khor_node::usage::Meters::tally`),
 *     so this is a state a person really sits in, not a theoretical one.
 *   - **cannot tell** — a sentence in the chart's seat. Three different
 *     ones, and they are not the same fact: this provider publishes no
 *     token report at all (the ordinary answer — the wallets publish a
 *     balance and nothing else); the answering khor predates the
 *     per-model cut (`Usage::models` empty beside a non-empty `days`);
 *     or nothing was spent in this window.
 *   - **have it** — the two zones.
 */
import { useMemo, useState } from "react";

import type { AccountReading } from "@/gen/bindings/AccountReading";
import type { Usage } from "@/gen/bindings/Usage";
import { cli, gui } from "@/gen/catalog";
import { bandsFor } from "@/lib/model-band";
import { ruler, tickSet, type Ruler } from "@/lib/ruler";
import { cn } from "@/lib/utils";

/** One (day, model) reading, in whichever road's spelling. */
export type UsageRow = {
  /** Civil date `YYYY-MM-DD`, **in the answering machine's zone** — see
      `Usage::today` for why this face must not cut days its own way. */
  day: string;
  /** `null` is 「the source did not record which model」, which is a row
      and not a silence (`UsageModelDay::model`). */
  model: string | null;
  input: number;
  output: number;
  cached_input: number;
  cache_write: number;
};

/**
 * **The four quantities either scatter axis can be, and how each is
 * read off a row and written on a tick.**
 *
 * The same set the table has columns for — `MachineCard`'s `AXES` says
 * why, and the reason carries over word for word: those are the
 * quantities a person is already looking at when they decide they want
 * to see two of them against each other.
 *
 * A tick's precision is decided beside its neighbours by `tickSet`, not
 * here: `tick(v, d)` writes the reading with `d` decimals and `tickSet`
 * picks the fewest that keep every label on the axis apart **and keep a
 * reading above zero from printing as zero**.
 */
const AXES = {
  total: { word: () => gui.usage_axis_total, of: (r: Agg) => r.input + r.output, tick: count },
  input: { word: () => gui.usage_axis_input, of: (r: Agg) => r.input, tick: count },
  output: { word: () => gui.usage_axis_output, of: (r: Agg) => r.output, tick: count },
  read: { word: () => gui.usage_axis_read, of: (r: Agg) => r.cached_input, tick: count, cache: true },
  write: { word: () => gui.usage_axis_write, of: (r: Agg) => r.cache_write, tick: count, cache: true },
  rate: { word: () => gui.usage_axis_rate, of: (r: Agg) => r.rate, tick: pct, cache: true },
  reuse: { word: () => gui.usage_axis_reuse, of: (r: Agg) => r.reuse, tick: (v: number, d: number) => `${v.toFixed(d)}×`, cache: true },
  days: { word: () => gui.usage_axis_days, of: (r: Agg) => r.days, tick: (v: number) => String(Math.round(v)) },
} as const;
type AxisKey = keyof typeof AXES;
const AXIS_KEYS = Object.keys(AXES) as AxisKey[];

/**
 * A count in the units a person reads them in.
 *
 * **Base 1000, not 1024** — these are tokens, and a thousand of them is
 * a thousand. `bytes` next door divides by 1024 because it measures
 * memory; sharing that function would have made 「1.0M tokens」 mean
 * 1,048,576 of them, which is a number nobody wrote down.
 *
 * `d` is the decimals, so `tickSet` can ask for more where an axis's
 * labels would otherwise collide.
 */
function count(n: number, d = 1): string {
  const units = ["", "k", "M", "G", "T"];
  let v = n;
  let u = 0;
  while (Math.abs(v) >= 1000 && u + 1 < units.length) {
    v /= 1000;
    u += 1;
  }
  return u === 0 ? `${Math.round(n)}` : `${v.toFixed(d)}${units[u]}`;
}
function pct(v: number, d = 1): string {
  return `${(100 * v).toFixed(d)}%`;
}

/** One model over the window — what the table lists and the scatter draws. */
type Agg = {
  model: string | null;
  /** The name as drawn: the vendor prefix is dropped because every row
      on one account carries the same one, and a column of identical
      prefixes is a column of nothing. */
  word: string;
  input: number;
  output: number;
  cached_input: number;
  cache_write: number;
  /** Cache reads as a share of everything that went in. `null` where the
      source does not report the cache — **never 0, which would be the
      claim that nothing was cached** (`ModelTokenDay::cache_split`). */
  rate: number;
  reuse: number;
  /** Days this model actually ran, not days in the window. */
  days: number;
  from: string;
  to: string;
};

/** How many days back each window reaches; 0 is 「all of them」. */
const WINDOWS = [7, 30, 0] as const;

/**
 * **The days the window covers — calendar days, not 「days that have a
 * row」.**
 *
 * A day nobody spent anything on has no row at all (`UsageDay`'s head
 * says so), so slicing the *rows* would make 「7 天」 mean 「the last
 * seven days that had spend」 — which reads as seven consecutive days
 * and can silently span a month. The axis is therefore built from the
 * calendar and the gaps are drawn as the empty days they are.
 *
 * The end of the axis is the answering machine's own today where it
 * said one (`Usage::today`), falling back to the newest day present.
 * **Not this browser's today**: the rows were cut in the other
 * machine's zone, and a face that used its own would be right about the
 * box it runs on and off by a day about every other one.
 *
 * ⚠️ **The start is clamped to the first day the source knows about,
 * and that is not the same rule as the gaps.** Inside the record, a day
 * with no row is a day nothing was spent — that is what `UsageDay`
 * promises, and it is drawn as the empty column it is. **Before the
 * record there is no such promise**: a transcript directory only
 * reaches as far back as the files on disk, so 30 天 over ten days of
 * transcripts would draw twenty columns saying 「nothing was spent」
 * about days nobody has any reading for. So a window is 「at most this
 * many days back」 and the axis never starts before the data does.
 */
function dayAxis(rows: UsageRow[], today: string | null, back: number): string[] {
  const seen = [...new Set(rows.map((r) => r.day))].sort();
  if (seen.length === 0) return [];
  const last = today && today > seen[seen.length - 1] ? today : seen[seen.length - 1];
  const wanted = back === 0 ? seen[0] : step(last, -(back - 1));
  const out: string[] = [];
  for (let d = wanted > seen[0] ? wanted : seen[0]; d <= last; d = step(d, 1)) out.push(d);
  return out;
}
/** `YYYY-MM-DD` plus `n` days, through UTC so the arithmetic cannot
    inherit this browser's zone — these strings are another machine's
    civil dates and are being counted, not converted. */
function step(day: string, n: number): string {
  const t = Date.parse(`${day}T00:00:00Z`);
  return new Date(t + n * 86_400_000).toISOString().slice(0, 10);
}

/**
 * Every model's totals over `days`.
 *
 * **The window decides which days are summed and nothing else.** The
 * per-(day, model) readings are the ones that came off the wire, so two
 * windows that overlap report the same numbers for the days they share
 * — 「switch the range」 is a question about days, never a recalculation.
 */
function aggregate(rows: UsageRow[], days: string[], cacheKnown: boolean): Agg[] {
  const inWindow = new Set(days);
  const by = new Map<string | null, Agg>();
  const ran = new Map<string | null, string[]>();
  for (const r of rows) {
    if (!inWindow.has(r.day)) continue;
    const got =
      by.get(r.model) ??
      {
        model: r.model,
        word: r.model === null ? cli.usage_model_unnamed : r.model.replace(/^claude-/, ""),
        input: 0,
        output: 0,
        cached_input: 0,
        cache_write: 0,
        rate: 0,
        reuse: 0,
        days: 0,
        from: r.day,
        to: r.day,
      };
    got.input += r.input;
    got.output += r.output;
    got.cached_input += r.cached_input;
    got.cache_write += r.cache_write;
    by.set(r.model, got);
    // **A day counts as 「ran」 when something came out of it**, not when
    // a row exists: a row of four zeroes is a day the source mentioned,
    // not a day the model worked.
    if (r.output > 0 || r.cached_input > 0 || r.input > 0) {
      const list = ran.get(r.model) ?? [];
      if (!list.includes(r.day)) list.push(r.day);
      ran.set(r.model, list);
    }
  }
  const out: Agg[] = [];
  for (const [model, a] of by) {
    const live = (ran.get(model) ?? []).sort();
    if (live.length === 0) continue;
    const went = a.input + a.cached_input + a.cache_write;
    out.push({
      ...a,
      // `NaN` rather than 0 where the source does not report the cache:
      // it fails every comparison, so a row that reaches for it gets
      // 「—」 wherever it is drawn instead of a zero that reads as a
      // measurement. See `cacheKnown`.
      rate: cacheKnown && went > 0 ? a.cached_input / went : NaN,
      reuse: cacheKnown && a.cache_write > 0 ? a.cached_input / a.cache_write : NaN,
      days: live.length,
      from: live[0],
      to: live[live.length - 1],
    });
  }
  return out.sort((x, y) => y.input + y.output - (x.input + x.output));
}

/**
 * Where the ticks go on an axis.
 *
 * **A flat axis gets one**, not five. Every reading on it is the same
 * number, so `at` answers the middle for all of them — five ticks would
 * be five identical labels drawn over each other on one line, which
 * reads as a rendering fault rather than as the one thing that axis has
 * to say. `tickSet`'s de-duplication is not the tool for this: it folds
 * only where a formatter's unit ladder makes two readings share a word,
 * and on an axis with decimals two ticks under one label is a broken
 * formatter it is meant to expose rather than hide.
 */
const steps = (r: Ruler): number[] => (r.kind === "flat" ? [0.5] : [0, 0.25, 0.5, 0.75, 1]);

const RULER_WORD: Record<Ruler["kind"], () => string> = {
  linear: () => gui.usage_ruler_linear,
  sqrt: () => gui.usage_ruler_sqrt,
  log: () => gui.usage_ruler_log,
  flat: () => gui.usage_ruler_flat,
};

/**
 * The half.
 *
 * `rows === null` is 「still asking」 and draws nothing at all; `why`
 * draws that sentence in the chart's seat. See this file's head for why
 * neither of those may become an empty chart.
 */
export function UsageHalf({
  rows,
  why,
  today,
  cacheKnown,
  source,
}: {
  rows: UsageRow[] | null;
  why: string | null;
  today: string | null;
  /** Whether `cached_input` and `cache_write` are figures the source
      **stated**. `false` is not 「zero」 (`ModelTokenDay::cache_split`),
      so the four quantities that rest on them leave the page rather
      than being drawn as nothing. */
  cacheKnown: boolean;
  source: "transcript" | "vendor";
}) {
  const [win, setWin] = useState<number>(0);
  const [view, setView] = useState<"day" | "model">("day");
  const [hot, setHot] = useState<string | null>(null);
  const [sort, setSort] = useState<{ key: string; dir: 1 | -1 }>({ key: "total", dir: -1 });
  const [axes, setAxes] = useState<{ x: AxisKey; y: AxisKey; z: AxisKey }>({
    x: "write",
    y: "read",
    z: "total",
  });
  const [force, setForce] = useState("");

  // **Colours are worked out over every model the account has, never
  // over the window's** — see `lib/model-band`. Fed the window's, a
  // model changes colour when somebody presses 7 天.
  const bands = useMemo(() => bandsFor([...new Set((rows ?? []).map((r) => r.model))]), [rows]);
  const days = useMemo(() => dayAxis(rows ?? [], today, win), [rows, today, win]);
  const agg = useMemo(() => aggregate(rows ?? [], days, cacheKnown), [rows, days, cacheKnown]);

  if (why) {
    return (
      <div data-usage-half data-usage-mute className="border-t px-4 py-3 text-sm text-muted-foreground">
        {why}
      </div>
    );
  }
  // Still asking. **Nothing**, not a frame with no bars in it.
  if (rows === null) return null;
  // A caller that says 「have it」 with nothing in it is a caller bug,
  // and the answer to a bug is still not an empty chart.
  if (rows.length === 0 || agg.length === 0) return null;

  const shown = AXIS_KEYS.filter((k) => cacheKnown || !("cache" in AXES[k]));

  return (
    <div data-usage-half data-usage-source={source} className="flex flex-col border-t">
      <Trend
        days={days}
        agg={agg}
        rows={rows}
        bands={bands}
        view={view}
        win={win}
        force={force}
        onView={setView}
        onWin={setWin}
      />
      <Stats
        agg={agg}
        days={days}
        bands={bands}
        axes={axes}
        shown={shown}
        cacheKnown={cacheKnown}
        force={force}
        hot={hot}
        sort={sort}
        onAxes={setAxes}
        onForce={setForce}
        onHot={setHot}
        onSort={setSort}
      />
    </div>
  );
}

const W = 880;
const TREND_H = 200;
/** The machine day strip draws its hairlines at 0.6 in ink; a coloured
    line needs about 1.8× that to read as the same weight
    (`day/day-core.js`). */
const HAIR = 1.1;

/**
 * **趋势 — one stacked column per day.** The column's height is that
 * day's total and the bands inside it are who spent it, so both
 * questions are answered off one shape with nothing to add up by eye.
 *
 * **Columns rather than an area or a line.** A day's usage is a
 * discrete event, and both of those draw a continuous curve between two
 * days that never happened. The area form also loses the total's own
 * shape into its top edge; the line form asks the reader to sum five
 * coloured lines by eye.
 *
 * ⚠️ **This axis is linear and may never be bent.** The bands are
 * summed to make the column, so a square root would draw segments that
 * do not add up to the shape they are inside — it would break the one
 * thing a stack promises. That is why `ruler` is not called here and
 * why the median rule below exists instead: without it 「an ordinary
 * day」 has no position on a chart whose height belongs to the peak.
 */
function Trend({
  days,
  agg,
  rows,
  bands,
  view,
  win,
  force,
  onView,
  onWin,
}: {
  days: string[];
  agg: Agg[];
  rows: UsageRow[];
  bands: Map<string | null, string>;
  view: "day" | "model";
  win: number;
  force: string;
  onView: (v: "day" | "model") => void;
  onWin: (w: number) => void;
}) {
  // **Smallest at the bottom.** The order has to be the same on every
  // column or a model jumps layers and two columns stop comparing, so
  // one rule has to be picked — and the small bands are the ones that
  // need an exact read, since a big one can be estimated off the
  // column's height. Sitting on the zero line gives them a fixed
  // baseline. The legend still runs largest first, like the table.
  const stack = [...agg].reverse();
  const cell = useMemo(() => {
    const m = new Map<string, number>();
    for (const r of rows) m.set(`${r.day} ${r.model}`, (m.get(`${r.day} ${r.model}`) ?? 0) + r.input + r.output);
    return m;
  }, [rows]);
  const at = (day: string, model: string | null) => cell.get(`${day} ${model}`) ?? 0;
  const totals = days.map((d) => stack.reduce((a, b) => a + at(d, b.model), 0));
  const peak = Math.max(...totals, 1);
  const live = totals.filter((v) => v > 0).sort((a, b) => a - b);
  const median = live.length > 2 ? live[live.length >> 1] : null;

  const pad = { l: 46, r: 12, t: 12, b: 20 };
  const iw = W - pad.l - pad.r;
  const ih = TREND_H - pad.t - pad.b;
  const y = (v: number) => pad.t + ih - (ih * v) / peak;
  const stepW = iw / Math.max(days.length, 1);
  const bw = Math.min(34, Math.max(2, stepW * 0.74));
  const cx = (i: number) => pad.l + stepW * (i + 0.5);
  const busiest = totals.reduce((a, v, i) => (v > totals[a] ? i : a), 0);

  const modelRuler = ruler(
    agg.map((r) => r.input + r.output),
    force,
  );

  return (
    <section data-usage-zone="trend" className="flex flex-col gap-in px-4 py-3">
      <header className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-2">
        <h3 className="text-aux font-title text-muted-foreground">{gui.usage_zone_trend}</h3>
        <div className="flex flex-wrap gap-1">
          {(["day", "model"] as const).map((v) => (
            <Chip key={v} on={view === v} onClick={() => onView(v)} name={`usage-view-${v}`}>
              {v === "day" ? gui.usage_view_day : gui.usage_view_model}
            </Chip>
          ))}
          <span className="w-2" />
          {WINDOWS.map((w) => (
            <Chip key={w} on={win === w} onClick={() => onWin(w)} name={`usage-window-${w}`}>
              {w === 0 ? gui.usage_window_all : gui.usage_window_days(w)}
            </Chip>
          ))}
        </div>
      </header>
      {/* The legend is the same list the table is, in the same order, so
          a colour looked up in one place means the same in the other. */}
      <ul data-usage-key className="flex flex-wrap gap-x-3 gap-y-0.5 text-aux text-muted-foreground">
        {agg.map((r) => (
          <li key={String(r.model)} data-usage-key-model={r.model ?? ""} className="flex items-center gap-1.5">
            <i className="size-2 rounded-[2px]" style={{ background: bands.get(r.model) }} />
            {r.word}
          </li>
        ))}
      </ul>
      {view === "day" ? (
        <svg
          data-usage-chart="day"
          // **Printed, and it is not decoration**: the bands add up to
          // the column, so this is the one axis on the page that is not
          // allowed to be anything else, and a gate reads it here.
          data-usage-scale="linear"
          data-usage-peak={peak}
          viewBox={`0 0 ${W} ${TREND_H}`}
          className="h-auto w-full overflow-visible text-foreground"
        >
          {[0, 1, 2, 3].map((k) => (
            <g key={k}>
              <line x1={pad.l} x2={W - pad.r} y1={y((peak * k) / 3)} y2={y((peak * k) / 3)} stroke="currentColor" strokeOpacity={0.1} strokeWidth={0.8} />
              <text x={pad.l - 6} y={y((peak * k) / 3) + 3} fontSize={8} textAnchor="end" fill="currentColor" fillOpacity={0.5}>
                {k === 0 ? "0" : count((peak * k) / 3)}
              </text>
            </g>
          ))}
          {days.map((d, i) => {
            let base = y(0);
            return (
              <g key={d} data-usage-day={d} data-usage-day-total={totals[i]}>
                <title>
                  {`${d} · ${count(totals[i])}\n` +
                    stack
                      .filter((b) => at(d, b.model) > 0)
                      .map((b) => `  ${b.word} ${count(at(d, b.model))}`)
                      .join("\n")}
                </title>
                {stack.map((b) => {
                  const v = at(d, b.model);
                  if (v === 0) return null;
                  const h = y(0) - y(v);
                  base -= h;
                  return (
                    <rect
                      key={String(b.model)}
                      data-usage-band={b.model ?? ""}
                      data-usage-band-value={v}
                      x={cx(i) - bw / 2}
                      y={base}
                      width={bw}
                      // **A band never rounds to nothing.** A model that
                      // ran is a model that has to be visible: a segment
                      // drawn 0px tall and a model that never ran are
                      // the same screen.
                      height={Math.max(h, 0.6)}
                      fill={bands.get(b.model)}
                      stroke="var(--card)"
                      strokeWidth={bw > 6 ? 0.8 : 0}
                    />
                  );
                })}
              </g>
            );
          })}
          {days.map((d, i) =>
            i % Math.max(1, Math.ceil(days.length / 6)) === 0 || i === days.length - 1 ? (
              <text key={d} x={cx(i)} y={TREND_H - 6} fontSize={7.5} textAnchor="middle" fill="currentColor" fillOpacity={0.5}>
                {d.slice(5)}
              </text>
            ) : null,
          )}
          {median !== null && (
            <g data-usage-median={median}>
              <line x1={pad.l} x2={W - pad.r} y1={y(median)} y2={y(median)} stroke="currentColor" strokeOpacity={0.45} strokeWidth={0.9} strokeDasharray="4 3" />
              <text x={W - pad.r} y={y(median) - 4} fontSize={8} textAnchor="end" fill="currentColor" fillOpacity={0.6}>
                {gui.usage_median(count(median))}
              </text>
            </g>
          )}
          {/* The peak day, named — the machine card marks its CPU day
              the same way, and for the same reason: the tallest thing
              on a chart is the one reading people want in words. */}
          {totals[busiest] > 0 && (
            <text
              data-usage-peak-day={days[busiest]}
              x={cx(busiest) + (busiest > days.length * 0.75 ? -4 : 4)}
              y={y(totals[busiest]) - 5}
              fontSize={8.5}
              fontWeight={700}
              textAnchor={busiest > days.length * 0.75 ? "end" : "start"}
              fill="currentColor"
            >
              {`${days[busiest].slice(5)} · ${count(totals[busiest])}`}
            </text>
          )}
        </svg>
      ) : (
        <svg
          data-usage-chart="model"
          data-usage-scale={modelRuler.kind}
          viewBox={`0 0 ${W} ${TREND_H}`}
          className="h-auto w-full overflow-visible text-foreground"
        >
          {tickSet(modelRuler, { tick: count }, steps(modelRuler)).map(({ v, label }) => (
            <g key={v}>
              <line x1={pad.l} x2={W - pad.r} y1={pad.t + ih - ih * modelRuler.at(v)} y2={pad.t + ih - ih * modelRuler.at(v)} stroke="currentColor" strokeOpacity={0.1} strokeWidth={0.8} />
              <text x={pad.l - 6} y={pad.t + ih - ih * modelRuler.at(v) + 3} fontSize={8} textAnchor="end" fill="currentColor" fillOpacity={0.5}>
                {label}
              </text>
            </g>
          ))}
          {agg.map((r, k) => {
            const step2 = iw / agg.length;
            const bx = pad.l + step2 * (k + 0.5);
            const v = r.input + r.output;
            const top = pad.t + ih - ih * modelRuler.at(v);
            return (
              <g key={String(r.model)} data-usage-bar={r.model ?? ""} data-usage-bar-value={v}>
                <rect x={bx - Math.min(46, step2 * 0.62) / 2} y={top} width={Math.min(46, step2 * 0.62)} height={Math.max(pad.t + ih - top, 0)} fill={bands.get(r.model)} rx={2} />
                <text x={bx} y={pad.t + ih + 12} fontSize={7.5} textAnchor="middle" fill="currentColor" fillOpacity={0.6}>
                  {r.word.length > 13 ? `${r.word.slice(0, 12)}…` : r.word}
                </text>
                <text x={bx} y={top - 4} fontSize={8} fontWeight={700} textAnchor="middle" fill="currentColor">
                  {count(v)}
                </text>
              </g>
            );
          })}
          {/* **A bent axis that does not say so is a chart that lies
              quietly.** The day view has no such line because it cannot
              bend; this one can, so it says which ruler it is on. */}
          <text x={W - pad.r} y={TREND_H - 2} fontSize={7.5} fontWeight={700} textAnchor="end" fill="currentColor" fillOpacity={0.5}>
            {RULER_WORD[modelRuler.kind]()}
          </text>
        </svg>
      )}
      <p className="text-aux text-muted-foreground">{gui.usage_trend_note}</p>
    </section>
  );
}

function Chip({ on, onClick, name, children }: { on: boolean; onClick: () => void; name: string; children: React.ReactNode }) {
  return (
    <button
      type="button"
      data-usage-chip={name}
      aria-pressed={on}
      onClick={onClick}
      className={cn(
        "min-h-touch rounded-full border px-2.5 text-aux font-title",
        on ? "border-foreground bg-foreground text-background" : "text-muted-foreground",
      )}
    >
      {children}
    </button>
  );
}

const SCAT_H = 280;
const R_MIN = 5;
const R_MAX = 19;
/**
 * **The area carries the third quantity, so the radius is its square
 * root.** A radius read straight off the number makes a 4× value look
 * 16× big. The floor is not zero — the smallest point still has to be
 * pressable — so the two ends are interpolated **in area** and taken
 * back to a radius, rather than interpolating the radius and leaving
 * the areas wrong at both ends.
 */
function radius(v: number, max: number): number {
  const t = max > 0 ? Math.max(v, 0) / max : 0;
  return Math.sqrt(R_MIN * R_MIN + (R_MAX * R_MAX - R_MIN * R_MIN) * t);
}

/**
 * **统计 — who and how much.** A scatter over the same rows the table
 * lists, and pressing either one lights the other: the machine card's
 * pairing, for the machine card's reason — a point that cannot be
 * named is a dot, and a row that cannot be placed is a number.
 */
function Stats({
  agg,
  days,
  bands,
  axes,
  shown,
  cacheKnown,
  force,
  hot,
  sort,
  onAxes,
  onForce,
  onHot,
  onSort,
}: {
  agg: Agg[];
  days: string[];
  bands: Map<string | null, string>;
  axes: { x: AxisKey; y: AxisKey; z: AxisKey };
  shown: AxisKey[];
  cacheKnown: boolean;
  force: string;
  hot: string | null;
  sort: { key: string; dir: 1 | -1 };
  onAxes: (a: { x: AxisKey; y: AxisKey; z: AxisKey }) => void;
  onForce: (f: string) => void;
  onHot: (m: string | null) => void;
  onSort: (s: { key: string; dir: 1 | -1 }) => void;
}) {
  // An axis the source cannot answer is not offered; if the stored
  // choice was one of those, it falls back to a quantity every source
  // has rather than drawing an axis with no readings on it.
  const ax = AXES[shown.includes(axes.x) ? axes.x : "total"];
  const ay = AXES[shown.includes(axes.y) ? axes.y : "output"];
  const az = AXES[shown.includes(axes.z) ? axes.z : "total"];
  const pts = agg
    .map((r) => ({ r, x: ax.of(r), y: ay.of(r), z: az.of(r) }))
    .filter((p) => Number.isFinite(p.x) && Number.isFinite(p.y));
  const off = agg.length - pts.length;
  const rx = ruler(pts.map((p) => p.x), force);
  const ry = ruler(pts.map((p) => p.y), force);
  const zmax = Math.max(...pts.map((p) => (Number.isFinite(p.z) ? p.z : 0)), 0);
  const pad = { l: 58, r: 18, t: 22, b: 34 };
  const iw = W - pad.l - pad.r;
  const ih = SCAT_H - pad.t - pad.b;
  const X = (v: number) => pad.l + iw * rx.at(v);
  const Y = (v: number) => pad.t + ih - ih * ry.at(v);

  const cols: { key: string; word: string; of?: (r: Agg) => number; text?: (r: Agg) => string }[] = [
    { key: "model", word: gui.accounts_models, text: (r) => r.word },
    { key: "input", word: gui.usage_axis_input, of: (r) => r.input },
    { key: "output", word: gui.usage_axis_output, of: (r) => r.output },
    { key: "total", word: gui.usage_axis_total, of: (r) => r.input + r.output },
    ...(cacheKnown
      ? [
          { key: "read", word: gui.usage_axis_read, of: (r: Agg) => r.cached_input },
          { key: "write", word: gui.usage_axis_write, of: (r: Agg) => r.cache_write },
          { key: "rate", word: gui.usage_axis_rate, of: (r: Agg) => r.rate },
          { key: "reuse", word: gui.usage_axis_reuse, of: (r: Agg) => r.reuse },
        ]
      : []),
    { key: "days", word: gui.usage_axis_days, of: (r) => r.days },
    { key: "span", word: gui.usage_col_span, text: (r) => `${r.from.slice(5)} → ${r.to.slice(5)}` },
  ];
  const listed = [...agg].sort((a, b) => {
    const c = cols.find((x) => x.key === sort.key);
    if (!c) return 0;
    if (c.of) {
      const [p, q] = [c.of(a), c.of(b)];
      // A row that has no reading on this column sorts to the bottom
      // whichever way the column is pointed — 「did not say」 is not a
      // small number.
      if (!Number.isFinite(p) || !Number.isFinite(q)) return Number.isFinite(p) ? -1 : Number.isFinite(q) ? 1 : 0;
      return (p - q) * sort.dir;
    }
    return (c.text?.(a) ?? "").localeCompare(c.text?.(b) ?? "") * sort.dir;
  });

  const sum = agg.reduce(
    (a, r) => ({
      input: a.input + r.input,
      output: a.output + r.output,
      cached_input: a.cached_input + r.cached_input,
      cache_write: a.cache_write + r.cache_write,
    }),
    { input: 0, output: 0, cached_input: 0, cache_write: 0 },
  );

  const taken: { x0: number; y0: number; x1: number; y1: number }[] = [];

  return (
    <section data-usage-zone="stats" className="flex flex-col gap-in border-t px-4 py-3">
      <header className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-2">
        <h3 className="text-aux font-title text-muted-foreground">{gui.usage_zone_stats}</h3>
        <span data-usage-range className="text-aux text-muted-foreground">
          {days.length > 0 ? gui.usage_range(days[0], days[days.length - 1], days.length) : ""}
        </span>
      </header>
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-aux text-muted-foreground">
        {(["x", "y", "z"] as const).map((which) => (
          <label key={which} className="flex items-center gap-1.5">
            {which === "z" ? gui.usage_pick_area : which.toUpperCase()}
            <select
              data-usage-pick={which}
              className="rounded-md border bg-card px-1.5 py-0.5 text-foreground"
              value={which === "x" ? axes.x : which === "y" ? axes.y : axes.z}
              onChange={(e) => onAxes({ ...axes, [which]: e.target.value as AxisKey })}
            >
              {shown.map((k) => (
                <option key={k} value={k}>
                  {AXES[k].word()}
                </option>
              ))}
            </select>
          </label>
        ))}
        <label className="flex items-center gap-1.5">
          {gui.usage_pick_ruler}
          {/* **The automatic answer is a default, not a claim that the
              other two are unreadable**, so it can be overruled — and
              whichever is in use is printed on the drawing either way. */}
          <select
            data-usage-pick="ruler"
            className="rounded-md border bg-card px-1.5 py-0.5 text-foreground"
            value={force}
            onChange={(e) => onForce(e.target.value)}
          >
            <option value="">{gui.usage_ruler_auto}</option>
            <option value="linear">{gui.usage_ruler_linear}</option>
            <option value="sqrt">{gui.usage_ruler_sqrt}</option>
            <option value="log">{gui.usage_ruler_log}</option>
          </select>
        </label>
      </div>
      <svg
        data-usage-scatter
        data-usage-ruler-x={rx.kind}
        data-usage-ruler-y={ry.kind}
        data-usage-scatter-off={off}
        viewBox={`0 0 ${W} ${SCAT_H}`}
        className="h-auto w-full overflow-visible text-foreground"
      >
        {tickSet(ry, ay, steps(ry)).map(({ v, label }) => (
          <g key={`y${v}`} data-usage-tick-y={v}>
            <line x1={pad.l} x2={W - pad.r} y1={Y(v)} y2={Y(v)} stroke="currentColor" strokeOpacity={0.1} strokeWidth={0.8} />
            <text x={pad.l - 7} y={Y(v) + 3} fontSize={8} textAnchor="end" fill="currentColor" fillOpacity={0.5}>
              {label}
            </text>
          </g>
        ))}
        {tickSet(rx, ax, steps(rx)).map(({ v, label }) => (
          <g key={`x${v}`} data-usage-tick-x={v}>
            <text x={X(v)} y={SCAT_H - 18} fontSize={8} textAnchor="middle" fill="currentColor" fillOpacity={0.5}>
              {label}
            </text>
          </g>
        ))}
        <text x={W - pad.r} y={SCAT_H - 3} fontSize={7.5} fontWeight={700} textAnchor="end" fill="currentColor" fillOpacity={0.5}>
          {gui.usage_scatter_foot(ax.word(), ay.word(), az.word())}
          {rx.kind === ry.kind ? ` · ${RULER_WORD[rx.kind]()}` : ` · X ${RULER_WORD[rx.kind]()} · Y ${RULER_WORD[ry.kind]()}`}
          {/* **How many models are not on the picture, said out loud.**
              A model with no input at all has no cache rate to be
              placed by, and a chart quietly missing a row is the same
              screen as one whose model never ran — the machine card's
              `top-plot-off` is the same sentence about the same
              silence. */}
          {off > 0 ? ` · ${gui.usage_scatter_off(off)}` : ""}
        </text>
        {/* Biggest first, so a small point lands on top of a big one and
            never under it. */}
        {[...pts]
          .sort((a, b) => (Number.isFinite(b.z) ? b.z : 0) - (Number.isFinite(a.z) ? a.z : 0))
          .map((p) => {
            const on = hot === keyOf(p.r);
            const r = radius(Number.isFinite(p.z) ? p.z : 0, zmax);
            const berthed = berth(X(p.x), Y(p.y), r, p.r.word.length * 5.1 + 2, taken, W, SCAT_H - 16);
            return (
              <g key={keyOf(p.r)} data-usage-point={p.r.model ?? ""} data-usage-point-r={r} className="cursor-pointer" onClick={() => onHot(on ? null : keyOf(p.r))}>
                <title>{`${p.r.word}\n  ${ax.word()} ${ax.tick(p.x, 1)}\n  ${ay.word()} ${ay.tick(p.y, 1)}`}</title>
                <circle cx={X(p.x)} cy={Y(p.y)} r={r} fill={bands.get(p.r.model)} stroke="var(--card)" strokeWidth={on ? 2 : HAIR + 0.3} opacity={on ? 1 : 0.92} />
                {berthed && (
                  <text
                    x={berthed.x}
                    y={berthed.y}
                    fontSize={8.5}
                    fontWeight={on ? 800 : 600}
                    textAnchor={berthed.anchor}
                    fill="currentColor"
                    fillOpacity={on ? 1 : 0.75}
                    paintOrder="stroke"
                    stroke="var(--card)"
                    strokeWidth={2.5}
                    strokeLinejoin="round"
                  >
                    {p.r.word}
                  </text>
                )}
              </g>
            );
          })}
      </svg>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-aux tabular-nums">
          <thead>
            <tr>
              {cols.map((c) => (
                <th
                  key={c.key}
                  data-usage-col={c.key}
                  aria-sort={sort.key === c.key ? (sort.dir < 0 ? "descending" : "ascending") : undefined}
                  onClick={() => onSort(sort.key === c.key ? { key: c.key, dir: (-sort.dir) as 1 | -1 } : { key: c.key, dir: c.key === "model" ? 1 : -1 })}
                  className={cn(
                    "cursor-pointer whitespace-nowrap border-b px-1 pb-1 font-title text-muted-foreground",
                    c.key === "model" ? "text-left" : "text-right",
                    sort.key === c.key && "text-foreground",
                  )}
                >
                  {c.word}
                  {sort.key === c.key ? (sort.dir < 0 ? " ↓" : " ↑") : ""}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {listed.map((r) => (
              <tr
                key={keyOf(r)}
                data-usage-row={r.model ?? ""}
                data-usage-row-hot={hot === keyOf(r) ? "" : undefined}
                onClick={() => onHot(hot === keyOf(r) ? null : keyOf(r))}
                className={cn("cursor-pointer border-b", hot === keyOf(r) && "bg-accent")}
              >
                {cols.map((c) => (
                  <td key={c.key} className={cn("px-1 py-1", c.key === "model" ? "text-left" : "text-right")}>
                    {c.key === "model" && <i className="mr-1.5 inline-block size-2 rounded-[2px] align-[-1px]" style={{ background: bands.get(r.model) }} />}
                    {/* **「—」, never 0.** A source that does not report
                        the cache has not reported nought of it. */}
                    {c.text ? c.text(r) : cell(c.key, c.of!(r))}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <dl data-usage-sums className="grid grid-cols-2 gap-x-4 gap-y-2 border-t pt-2 sm:grid-cols-4">
        {[
          [count(sum.input + sum.output), gui.usage_axis_total],
          [cacheKnown ? pct(sum.cached_input / Math.max(sum.input + sum.cached_input + sum.cache_write, 1)) : "—", gui.usage_axis_rate],
          [String(agg.length), gui.accounts_models],
          [String(days.length), gui.usage_sum_days],
        ].map(([v, w]) => (
          <div key={w}>
            <dd className="font-title text-base">{v}</dd>
            <dt className="text-aux text-muted-foreground">{w}</dt>
          </div>
        ))}
      </dl>
    </section>
  );
}

/** The unnamed row has no name to key on, and `""` would collide with a
    model actually called that. */
const keyOf = (r: Agg) => r.model ?? " unnamed";
const cell = (key: string, v: number) =>
  !Number.isFinite(v) ? "—" : key === "rate" ? pct(v) : key === "reuse" ? `${v.toFixed(1)}×` : key === "days" ? String(v) : count(v);

/**
 * A label goes to the first of four berths that does not overlap one
 * already placed. Ten-odd models is few enough that this always
 * settles; **with many more the right answer is fewer labels, not
 * smaller type**, so this returns nothing rather than shrinking.
 */
function berth(
  cx: number,
  cy: number,
  r: number,
  w: number,
  taken: { x0: number; y0: number; x1: number; y1: number }[],
  W2: number,
  H2: number,
): { x: number; y: number; anchor: "middle" | "start" | "end" } | null {
  const tries: [number, number, "middle" | "start" | "end"][] = [
    [cx, cy - r - 6, "middle"],
    [cx + r + 5, cy + 3.5, "start"],
    [cx - r - 5, cy + 3.5, "end"],
    [cx, cy + r + 11, "middle"],
  ];
  for (const [x, y, anchor] of tries) {
    const x0 = anchor === "middle" ? x - w / 2 : anchor === "end" ? x - w : x;
    const box = { x0, y0: y - 8, x1: x0 + w, y1: y + 2 };
    if (box.x0 < 2 || box.x1 > W2 - 2 || box.y0 < 2 || box.y1 > H2 - 2) continue;
    if (taken.some((t) => !(box.x1 < t.x0 || box.x0 > t.x1 || box.y1 < t.y0 || box.y0 > t.y1))) continue;
    taken.push(box);
    return { x, y, anchor };
  }
  return null;
}

/**
 * **The subscription road**: this machine's own transcripts, cut to one
 * vendor.
 *
 * The three answers this can give are the three faces, and telling them
 * apart is the whole job — `Usage`'s own head is explicit that an empty
 * `models` beside a non-empty `days` is 「the answering khor cannot cut
 * it that way」 and **is distinguishable from having spent nothing**,
 * which empties both.
 *
 * ⚠️ **A transcript is a fact about a machine, not about an account.**
 * `khor_node::usage` knows what the claude agents on this box spent; it
 * does not know which subscription's allowance paid for it. One
 * subscription on the machine and the two coincide, which is the case
 * this draws. **Two, and this chart is attributed to whichever row the
 * reader opened** — an open question, not a solved one, and worth
 * naming here because nothing on the screen would look wrong.
 */
export function fromUsage(
  usage: Usage | null,
  vendor: string,
): { rows: UsageRow[] | null; why: string | null; today: string | null; cacheKnown: boolean } {
  // Not answered yet. **`null`, never `[]`** — the difference between
  // them is the difference between drawing nothing and drawing a claim.
  if (usage === null) return { rows: null, why: null, today: null, cacheKnown: true };
  const mine = usage.models.filter((m) => m.category === vendor);
  const why =
    usage.models.length === 0 && usage.days.length > 0
      ? gui.usage_no_models
      : mine.length === 0
        ? cli.usage_none
        : null;
  return {
    rows: why
      ? null
      : mine.map((m) => ({
          day: m.day,
          model: m.model,
          input: m.tokens.input,
          output: m.tokens.output,
          cached_input: m.tokens.cached_input,
          cache_write: m.tokens.cache_write,
        })),
    why,
    today: usage.today,
    // A transcript itemises all four kinds, always — the 「did the
    // source say?」 question only arises on the provider road below.
    cacheKnown: true,
  };
}

/**
 * **The provider road**: what the vendor billed.
 *
 * ⚠️ **Nothing here has ever been run against a real admin key.** Both
 * vendors' usage reports are organisation-level calls, so the shapes
 * `khor_node::accounts` decodes come from documentation. What is drawn
 * from this is only as right as that reading.
 */
export function fromReading(
  reading: AccountReading,
): { rows: UsageRow[] | null; why: string | null; cacheKnown: boolean } {
  const tok = reading.model_tokens;
  // **Empty is the ordinary answer, not a failure** — the wallets
  // publish a balance and nothing else, and a registered subscription's
  // endpoint publishes percentages (`AccountReading::model_tokens`). So
  // the sentence says what the provider does, never what khor could not
  // do.
  if (tok.length === 0) return { rows: null, why: gui.usage_no_report, cacheKnown: false };
  return {
    rows: tok.map((m) => ({
      // `at` is unix seconds at the provider's own day start, and it is
      // read back **in UTC on purpose**: the boundary was cut upstream
      // by whoever billed it, and re-cutting it in this browser's zone
      // would move somebody else's day by the reader's offset.
      day: new Date(m.at * 1000).toISOString().slice(0, 10),
      model: m.model,
      input: m.tokens.input,
      output: m.tokens.output,
      cached_input: m.tokens.cached_input,
      cache_write: m.tokens.cache_write,
    })),
    why: null,
    // **`false` means those two were not reported, not that they were
    // nought** (`ModelTokenDay::cache_split`). A mixed answer is treated
    // as not reported: the field is documented as a property of the
    // source rather than of a row, so a source that answered both ways
    // is one khor has misread, and the safe reading of a misreading is
    // to claim nothing.
    cacheKnown: tok.every((m) => m.cache_split),
  };
}
