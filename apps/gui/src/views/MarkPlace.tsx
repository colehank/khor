import type { DeviceRow, SessionRow, QuotaAnswer, Usage } from "@/api";
import type { Track } from "@/components/Ring";
import { useEffect, useState } from "react";
import { MachineAvatar } from "@/components/Avatar";
import { Ring } from "@/components/Ring";
import { IconClose } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { MandalaMap } from "@/views/MandalaMap";
import { QuotaPanel } from "@/views/QuotaPanel";
import { cli, gui, state } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { vitalsTracks } from "@/lib/vitals";
import { ageIfOld } from "@/words";

/**
 * The mark's place: the mesh, and what is left of the subscription.
 *
 * # Two things, and the second one is under the first
 *
 * 「Khor 主页不显示输出 token 什么的。只显示地图和 AI usage。我们的
 * usage 就放在地图下面。」 — the user, 2026-08-28, and 「AI usage」 was
 * confirmed to mean the two quota arcs (5 小时 / 7 天), not the daily
 * token ledger that used to fill the right-hand column. **The ledger
 * has no other landing in the app and was not given one**: the arcs
 * answer 还能用多少, which is what a person opens this screen to find
 * out, and 花了多少 is a different question with a CLI verb of its own
 * (`khor usage`). Removing a panel and quietly re-hanging it somewhere
 * else would have been the same screen with an extra press in front of
 * it.
 *
 * Under, not beside, and the difference is not taste: side by side the
 * two halves were peers, and the numbers were half the screen while
 * being the smaller question. Under the picture they are a footing —
 * the map is what this place is, and the arcs are what it costs.
 *
 * # Everything here is a node, and pressing one opens a card
 *
 * There are two kinds and they are not the same kind of thing, which is
 * why one is in the mandala and one is not. **A seat is a machine**, and
 * the ring of seats is `khor_core::map`'s. **The session node is not a
 * machine** — sessions live on machines, several to one — so putting it
 * on that ring would draw a member the mesh does not have. It sits above
 * the picture, in the same grammar (a mark, a caption under it) so that
 * it reads as one more thing to press, and outside the ring so that it
 * claims nothing about membership.
 *
 * **The card is a press, never a hover.** A preview that only appears
 * under a pointer does not exist on a phone, and this screen is the same
 * screen on both (docs/UX.md 两端对齐的是事实). Three ways out, and the
 * first two are the ones a finger has: its own close button, pressing
 * the node again (the thing you just pressed is still under your
 * thumb), and Escape.
 *
 * **The map is handed `onOpen` now, and the head of this file used to
 * say why it must not be.** The rule has not changed — an affordance
 * that answers nothing teaches people to stop trying the ones that do —
 * only its input: there is somewhere to open into on this screen now.
 */
export function MarkPlace({
  rows,
  devices,
  quota,
  onPickWaiting,
}: {
  rows: SessionRow[];
  devices: DeviceRow[];
  quota: QuotaAnswer | null;
  /**
   * **Neither of these is read any more, and both are still in the
   * type on purpose.** The daily ledger was `usage`'s only consumer
   * here and it is gone; `narrow` shaped the columns, and there are no
   * columns — the map and the arcs stack the same way in both postures
   * (a footing does not become a sidebar because the window got wide).
   *
   * They stay declared for exactly as long as it takes the shell to
   * stop passing them. `App.tsx` belongs to another lane this hour;
   * dropping them from this side alone turns two of its lines into type
   * errors that somebody else has to find and that nothing on screen
   * explains. Two lines, whenever that file is free.
   */
  usage?: Usage | null;
  narrow?: boolean;
  /** Where the waiting call leads, when anything is waiting. */
  onPickWaiting: (row: SessionRow) => void;
}) {
  // Which node's card is open — a machine's id, the sessions node, or
  // nothing. One piece of state rather than two booleans: the two cards
  // occupy the same space and only one can be open, and two flags can
  // represent a state the screen cannot show.
  const [open, setOpen] = useState<{ kind: "machine"; id: string } | { kind: "sessions" } | null>(
    null,
  );
  // Escape closes, wherever the focus is. Bound while a card is open
  // and not before: a listener that is always on is a key this screen
  // has taken away from everything under it for the 99% of the time
  // nothing is open.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);
  // A machine that leaves the table takes its card with it. Read from
  // the row rather than held in state, so the card cannot outlive the
  // fact it is about.
  const machine =
    open?.kind === "machine" ? (devices.find((d) => d.id === open.id) ?? null) : null;
  return (
    <div data-mark-place className="flex h-full min-h-0 min-w-0 flex-col">
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        {/* The picture, and the cards that open over it. `relative` is
            what the card is positioned against — it covers the map and
            nothing else, so the arcs below stay readable while a card
            is open. */}
        <div className="relative flex min-h-0 min-w-0 flex-1 flex-col">
          <SessionNode
            rows={rows}
            on={open?.kind === "sessions"}
            onPress={() => setOpen((was) => (was?.kind === "sessions" ? null : { kind: "sessions" }))}
          />
          <div className="min-h-0 min-w-0 flex-1">
            <MandalaMap
              rows={devices}
              selected={machine?.id ?? null}
              onOpen={(row) =>
                setOpen((was) =>
                  was?.kind === "machine" && was.id === row.id ? null : { kind: "machine", id: row.id },
                )
              }
            />
          </div>
          {machine && <MachineNodeCard row={machine} onClose={() => setOpen(null)} />}
          {open?.kind === "sessions" && (
            <SessionNodeCard rows={rows} onClose={() => setOpen(null)} onPickWaiting={onPickWaiting} />
          )}
        </div>
        {/* The footing. **The rule between it and the picture is drawn
            by the panel, not by this box**: the panel draws nothing at
            all until the first answer is in, and a border here would be
            a line promising a section that has not arrived. A box that
            is empty and a box that is not are the same box; only the
            thing inside knows which. */}
        <div className="min-h-0 min-w-0 flex-none">
          <QuotaPanel quota={quota} />
        </div>
      </div>
    </div>
  );
}

