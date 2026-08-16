// Wording helpers on top of the generated catalog. The display word is
// looked up at the last moment; unknown keys echo — an old face must
// still render a session whose kind it has never heard of.
import { cli, state } from "./gen/catalog";

export function word(key: string): string {
  return (state as Record<string, string>)[key] ?? key;
}

/** How long ago, in the catalog's units. */
export function ago(ms: number): string {
  const s = Math.max(0, Math.floor((Date.now() - ms) / 1000));
  if (s < 60) return cli.age_seconds(s);
  const m = Math.floor(s / 60);
  if (m < 60) return cli.age_minutes(m);
  const h = Math.floor(m / 60);
  if (h < 24) return cli.age_hours(h);
  return cli.age_days(Math.floor(h / 24));
}
