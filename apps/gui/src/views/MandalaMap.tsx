import type { DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { ago } from "@/words";

/**
 * The network as a mandala: this machine at the middle, everyone else
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
 * **And no line is drawn between any two of them, which is the judgment
 * this picture is built around.** khor knows *membership* — one pairing
 * puts a machine in the whole mesh (docs/NET.md) — and it does not know
 * reachability: who can currently dial whom, over which relay, through
 * whose network. A line is the most believable thing a diagram can draw
 * and it would be a claim nobody checked. So the members sit apart, and
 * what holds them together on screen is the arrangement, not an edge.
 *
 * **Position carries nothing either.** The seats are evenly spaced
 * starting at the top, in the order the node listed the machines; the
 * distance from the middle is the same for everyone. Neither angle nor
 * radius means anything, which is why they are both uniform — a varying
 * one would be read as data within about a second of somebody noticing
 * it.
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
        <Seat row={me} at={CENTRE} big onOpen={onOpen} selected={selected} />
        {others.map((row, i) => (
          <Seat
            key={row.id}
            row={row}
            at={seat(i, others.length)}
            onOpen={onOpen}
            selected={selected}
          />
        ))}
      </div>
    </section>
  );
}

/** Where the middle is, as a percentage of the square. */
const CENTRE = { left: 50, top: 50 };

/**
 * How far out the ring sits, as a percentage of the square's side.
 *
 * Under half, because a seat is placed by its centre and carries a face
 * and two lines of text around it — at 50 the outermost ones would hang
 * off the edge. This is a proportion rather than a length, so it holds
 * at every size the square takes.
 */
const RING = 34;

/**
 * The i-th of n seats, going clockwise, arranged symmetrically about the
 * vertical.
 *
 * **Half a step past twelve o'clock, and that half step is the whole
 * reason this is not `i/n` from the top.** With a seat *at* twelve, two
 * machines land directly above and below the middle — three faces in a
 * column, which reads as a list rather than as anything surrounding
 * anything, and two machines is the ordinary case for this product. The
 * half step puts those two at left and right instead, and for three it
 * gives a triangle and for four a diamond. One formula, and the case it
 * rescues is the common one.
 */
function seat(i: number, n: number) {
  const step = (2 * Math.PI) / Math.max(1, n);
  const angle = -Math.PI / 2 + step / 2 + i * step;
  return { left: 50 + Math.cos(angle) * RING, top: 50 + Math.sin(angle) * RING };
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
  at,
  big,
  onOpen,
  selected,
}: {
  row: DeviceRow | null;
  at: { left: number; top: number };
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
  const place = { left: `${at.left}%`, top: `${at.top}%` };
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
