// Lab only (lane ux-q2): the proposed model colours. The four Claude
// families keep their hues and generation steps (`lib/model-band`); every
// other model id takes a slot of `--model-series-1..6` (lab/quota.css),
// so no two series share the grey `--model-tail` any more.
//
// **Stable across pages**: the slots are handed out over every model id
// on the whole board (every account's, sorted by name), not over one
// account's — the same id wears the same slot on every page and in
// every window. Past six ids the slots repeat with a lightness step,
// the families' own trick.
import { famOf } from "@/lib/model-band";

const KNOWN = ["opus", "fable", "sonnet", "haiku"];
const SERIES = 6;
const FADE_MAX = 58;

export function boardBands(all: (string | null)[]): Map<string | null, string> {
  const out = new Map<string | null, string>();
  const families = new Map<string, string[]>();
  const strangers: string[] = [];
  for (const m of new Set(all)) {
    if (m === null) continue;
    const f = famOf(m);
    if (f) {
      if (!families.has(f)) families.set(f, []);
      families.get(f)!.push(m);
    } else strangers.push(m);
  }
  for (const [f, names] of families) {
    // Newest first by name, the same rule `lib/model-band` keeps.
    const order = [...names].sort((a, b) => b.localeCompare(a, undefined, { numeric: true }));
    order.forEach((name, k) => {
      const t = order.length < 2 ? 0 : Math.round((FADE_MAX * k) / (order.length - 1));
      const hue = `var(--model-${KNOWN.includes(f) ? f : "tail"})`;
      out.set(name, t === 0 ? hue : `color-mix(in srgb, ${hue} ${100 - t}%, var(--card))`);
    });
  }
  strangers.sort((a, b) => a.localeCompare(b));
  strangers.forEach((name, i) => {
    const hue = `var(--model-series-${(i % SERIES) + 1})`;
    const lap = Math.floor(i / SERIES);
    out.set(name, lap === 0 ? hue : `color-mix(in srgb, ${hue} ${100 - 35 * lap}%, var(--card))`);
  });
  // 「Nobody said which model」 is not a series: it stays the neutral.
  out.set(null, "var(--model-tail)");
  return out;
}