/**
 * The one node on this screen that is not a machine: how many sessions
 * khor is holding, **and whether any of them is waiting on you**.
 *
 * # It has to answer that second question standing still
 *
 * **This is the whole design of it, and the first version failed it.**
 * That one drew the total and the word 会话 and put 待批 inside the card,
 * one press away — which took the one state in the six with a person on
 * the other end of it (`khor_core::State::Blocked`: an action is stuck
 * and an approval is the only way out; `rank()` puts it at 0, ahead of
 * 失败) and turned it from something seen into something looked up. The
 * rail's badge cannot cover for it: that number counts rows that are
 * waiting **or** unread and cannot say which.
 *
 * So 待批 rides the node itself, in two channels at once, and neither is
 * a new piece of furniture:
 *
 *   - **the arc** — the ring goes from track-only to a real arc, and the
 *     arc's fraction is 待批 out of the total. That is a denominator khor
 *     actually holds, which is why the arc is allowed at all: a ring's
 *     job is 「how much of this」 and here 「this」 is the number in the
 *     middle;
 *   - **the count**, printed under the word, in `--state-blocked` —
 *     the same colour that state wears on every row in the app, so it is
 *     recognised rather than decoded. A colour alone would have to be
 *     learnt, which is the mistake the star map's bars were removed for.
 *
 * **Neither is drawn at zero**, and that is not the vanishing act the
 * 「N 等你的事」 call was removed for. That call *was* the whole element,
 * so its absence and 「this screen forgot about sessions」 looked the
 * same. The node is always here; what changes is whether it is carrying
 * an alarm. A permanent 待批 0 would be the other failure this repo has
 * a rule for — a badge that can never reach zero is no badge, and its
 * mirror is a badge that is always at zero.
 */
function SessionNode({
  rows,
  on,
  onPress,
}: {
  rows: SessionRow[];
  on: boolean;
  onPress: () => void;
}) {
  const blocked = rows.filter((r) => r.word === "blocked").length;
  return (
    <div className="flex flex-none justify-center pt-pane">
      <button
        type="button"
        data-session-node
        data-blocked={blocked}
        data-on={on}
        aria-expanded={on}
        onClick={onPress}
        // The floor is cleared by the ring itself (`RING_SIZE` is 64),
        // and it is stated anyway: the ring's diameter is a decision
        // about drawing that this button does not own, and a press
        // target that clears the floor by accident stops clearing it
        // the day somebody shrinks the drawing.
        className="flex min-h-touch min-w-touch flex-col items-center gap-in rounded-lg px-2 py-1 hover:bg-accent data-[on=true]:bg-accent"
      >
        {/* `null` while there is nothing to be a fraction of: a session
            count is not out of anything, and at zero blocked there is no
            share to draw either. */}
        <Ring
          fraction={blocked > 0 && rows.length > 0 ? blocked / rows.length : null}
          color={blocked > 0 ? "var(--state-blocked)" : "var(--muted-foreground)"}
        >
          {rows.length}
        </Ring>
        <span className="text-aux text-muted-foreground">{gui.sessions_tab}</span>
        {/* The word before the number, because the word is what makes the
            number readable without pressing anything — and the state's
            own colour, never a second one invented here. */}
        {blocked > 0 && (
          <span
            data-node-blocked
            className="text-aux tabular-nums"
            style={{ color: "var(--state-blocked)" }}
          >
            {`${state.blocked} ${blocked}`}
          </span>
        )}
      </button>
    </div>
  );
}

