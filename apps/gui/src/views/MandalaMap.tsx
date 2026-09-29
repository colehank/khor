import { useEffect, useLayoutEffect, useRef, useState } from "react";

import type { DeviceRow } from "@/api";
// Aliased: `Seat` the name belongs to the component below (one machine
// in its place); the binding is the place itself.
import type { Seat as SeatPoint } from "@/gen/bindings/Seat";
import { MachineAvatar } from "@/components/Avatar";
import { PageSubtitle } from "@/components/Region";
import { cli, gui, state } from "@/gen/catalog";
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
 * **待批 rides a seat, and it is not a reading.** A reading is a
 * quantity out of some whole that has to be learnt before it means
 * anything — which is what sank the bars. This is a count of rows in
 * one named state (`khor_core::State::Blocked`: an action is stuck and
 * an approval is the only way out), printed as the word plus the
 * number, in the colour that state already wears on every row in the
 * app. Nothing to learn and nothing to decode.
 *
 * It is here because sessions were moved onto the machines they live on
 * (「会话是在机器节点点击后看的,不是在地图外面」 — the user,
 * 2026-08-28), and a card is a press away. Without a mark out here, the
 * one fact with a person on the other end of it would only be findable
 * by opening every machine in turn; a screen that has to be searched
 * for an alarm has no alarm. **Drawn only above zero** — a mark that is
 * always present is not a mark, which is this repo's own rule about a
 * badge that can never reach zero, read from the other end.
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
  waiting,
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
  /**
   * How many of each machine's sessions are waiting on a person, by
   * device id (file head: 待批 rides a seat). Absent, and every seat
   * draws none — a map with no session table behind it says nothing
   * about sessions rather than saying zero.
   */
  waiting?: Record<string, number>;
}) {
  // Which one is the middle is a fact about the machine, not about
  // where it was placed — mandala keeps those two apart even where they
  // always coincide (its docs/MAP.md), and so does this: `me` picks the
  // machine, `seat` says where it goes, and neither is read off the
  // other.
  const me = rows.find((r) => r.me) ?? null;
  const others = rows.filter((r) => !r.me);
  // **Where each edge is drawn, worked out from what is on the square**
  // (2026-09-28 UI 审计): every edge used to run centre to centre under
  // everything — through the discs at both ends and through every name
  // and age hanging under them — and the hop word stood on a chip that
  // hid only its own patch of line. Now a line stops at a disc's edge,
  // breaks around every piece of text on the map, and the hop word sits
  // in the longest stretch that is left. `check-face` 「地图的线」.
  const square = useRef<HTMLDivElement>(null);
  const [roads, setRoads] = useState<Roads | null>(null);
  // A size change moves every seat, and a font arriving late changes
  // every caption's width; either one re-runs the layout below.
  const [, setBump] = useState(0);
  useEffect(() => {
    const el = square.current;
    if (!el) return;
    const again = () => setBump((n) => n + 1);
    const watch = new ResizeObserver(again);
    watch.observe(el);
    void document.fonts?.ready.then(again);
    return () => watch.disconnect();
  }, []);
  // After every render, and before paint: the captions and the words
  // are this render's, so the lines must be too. Equal answers keep the
  // old object, so this settles in one pass.
  useLayoutEffect(() => {
    const el = square.current;
    const next = el && me ? layRoads(el, me, others) : null;
    setRoads((was) => (sameRoads(was, next) ? was : next));
  });
  // **R0's second line on the machines overview** (2026-09-30, the A
  // pages 「和工作区看齐」): how many machines the map holds and how many
  // this one is talking to — counted off the rows the map is drawn from,
  // with the same roads the network page counts (direct and relayed
  // carry something; 未连接 / serve 未运行 do not).
  const connected = others.filter((r) => r.hop === "direct" || r.hop === "relay").length;
  return (
    <>
    {rows.length > 0 && (
      <PageSubtitle>
        <span
          data-machines-head-sub
          className="min-w-0 truncate text-callout leading-(--text-caption--line-height) text-muted-foreground tabular-nums"
        >
          {gui.head_machines(rows.length)}
          {others.length > 0 && (
            <>
              <span aria-hidden="true"> · </span>
              {gui.head_connected(connected)}
            </>
          )}
        </span>
      </PageSubtitle>
    )}
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
        ref={square}
        className="relative aspect-square"
        // …and the inner one: the fluid sizes inside the picture
        // resolve against the square itself, not the pane around it.
        style={{ width: "100cqmin", containerType: "size" }}
      >
        {/* The edges, under everything, in the square's own pixels —
            the stretches `layRoads` left once the discs and the words
            were cut out of each line. Nothing is drawn until that has
            run once: a centre-to-centre line for one frame is the
            picture this replaced. */}
        {me && others.length > 0 && roads && (
          <svg
            data-mesh-edges
            aria-hidden="true"
            viewBox={`0 0 ${roads.side} ${roads.side}`}
            className="pointer-events-none absolute inset-0 size-full"
          >
            {roads.edges.map((e) =>
              e.parts.map((p, i) => <Edge key={`${e.id} ${i}`} id={e.id} road={e.road} part={p} />),
            )}
          </svg>
        )}
        {/* Each edge's word, in the longest stretch of its line that no
            disc or caption covers — HTML rather than SVG text so the
            type tokens reach it. **No chip of ground under it**: the
            line is cut around the word instead, so nothing shows
            through it on either theme. Hidden until placed; it is
            measured before it is placed. */}
        {me &&
          others.map((row) => {
            if (!row.hop) return null;
            const at = roads?.edges.find((e) => e.id === row.id)?.word ?? null;
            return (
              <span
                key={row.id}
                data-edge-word={row.id}
                data-hop={row.hop}
                className={cn(
                  "absolute -translate-x-1/2 -translate-y-1/2 whitespace-nowrap text-caption text-muted-foreground",
                  !at && "invisible",
                )}
                style={{
                  left: at ? `${at.x}%` : 0,
                  top: at ? `${at.y}%` : 0,
                  fontSize: TEXT_FINE,
                  lineHeight: TEXT_RATIO,
                }}
              >
                {hopWord(row.hop)}
              </span>
            );
          })}
        <Seat
          row={me}
          big
          onOpen={onOpen}
          selected={selected}
          waiting={me ? (waiting?.[me.id] ?? 0) : 0}
        />
        {others.map((row) => (
          <Seat
            key={row.id}
            row={row}
            onOpen={onOpen}
            selected={selected}
            waiting={waiting?.[row.id] ?? 0}
          />
        ))}
      </div>
    </section>
    </>
  );
}

