import type { DeviceRow, SessionRow, QuotaAnswer } from "@/api";
import type { Track } from "@/components/Ring";
import { useEffect, useState } from "react";
import { MachineAvatar } from "@/components/Avatar";
import { Ring } from "@/components/Ring";
import { IconClose } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { MandalaMap } from "@/views/MandalaMap";
import { QuotaRings } from "@/views/QuotaRings";
import { cli, gui, state } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { vitalsTracks } from "@/lib/vitals";
import { ageIfOld } from "@/words";

/**
 * The mark's place: the mesh, and what is left of the subscription.
 *
 * # One thing: the picture. Everything else is a press away
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
 * **The arcs then left the footing too**: 「额度都是机器卡里的」 (the
 * same person, later the same day). So this screen is the picture and
 * nothing else, and the arcs are on this machine's card with the rest
 * of what khor reads locally — `QuotaRings` carries why that is the
 * honest place for them rather than a compromise.
 *
 * # Everything here is a machine, and pressing one opens its card
 *
 * **There used to be a second kind of node**: one mark above the
 * picture standing for every session khor holds, pressed for a card of
 * four counts. 「会话是在机器节点点击后看的,不是在地图外面」 — the user,
 * 2026-08-28. It is gone, and the counts went onto the machine's own
 * card, scoped to that machine.
 *
 * The judgment it was built on was that a session is not a machine
 * (several live on one), so it could not take a seat on a ring whose
 * members are machines — true, and it does not follow that it belongs
 * beside the ring. **A session does have a machine**, `SessionRow::home`
 * has said so all along, and the honest place for a count of them is the
 * thing they are counted on. One mark for the whole mesh could only ever
 * answer 一共几个, which is a question with a landing of its own; asked
 * per machine, the same four counts answer 这台在忙什么.
 *
 * What that costs is a standing signal, and the cost is paid on the map
 * rather than waived: 待批 is drawn on the seat of the machine that owns
 * it (`MandalaMap`'s head). Without it the one state with a person on
 * the other end would be findable only by opening every machine in turn.
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
  /** Where the waiting call leads, when anything is waiting. */
  onPickWaiting: (row: SessionRow) => void;
}) {
  // Which machine's card is open, if any — its id, or nothing.
  //
  // **This used to be a tagged union**, back when a second kind of node
  // could be open in the same space. There is one kind now, and a
  // discriminant with one arm is the sort of leftover that outlives the
  // structure it was for: it costs a `kind` at every read and quietly
  // teaches the next person that this screen has two things to open.
  const [open, setOpen] = useState<string | null>(null);
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
  const machine = open ? (devices.find((d) => d.id === open) ?? null) : null;
  // How many rows on each machine are waiting on a person, keyed by
  // device id for the map. Counted here rather than in `MandalaMap`:
  // that file is handed devices and has never been handed sessions, and
  // a picture that went looking for a second table to draw itself is a
  // picture with an opinion about what a machine is.
  //
  // Joined on the name and not on an id, because the name is the join
  // the node itself made (`khor_node::list::place_of` builds `home`
  // from the same device table this row's `name` comes from) — a second
  // mapping invented here would be a second answer that can disagree.
  const waiting: Record<string, number> = {};
  for (const d of devices) {
    const n = rows.filter((r) => r.home === d.name && r.word === "blocked").length;
    if (n > 0) waiting[d.id] = n;
  }
  return (
    <div data-mark-place className="flex h-full min-h-0 min-w-0 flex-col">
      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        {/* The picture, and the cards that open over it. `relative` is
            what the card is positioned against — it covers the map and
            nothing else, so the arcs below stay readable while a card
            is open. */}
        <div className="relative flex min-h-0 min-w-0 flex-1 flex-col">
          <div className="min-h-0 min-w-0 flex-1">
            <MandalaMap
              rows={devices}
              selected={machine?.id ?? null}
              waiting={waiting}
              onOpen={(row) => setOpen((was) => (was === row.id ? null : row.id))}
            />
          </div>
          {machine && (
            <MachineNodeCard
              row={machine}
              rows={rows.filter((r) => r.home === machine.name)}
              // Only this machine's card gets it, and the reason is not
              // tidiness — khor reads the quota from a login stored on
              // this machine and no frame carries another machine's
              // (`QuotaRings`). Passed as `null` rather than guarded
              // inside the card, so the card never holds an answer it
              // is not allowed to paint.
              quota={machine.me ? quota : null}
              onClose={() => setOpen(null)}
              onPickWaiting={onPickWaiting}
            />
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * The shell a machine's card wears.
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
 *
 * # …and what it is holding: four counts, under a rule
 *
 * The user named all four (2026-08-28 —「几个 session、几个忙碌、几个
 * 待批、几个完成」), and named where they belong (「会话是在机器节点点击
 * 后看的」). So they are this machine's, filtered by `SessionRow::home`
 * before they get here.
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
 *
 * **The rule between the readings and the counts is doing work.** They
 * are both rows of numbers under one heading and they are about
 * different things — how hard this machine is working, and what it is
 * working on. Without a line they read as one set of six.
 */
function MachineNodeCard({
  row,
  rows,
  quota,
  onClose,
  onPickWaiting,
}: {
  row: DeviceRow;
  /** Only this machine's sessions — filtered by the caller, which is
      the one holding the whole table. */
  rows: SessionRow[];
  /** The subscription's arcs, and **`null` on every card but this
      machine's** — the caller decides, see its note. */
  quota: QuotaAnswer | null;
  onClose: () => void;
  onPickWaiting: (row: SessionRow) => void;
}) {
  const tracks = vitalsTracks(row);
  const aged = row.vitals ? ageIfOld(row.vitals.age_ms) : null;
  const blocked = rows.filter((r) => r.word === "blocked");
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
      {/* This machine's sessions. **Zero of them is one sentence, not
          four zeros**: four counts all reading 0 is a table of nothing,
          and the list's own empty word already says it in one line. */}
      <div data-node-card-sessions className="mt-in border-t pt-in">
        {rows.length === 0 ? (
          <div data-count-none className="text-aux text-muted-foreground">
            {gui.empty_sessions}
          </div>
        ) : (
          <div className="flex flex-wrap justify-center gap-row">
            {/* **`全部`, not `会话`.** The three labels beside it are
                state words, and this one is not a state — it is their
                denominator. Reusing the rail landing's name here would
                read as 「会话 (that station), 6」 rather than 「6 of them,
                all told」; the catalog carries the rest of that judgment
                on `sessions-all`. */}
            <Count anchor="all" word={gui.sessions_all} n={rows.length} />
            <Count
              anchor="busy"
              word={state.busy}
              n={rows.filter((r) => r.word === "busy").length}
            />
            <Count
              anchor="blocked"
              word={state.blocked}
              n={blocked.length}
              // Only above zero. A zero in the alarm's own colour is
              // the alarm pointing at nothing, and a mark that is
              // always lit is not a mark — the same rule the seat's
              // line is drawn by out on the map.
              tone={blocked.length > 0 ? "var(--state-blocked)" : undefined}
              onPress={blocked.length > 0 ? () => onPickWaiting(blocked[0]) : undefined}
            />
            <Count
              anchor="done"
              word={state.done}
              n={rows.filter((r) => r.word === "done").length}
            />
          </div>
        )}
      </div>
      {/* The subscription, under the same kind of rule as the counts —
          it is a third group of numbers and reads as one more row of
          them without a line. Draws nothing at all on a machine that is
          not this one, and nothing before the first answer is in. */}
      <QuotaRings quota={quota} className="mt-in border-t pt-in" />
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
