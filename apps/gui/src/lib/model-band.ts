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
 * **How far an older generation is mixed toward the page**, at the
 * oldest step. Below this the faintest band stops being separable from
 * the paper; above it, a family with four generations has two steps
 * nobody can tell apart.
 */
const FADE_MAX = 58;

/**
 * A colour per model, as a CSS value ready for `fill`.
 *
 * `models` is **every model the account has**, not the window's — see
 * this file's head. Feed it the windowed subset and the colours move
 * when the window does.
 */
export function bandsFor(models: (string | null)[]): Map<string | null, string> {
  // **Which slot each model sits in.** The four khor knows the names of
  // are families; the unnamed row is a slot of its own; and a name khor
  // does not recognise is **its own slot rather than a shared 「other」**
  // — khor cannot tell whether two unfamiliar names are two generations
  // of one family or two families, and guessing either way draws a
  // relationship nobody established.
  const UNNAMED = "\u0000unnamed";
  const slots = new Map<string, (string | null)[]>();
  for (const m of models) {
    const at = m === null ? UNNAMED : (famOf(m) ?? m.toLowerCase());
    if (!slots.has(at)) slots.set(at, []);
    const seen = slots.get(at)!;
    if (!seen.includes(m)) seen.push(m);
  }
  // **Which hue each slot gets.** The four keep theirs for good. Beyond
  // them there are not more hues to hand out — the count is in
  // `tokens.css` — so the first unrecognised slot takes the one spare
  // hue and any further one takes the neutral tail, **ordered by name
  // so it is the same choice on every machine and in every window**.
  // Sharing the tail loses information, and it is the honest loss: a
  // hue those bands do not own would say they belong to a family they
  // do not.
  const strangers = [...slots.keys()]
    .filter((at) => at !== UNNAMED && !(KNOWN as readonly string[]).includes(at))
    .sort();
  const hueOf = (at: string): string => {
    if ((KNOWN as readonly string[]).includes(at)) return `var(--model-${at})`;
    // The unnamed row is neutral for the same reason the tail is:
    // 「nobody said which model」 is not a family, and a hue would say
    // it were one.
    if (at === UNNAMED) return "var(--model-tail)";
    return at === strangers[0] ? "var(--model-spare)" : "var(--model-tail)";
  };

  const out = new Map<string | null, string>();
  for (const [at, names] of slots) {
    const hue = hueOf(at);
    const order = [...names].sort((a, b) => newerFirst(a ?? "", b ?? ""));
    order.forEach((name, k) => {
      // The newest keeps the hue at full strength; each older step is
      // mixed further toward the page. **Cut by how many generations
      // this slot actually has** — a fixed ladder of three gives a
      // fourth generation the third's colour, which is two models drawn
      // as one band, and a band drawn as another band is the same
      // screen as a band drawn as nothing.
      const t = order.length < 2 ? 0 : Math.round((FADE_MAX * k) / (order.length - 1));
      // Toward the paper the bands are drawn on (`--card`; it was
      // `--background` until 2026-09-30, when the ground became the
      // frame round the board and stopped being what is behind a chart).
      out.set(name, t === 0 ? hue : `color-mix(in srgb, ${hue} ${100 - t}%, var(--card))`);
    });
  }
  return out;
}