/**
 * How many rows on each machine are waiting on a person, by device id —
 * the map's `waiting`. **Counted by the caller's rows, not by this
 * file**: the map is handed devices and has never been handed
 * sessions, and a picture that went looking for a second table to draw
 * itself is a picture with an opinion about what a machine is.
 *
 * Joined on the name and not on an id, because the name is the join
 * the node itself made (`khor_node::list::place_of` builds `home` from
 * the same device table this row's `name` comes from). Moved here from
 * `MarkPlace` when that retired (2026-09-28).
 */
export function waitingOn(devices: DeviceRow[], rows: { home: string; word: string }[]): Record<string, number> {
  const out: Record<string, number> = {};
  for (const d of devices) {
    const n = rows.filter((r) => r.home === d.name && r.word === "blocked").length;
    if (n > 0) out[d.id] = n;
  }
  return out;
}

/* The edges' line weights and the relay's dash rhythm, named the way
   Ring.tsx names its STROKE: screen units (non-scaling-stroke), one
   place. The no-reading stroke is thinner as well as fainter, so the
   difference survives a screenshot in one channel colour-blindness
   cannot take away. The viewBox is the square's own pixels since
   2026-09-28, so the dash is too. */
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
const TEXT_NAME = "clamp(var(--text-callout), 2.6cqmin, var(--text-title-md))";
const TEXT_FINE = "clamp(var(--text-caption), 2.2cqmin, var(--text-callout))";
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
 * One `<line>` per stretch `layRoads` left of the edge: a line broken
 * around a caption or its own word is several of these, all carrying
 * the edge's id.
 */
