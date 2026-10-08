/**
 * **Which colour a model's band is** — hue for the family, lightness
 * for the generation inside it.
 *
 * The values and the reasoning behind the split live on `--model-*` in
 * `styles/tokens.css`; this file is the arithmetic, and it has two
 * jobs the tokens cannot do.
 *
 * # 1. A model this face has never seen must land in the right hue
 *
 * The family is read out of the **name**, so `claude-opus-6` is opus
 * the first time it appears rather than 「the next free colour」. Colour
 * handed out in arrival order would mean the same model wearing a
 * different band on two machines, and nobody could compare two
 * screenshots.
 *
 * # 2. Switching the window must not recolour anything
 *
 * ⚠️ **The design this was built from got this wrong, and it is worth
 * writing down because the bug is invisible.** It worked the
 * generations out from whichever models were on screen, ordered by how
 * much each had spent — and called the first one 「the newest」. Two
 * things follow, neither of which shows up as an error:
 *
 *   - the busiest model is not the newest one (`fable-5` outspends
 *     `fable-5-1` here by an order of magnitude), so 「newest at full
 *     strength」 was never what it did; and
 *   - **the set on screen changes when the reader presses 7 天**, so a
 *     model's colour moves under a control that is only supposed to
 *     decide which days are counted. Legend, bars, scatter and table
 *     all move together, so nothing looks broken — it just is not the
 *     same picture any more.
 *
 * So the order comes from the **name** (`age`, below) and is worked
 * out over **every model the account has ever had**, not over the
 * window. The window then cannot touch it: a colour is a fact about a
 * model, and the window is a question about days.
 */

/** The four families khor knows the names of, and their tokens. */
const KNOWN = ["opus", "fable", "sonnet", "haiku"] as const;

/**
 * The family a model's name puts it in, or `null` for one khor does
 * not recognise — **including the unnamed row**, whose model the
 * provider never wrote down (`UsageModelDay::model`).
 *
 * Matched on the name containing the word, not on a prefix: the
 * vendors spell these `claude-opus-5`, `claude-haiku-4-5-20251001` and
 * (through new-api) plain `opus-4-8`, and a prefix rule would have to
 * be rewritten for each.
 */
export function famOf(model: string | null): string | null {
  if (model === null) return null;
  const name = model.toLowerCase();
  return KNOWN.find((f) => name.includes(f)) ?? null;
}

/**
 * How old a model is **within its family**, as a sortable key read off
 * its name — never off its usage.
 *
 * The numeric run after the family word is the version (`opus-4-8` is
 * `[4, 8]`, `haiku-4-5-20251001` is `[4, 5, 20251001]`), compared
 * component by component with a missing component counting as 0 — so
 * `fable-5-1` is newer than `fable-5`, which is the reading a person
 * gives those two names.
 *
 * A trailing word (`-medium`) is a **variant, not a generation**: it
 * sorts after the plain name at the same version rather than being
 * given a number of its own, because 「medium」 says how hard the model
 * was asked to think, not when it shipped.
 *
 * **A name with no numbers at all sorts last**, not first: an unknown
 * spelling is not evidence of being new.
 */
function age(model: string): { v: number[]; variant: string } {
  const parts = model.toLowerCase().split(/[-_.]/);
  const fam = parts.findIndex((p) => (KNOWN as readonly string[]).includes(p));
  const rest = parts.slice(fam + 1);
  const v: number[] = [];
  let i = 0;
  for (; i < rest.length && /^\d+$/.test(rest[i]); i += 1) v.push(Number(rest[i]));
  return { v, variant: rest.slice(i).join("-") };
}

function newerFirst(a: string, b: string): number {
  const x = age(a);
  const y = age(b);
  // A name that carries no version at all goes last, whichever side of
  // the comparison it is on.
  if (x.v.length === 0 !== (y.v.length === 0)) return x.v.length === 0 ? 1 : -1;
  for (let i = 0; i < Math.max(x.v.length, y.v.length); i += 1) {
    const d = (y.v[i] ?? 0) - (x.v[i] ?? 0);
    if (d !== 0) return d;
  }
  if (x.variant !== y.variant) return x.variant.localeCompare(y.variant);
  // Never a tie: two different names must not be able to land on the
  // same step, or two models share a band and neither can be read.
  return a.localeCompare(b);
}

/**
 * **The series slots** (UX-53, the owner 2026-10-08: 「非订阅的，不同 model
 * id 的颜色都是灰色」): a model whose name is none of the four families
 * takes a slot of `--model-series-1..6` — never the grey any more.
 * ⚠️ 墓碑: the first such slot took `--model-spare` and every further one
 * the neutral `--model-tail`, so an API account's four models were one
 * green and three greys.
 */
