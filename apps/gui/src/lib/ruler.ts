/**
 * **How one axis turns a reading into a position** — and the tick
 * labels that go with it.
 *
 * # ⚠️ There are two of these in this repo, and they do not agree
 *
 * `MachineCard.tsx` has a `ruler` of its own and **still uses it**.
 * The two differ in exactly one thing: **which ruler gets chosen**.
 * Everything else — `log1p` rather than `log`, an axis that starts at
 * the smallest reading rather than at zero, ticks whose labels are
 * real readings — is the same in both, and `tickSet` below is the
 * literal same function, shared.
 *
 * The machine page's judge is `max / median > 44`: under a square root
 * a ratio `R` puts the median at `1/√R` of the axis, so `R > 44` is
 * exactly 「half the points sit in the first 15%」.
 *
 * **That judge is a proxy, and the proxy is blind to a two-clump
 * shape.** Measured 2026-09-08 on the machine page's own gate fixture
 * (`check-face.mjs`'s `load.top`, 23 rows): the 内存 axis reads
 * `max / median = 36.9` — under 44, judged healthy — while the square
 * root it picks puts **11 of those 23 points inside the first 15% of
 * the axis**, which is the thing the rule exists to prevent. A log
 * ruler puts 4 there. The reason is the shape: a median that lands
 * inside the upper clump cannot see the lower one, and a ratio taken
 * against it says nothing about how the points are laid out. **No gate
 * has ever measured that 15%** — the gates measure 「both rulers are
 * reachable」 and 「no two ticks read the same」 — which is why it has
 * been green throughout.
 *
 * So this file measures the thing itself instead of a proxy: lay the
 * points out with each of the three rulers and **keep the one that
 * holds the closest pair furthest apart**. There is no threshold to
 * tune, and a threshold was tried twice on the usage data and was
 * wrong both times (「half inside 15%」 passed a √ ruler with four
 * points stacked in the first 3%; tightening it to a third sent every
 * axis to a log, including a rate whose readings only span 0.70–0.98).
 *
 * # Why the machine page has not been switched over
 *
 * Not because it is right there — the 36.9 / 11-of-23 above is its own
 * fixture. Because switching it costs three things this file's batch
 * could not pay:
 *
 *   - **two words that do not exist.** This judge can answer `linear`
 *     and `flat`, and the machine page's axis prints one of exactly
 *     two words (`cli.top_plot_sqrt`, `cli.top_plot_log`). Wording is
 *     the user's call in this repo, not a lane's.
 *   - **a gate that goes red, measured.** `check-face.mjs` requires
 *     two *different* rulers to appear in one run — a control arm, so
 *     that 「it is chosen by measurement」 is tested rather than 「it is
 *     always this one」. Under this judge every axis of the two pairs
 *     that gate walks comes out `log` on that fixture, so the arm
 *     stops distinguishing anything. Deleting it would be the ledger's
 *     ninth false green; repairing it honestly means a fixture axis
 *     that genuinely wants another ruler, and six other assertions
 *     read that fixture.
 *   - **a reading nobody has taken.** All of the above is the fixture.
 *     The three live machines' crowding **has never been counted** —
 *     the machine page's own note records their `max / median` (内存 is
 *     16 / 34 / 52) and nothing about where the points land.
 *
 * **This comment is the divergence, written down with the number that
 * makes it checkable.** It is deliberately not a 「TODO: unify」: a
 * TODO is a wish, and a wish cannot be argued with. The two ways this
 * ends are that somebody counts the crowding on three real machines
 * and moves the machine page here, or counts it and finds 44 holds
 * there — either way the next person needs the 36.9, not the wish.
 */

/**
 * **How far apart a layout manages to hold its two closest points.**
 *
 * Equal readings are dropped first: two models with the same number
 * cannot be separated by any ruler, so they say nothing about which
 * one to reach for. Rounding to six places is what makes 「equal」 a
 * decidable question after three different transforms have been
 * through it.
 *
 * Fewer than two distinct positions is 「nothing to separate」, and
 * answering 1 (the widest an axis can be) makes every ruler tie, which
 * is what sends the choice to the plainest one.
 */
function spread(positions: number[]): number {
  const seen = [...new Set(positions.map((v) => Number(v.toFixed(6))))].sort((a, b) => a - b);
  if (seen.length < 2) return 1;
  let min = Infinity;
  for (let i = 1; i < seen.length; i += 1) min = Math.min(min, seen[i] - seen[i - 1]);
  return min;
}

export type Ruler = {
  kind: "linear" | "sqrt" | "log" | "flat";
  /** The data's two ends — the first and last tick, and the two
      readings a point can sit exactly on. */
  lo: number;
  hi: number;
  /** A reading's place on the axis, `0..=1`. */
  at: (v: number) => number;
  /** The reading whose place is `f` — so a tick's label is a real
      reading and the tick is drawn where that reading belongs, never a
      transformed number wearing a label. */
  tick: (f: number) => number;
};