function Edge({ id, road, part }: { id: string; road: Road; part: Part }) {
  return (
    <line
      data-edge={id}
      data-road={road}
      x1={part.x1}
      y1={part.y1}
      x2={part.x2}
      y2={part.y2}
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
  waiting = 0,
}: {
  row: DeviceRow | null;
  big?: boolean;
  onOpen?: (row: DeviceRow) => void;
  selected?: string | null;
  /** This machine's waiting count — drawn only above zero. */
  waiting?: number;
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
        data-seat-face
        className="block"
        style={big ? { width: FACE_ME, height: FACE_ME } : { width: FACE_OTHER, height: FACE_OTHER }}
      >
        <MachineAvatar face={row.face} className="size-full" />
      </span>
      {/* Out of the flow, so the box this sits in stays exactly the size
          of the face — see the note above. `data-seat-caption` is what
          `layRoads` reads the words' boxes under. */}
      <span
        data-seat-caption
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
          className="max-w-full truncate text-callout"
          style={{ fontSize: TEXT_NAME, lineHeight: TEXT_RATIO }}
        >
          {row.name}
        </span>
        {/* The one thing on this picture with a person on the other
            end of it (file head). The word before the number, because
            the word is what makes the number readable without pressing
            anything — and the state's own colour, never a second one
            invented here. */}
        {waiting > 0 && (
          <span
            data-seat-blocked
            className="max-w-full truncate text-caption tabular-nums"
            style={{
              fontSize: TEXT_FINE,
              lineHeight: TEXT_RATIO,
              color: "var(--state-blocked)",
            }}
          >
            {`${state.blocked} ${waiting}`}
          </span>
        )}
        {/* The age — **only once it is worth saying** (`ageIfOld`) — or
            the third state. **This machine says neither**: it answers by
            looking, so there is no age to report and nothing to be
            unsure about. */}
        {!row.me &&
          (reached ? (
            aged !== null && (
              <span
                data-seat-age
                className="max-w-full truncate text-caption text-muted-foreground"
                style={{ fontSize: TEXT_FINE, lineHeight: TEXT_RATIO }}
              >
                {cli.vitals_taken(aged)}
              </span>
            )
          ) : (
            <span
              data-seat-never
              className="max-w-full truncate text-caption text-muted-foreground"
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
      data-blocked={waiting}
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
        "motion-safe:transition-transform hover:scale-110 data-[on=true]:scale-110",
        // The face is round or nearly so, so the focus ring follows it
        // rather than boxing it.
        "rounded-full outline-none focus-visible:ring-2 focus-visible:ring-ring",
      )}
      style={place}
    >
      {body}
    </button>
  ) : (
    <div
      data-seat={row.id}
      data-seat-me={row.me}
      data-blocked={waiting}
      className={shell}
      style={place}
    >
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

/* ── where the lines go (2026-09-28 UI 审计) ─────────────────────────
   Every edge is the straight line from this machine's centre to the
   other one's, **less every stretch that would run into a disc or
   under a word**: the two discs at its ends (and any other it passes),
   every name, age and 待批 under a seat, and — once they are placed —
   every hop word. A little air (`--gap-xxs`) is kept round each of
   them, so a line visibly stops rather than touching.

   Read off the laid-out boxes, never off transformed ones: the seats
   scale on hover and zoom in on arrival, and a layout that followed
   that would move the lines under the pointer. So the geometry is
   `offset*` sizes placed at the seat's own point — the resting picture.

   Everything a shape needs is a rounded box (`Keep`): a face is one
   with its own crop (`Avatar::radius_ratio`; the blank is a circle), a
   word is one with square corners. The keep-out round a box is convex,
   so a line enters it at most once — one interval to cut. */

type Road = "direct" | "relay" | "none";
/** One drawn stretch of an edge, in the square's pixels. */
type Part = { x1: number; y1: number; x2: number; y2: number };
type Roads = {
  /** The square's side, in pixels — the viewBox. */
  side: number;
  edges: { id: string; road: Road; parts: Part[]; word: { x: number; y: number } | null }[];
};
/** A rounded box: centre, half extents, corner radius. */
type Keep = { cx: number; cy: number; hw: number; hh: number; r: number };
type Span = [number, number];

const roadOf = (row: DeviceRow): Road =>
  row.hop === "direct" ? "direct" : row.hop === "relay" ? "relay" : "none";

/** Signed distance from a point to a rounded box — negative inside. */
function signed(x: number, y: number, k: Keep): number {
  const qx = Math.abs(x - k.cx) - k.hw + k.r;
  const qy = Math.abs(y - k.cy) - k.hh + k.r;
  return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - k.r;
}

/** The stretch of `a→b` (as a fraction of it) that comes within `air`
    of the box, or null. The distance is convex along a line, so the
    closest point is found by ternary search and the two crossings by
    bisection either side of it. */
function cut(a: { x: number; y: number }, b: { x: number; y: number }, k: Keep, air: number): Span | null {
  const d = (t: number) => signed(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t, k) - air;
  let lo = 0;
  let hi = 1;
  for (let i = 0; i < 48; i += 1) {
    const m1 = lo + (hi - lo) / 3;
    const m2 = hi - (hi - lo) / 3;
    if (d(m1) < d(m2)) hi = m2;
    else lo = m1;
  }
  const near = (lo + hi) / 2;
  if (d(near) >= 0) return null;
  const edge = (inside: number, outside: number) => {
    let i = inside;
    let o = outside;
    for (let n = 0; n < 40; n += 1) {
      const m = (i + o) / 2;
      if (d(m) < 0) i = m;
      else o = m;
    }
    return o;
  };
  return [d(0) < 0 ? 0 : edge(near, 0), d(1) < 0 ? 1 : edge(near, 1)];
}

function without(spans: Span[], gone: Span | null): Span[] {
  if (!gone) return spans;
  const [g0, g1] = gone;
  return spans.flatMap(([s0, s1]): Span[] => {
    if (g1 <= s0 || g0 >= s1) return [[s0, s1]];
    const out: Span[] = [];
    if (g0 > s0) out.push([s0, g0]);
    if (g1 < s1) out.push([g1, s1]);
    return out;
  });
}

function layRoads(square: HTMLElement, me: DeviceRow, others: DeviceRow[]): Roads | null {
  const side = square.clientWidth;
  if (side <= 0) return null;
  const root = getComputedStyle(document.documentElement);
  const air = Number.parseFloat(root.getPropertyValue("--gap-xxs")) || 0;
  const at = (s: SeatPoint) => ({ x: (s.left / 100) * side, y: (s.top / 100) * side });
  const all = [me, ...others];
  const keeps: Keep[] = [];
  for (const el of square.querySelectorAll<HTMLElement>("[data-seat]")) {
    const row = all.find((r) => r.id === el.dataset.seat);
    const face = el.querySelector<HTMLElement>("[data-seat-face]");
    if (!row || !face) continue;
    const c = at(row.seat);
    const fw = face.offsetWidth;
    const fh = face.offsetHeight;
    keeps.push({ cx: c.x, cy: c.y, hw: fw / 2, hh: fh / 2, r: (row.face?.radius_ratio ?? 0.5) * Math.min(fw, fh) });
    // The caption hangs from the seat box's bottom edge, centred on it
    // (`left-1/2 top-full -translate-x-1/2`); its words are its children.
    const cap = el.querySelector<HTMLElement>("[data-seat-caption]");
    if (!cap) continue;
    const left = c.x - cap.offsetWidth / 2;
    const top = c.y + el.offsetHeight / 2;
    for (const w of cap.children) {
      if (!(w instanceof HTMLElement) || w.offsetWidth === 0) continue;
      keeps.push({
        cx: left + w.offsetLeft + w.offsetWidth / 2,
        cy: top + w.offsetTop + w.offsetHeight / 2,
        hw: w.offsetWidth / 2,
        hh: w.offsetHeight / 2,
        r: 0,
      });
    }
  }
  const from = at(me.seat);
  // First pass: the discs and the captions.
  const lines = others.map((row) => {
    const to = at(row.seat);
    let spans: Span[] = [[0, 1]];
    for (const k of keeps) spans = without(spans, cut(from, to, k, air));
    return { row, to, spans };
  });
  // The words, each in the longest stretch its own line has left.
  const words: Keep[] = [];
  const placed = lines.map(({ row, to, spans }) => {
    const el = square.querySelector<HTMLElement>(`[data-edge-word="${row.id}"]`);
    if (!el || spans.length === 0) return null;
    const [s0, s1] = spans.reduce((best, s) => (s[1] - s[0] > best[1] - best[0] ? s : best));
    const t = (s0 + s1) / 2;
    const x = from.x + (to.x - from.x) * t;
    const y = from.y + (to.y - from.y) * t;
    words.push({ cx: x, cy: y, hw: el.offsetWidth / 2, hh: el.offsetHeight / 2, r: 0 });
    return { x: (x / side) * 100, y: (y / side) * 100 };
  });
  // Second pass: every line breaks round every word, its own and the
  // others'. A stretch shorter than the air is not a line, it is a dot.
  const edges = lines.map(({ row, to, spans }, i) => {
    let left = spans;
    for (const k of words) left = without(left, cut(from, to, k, air));
    const len = Math.hypot(to.x - from.x, to.y - from.y);
    const parts = left
      .filter(([s0, s1]) => (s1 - s0) * len > air)
      .map(([s0, s1]) => ({
        x1: from.x + (to.x - from.x) * s0,
        y1: from.y + (to.y - from.y) * s0,
        x2: from.x + (to.x - from.x) * s1,
        y2: from.y + (to.y - from.y) * s1,
      }));
    return { id: row.id, road: roadOf(row), parts, word: placed[i] };
  });
  return { side, edges };
}

/** Two layouts that would paint the same picture — tenths of a pixel. */
function sameRoads(a: Roads | null, b: Roads | null): boolean {
  const key = (r: Roads | null) =>
    JSON.stringify(r, (_, v: unknown) => (typeof v === "number" ? Math.round(v * 10) / 10 : v));
  return key(a) === key(b);
}
