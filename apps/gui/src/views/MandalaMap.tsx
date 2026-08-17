import type { DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { ago } from "@/words";

/**
 * The network as a mandala: this machine in the middle, everyone else
 * around it.
 *
 * # Every mark on it is a fact khor holds
 *
 * There are four, and there are only four because khor knows four things
 * about a machine it has met:
 *
 * | on screen | the fact |
 * |---|---|
 * | the face | that machine's own palette × its own key (`khor_core::avatar`) |
 * | the name | what it calls itself in the device table |
 * | in the middle, and larger | it is this machine |
 * | how long ago, or never | the age of the last answer it gave |
 *
 * **No line is drawn between any two of them**, and **where they sit is
 * not worked out here** — both are `khor_core::map`, which carries the
 * reasoning: khor knows membership and not reachability, so there are no
 * edges and one ring, and the arrangement derives in the library because
 * a picture two faces work out separately is two pictures. This file
 * paints what it is handed, the way `Avatar.tsx` does.
 *
 * # Three states, and no invented threshold between them
 *
 * A member is either this machine (middle, no age — it answers by
 * looking), a machine khor has heard from (the age of that answer, in
 * words), or one it has never reached (dimmed, and it says so).
 *
 * **The dimming is categorical, not a timer.** "Offline" would need a
 * number of seconds past which khor declares a machine gone, and nobody
 * has made that judgment — a threshold set wrong is an alarm that
 * teaches people to ignore alarms, the same reasoning that keeps the
 * readings off severity colours. What khor has is an age, so the age is
 * what it shows, and the reader does the comparing. Never-reached is a
 * different kind of thing rather than a large number, so it looks
 * different (docs/SESSION.md 离线不是第七个词).
 *
 * # No title
 *
 * docs/UX.md. The region's name rides its `aria-label`, which takes no
 * room and is read aloud.
 */
export function MandalaMap({
  rows,
  onOpen,
  selected,
}: {
  rows: DeviceRow[];
  /**
   * Where a face goes when it is pressed, when there is anywhere.
   *
   * A prop rather than something read from a landing name, exactly as
   * `DevicesList` explains: what decides is whether a destination
   * exists, and the only code that knows is the code that would take you
   * there. Without one, the faces are pictures — no pointer, no hover,
   * no button — because an affordance that answers nothing teaches
   * people to stop trying the ones that do.
   */
  onOpen?: (row: DeviceRow) => void;
  /** The machine whose card is open, if this map sits beside one. */
  selected?: string | null;
}) {
  // Which one is the middle is a fact about the machine, not about
  // where it was placed — mandala keeps those two apart even where they
  // always coincide (its docs/MAP.md), and so does this: `me` picks the
  // machine, `seat` says where it goes, and neither is read off the
  // other.
  const me = rows.find((r) => r.me) ?? null;
  const others = rows.filter((r) => !r.me);
  return (
    <section
      data-mandala-map
      aria-label={gui.mesh_map}
      className="flex h-full min-w-0 items-center justify-center overflow-auto p-6"
    >
      {/* A square, so the ring is a ring at any width — and as large as
          the pane allows rather than capped at some size.
          **A mandala fills its field**, and a capped one sits stranded in
          the middle of a wide pane looking like something that failed to
          load. The height is the cap, which self-limits: the picture is
          never taller than the window it is in. */}
      <div className="relative aspect-square max-h-full w-full">
        <Seat row={me} big onOpen={onOpen} selected={selected} />
        {others.map((row) => (
          <Seat key={row.id} row={row} onOpen={onOpen} selected={selected} />
        ))}
      </div>
    </section>
  );
}

/**
 * One machine in its seat.
 *
 * **The face is what sits on the ring, and the caption hangs off it.**
 * The first version made the whole column — face, name, age — the thing
 * that was centred on the seat point, and the middle seat has no age
 * line, so its column was shorter and its face landed a few pixels
 * lower than everyone else's. Three faces that are nearly but not quite
 * on one circle look like a rendering fault. So the positioned box is
 * exactly the face, and the text is taken out of the flow beneath it.
 *
 * `null` is drawn as nothing at all rather than as a blank face: the
 * middle is empty for exactly as long as the first answer takes to
 * arrive, and a face-shaped placeholder in the middle of a mandala
 * would be read as a machine.
 */
function Seat({
  row,
  big,
  onOpen,
  selected,
}: {
  row: DeviceRow | null;
  big?: boolean;
  onOpen?: (row: DeviceRow) => void;
  selected?: string | null;
}) {
  if (!row) return null;
  const reached = row.vitals !== null;
  const body = (
    <>
      {/* Exactly one size class, never a base plus an override:
          tailwind-merge does not read `size-avatar-xl` as a size, so
          `cn()` would leave both in the markup and let the stylesheet's
          order decide (docs/handoff 坑节: `cn()` 盖不掉自定义尺寸类).
          Nothing needs overriding here anyway — the box around this
          *is* the face. */}
      <MachineAvatar face={row.face} className={big ? "size-avatar-xl" : "size-avatar-lg"} />
      {/* Out of the flow, so the box this sits in stays exactly the size
          of the face — see the note above. */}
      <span className="absolute left-1/2 top-full flex w-32 -translate-x-1/2 flex-col items-center pt-2 text-center">
        <span data-seat-name className="max-w-full truncate text-sm">
          {row.name}
        </span>
        {/* The age, or the third state. **This machine says neither**:
            it answers by looking, so there is no age to report and
            nothing to be unsure about. */}
        {!row.me &&
          (reached ? (
            <span data-seat-age className="max-w-full truncate text-xs text-muted-foreground">
              {cli.vitals_taken(ago(Date.now() - row.vitals!.age_ms))}
            </span>
          ) : (
            <span data-seat-never className="max-w-full truncate text-xs text-muted-foreground">
              {cli.vitals_never}
            </span>
          ))}
      </span>
    </>
  );
  const place = { left: `${row.seat.left}%`, top: `${row.seat.top}%` };
  const shell = cn(
    "absolute -translate-x-1/2 -translate-y-1/2",
    // Arrival: the mandala assembles itself once, and a machine that
    // joins later arrives the same way. React keeps the elements it
    // already has, so nothing replays on a poll — only a genuinely new
    // seat animates.
    "motion-safe:animate-in motion-safe:fade-in motion-safe:zoom-in-95",
    // Dimmed for the one categorical case, never on a timer.
    !row.me && !reached && "opacity-60",
  );
  return onOpen ? (
    <button
      type="button"
      data-seat={row.id}
      data-seat-me={row.me}
      data-on={selected === row.id}
      onClick={() => onOpen(row)}
      className={cn(
        shell,
        // **A scale and not a tinted box.** A face is whatever shape that
        // machine chose — round, or a rounded square — so a rectangular
        // hover plate would sit wrong behind half of them. Growing works
        // on any shape, and the selected seat stays grown so that "which
        // one is open" is answered without a second kind of mark.
        "cursor-pointer motion-safe:transition-transform hover:scale-110 data-[on=true]:scale-110",
        // The face is round or nearly so, so the focus ring follows it
        // rather than boxing it.
        "rounded-full outline-none focus-visible:ring-2 focus-visible:ring-ring",
      )}
      style={place}
    >
      {body}
    </button>
  ) : (
    <div data-seat={row.id} data-seat-me={row.me} className={shell} style={place}>
      {body}
    </div>
  );
}
