/**
 * One share, drawn as a horizontal bar on a track.
 *
 * # Why this is one component and not two
 *
 * Two cards draw it: a machine's parts (a card's share of the GPU, a
 * volume's of the disk) and a subscription's windows (how much of this
 * week is gone). **It lived in `MachineCard` as a local function and was
 * copied into the 額度 page on 2026-09-07**, with the measurements
 * matched by hand so the two would read as one thing.
 *
 * That copy lasted about an hour before being pulled out here, and the
 * reason is worth stating: **「the measurements are deliberately
 * identical」 has nothing keeping it true.** No test compares them, no
 * type ties them, and the two comments saying「keep this in step with
 * the other one」are the only mechanism — and a comment does not go red
 * when somebody edits past it. The cheapest moment to merge a
 * duplicated primitive is while both copies are still new and one
 * person still remembers why they matched.
 *
 * # What the two callers do differently, and why `strain` is optional
 *
 * ⚠️ **The 額度 page does not pass `strain`, and that is a judgment,
 * not an omission.** A disk at 95% *is* a warning, so a machine's bar
 * wears the strain ink. **A quota at 80% is a fact, not a warning** —
 * the same ruling `Ring`'s head carries, and the reason no strain ramp
 * reaches a quota arc either. Wiring strain into the quota bars would
 * quietly overturn it, and it would look like tidying up a caller that
 * forgot an argument.
 *
 * So: if you are about to pass `strain` from the 額度 page, the
 * question to answer first is 「has somebody decided that running out
 * of quota is an alarm?」 — and if they have, this note is where to
 * record it.
 */
import { gaugeColor } from "@/lib/vitals";
import type { Strain } from "@/api";

export function Bar({ pct, strain = null }: { pct: number | null; strain?: Strain | null }) {
  // **Two elements, and the outer one is the cell.** The row gives
  // every direct child the row's height so the three columns line up;
  // a bar that *is* that child inherits 27 and draws as a fat pill
  // (seen on the real face before this — the track swallowed the row
  // and the bars overlapped each other). So the cell is a full-height
  // box and the track is a hairline centred inside it.
  //
  // **`data-part-bar` stays on the cell**, not on the track: the narrow
  // layout moves this whole cell to a line of its own, and a selector
  // written for the row's direct child stops matching the moment the
  // attribute moves a level down — which is how the bars disappeared at
  // 390 the first time this was split.
  //
  // **One pair of names for both callers.** The 額度 page's own gate was
  // reading `data-quota-part-bar` while the copy lived there; it reads
  // these now. Two names for one drawing is how two gates come to
  // disagree about whether a bar is present.
  return (
    <span data-part-bar className="flex w-full items-center">
      <span className="block h-1.5 w-full overflow-hidden rounded-full bg-foreground/10">
        {/* `null` draws the track alone — a reading that could not be
            taken is not a zero. */}
        {pct !== null && (
          <span
            data-part-ink
            className="block h-full rounded-full"
            style={{ width: `${Math.max(0, Math.min(100, pct))}%`, background: gaugeColor(strain) }}
          />
        )}
      </span>
    </span>
  );
}