/**
 * The shell both cards wear.
 *
 * Over the picture rather than beside it: this screen's whole width is
 * the picture, and a card that pushed the map aside would move every
 * seat the moment you pressed one — the picture you were reading would
 * become a different picture as a result of asking about part of it.
 *
 * At the foot of the map and not at the pressed node: a card anchored
 * to a seat lands off-screen for the seats near the edge, and the seats
 * near the edge are most of them. One place, always the same place, is
 * one less thing to find.
 */
function NodeCard({
  name,
  onClose,
  children,
}: {
  name: React.ReactNode;
  onClose: () => void;
  children: React.ReactNode;
}) {
  return (
    <div
      data-node-card
      className={cn(
        "absolute inset-x-4 bottom-4 mx-auto max-w-md rounded-lg border bg-background p-4 shadow-lg",
        "motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-2",
      )}
    >
      <div className="flex items-center gap-row">
        <span className="flex min-w-0 flex-1 items-center gap-2">{name}</span>
        <Button
          variant="ghost"
          size="icon"
          data-node-card-close
          aria-label={gui.close}
          onClick={onClose}
          // The card's only exit for a finger — Escape and pressing the
          // node again are the other two, and neither is a finger's
          // (tokens.css `--touch`; the ghost's wash paints the grown box).
          className="min-h-touch min-w-touch flex-none text-muted-foreground"
        >
          <IconClose />
        </Button>
      </div>
      {children}
    </div>
  );
}

/**
 * A machine's card: what it is doing, as rings.
 *
 * **Rings and not the bars that used to be under its name on the map.**
 * A ring is drawn all the way round whether or not it is full, so the
 * shape carries its own 「满」 and a person reads 「most of it」 without
 * having been told what the whole is. That is the property the bars did
 * not have, and it is why the same reading is allowed here and not out
 * there (`MandalaMap`'s head).
 *
 * The arithmetic is `vitalsTracks`', which the machine card's own rings
 * and the list row's stack also draw from — a machine that looked busy
 * on one screen and calm on another would be two machines. This is the
 * quick look: the percentage in the ring and the reading's name under
 * it. The bytes, the core count, the strain word and the khor version
 * are a level in, on `MachineCard`, and they are not repeated here —
 * a preview that says everything is not a preview.
 */
function MachineNodeCard({ row, onClose }: { row: DeviceRow; onClose: () => void }) {
  const tracks = vitalsTracks(row);
  const aged = row.vitals ? ageIfOld(row.vitals.age_ms) : null;
  return (
    <NodeCard
      onClose={onClose}
      name={
        <>
          {/* `--avatar`, the size a face is drawn at when it stands for
              the machine rather than tagging a row — the card is about
              this machine, and the face is how it is recognised
              (docs/UX.md 两端对齐的是事实: 同一台机器同一张脸). */}
          <MachineAvatar face={row.face} className="size-avatar flex-none" />
          <span data-node-card-name className="min-w-0 truncate text-fact">
            {row.name}
            {row.me ? ` ${cli.this_machine}` : ""}
          </span>
        </>
      }
    >
      {tracks ? (
        <div data-node-card-vitals className="flex flex-wrap justify-center gap-row pt-in">
          {tracks.map((t) => (
            <span key={t.id} className="flex flex-col items-center gap-in">
              <Ring fraction={t.fraction} color={t.color}>
                {t.fraction === null ? null : `${Math.round(t.fraction * 100)}%`}
              </Ring>
              <span className="text-aux text-muted-foreground">{trackName(t.id)}</span>
            </span>
          ))}
        </div>
      ) : (
        // Never reached is not a reading of zero, and empty tracks would
        // say khor asked and got zeros (docs/SESSION.md 离线).
        <div data-node-card-never className="pt-in text-aux text-muted-foreground">
          {cli.vitals_never}
        </div>
      )}
      {/* Only once it is worth saying, the seat's own rule (`ageIfOld`):
          a reading taken a moment ago saying 「0 秒前」 reads as a fault. */}
      {aged !== null && (
        <div data-node-card-age className="pt-in text-aux text-muted-foreground">
          {cli.vitals_taken(aged)}
        </div>
      )}
    </NodeCard>
  );
}

