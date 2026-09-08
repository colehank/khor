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
 * ⚠️ **The provider road is three roads, and only one of them has ever
 * carried real numbers.** This comment said 「the provider road has
 * never been walked with a real admin key」 until 2026-09-08 — a true
 * sentence written as a universal, and it read as 「nothing here has
 * ever run」, which is false for the one road people actually see:
 *
 *   - **new-api** (`fetch_newapi`) reads `{base}/api/log/token` with
 *     the account's **own** bearer token — `TokenAuthReadOnly`, no
 *     admin anything. It is walked every time somebody opens such an
 *     account, and it is the only one of the three with a test against
 *     a real socket.
 *   - **claude** and **openai** (`claude_model_tokens`,
 *     `openai_model_tokens`) are organisation-level usage reports on an
 *     admin key, and **those** have never been met with a real one; the
 *     shapes come from documentation. `khor_node::accounts` says so at
 *     each of them, precisely — it was this file that over-generalised.
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
import { useEffect, useMemo, useRef, useState } from "react";

import type { AccountReading } from "@/gen/bindings/AccountReading";
import type { Usage } from "@/gen/bindings/Usage";
import { cli, gui } from "@/gen/catalog";
import { AxisLabel, PlotSettings } from "@/components/PlotControls";
import { PLOT, dotRadius, dotSpan, everyRowHas, labelSeat, tickFractions, usePlotBox } from "@/lib/plot";
import { Panel } from "@/components/Panel";
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
};

