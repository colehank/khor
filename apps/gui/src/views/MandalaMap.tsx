import type { DeviceRow, Strain } from "@/api";
// Aliased: `Seat` the name belongs to the component below (one machine
// in its place); the binding is the place itself.
import type { Seat as SeatPoint } from "@/gen/bindings/Seat";
import { MachineAvatar } from "@/components/Avatar";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { ago, hopWord } from "@/words";

/**
 * The network as an ego graph: this machine in the middle, everyone
 * else around it, and an edge from here to each of them.
 *
 * # Every mark on it is a fact khor holds
 *
 * There are six, and there are only six because khor knows six things
 * about a machine it has met:
 *
 * | on screen | the fact |
 * |---|---|
 * | the face | that machine's own palette × its own key (`khor_core::avatar`) |
 * | the name | what it calls itself in the device table |
 * | in the middle, and larger | it is this machine |
 * | how long ago, or never | the age of the last answer it gave |
 * | the readings | what it said it was doing, in that same answer |
 * | the edge to it | which road this machine reaches it by (`DeviceRow::hop`) |
 *
 * **Every edge starts here**, and that is the data's shape, not a
 * style: the hop reading is first-hand — the same word the machine card
 * prints — and it only exists from this machine outward. What two
 * *other* machines have between them, this one does not know, so no
 * edge is drawn between them: 不知道就不画, never guessed. (The old
 * head of this file said "no line is drawn" outright; the hop reading
 * is what overturned it — khor now does know this one thing about
 * reachability, exactly one edge's worth per machine.)
 *
 * The edge wears its word (looked up from the same catalog keys the
 * card uses) and one of two strokes: **direct is solid, relay is
 * dashed**; every no-reading state (没在连 / serve 没在跑 / 说不出) is
 * the same faint stroke, because the visual channel only has to say
 * "there is a road" / "the road borrows a relay" / "no road to show" —
 * the word carries the rest, and inventing a stroke per word would be
 * five styles for three facts.
 *
 * **Where seats sit is still `khor_core::map`'s** — a picture two
 * faces work out separately is two pictures. Edges take explicit
 * endpoints rather than assuming "centre to rim": when sponsored
 * reaching lands (本机—A—B two-segment routes), a middle machine's
 * seat becomes a joint and `Edge` already speaks in arbitrary pairs —
 * the layout library grows the seat, this file only paints one more
 * segment. Not pre-built, just not walled off.
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
        {/* The edges, under everything: the seats' percentages are the
            viewBox's own coordinates (both speak 0–100 of the square),
            and `non-scaling-stroke` keeps the line's weight in screen
            units however large the square is. `preserveAspectRatio`
            can be none because the box is square by construction. */}
        {me && others.length > 0 && (
          <svg
            data-mesh-edges
            aria-hidden="true"
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
            className="pointer-events-none absolute inset-0 size-full"
          >
            {others.map((row) => (
              <Edge key={row.id} from={me.seat} to={row.seat} row={row} />
            ))}
          </svg>
        )}
        {/* Each edge's word, at its midpoint — HTML rather than SVG
            text so the type tokens reach it. On a small chip of ground
            so the line does not strike through its own name. */}
        {me &&
          others.map(
            (row) =>
              row.hop && (
                <span
                  key={row.id}
                  data-edge-word={row.id}
                  data-hop={row.hop}
                  className="absolute -translate-x-1/2 -translate-y-1/2 rounded-sm bg-background px-1 text-xs text-muted-foreground"
                  style={{
                    left: `${(me.seat.left + row.seat.left) / 2}%`,
                    top: `${(me.seat.top + row.seat.top) / 2}%`,
                  }}
                >
                  {hopWord(row.hop)}
                </span>
              ),
          )}
        <Seat row={me} big onOpen={onOpen} selected={selected} />
        {others.map((row) => (
          <Seat key={row.id} row={row} onOpen={onOpen} selected={selected} />
        ))}
      </div>
    </section>
  );
}