/** The name of a reading, in the words both the CLI and the card use. */
function trackName(id: Track["id"]): string {
  switch (id) {
    case "cpu":
      return cli.vitals_name_cpu;
    case "mem":
      return cli.vitals_name_mem;
    case "disk":
      return cli.vitals_name_disk;
    case "gpu":
      return cli.vitals_name_gpu;
    default:
      return id;
  }
}

/**
 * The sessions card: four counts, and the user named all four
 * (2026-08-28 —「几个 session、几个忙碌、几个待批、几个完成」).
 *
 * **Counts, not a list.** The list is a landing of its own with a search
 * box, a filter and its own order; a second, shorter, unsorted copy of
 * it on the home screen would be a list nobody could trust to be the
 * same one.
 *
 * **Three of the six words, and the three are not a ranking.** They are
 * the three the user asked for, and they happen to be the three at the
 * top of `State::rank` bar one — 待批 (an approval is owed), 忙碌
 * (working, the one word that explicitly does not need you), 完成 (the
 * turn ended, waiting to be read). 中断 / 失败 / 空闲 are not counted
 * here and that is not this screen hiding them: 失败 沉底 by standing
 * rule, and a count nobody asked for is a number to read past.
 *
 * **待批 is the only one that leads anywhere**, and the reason is in the
 * word's definition rather than in its size: it is the one state of the
 * six with a person on the other end of it (`khor_core::State::Blocked`
 * — an action is stuck and an approval is the only way out). The other
 * three are counts of things that are not asking for anything, so a
 * press on them would land somewhere nobody was going.
 */
function SessionNodeCard({
  rows,
  onClose,
  onPickWaiting,
}: {
  rows: SessionRow[];
  onClose: () => void;
  onPickWaiting: (row: SessionRow) => void;
}) {
  const blocked = rows.filter((r) => r.word === "blocked");
  return (
    <NodeCard
      onClose={onClose}
      name={<span className="truncate text-fact">{gui.sessions_tab}</span>}
    >
      <div className="flex flex-wrap justify-center gap-row pt-in">
        {/* **`全部`, not `会话`.** The three labels beside it are state
            words, and this one is not a state — it is their denominator.
            Reusing the rail landing's name here would read as 「会话 (that
            station), 6」 rather than 「6 of them, all told」; the catalog
            carries the rest of that judgment on `sessions-all`. */}
        <Count anchor="all" word={gui.sessions_all} n={rows.length} />
        <Count anchor="busy" word={state.busy} n={rows.filter((r) => r.word === "busy").length} />
        <Count
          anchor="blocked"
          word={state.blocked}
          n={blocked.length}
          tone="var(--state-blocked)"
          onPress={blocked.length > 0 ? () => onPickWaiting(blocked[0]) : undefined}
        />
        <Count anchor="done" word={state.done} n={rows.filter((r) => r.word === "done").length} />
      </div>
    </NodeCard>
  );
}

/**
 * One count: the number, and the word for what was counted.
 *
 * The number leads on size alone and never on weight as well — rank
 * said twice is the trap tokens.css names. `tabular-nums` because four
 * of these stand in a row and digits that do not line up have to be
 * read one at a time.
 *
 * **A button only when there is somewhere to go**, which is the same
 * rule the map's seats are built on: an affordance that answers nothing
 * teaches people to stop trying the ones that do.
 */
function Count({
  anchor,
  word,
  n,
  tone,
  onPress,
}: {
  anchor: string;
  word: string;
  n: number;
  /** The state's own colour, for the one count that wears one. */
  tone?: string;
  onPress?: () => void;
}) {
  const body = (
    <>
      <span className="text-title tabular-nums" style={tone ? { color: tone } : undefined}>
        {n}
      </span>
      <span className="text-aux text-muted-foreground">{word}</span>
    </>
  );
  const shell = "flex min-h-touch min-w-touch flex-col items-center justify-center rounded-md px-2";
  return onPress ? (
    <button type="button" data-count={anchor} onClick={onPress} className={cn(shell, "hover:bg-accent")}>
      {body}
    </button>
  ) : (
    <span data-count={anchor} className={shell}>
      {body}
    </span>
  );
}
