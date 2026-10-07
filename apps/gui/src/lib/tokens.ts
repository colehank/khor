/**
 * **A token count as a person reads it** — the one formatter for every
 * place the face abbreviates tokens: the usage half's axes, sums, table
 * and tooltips, the composer's context reading.
 *
 * **Base 1000, not 1024** — these are tokens, and a thousand of them is a
 * thousand. `bytes` divides by 1024 because it measures memory; sharing
 * that function would have made "1.0M tokens" mean 1,048,576 of them.
 *
 * **K / M / B in every language** (UX-50, the owner 2026-10-08): billions
 * are B, not the SI G a byte size wears, and a Chinese face does not
 * switch to its own myriads either — one spelling of a count, whatever
 * the language. Byte sizes keep their own units.
 */
export const TOKEN_UNITS = ["", "K", "M", "B", "T"] as const;

/** `d` is the decimals past the first step, so an axis can ask for more
    where its labels would otherwise collide. Under a thousand: the whole
    number. */
export function tokens(n: number, d = 1): string {
  let v = n;
  let u = 0;
  while (Math.abs(v) >= 1000 && u + 1 < TOKEN_UNITS.length) {
    v /= 1000;
    u += 1;
  }
  return u === 0 ? `${Math.round(n)}` : `${v.toFixed(d)}${TOKEN_UNITS[u]}`;
}

/** The glance form (`82K`, `1.2M`): whole thousands, millions and up with
    one decimal under ten of them and none past. */
export function tokensShort(n: number): string {
  if (n < 1_000) return String(n);
  let v = n;
  let u = 0;
  while (Math.abs(v) >= 1000 && u + 1 < TOKEN_UNITS.length) {
    v /= 1000;
    u += 1;
  }
  if (u === 1) return `${Math.round(v)}${TOKEN_UNITS[u]}`;
  return `${v.toFixed(v >= 10 ? 0 : 1).replace(/\.0$/, "")}${TOKEN_UNITS[u]}`;
}
