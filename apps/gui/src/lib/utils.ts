import { clsx, type ClassValue } from "clsx";
import { extendTailwindMerge } from "tailwind-merge";

/**
 * This app's own font sizes, told to tailwind-merge.
 *
 * **Without this line every one of them is silently thrown away.**
 * tailwind-merge carries a list of the size names Tailwind ships (`xs`,
 * `sm`, `base`, `lg`…); a `text-*` class whose tail is not on that list
 * it files as a *colour*, because that is what `text-*` usually is. So
 * `text-read` was in the colour group, and any other `text-<something>`
 * in the same `cn()` call superseded it. Measured before this, with the
 * stock merge:
 *
 *   text-read  + text-read-ink         → text-read-ink
 *   text-read  + text-foreground       → text-foreground
 *   text-aux   + text-muted-foreground → text-muted-foreground
 *   text-title + text-foreground       → text-foreground
 *   text-sm    + text-muted-foreground → text-sm text-muted-foreground   ← known name, kept
 *
 * The last row is the tell: nothing is wrong with the *colour* being
 * passed, the wrong half is that this app's own size names are unknown.
 * Renaming the ink does not help — the class being dropped is the size.
 *
 * **What that failure looks like**: a paragraph quietly at the shell's
 * base size with the browser's own leading. No error, no warning, `tsc`
 * clean, the build clean. It was found by breaking a gate on purpose and
 * reading a number that should not have moved.
 *
 * **Two homes, one fact, held equal by a gate.** These names also exist
 * as `--text-*` in `tokens.css`, and this list cannot read that file —
 * it runs in a browser. So `check-type.mjs` asserts the *behaviour* over
 * every `--text-*` the stylesheet declares: each one has to survive
 * beside a colour class. Add a size to the theme and forget this line
 * and that gate goes red. (Same arrangement as `--time-yield` and
 * `lib/tempo.ts` — one name, two homes, a gate holding them together.)
 *
 * `extend` rather than `override`: the built-in names keep working, and
 * the gate holds that half too — this file changes the merge rules for
 * the whole app, so「the new ones work」is only half a claim.
 */
const OWN_SIZES = ["title", "fact", "aux", "hero", "code", "read", "term"];

const merge = extendTailwindMerge({
  extend: { classGroups: { "font-size": [{ text: OWN_SIZES }] } },
});

export function cn(...inputs: ClassValue[]) {
  return merge(clsx(inputs));
}
