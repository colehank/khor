// Wording helpers on top of the generated catalog. The display word is
// looked up at the last moment; unknown keys echo — an old face must
// still render a session whose kind it has never heard of.
import { avatar, category, cli, state } from "./gen/catalog";

export function word(key: string): string {
  return (state as Record<string, string>)[key] ?? key;
}

/**
 * The word for one face option — a factory palette's id, a variant key
 * or a shape key, exactly as the node sent it.
 *
 * Unknown keys echo, same as above. It should never fire: a Rust gate
 * (`every_face_option_has_a_word_and_every_word_names_an_option`) walks
 * both directions across this section. The echo is what a *newer* node
 * talking to an older screen falls back to — a palette named `okabe` on
 * a button is ugly, a button with no name at all is unusable.
 */
export function faceWord(key: string): string {
  // Guarded on the type rather than cast through, because this section
  // is not all plain words: `axis-palette-slot` takes an argument, so a
  // straight lookup can hand back a function, and a function rendered
  // into JSX is a blank where a name belongs.
  const found: unknown = (avatar as Record<string, unknown>)[key];
  return typeof found === "string" ? found : key;
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
