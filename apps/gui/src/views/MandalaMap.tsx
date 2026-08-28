import type { DeviceRow } from "@/api";
// Aliased: `Seat` the name belongs to the component below (one machine
// in its place); the binding is the place itself.
import type { Seat as SeatPoint } from "@/gen/bindings/Seat";
import { MachineAvatar } from "@/components/Avatar";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { ageIfOld, hopWord } from "@/words";

/**
 * The network as an ego graph: this machine in the middle, everyone
 * else around it, and an edge from here to each of them.
 *
 * # Every mark on it is a fact khor holds — and no reading is one
 *
 * | on screen | the fact |
 * |---|---|
 * | the face | that machine's own palette × its own key (`khor_core::avatar`) |
 * | the name | what it calls itself in the device table |
 * | in the middle, and larger | it is this machine |
 * | how long ago, or never | the age of the last answer it gave —
 *   **printed only once it is older than a minute** (`ageIfOld`): a seat
 *   that had just replied said 「0 秒前的读数」, which reads as a fault,
 *   not as freshness |
 * | the edge to it | which road this machine reaches it by (`DeviceRow::hop`) |
 *
 * **There used to be a sixth: three unlabelled bars under every name,
 * CPU / 内存 / GPU.** The user's word for them was「看不懂,可以取消」
 * (2026-08-28), and the judgment they were built on has been replaced
 * rather than patched. That judgment was 「概览用画的,细节用写的」 — it
 * took the wall of numbers those bars replaced as the whole problem and
 * assumed a picture of the same reading was free. It is not: **a bar
 * with no label has to be learnt exactly like a number does, and it has
 * one thing less to learn from.** Which of the three is 内存 is not on
 * the screen, and nothing on the screen says it is not.
 *
 * The rule that replaced it is flatter and can be checked by looking:
 * **an overview carries no readings at all.** This map answers 谁在网里
 * · 叫什么 · 通不通 · 多久没说话; how busy a machine is, is a question
 * with a landing one press away — its node's card here, and the machine
 * card a level in, both of which draw rings with the percentages inside
 * them. A ring carries its own full, so it says how full without being
 * learnt; that is why the card may draw what the map may not.
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
 * # The whole picture scales with the window
 *
 * Mandala's map draws everything into one proportional coordinate
 * system (an SVG viewBox sized off the layout) and lets the browser
 * scale the lot — faces, edges, labels move together. The same idea,
 * translated to this file's mixed SVG/HTML: the square is a size
 * container, and every visual dimension speaks **cqmin** (1cqmin = 1%
 * of the square's side) with **tokens as its floor and ceiling** —
 * `clamp(token, Ncqmin, token)`. So the picture breathes with the
 * window, and the type can never shrink below `--text-xs` (the floor
 * the word chips are measured against) nor faces below their smallest
 * token: past the floor the layout gives way (truncation, wrap), not
 * the letterforms. The adaptivity lives here and not in Rust on the
 * library's own reasoning: `Seat` is already a proportion ("one
 * derivation serves a phone and a wide window"), so the seats were
 * never the fixed part — the pixels this file used to hard-wear were.
 * A browser without container units drops the style and falls back to
 * the token classes underneath — smaller, never broken.
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
      className="flex h-full min-w-0 items-center justify-center overflow-hidden p-6"
      // The outer size container: the square's side is measured off
      // this pane. Inline because no utility says it, and one element
      // needs it — a stylesheet rule for one attribute would be a
      // second place to look.
      style={{ containerType: "size" }}
    >
      {/* A square, so the ring is a ring at any width — and as large as
          the pane allows rather than capped at some size. **The side is
          said outright: the pane's smaller dimension less its padding**
          (`100cqmin`). The previous spelling — `w-full max-h-full
          aspect-square` — relied on max-height reining the ratio box
          in, and measured 880 tall inside an 800 pane: the picture
          overflowed into a scroll on every window wider than tall,
          which is most of them. `overflow-hidden` above replaces that
          scroll — with the side computed to fit there is nothing left
          to scroll to, and a stray caption pixel must not summon a
          scrollbar back. Container units measure the content box, so
          the padding is already out of `100cqmin` — measured: a
          further subtraction left a uniform 48 of slack on every
          window size. */}
      <div
        className="relative aspect-square"
        // …and the inner one: the fluid sizes inside the picture
        // resolve against the square itself, not the pane around it.
        style={{ width: "100cqmin", containerType: "size" }}
      >
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
                    fontSize: TEXT_FINE,
                    lineHeight: TEXT_RATIO,
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
   cannot take away. The dash is in the viewBox's own units on purpose
   — it scales with the picture, which is what "everything scales
   together" means for a pattern. */
