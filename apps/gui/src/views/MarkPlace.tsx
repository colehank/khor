import type { DeviceRow, SessionRow } from "@/api";
import type { Track } from "@/components/Ring";
import { useEffect, useState } from "react";
import { forgetDevice } from "@/api";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { FaceSettings } from "@/views/FaceSettings";
import { MachineAvatar } from "@/components/Avatar";
import { Ring } from "@/components/Ring";
import { IconClose } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { MandalaMap } from "@/views/MandalaMap";
import { cli, gui, state } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { vitalsTracks } from "@/lib/vitals";
import { ageIfOld, connectionWord } from "@/words";

/**
 * The mark's place: the mesh.
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
 * same person, later the same day) moved them onto this machine's
 * cards — and on 2026-09-02 they left the machine surfaces altogether
 * (「机器页面不需要有token usage的环」): the 额度 landing is their one
 * home now, where 额度 is read — its foot for a while, and since
 * 2026-09-06 a row in its list (「不要有现在下方单独一个订阅的横条」).
 * So this screen is the picture and nothing else, twice over;
 * `QuotaLanding`'s head and tombstone carry the whole road.
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
  onChanged,
  onPickWaiting,
}: {
  rows: SessionRow[];
  devices: DeviceRow[];
  /** Something on the map wrote to the device table — the shell re-reads
      it. **Two acts do**, and they part company inside this file: a
      machine that left takes its card with it, a machine that was
      restyled keeps it.

      **移除设备 is on that card and nowhere else** (docs/MORE.md 第二轮:
      移除设备在地图上那台机器的卡里). It was a row on 安全 until
      2026-09-20, beside a second copy of the same list on the identity
      card; the list lives here now, which is why the act does. */
  onChanged: () => void;
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
  // Whether the card has the 头像 editor open over it.
  //
  // **Held here rather than in the card**, and it is the Escape
  // listener below that decides where it lives: this screen can have
  // two things open at once now, and the key that closes them has to
  // know which one is on top. It was the card's own state for an
  // afternoon, and one Escape took the editor *and* the machine under
  // it — a person putting a colour picker away lost the card they were
  // reading. The first repair asked the DOM (`[role="dialog"]`) instead
  // of keeping this flag; it went red exactly as before, because
  // whether that attribute has caught up with the press is the dialog
  // library's timing and not a fact this file can read.
  const [editing, setEditing] = useState(false);
  // Escape closes, wherever the focus is. Bound while a card is open
  // and not before: a listener that is always on is a key this screen
  // has taken away from everything under it for the 99% of the time
  // nothing is open.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      // The editor is above the card, so the press is its to answer —
      // the dialog closes itself, and the card stays where it was.
      if (e.key === "Escape" && !editing) setOpen(null);
    };
    // **Capture, and the third argument is the whole repair.** The
    // dialog library listens on `document` in the bubble phase; a
    // bubble listener here would be *replaced* while that handler runs,
    // because closing the dialog re-renders this component and this
    // effect re-binds — and a listener added while the event is still
    // at `document` is copied in when the event reaches `window`. So
    // the press arrived at the freshly-bound listener, whose `editing`
    // already read false, and one Escape took both layers. Measured
    // three times against three different repairs (2026-09-21); in the
    // capture phase this runs before anything has closed anything, so
    // what it reads is the screen as the key found it.
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [open, editing]);
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
              // Both exits put the editor away with the card: a flag
              // left standing would open the next card with a dialog
              // already on it.
              onClose={() => {
                setEditing(false);
                setOpen(null);
              }}
              onForgot={() => {
                // The card is about a row that is leaving the table, so
                // it closes first — a card that outlived its machine
                // would be a fact about nothing.
                setEditing(false);
                setOpen(null);
                onChanged();
              }}
              // The same re-read **without** the close: the machine is
              // staying, only its face changed.
              onRestyled={onChanged}
              editing={editing}
              onEditing={setEditing}
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
  onClose,
  onForgot,
  onRestyled,
  editing,
  onEditing,
  onPickWaiting,
}: {
  row: DeviceRow;
  /** Only this machine's sessions — filtered by the caller, which is
      the one holding the whole table. */
  rows: SessionRow[];
  onClose: () => void;
  onForgot: () => void;
  /** A face was written — the table is re-read and **the card stays
      open**.

      ⚠️ **Not `onForgot`.** It was, for one afternoon, and every colour
      press closed the card: 移除设备's callback shuts the card because
      the row it is about is leaving, and an act that removes nothing
      borrowed that shape. The two callbacks differ by exactly the thing
      a person sees, which is why they are two. */
  onRestyled: () => void;
  /** Whether the 头像 editor is open over this card — **the map's
      state, not this card's**, because the map is what owns the Escape
      key for this screen (see its listener). */
  editing: boolean;
  onEditing: (open: boolean) => void;
  onPickWaiting: (row: SessionRow) => void;
}) {
  const tracks = vitalsTracks(row);
  const aged = row.vitals ? ageIfOld(row.vitals.age_ms) : null;
  const blocked = rows.filter((r) => r.word === "blocked");
  const [forgetError, setForgetError] = useState<string | null>(null);
  // 地图上点任何一台, 进的是同一个编辑器 (docs/MORE.md 第二轮).
  // A dialog rather than another page: restyling a machine you pressed
  // on the map is an errand — 办完就回去 — and the map underneath is
  // where you were. **Whether it is open is the map's state**, handed
  // down: the key that closes both layers is bound up there.
  const setEditing = onEditing;
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
          <span data-node-card-name className="min-w-0 truncate text-body">
            {row.name}
          </span>
          {connectionWord(row) && (
            <span data-node-card-connection className="flex-none text-callout text-muted-foreground">
              {connectionWord(row)}
            </span>
          )}
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
              <span className="text-callout text-muted-foreground">{trackName(t.id)}</span>
            </span>
          ))}
        </div>
      ) : (
        // Never reached is not a reading of zero, and empty tracks would
        // say khor asked and got zeros (docs/SESSION.md 离线).
        <div data-node-card-never className="pt-in text-callout text-muted-foreground">
          {cli.vitals_never}
        </div>
      )}
      {/* This machine's sessions. **Zero of them is one sentence, not
          four zeros**: four counts all reading 0 is a table of nothing,
          and the list's own empty word already says it in one line. */}
      <div data-node-card-sessions className="mt-in border-t pt-in">
        {rows.length === 0 ? (
          <div data-count-none className="text-callout text-muted-foreground">
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
      {/* The subscription's arcs left this card with the machine card's
          on 2026-09-02 (「机器页面不需要有token usage的环」) — the 额度
          landing's foot holds them now. */}
      {/* Only once it is worth saying, the seat's own rule (`ageIfOld`):
          a reading taken a moment ago saying 「0 秒前」 reads as a fault. */}
      {aged !== null && (
        <div data-node-card-age className="pt-in text-callout text-muted-foreground">
          {cli.vitals_taken(aged)}
        </div>
      )}
      {/* 头像: the same editor the avatar's own page draws, opened on
          **this** machine. A face is 全网同一份 and anyone may set
          anyone's (`khor_sync::devices`), so there is nothing to refuse
          here and nothing about this machine being 「not mine」. */}
      <div className="mt-in flex flex-wrap items-center gap-row border-t pt-in">
        <Button
          size="sm"
          data-node-card-avatar={row.id}
          onClick={() => setEditing(true)}
          className="min-h-touch flex-none"
        >
          {gui.settings_face}
        </Button>
      </div>
      <Dialog open={editing} onOpenChange={setEditing}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{row.shown}</DialogTitle>
          </DialogHeader>
          <FaceSettings machine={row.id} onChanged={onRestyled} />
        </DialogContent>
      </Dialog>

      {/* 移除设备 (`khor forget`, docs/MORE.md 第二轮). **On this card and
          on no other surface**: the machine it is about is the one whose
          card you opened, so nothing has to be said about which one. It
          is drawn on every machine except this one — `Node::forget_device`
          refuses this machine by name and sends you to `khor leave`,
          which is the act on the page under this card. */}
      {!row.me && (
        <div className="mt-in flex flex-wrap items-center gap-row border-t pt-in">
          <Button
            variant="ghost"
            size="sm"
            data-node-card-forget={row.id}
            onClick={() => {
              setForgetError(null);
              forgetDevice(row.name).then(onForgot, (e: unknown) =>
                setForgetError(String(e instanceof Error ? e.message : e)),
              );
            }}
            className="min-h-touch flex-none text-destructive"
          >
            {gui.delete}
          </Button>
          {/* The whole cost in one clause, and **the same clause the row
              menu's confirm uses** — it is one act, so it is one
              sentence (`forget-machine-confirm`). */}
          <span className="min-w-0 flex-1 text-callout text-muted-foreground">
            {gui.forget_machine_confirm}
          </span>
        </div>
      )}
      {forgetError && (
        <div data-node-card-forget-refused className="pt-in text-callout text-destructive">
          {forgetError}
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
      <span className="text-title-md tabular-nums" style={tone ? { color: tone } : undefined}>
        {n}
      </span>
      <span className="text-callout text-muted-foreground">{word}</span>
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