/**
 * The ruler this set of readings is best drawn with.
 *
 * ⚠️ **There is no override, and there was one for a few hours on
 * 2026-09-08.** A `force` argument fed a 「尺子」 dropdown on the 額度
 * page, on the reasoning that the automatic answer is a default rather
 * than a claim the other rulers are unreadable. Both went the same
 * afternoon: **the machine card has no such control**, and one idea
 * wearing two different controls on two pages is what 「看起来是两个
 * 产品」 turned out to mean. The ruler is measured, and printed on the
 * axis so the reader knows which one they are reading.
 *
 * If an override is ever wanted, it is wanted on **both** figures, and
 * it is one control — not a second one grown next to the first.
 */
export function ruler(vals: number[]): Ruler {
  const v = vals.filter((x) => x != null && Number.isFinite(x)).sort((a, b) => a - b);
  // Nothing to draw. A flat 0..1 rather than a throw: the caller draws
  // no points either, and an axis is still a frame.
  if (v.length === 0) return { kind: "flat", lo: 0, hi: 1, at: () => 0, tick: (f) => f };
  const lo = v[0];
  const hi = v[v.length - 1];
  // **Every reading is the same number.** No spread to draw and no
  // ruler that could help; the points go in the middle rather than on
  // an edge, and every tick reads that one value. Its own `kind` so
  // the axis can say so instead of claiming a ruler it did not use.
  if (hi === lo) return { kind: "flat", lo, hi, at: () => 0.5, tick: () => lo };

  // **The axis runs from the smallest reading to the largest, not from
  // zero.** Below the minimum is a stretch no point can ever be in.
  const linear: Ruler = {
    kind: "linear",
    lo,
    hi,
    at: (x) => Math.max(x - lo, 0) / (hi - lo),
    tick: (f) => lo + f * (hi - lo),
  };
  // Ticks at even fractions of the **value**; the transform decides
  // where they sit, so they crowd towards the top — and that crowding
  // is the evidence the ruler is not linear.
  const sqrt: Ruler = {
    kind: "sqrt",
    lo,
    hi,
    at: (x) => Math.sqrt(Math.max(x - lo, 0) / (hi - lo)),
    tick: (f) => lo + f * (hi - lo),
  };
  // `log1p`, not `log`: `log 0` is undefined and 0 is common here,
  // while `log1p(0) = 0` draws the smallest reading at the origin. The
  // argument is `v / s` because `log1p` only compresses above 1, where
  // `s` is the smallest positive step above the floor, held at or over
  // `(hi - lo) / 1e6` so one tiny outlier cannot stretch the ruler.
  //
  // **Ticks here are even fractions of the axis**, and the labels come
  // out uneven — the same evidence on the other channel, and it has to
  // be that way round: even value fractions on a log axis put every
  // tick but the first in the top fifth, leaving the stretch that
  // holds all the points unlabelled.
  const gaps = v.map((x) => x - lo).filter((x) => x > 0);
  const s = Math.max(gaps.length > 0 ? gaps[0] : 1, (hi - lo) / 1e6);
  const span = Math.log1p((hi - lo) / s) || 1;
  const log: Ruler = {
    kind: "log",
    lo,
    hi,
    at: (x) => Math.log1p(Math.max(x - lo, 0) / s) / span,
    tick: (f) => lo + s * Math.expm1(f * span),
  };

  // **The ruler that keeps the closest pair furthest apart wins, and
  // ties go to the plainer one** — so an axis is only bent when
  // bending it buys something measurable. The 15% is the margin that
  // decides what counts as 「buys something」: without it a rounding
  // difference in the sixth place would be enough to bend an axis.
  let best = linear;
  let score = spread(v.map(linear.at));
  for (const r of [sqrt, log]) {
    const s2 = spread(v.map(r.at));
    if (s2 > score * 1.15) {
      best = r;
      score = s2;
    }
  }
  return best;
}

/**
 * The ticks of one axis, each with the label it can wear without
 * colliding: the fewest decimals that keep it apart from every other
 * tick's label at that same precision, and that keep a reading above
 * zero from reading as zero (a `0.00%` on an axis whose lowest reading
 * is 0.06% is a label that lies). Two ticks whose labels cannot be
 * told apart at all (a unit ladder's) become one — the second is
 * dropped, never drawn twice.
 *
 * ⚠️ **Moved here from `MachineCard.tsx` 2026-09-08, byte for byte.**
 * Both pages call this one; unlike `ruler` above there is no judgment
 * in it to diverge over, so a second copy would only be a second place
 * for the same rule to rot.
 */
export function tickSet(
  r: { tick: (f: number) => number },
  a: { tick: (v: number, d: number) => string; fold?: boolean },
  fractions: number[],
) {
  const vs = fractions.map((f) => r.tick(f));
  const at = (i: number, d: number) => a.tick(vs[i], d);
  const set = vs.map((v, i) => {
    // Zero is 「0」, at no decimals: nothing else may read as zero (the
    // rule below), so it cannot collide, and 「0.000%」 is a zero
    // dressed as a measurement.
    if (v === 0) return { v, label: at(i, 0) };
    let d = 0;
    const clashes = () =>
      vs.some((_, j) => j !== i && at(j, d) === at(i, d)) || parseFloat(at(i, d)) === 0;
    while (d < 6 && clashes()) d += 1;
    return { v, label: at(i, d) };
  });
  return a.fold ? set.filter((t, i) => set.findIndex((o) => o.label === t.label) === i) : set;
}