const SERIES = 6;

/**
 * **Which slots sit too close to a family hue** — measured, not guessed
 * (UX-53: Machado 2009 at full severity, OKLab ΔE×100, both themes): a
 * slot is listed against a family when the pair falls under 8 for a red
 * or green deficiency or under 15 for full colour vision. A relay account
 * (new-api, OpenRouter) can draw a family and a series model on one
 * chart; there the series model moves off a listed slot (`bandsFor`).
 */
const CLOSE: Record<number, readonly string[]> = {
  1: ["haiku", "fable"],
  3: ["fable", "haiku", "sonnet"],
  4: ["opus", "sonnet"],
  5: ["fable", "haiku", "sonnet"],
  6: ["opus"],
};

/**
 * A colour per model, as a CSS value ready for `fill`.
 *
 * `models` is **every model this account has**, not the window's — see
 * this file's head. Feed it the windowed subset and the colours move
 * when the window does. `board` is **every model on the whole 额度
 * board** (every account's): the series slots are handed out over it,
 * by name, so one id wears one slot on every page (UX-53, 「same model,
 * same colour」). Absent, the account's own models stand in for it.
 */
export function bandsFor(models: (string | null)[], board: (string | null)[] = models): Map<string | null, string> {
  // **Which slot each model sits in.** The four khor knows the names of
  // are families, and the unnamed row is the neutral. A name khor does not
  // recognise is its own series slot rather than a shared 「other」 — khor
  // cannot tell whether two unfamiliar names are two generations of one
  // family or two families, and guessing either way draws a relationship
  // nobody established.
  const UNNAMED = "\u0000unnamed";
  const families = new Map<string, (string | null)[]>();
  const strangers: string[] = [];
  for (const m of new Set(models)) {
    const fam = m === null ? UNNAMED : famOf(m);
    if (fam === null) strangers.push(m as string);
    else {
      if (!families.has(fam)) families.set(fam, []);
      families.get(fam)!.push(m);
    }
  }
  const out = new Map<string | null, string>();
  for (const [fam, names] of families) {
    if (fam === UNNAMED) {
      // 「Nobody said which model」 is not a family; a hue would say it were.
      out.set(null, "var(--model-tail)");
      continue;
    }
    const hue = `var(--model-${fam})`;
    const order = [...names].sort((a, b) => newerFirst(a ?? "", b ?? ""));
    order.forEach((name, k) => {
      // The newest keeps the hue at full strength; each older step is
      // mixed further toward its oldest, **cut by how many generations
      // this family actually has** — a fixed ladder of three gives a
      // fourth generation the third's colour. The oldest is the family's
      // own token (`--model-<family>-old`): on the dark theme each stops
      // where it still reads 3:1 on the black paper (UX-53; ⚠️ 墓碑: 58%
      // toward `--card` for every family and theme, which left dark
      // opus-4-8 at 1.74 — 「看不见」).
      const t = order.length < 2 ? 0 : Math.round((100 * k) / (order.length - 1));
      out.set(name, t === 0 ? hue : `color-mix(in srgb, ${hue} ${100 - t}%, var(--model-${fam}-old))`);
    });
  }
  // **The board's order decides the slot**, by name, so the same id lands
  // in the same slot on every page and in every window.
  const all = [...new Set(board.filter((m): m is string => m !== null && famOf(m) === null))].sort();
  const home = (m: string) => (all.indexOf(m) < 0 ? all.length + strangers.indexOf(m) : all.indexOf(m)) % SERIES + 1;
  const lap = (m: string) => Math.floor(Math.max(0, all.indexOf(m)) / SERIES);
  // **On this account, off a slot that sits by a family on the same
  // chart** (`CLOSE`), and off a slot another of its models holds — the
  // first slot, in order, that is clear of both. Only a relay account
  // that carries both kinds ever moves; when no slot is clear the model
  // keeps its own, and the legend and the labels carry the difference.
  const present = [...families.keys()];
  const near = (slot: number) => (CLOSE[slot] ?? []).some((f) => present.includes(f));
  const taken = new Set<number>();
  for (const m of [...strangers].sort()) {
    let slot = home(m);
    if (near(slot) || taken.has(slot)) {
      for (let k = 1; k <= SERIES; k += 1) {
        if (!near(k) && !taken.has(k)) {
          slot = k;
          break;
        }
      }
    }
    taken.add(slot);
    const hue = `var(--model-series-${slot})`;
    // Past six names on one board the slots come round again, one step
    // toward the paper — the families' own device for 「related, not the
    // same」, and the honest one: no seventh hue clears the floors.
    out.set(m, lap(m) === 0 ? hue : `color-mix(in srgb, ${hue} 65%, var(--card))`);
  }
  return out;
}