const EDGE_STROKE = 1.5;
const EDGE_STROKE_FAINT = 1;
const RELAY_DASH = "6 5";

/* ── the fluid sizes (file head: the whole picture scales) ──────────
   Each is clamp(floor-token, proportion, ceiling): the proportion is
   what tracks the window, the tokens are where it stops. Numbers here
   are ratios of the square's side, the same kind of judgment as the
   node radii mandala states in its viewBox units. */
const FACE_ME = "clamp(var(--avatar-lg), 14cqmin, calc(var(--avatar-xl) * 1.5))";
const FACE_OTHER = "clamp(var(--avatar), 10cqmin, var(--avatar-xl))";
/* Captions may outgrow their old fixed width on a big canvas — that is
   the readings line coming back onto one line — and shrink to roughly
   the face's own width on a phone, where truncation takes over. */
const CAPTION_W = "min(36cqmin, calc(var(--avatar-xl) * 2))";
/* Type between its two smallest tokens; the classes underneath stay as
   the no-container-units fallback. A fluid size cannot carry a stepped
   token line-height, so the ratio is unitless — both token pairs sit
   at ~1.44 (16/11, 18/12.5), this simply keeps that ratio moving. */
const TEXT_NAME = "clamp(var(--text-sm), 2.6cqmin, var(--text-lg))";
const TEXT_FINE = "clamp(var(--text-xs), 2.2cqmin, var(--text-sm))";
const TEXT_RATIO = "1.44";

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
  const aged = row.vitals ? ageIfOld(row.vitals.age_ms) : null;
  const body = (
    <>
      {/* The face fills a box the fluid size owns (file head): the
          wrapper wears the cqmin clamp, the face wears `size-full`.
          Still exactly one size per element — the wrapper's is inline,
          the face's is the one class — so the `cn()` merge trap
          (docs/handoff 坑节) has nothing to bite. */}
      <span
        className="block"
        style={big ? { width: FACE_ME, height: FACE_ME } : { width: FACE_OTHER, height: FACE_OTHER }}
      >
        <MachineAvatar face={row.face} className="size-full" />
      </span>
      {/* Out of the flow, so the box this sits in stays exactly the size
          of the face — see the note above. */}
      <span
        className="absolute left-1/2 top-full flex -translate-x-1/2 flex-col items-center pt-2 text-center"
        style={{ width: CAPTION_W }}
      >
        {/* **The name is the caption, and everything else qualifies
            it.** It carries the whole hierarchy on the size axis alone —
            never on weight as well, which would be rank said twice
            (tokens.css 纸墨). Before this it was `--text-sm` like the
            three lines under it, and a seat arrived as five pieces of
            text of one size. */}
        <span
          data-seat-name
          className="max-w-full truncate text-sm"
          style={{ fontSize: TEXT_NAME, lineHeight: TEXT_RATIO }}
        >
          {row.name}
        </span>
        {/* The age — **only once it is worth saying** (`ageIfOld`) — or
            the third state. **This machine says neither**: it answers by
            looking, so there is no age to report and nothing to be
            unsure about. */}
        {!row.me &&
          (reached ? (
            aged !== null && (
              <span
                data-seat-age
                className="max-w-full truncate text-xs text-muted-foreground"
                style={{ fontSize: TEXT_FINE, lineHeight: TEXT_RATIO }}
              >
                {cli.vitals_taken(aged)}
              </span>
            )
          ) : (
            <span
              data-seat-never
              className="max-w-full truncate text-xs text-muted-foreground"
              style={{ fontSize: TEXT_FINE, lineHeight: TEXT_RATIO }}
            >
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
        // The finger floor (tokens.css `--touch`). A seat's face is a
        // proportion of the picture (`FACE_OTHER`), so on a phone-width
        // map it comes out around 37 — under the floor, and it is the
        // one thing on this screen a finger is aimed at. The box grows,
        // the face does not: it is centred in the grown box, and the
        // press feedback is the scale of that whole box, so nothing is
        // pressable where nothing is drawn.
        "grid min-h-touch min-w-touch place-items-center",
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

/* A seat has carried a machine's readings twice and lost them twice,
   and the two removals are not the same removal — that is worth one
   note, because the second one reads like a retreat from the first.

   They were **words** first (`CPU 24% 内存 80% GPU 63%`): five pieces of
   text of one size arrived under every face at once and the name had no
   way to be first. They became **bars**, on the judgment that a picture
   of a reading costs a glance rather than a read. Now they are nothing,
   because that judgment was wrong about labels: a bar says how full
   *something* is, and which something is exactly what a seat has no
   room to say.

   What is left is not less information in the same place — it is the
   same information one press away, in the shape that carries its own
   scale (the card's rings). The file head has the rule this leaves
   behind. */