/* The edges' line weights and the relay's dash rhythm, named the way
   Ring.tsx names its STROKE: screen units (non-scaling-stroke), one
   place. The no-reading stroke is thinner as well as fainter, so the
   difference survives a screenshot in one channel colour-blindness
   cannot take away. */
const EDGE_STROKE = 1.5;
const EDGE_STROKE_FAINT = 1;
const RELAY_DASH = "6 5";

/**
 * One first-hand edge: this machine to one it knows.
 *
 * The three visual states carry three facts (the file head's table):
 * `direct` is a solid line — the hole got punched; `relay` is dashed —
 * the road borrows a middleman; everything else (没在连 / serve 没在跑
 * / 说不出 / a hop this build does not know) is one faint stroke,
 * because "no road to show" is one fact wearing several words, and the
 * word itself rides the edge's midpoint chip. Never `hidden`: a member
 * with no road is still a member, and a missing edge would read as a
 * machine the map forgot.
 *
 * Endpoints are explicit `Seat` pairs, not "centre to rim" — the shape
 * a future two-segment sponsored route needs (file head).
 */
function Edge({ from, to, row }: { from: SeatPoint; to: SeatPoint; row: DeviceRow }) {
  const road = row.hop === "direct" ? "direct" : row.hop === "relay" ? "relay" : "none";
  return (
    <line
      data-edge={row.id}
      data-road={road}
      x1={from.left}
      y1={from.top}
      x2={to.left}
      y2={to.top}
      vectorEffect="non-scaling-stroke"
      strokeWidth={road === "none" ? EDGE_STROKE_FAINT : EDGE_STROKE}
      strokeDasharray={road === "relay" ? RELAY_DASH : undefined}
      className={cn(
        "stroke-muted-foreground",
        road === "none" ? "opacity-25" : "opacity-50",
      )}
    />
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
        {row.vitals && <SeatVitals row={row} />}
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

/**
 * The readings under a seat, in words rather than rings.
 *
 * The same numbers the machine row's ring stack draws — CPU, 内存, and
 * a GPU when the machine has one — as percentages, because that is what
 * the ring's front face says too and the two screens must agree about
 * one machine. The disk stays off the seat: this picture answers "what
 * is everyone doing", and a disk filling up is not doing.
 *
 * How current the numbers are is the age line's job, one line up —
 * printing it again here would be the same fact twice on one caption.
 * Strain wears the strain token and critical adds weight, exactly the
 * card's channels (`MachineCard`): one hue for one thing, escalation by
 * weight, never a borrowed state colour.
 */
function SeatVitals({ row }: { row: DeviceRow }) {
  const { vitals: v } = row.vitals!;
  const mem = v.mem.total > 0 ? Math.round((v.mem.used / v.mem.total) * 100) : null;
  return (
    <span
      data-seat-vitals
      className="flex max-w-full flex-wrap items-baseline justify-center gap-x-1.5 text-xs text-muted-foreground"
    >
      <span data-seat-cpu>{`${cli.vitals_name_cpu} ${Math.round(v.cpu_pct)}%`}</span>
      {mem !== null && (
        <Micro name={cli.vitals_name_mem} pct={mem} strain={row.vitals!.mem_strain} anchor="mem" />
      )}
      {v.gpu && (
        <span data-seat-gpu>{`${cli.vitals_name_gpu} ${Math.round(v.gpu.util_pct)}%`}</span>
      )}
    </span>
  );
}

/** One reading that can carry a strain word. */
function Micro({
  name,
  pct,
  strain,
  anchor,
}: {
  name: string;
  pct: number;
  strain: Strain | null;
  anchor: string;
}) {
  return (
    <span
      data-seat-vital={anchor}
      data-strain={strain ?? undefined}
      className="data-[strain]:text-strain data-[strain=critical]:font-label"
    >
      {`${name} ${pct}%`}
    </span>
  );
}