/* ⚠️ **Tombstone: the 「首 → 末」 column is gone, 2026-09-08** — the user
   did not want it. **`from` and `to` went with it**, which is the half
   of a removal that gets left behind: they were computed on every row
   of every window and read by nothing else, so keeping them would have
   been a fact this file goes on working out for a reader who no longer
   exists. `live` still decides `days`; only its two ends stopped being
   carried. The word `gui.usage-col-span` went too, and its key has a
   tombstone of its own in `zh.toml`. */

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
const steps = (r: Ruler): number[] => (r.kind === "flat" ? [0.5] : tickFractions());

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
  /** Which row the scatter last asked for, and how many times it has
      asked — the machine card's shape, and for its reason: pressing the
      same point twice has to be two answers rather than one. **What
      travels is the model's name**, never a row index: the table's
      order changes under the reader's hand. */
  const [hot, setHot] = useState<{ model: string; n: number } | null>(null);
  const [sort, setSort] = useState<{ key: string; dir: 1 | -1 }>({ key: "total", dir: -1 });
  const [axes, setAxes] = useState<{ x: AxisKey; y: AxisKey; z: AxisKey }>({
    x: "write",
    y: "read",
    z: "total",
  });

  // **Colours are worked out over every model the account has, never
  // over the window's** — see `lib/model-band`. Fed the window's, a
  // model changes colour when somebody presses 7 天.
  const bands = useMemo(() => bandsFor([...new Set((rows ?? []).map((r) => r.model))]), [rows]);
  const days = useMemo(() => dayAxis(rows ?? [], today, win), [rows, today, win]);
  const agg = useMemo(() => aggregate(rows ?? [], days, cacheKnown), [rows, days, cacheKnown]);

  if (why) {
    return (
      // **The sentence sits on the same card the chart would have.**
      // The machine card's refusal keeps its panel and loses only the
      // rows, and the reason carries: a block that changes *shape*
      // when it has nothing to report reads as a block that failed to
      // load, on top of whatever it is actually saying.
      // ⚠️ What does **not** carry is that card's other half — it
      // keeps its column heads because pressing one is the way out of
      // the refusal. Nothing here is a way out, and an empty table
      // head is a control that answers with nothing. **If this half
      // ever grows something to press, that ruling has to be looked at
      // again.**
      <div data-usage-half data-usage-mute data-usage-source={source} className="flex flex-col gap-row">
        <p data-usage-title className="text-aux font-title text-muted-foreground">{gui.usage_title}</p>
        <Panel>
          <p className="text-sm text-muted-foreground">{why}</p>
        </Panel>
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
    <div data-usage-half data-usage-source={source} className="flex flex-col gap-row">
      {/* **One line, and it is the only heading in this block.**
          ⚠️ The two zone headings this replaces were control strips
          with titles on them; the strips went (see `Trend` and
          `Stats`) and the machine card's own figure-and-table panel
          carries no heading at all — but that panel is on a card whose
          every part is about one machine.
          **Measured 2026-09-08**: the subscription's detail pane has
          exactly three children — the account's identity line, the
          dial with its windows, and this. Nothing in the first two
          says what this one is, and the block above it is **money**
          while this is tokens. Delete this line and the only thing
          separating the two quantities is a horizontal rule. */}
      <p data-usage-title className="text-aux font-title text-muted-foreground">{gui.usage_title}</p>
      {/* **One card, not two flat sections** (`components/Panel`, the
          machine card's own). The two zones are two *parts of one
          subject* — the same models, drawn over days and then listed —
          so they are one panel with a seam through it, exactly the way
          that card holds its figure and its table. Two cards would say
          they are two subjects. */}
      <Panel>
        <Trend days={days} agg={agg} rows={rows} bands={bands} view={view} win={win} onView={setView} onWin={setWin} />
        <Stats
          agg={agg}
          days={days}
          bands={bands}
          axes={axes}
          shown={shown}
          cacheKnown={cacheKnown}
          hot={hot}
          sort={sort}
          onAxes={setAxes}
          onHot={(model) => setHot({ model, n: (hot?.n ?? 0) + 1 })}
          onSort={setSort}
        />
      </Panel>
    </div>
  );
}

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
  onView,
  onWin,
}: {
  days: string[];
  agg: Agg[];
  rows: UsageRow[];
  bands: Map<string | null, string>;
  view: "day" | "model";
  win: number;
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
    for (const r of rows) m.set(`${r.day}\u0000${r.model}`, (m.get(`${r.day}\u0000${r.model}`) ?? 0) + r.input + r.output);
    return m;
  }, [rows]);
  const at = (day: string, model: string | null) => cell.get(`${day}\u0000${model}`) ?? 0;
  const totals = days.map((d) => stack.reduce((a, b) => a + at(d, b.model), 0));
  const peak = Math.max(...totals, 1);
  const live = totals.filter((v) => v > 0).sort((a, b) => a - b);
  const median = live.length > 2 ? live[live.length >> 1] : null;

  // **The same drawing method the scatter uses** (`lib/plot`): one unit
  // is one pixel, and the padding, tick ink and tick words are the
  // machine card's. This chart has no counterpart over there, but it
  // sits beside one that does — two figures on one card drawn to two
  // conventions is the same complaint one screen smaller.
  const { box: canvas, w: W, h: H } = usePlotBox({ w: 900, h: 200 });
  const iw = W - PLOT.padL - PLOT.padR;
  const ih = H - PLOT.padT - PLOT.padB;
  const y = (v: number) => PLOT.padT + ih - (ih * v) / peak;
  const stepW = iw / Math.max(days.length, 1);
  const bw = Math.min(34, Math.max(2, stepW * 0.74));
  const cx = (i: number) => PLOT.padL + stepW * (i + 0.5);
  const busiest = totals.reduce((a, v, i) => (v > totals[a] ? i : a), 0);

  const modelRuler = ruler(agg.map((r) => r.input + r.output));

  return (
    <section data-usage-zone="trend" className="flex flex-col gap-in">
      {/* ⚠️ **No heading and no control strip.** Both were here until
          2026-09-08: a title beside a row of chips, above the figure.
          The machine card puts every control **on the thing it
          controls** — sorting on the column heads, an axis's quantity
          on that axis — and has no strip at all, so a reader arriving
          from that card met a second idea of 「where the controls are」
          on this screen.
          These two are the figure's x axis: 「按天 / 按模型」 is **what
          that axis is**, and the window is **how far it reaches**. So
          they sit under it — the axis says both words, and one settings
          button changes them, which is the shape the machine card's
          figure wears too (`components/PlotControls`). */}
      {/* **The box the figure is measured into.** Its height is this
          page's own — a stack of ten days needs less air than a scatter
          of labelled models — while everything inside it is
          `lib/plot`'s. */}
      <div ref={canvas} className="relative h-52 w-full">
      {view === "day" ? (
        <svg
          data-usage-chart="day"
          // **Printed, and it is not decoration**: the bands add up to
          // the column, so this is the one axis on the page that is not
          // allowed to be anything else, and a gate reads it here.
          data-usage-scale="linear"
          data-usage-peak={peak}
          viewBox={`0 0 ${W} ${H}`}
          width={W}
          height={H}
          className="overflow-visible text-foreground"
        >
          {[0, 1, 2, 3].map((k) => (
            <g key={k}>
              <line x1={PLOT.padL} x2={W - PLOT.padR} y1={y((peak * k) / 3)} y2={y((peak * k) / 3)} stroke="currentColor" strokeOpacity={PLOT.frameInk} />
              <text x={PLOT.padL - 6} y={y((peak * k) / 3) + 3} fontSize={PLOT.tick} textAnchor="end" fill="currentColor" fillOpacity={PLOT.tickWordInk}>
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
              <text key={d} x={cx(i)} y={H - 6} fontSize={PLOT.tick} textAnchor="middle" fill="currentColor" fillOpacity={PLOT.tickWordInk}>
                {d.slice(5)}
              </text>
            ) : null,
          )}
          {median !== null && (
            <g data-usage-median={median}>
              <line x1={PLOT.padL} x2={W - PLOT.padR} y1={y(median)} y2={y(median)} stroke="currentColor" strokeOpacity={0.45} strokeWidth={0.9} strokeDasharray="4 3" />
              <text x={W - PLOT.padR} y={y(median) - 4} fontSize={PLOT.tick} textAnchor="end" fill="currentColor" fillOpacity={0.6}>
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
          viewBox={`0 0 ${W} ${H}`}
          width={W}
          height={H}
          className="overflow-visible text-foreground"
        >
          {tickSet(modelRuler, { tick: count }, steps(modelRuler)).map(({ v, label }) => (
            <g key={v}>
              <line x1={PLOT.padL} x2={W - PLOT.padR} y1={PLOT.padT + ih - ih * modelRuler.at(v)} y2={PLOT.padT + ih - ih * modelRuler.at(v)} stroke="currentColor" strokeOpacity={PLOT.frameInk} />
              <text x={PLOT.padL - 6} y={PLOT.padT + ih - ih * modelRuler.at(v) + 3} fontSize={PLOT.tick} textAnchor="end" fill="currentColor" fillOpacity={PLOT.tickWordInk}>
                {label}
              </text>
            </g>
          ))}
          {agg.map((r, k) => {
            const step2 = iw / agg.length;
            const bx = PLOT.padL + step2 * (k + 0.5);
            const v = r.input + r.output;
            const top = PLOT.padT + ih - ih * modelRuler.at(v);
            return (
              <g key={String(r.model)} data-usage-bar={r.model ?? ""} data-usage-bar-value={v}>
                <rect x={bx - Math.min(46, step2 * 0.62) / 2} y={top} width={Math.min(46, step2 * 0.62)} height={Math.max(PLOT.padT + ih - top, 0)} fill={bands.get(r.model)} rx={2} />
                <text x={bx} y={PLOT.padT + ih + 12} fontSize={PLOT.tick} textAnchor="middle" fill="currentColor" fillOpacity={0.6}>
                  {r.word.length > 13 ? `${r.word.slice(0, 12)}…` : r.word}
                </text>
                <text x={bx} y={top - 4} fontSize={PLOT.tick} fontWeight={700} textAnchor="middle" fill="currentColor">
                  {count(v)}
                </text>
              </g>
            );
          })}
          {/* **A bent axis that does not say so is a chart that lies
              quietly.** The day view has no such line because it cannot
              bend; this one can, so it says which ruler it is on. */}
          <text x={W - PLOT.padR} y={H - 2} fontSize={PLOT.tick} fontWeight={700} textAnchor="end" fill="currentColor" fillOpacity={PLOT.tickWordInk}>
            {RULER_WORD[modelRuler.kind]()}
          </text>
        </svg>
      )}
      </div>
      {/* **The x axis says what it is; one button changes it**
          (2026-09-08:「都做成一个小的设置按钮,点击后进行选择」). The two
          words were controls until that day — press 「按天」 and the
          candidates replaced it in place. What the axis keeps is the
          half of the 2026-09-05 ruling that still holds: **an axis
          carries one word, and it is the right one.** */}
      <div className="flex items-baseline justify-between">
        <AxisLabel
          which="view"
          word={view === "model" ? gui.usage_view_model : gui.usage_view_day}
          rulerWord={win === 0 ? gui.usage_window_all : gui.usage_window_days(win)}
          align="left"
        />
        <PlotSettings
          rows={[
            {
              key: "view",
              word: gui.usage_view_day,
              chosen: view,
              options: [
                { key: "day", word: gui.usage_view_day },
                { key: "model", word: gui.usage_view_model },
              ],
            },
            {
              key: "window",
              word: gui.usage_window_all,
              chosen: String(win),
              options: WINDOWS.map((w) => ({ key: String(w), word: w === 0 ? gui.usage_window_all : gui.usage_window_days(w) })),
            },
          ]}
          onPick={(row, k) => (row === "view" ? onView(k === "model" ? "model" : "day") : onWin(Number(k)))}
        />
      </div>
      {/* **The legend sits under the drawing, not over it** (2026-09-08,
          the user: 「模型 legend 写在柱状图的下方」). It is the same list
          the table is, in the same order, so a colour looked up in one
          place means the same in the other — and under the figure it is
          read *after* the shape rather than as a preamble to it.
          **Below the axis row on purpose**: that row is the x axis, and
          an axis belongs against the drawing it measures.

          ⚠️ **Tombstone: the sentence 「柱高是当天的输入加输出,柱里的
          分段是各个模型」 was here and is gone** (2026-09-08, the user
          did not want it). Its key went with it — see `zh.toml`. */}
      <ul data-usage-key className="flex flex-wrap gap-x-3 gap-y-0.5 text-aux text-muted-foreground">
        {/* **A square swatch, with no corner of its own.** It carried
            an arbitrary 2px corner until 2026-09-08, and the type gate
            was red on it for as long (it reads that class as a radius
            literal — including one written inside a comment, which is
            why this sentence spells the number out instead):
            `tokens.css` owns every radius, and the ladder's smallest
            rung is 6px — on an 8px chip that is a
            circle, which is a different mark, not a softer one. So it
            wears none: the swatch is the colour, and a colour does not
            need a corner. */}
        {agg.map((r) => (
          <li key={String(r.model)} data-usage-key-model={r.model ?? ""} className="flex items-center gap-1.5">
            <i className="size-2" style={{ background: bands.get(r.model) }} />
            {r.word}
          </li>
        ))}
      </ul>
    </section>
  );
}

/* ⚠️ **Tombstone: `Chip` was here until 2026-09-08, and with it the two
   control strips it filled.** It was a capsule — `rounded-full`, a
   shape nothing else in this app wears — and the machine card's own
   `PlotControls` head says why that is the wrong thing to be first at: a
   bordered capsule says 「button」 louder than anything else on the
   surface, and this card had no such box before.
   The controls did not go, they moved onto the things they control. */

/* ⚠️ **Tombstone: `R_MIN` 5 / `R_MAX` 19 were constants here until
   2026-09-08.** They were fine for ten models and would have tiled the
   machine card's two hundred processes — the same range cannot serve
   both figures, and 「the same drawing method」 has to mean one rule
   rather than one number. `lib/plot`'s `dotSpan` is that rule: the
   range is a function of how many points there are and how much room
   they have. **`radius()` went with them** — the square root that made
   the area carry the value is `lib/plot`'s `dotRadius`, and its head
   still says why a radius read straight off the number makes a 4×
   value look 16× big. */

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
  hot,
  sort,
  onAxes,
  onHot,
  onSort,
}: {
  agg: Agg[];
  days: string[];
  bands: Map<string | null, string>;
  axes: { x: AxisKey; y: AxisKey; z: AxisKey };
  shown: AxisKey[];
  cacheKnown: boolean;
  /** The row a point asked for, with a counter so the same point
      pressed twice is two answers. */
  hot: { model: string; n: number } | null;
  sort: { key: string; dir: 1 | -1 };
  onAxes: (a: { x: AxisKey; y: AxisKey; z: AxisKey }) => void;
  /** **One way only: a point asks for a row.** The table does not ask
      for a point back — see the effect below for what this is and is
      not. */
  onHot: (model: string) => void;
  onSort: (s: { key: string; dir: 1 | -1 }) => void;
}) {
  const box = useRef<HTMLDivElement>(null);
  // **Locate, not select** — the machine card's gesture, copied whole
  // because half of it is a different gesture. Three parts:
  //
  //   - the row is **brought into view**, stopping below the head
  //     rather than under it (a sticky head would otherwise hide the
  //     row that was just asked for);
  //   - it is marked for **a moment** and the mark clears itself. A
  //     highlight that stays is a selection, and a selection is a state
  //     the reader then has to undo;
  //   - **reduced motion keeps the mark and drops the fade**, holding
  //     it longer instead. What that setting asks to remove is the
  //     movement, not the answer.
  //
  // ⚠️ This half had the other gesture until 2026-09-08: pressing a
  // point *or* a row toggled a lasting tint. Two directions and a
  // sticky state, against the machine card's one direction and a
  // flash — the same table, two ideas of what pressing it means.
  useEffect(() => {
    if (!hot) return;
    const b = box.current;
    const row = b?.querySelector(`[data-usage-row="${CSS.escape(hot.model)}"]`);
    if (!b || !row) return;
    const head = b.querySelector("thead");
    const sticky = head && getComputedStyle(head).position === "sticky" ? head.getBoundingClientRect().height : 0;
    // The nearest thing that actually scrolls — on the wide face that
    // is the detail column, not this block.
    let port: HTMLElement | null = b;
    while (port && port.scrollHeight <= port.clientHeight) port = port.parentElement;
    if (port) port.scrollTop += row.getBoundingClientRect().top - port.getBoundingClientRect().top - sticky - 4;
    row.setAttribute("data-hot", "");
    const still = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    const t = window.setTimeout(() => row.removeAttribute("data-hot"), still ? 2400 : 1200);
    return () => {
      window.clearTimeout(t);
      row.removeAttribute("data-hot");
    };
  }, [hot]);
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
  const rx = ruler(pts.map((p) => p.x));
  const ry = ruler(pts.map((p) => p.y));
  const zmax = Math.max(...pts.map((p) => (Number.isFinite(p.z) ? p.z : 0)), 0);
  // **One unit is one pixel here too, since 2026-09-08.** This figure
  // was drawn into a scaled `viewBox`, which is why its axis labels sat
  // at `6.59%` and `2.05%` — a pixel offset divided by a viewBox width,
  // because on a scaling canvas a pixel means nothing. The geometry is
  // `lib/plot`'s now, and it is the machine card's.
  const { box: canvas, w: W, h: H } = usePlotBox({ w: 900, h: 260 });
  const iw = W - PLOT.padL - PLOT.padR - 2 * PLOT.inset;
  const ih = H - PLOT.padT - PLOT.padB - 2 * PLOT.inset;
  const span = dotSpan(pts.length, W - PLOT.padL - PLOT.padR, H - PLOT.padT - PLOT.padB);
  // **Which quantities may be an area here** — the same data-driven
  // rule the machine card uses. On this page every quantity a source
  // reports is reported for every model, so this usually equals
  // `shown`; when it does not (a model with no input has no cache rate)
  // the offer shrinks rather than a dot landing on the floor for a
  // reading nobody took.
  //
  // ⚠️ **This call has no gate of its own, and that was measured, not
  // assumed** (2026-09-08): tearing `everyRowHas` open reddens the
  // machine card twice and this half not at all, because the browser
  // gate's fixture gives every model every figure, so `areaKeys` there
  // is always `shown`. The rule is guarded over there (显存 is on 2 of
  // 23 rows); what is unguarded is that **this** page still asks it.
  // To give it an arm the fixture needs a model with no input tokens —
  // the one shape that makes a cache rate unreadable on a page whose
  // source does report the cache.
  const areaKeys = everyRowHas(shown, agg, (k, r) => AXES[k].of(r));
  const X = (v: number) => PLOT.padL + PLOT.inset + iw * rx.at(v);
  const Y = (v: number) => H - PLOT.padB - PLOT.inset - ih * ry.at(v);

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
    <section
      data-usage-zone="stats"
      ref={box}
      // **The seam, and it is `--gap-group` on both sides of a
      // hairline** — `data-top-seam` next door is the same three
      // classes for the same reason: these two are one block, and the
      // token whose whole definition is 「between groups that must not
      // read as one list」 is this one. `--gap-row` was tried there and
      // measured 25px against an 18px row, a break narrower than twice
      // the rhythm it breaks.
      data-usage-seam
      className="mt-group flex flex-col gap-in border-t pt-group"
    >
      {/* ⚠️ **No heading, and no row of `<select>`s.** Both were here
          until 2026-09-08. The four native selects were 23px tall —
          half the finger floor — and they sat in a strip above the
          figure, so 「change what this axis shows」 was a different act
          here from what it is on the machine card, where the axis's own
          word is the control.
          The window this covers moved with them: it is printed at the
          figure's foot, where it is a fact about the drawing rather
          than a title over it. */}
      <div ref={canvas} className="relative h-64 w-full">
        {/* **The two quantities, each printed on its own axis** —
            `components/PlotControls`, the machine card's own. They are
            words now, not controls: what changes them is the settings
            button below. Tucked into the two corners a scatter is least
            likely to put a point, which is also where the ruler words
            already were. */}
        <span className="pointer-events-none absolute inset-0">
          <span className="pointer-events-auto absolute" style={labelSeat("y")}>
            <AxisLabel which="y" word={AXES[axes.y].word()} rulerWord={RULER_WORD[ry.kind]()} align="left" />
          </span>
          <span className="pointer-events-auto absolute" style={labelSeat("x")}>
            <AxisLabel which="x" word={AXES[axes.x].word()} rulerWord={RULER_WORD[rx.kind]()} align="right" />
          </span>
          {/* **Three rows, and the third is the dot's area.** The
              machine card grew the same row on 2026-09-08, so this is
              one rule on two figures rather than a shape this card has
              to itself. The axis rows offer everything this source
              reports; the area row offers what every model reports,
              because a point must exist to carry X and Y and so a
              missing reading has nowhere to go but the floor — where
              it would read as 「the smallest」. */}
          <span className="pointer-events-auto absolute" style={labelSeat("settings")}>
            <PlotSettings
              rows={[
                { key: "x", word: "X", chosen: axes.x, options: shown.map((k) => ({ key: k, word: AXES[k].word() })) },
                { key: "y", word: "Y", chosen: axes.y, options: shown.map((k) => ({ key: k, word: AXES[k].word() })) },
                { key: "z", word: gui.usage_pick_area, chosen: axes.z, options: areaKeys.map((k) => ({ key: k, word: AXES[k].word() })) },
              ]}
              onPick={(row, k) => onAxes({ ...axes, [row]: k as AxisKey })}
            />
          </span>
        </span>
      <svg
        data-usage-scatter
        data-usage-ruler-x={rx.kind}
        data-usage-ruler-y={ry.kind}
        data-usage-scatter-off={off}
        viewBox={`0 0 ${W} ${H}`}
        width={W}
        height={H}
        className="overflow-visible text-foreground"
      >
        {tickSet(ry, ay, steps(ry)).map(({ v, label }) => (
          <g key={`y${v}`} data-usage-tick-y={v}>
            <line x1={PLOT.padL} x2={W - PLOT.padR} y1={Y(v)} y2={Y(v)} stroke="currentColor" strokeOpacity={PLOT.frameInk} />
            <text x={PLOT.padL - 6} y={Y(v) + 3} fontSize={PLOT.tick} textAnchor="end" fill="currentColor" fillOpacity={PLOT.tickWordInk}>
              {label}
            </text>
          </g>
        ))}
        {tickSet(rx, ax, steps(rx)).map(({ v, label }) => (
          <g key={`x${v}`} data-usage-tick-x={v}>
            <text x={X(v)} y={H - PLOT.padB + 14} fontSize={PLOT.tick} textAnchor="middle" fill="currentColor" fillOpacity={PLOT.tickWordInk}>
              {label}
            </text>
          </g>
        ))}
        {/* Biggest first, so a small point lands on top of a big one and
            never under it. */}
        {[...pts]
          .sort((a, b) => (Number.isFinite(b.z) ? b.z : 0) - (Number.isFinite(a.z) ? a.z : 0))
          .map((p) => {
            const on = hot?.model === (p.r.model ?? "");
            const r = dotRadius(Number.isFinite(p.z) ? p.z : 0, zmax, span);
            const berthed = berth(X(p.x), Y(p.y), r, p.r.word.length * 5.1 + 2, taken, W, H - 16);
            return (
              // **It says it is a target**, the way the machine card's
              // points do: a role and a name, so a reader who cannot
              // see the drawing is offered the same act.
              //
              // **And it is a finger wide, where the machine card
              // waives that.** Its waiver is 「two hundred 44px points
              // would tile the drawing and leave nothing to read」 —
              // true there, and not true here: this picture has one dot
              // per model, ten at the outside. So instead of a waiver
              // there is a transparent circle at the floor (20 viewBox
              // units ≈ 22px of radius at the width this is drawn at),
              // the same trick under the machine card's own dots, sized
              // for a table that is small enough to afford it.
              <g
                key={keyOf(p.r)}
                data-usage-point={p.r.model ?? ""}
                data-usage-point-r={r}
                role="button"
                aria-label={p.r.word}
                className="cursor-pointer"
                onClick={() => onHot(p.r.model ?? "")}
              >
                <title>{`${p.r.word}\n  ${ax.word()} ${ax.tick(p.x, 1)}\n  ${ay.word()} ${ay.tick(p.y, 1)}`}</title>
                <circle cx={X(p.x)} cy={Y(p.y)} r={Math.max(r, 20)} fill="transparent" />
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
      </div>
      {/* **The figure's foot: what the dot's area means, in words.**
          ⚠️ **It was the control until 2026-09-08** — the word itself
          opened a list. Switching lives in the settings button on the
          figure now, so this is a sentence again, the same kind the
          machine card's foot says about its halo (`top-plot-halo`).
          The window sits here too: it is a fact about what was drawn,
          not a title over it. */}
      <div className="flex flex-wrap items-baseline justify-between gap-x-row text-aux text-muted-foreground">
        {/* `data-plot-area-of` rather than a name of this page's own:
            both figures say this sentence from one key now, so a gate
            comparing them looks for one attribute. */}
        <span data-plot-area-of={axes.z}>{gui.plot_area_is(AXES[axes.z].word())}</span>
        <span className="flex items-baseline gap-x-row">
          {/* **How many models are not on the picture, said out loud.**
              A model with no reading on one of these two axes is not
              drawn at zero — that would turn 「not read」 into 「the
              smallest」 — so it is left off, and how many were left off
              has to be said. `top-plot-off` is the same sentence about
              the same silence. */}
          {off > 0 && <span data-usage-off={off}>{gui.usage_scatter_off(off)}</span>}
          <span data-usage-range>
            {days.length > 0 ? gui.usage_range(days[0], days[days.length - 1], days.length) : ""}
          </span>
        </span>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full border-collapse text-aux tabular-nums">
          <thead>
            <tr>
              {cols.map((c) => (
                // **`aria-sort` on every head, `none` where it is not the
                // one in use.** Only marking the active column says
                // 「this column is sorted」 and leaves 「those columns
                // can be sorted」 unsaid — the machine card's heads all
                // carry it, which is what makes the row of them read as
                // a row of controls.
                //
                // **And the head is a finger tall.** It was 23.3px
                // (measured 2026-09-08), a press target at half the
                // floor, and the walk that asserts that floor never
                // reaches this surface — see the gate's 額度 stop.
                <th
                  key={c.key}
                  data-usage-col={c.key}
                  aria-sort={sort.key === c.key ? (sort.dir < 0 ? "descending" : "ascending") : "none"}
                  className={cn(
                    "whitespace-nowrap border-b p-0 font-title text-muted-foreground",
                    sort.key === c.key && "text-foreground",
                  )}
                >
                  {/* **The press is on a `<button>`, not on the cell** —
                      the machine card's heads are built the same way,
                      and it is not decoration: a handler on a bare
                      `<th>` is a control only a pointer can find, which
                      is the thing section 4 of the gate is about. It
                      also carries the finger floor, which the cell
                      cannot. */}
                  <button
                    type="button"
                    onClick={() => onSort(sort.key === c.key ? { key: c.key, dir: (-sort.dir) as 1 | -1 } : { key: c.key, dir: c.key === "model" ? 1 : -1 })}
                    // The floor on **both** axes, which is the machine
                    // card's own head verbatim: a head 44 tall and 34
                    // wide is a target you miss sideways, and 「输入」
                    // measured 34 on the narrow face. `min-w-touch` is
                    // what the narrow columns grow to; `w-full` spreads
                    // the press over the whole head rather than the
                    // word.
                    className={cn(
                      "inline-flex min-h-touch w-full min-w-touch items-center px-1",
                      c.key === "model" ? "justify-start" : "justify-end",
                    )}
                  >
                    {c.word}
                    {sort.key === c.key ? (sort.dir < 0 ? " ↓" : " ↑") : ""}
                  </button>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {listed.map((r) => (
              // **Not pressable.** The row is where an answer lands, not
              // a control — pressing it used to toggle a lasting tint,
              // which made the same table mean 「select」 here and
              // 「locate」 on the machine card. The mark it wears now is
              // put on by the effect above and takes itself off.
              <tr key={keyOf(r)} data-usage-row={r.model ?? ""} className="border-b">
                {cols.map((c) => (
                  <td key={c.key} className={cn("px-1 py-1", c.key === "model" ? "text-left" : "text-right")}>
                    {/* `align-middle`, not a one-pixel nudge: `check-px` reads a
                        literal in a class as a second source of truth, and
                        `npm run test` was red on it from 30f6a88 (12:49) to
                        the fix. A stock alignment word is the same idea
                        without a number nobody owns. */}
                    {c.key === "model" && <i className="mr-1.5 inline-block size-2 align-middle" style={{ background: bands.get(r.model) }} />}
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
      <dl data-usage-sums className="grid grid-cols-2 gap-x-row gap-y-in border-t pt-in sm:grid-cols-4">
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
    model actually called that.
 *
 * ⚠️ **`\u0000`, the escape — never the byte itself.** These three
 * separators were literal NUL bytes in this file until 2026-09-08, and
 * the runtime could not tell the difference. **Everything else could**:
 * `file` calls a source file with a NUL in it `data`, and a `grep`
 * running with `-I` skips binary files — so every search of this file
 * came back **empty and said nothing about why**. It was found by a
 * disagreement between `git grep` (which read the blob) and `grep`
 * (which skipped the file), after two earlier runs where `grep -c ""`
 * returned nothing at all and that was shrugged off.
 *
 * The same rule holds for `cell`'s composite key below. A separator
 * that cannot occur in a model's name is the right idea; writing it as
 * a byte instead of an escape is what made this file unsearchable. */
const keyOf = (r: Agg) => r.model ?? "\u0000unnamed";
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
 * ⚠️ **Which of the three sources fed this row decides how much to
 * trust it**, and the head of this file has the split. In short:
 * new-api's rows come off that gateway's own log on the account's
 * ordinary key and are real; claude's and openai's come from
 * admin-key usage reports whose shapes were read out of documentation
 * and have never been met with a live answer.
 *
 * **`cache_split` is the one field where that difference shows on the
 * glass**: a gateway log has no cache columns to state, so it says
 * `false` and the four cache quantities leave the page — while a
 * vendor usage report itemises them and says `true`.
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
