// Wording helpers on top of the generated catalog. The display word is
// looked up at the last moment; unknown keys echo — an old face must
// still render a session whose kind it has never heard of.
import { category, cli, state } from "./gen/catalog";

export function word(key: string): string {
  return (state as Record<string, string>)[key] ?? key;
}

/**
 * A group heading, from the key the node put on the row.
 *
 * The prefix says what the rest of the key is, so this dispatches
 * instead of guessing (`khor_node::list` module head): a state key and a
 * category key are looked up, a machine name is printed as it stands.
 * Without the prefixes a machine called `busy` would come out 忙碌.
 *
 * An unknown category echoes, which is not a fallback but the design:
 * vendor names are proper nouns with no catalog entry, so echoing them
 * *is* the translation.
 */
export function groupLabel(group: string): string {
  if (group === "pin") return cli.group_pinned;
  if (group.startsWith("state:")) return word(group.slice("state:".length));
  if (group.startsWith("cat:")) {
    const name = group.slice("cat:".length);
    const table = category as Record<string, string>;
    return name === "" ? table.unknown : (table[name] ?? name);
  }
  return group.startsWith("dev:") ? group.slice("dev:".length) : group;
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
