import { useEffect, useRef, useState } from "react";

import type { DeviceRow, HooksState, Load, Strain } from "@/api";
import { fetchLoad } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { Face, Glyph, type Pip } from "@/components/Face";
import { RING_SIZE_CARD, RING_STROKE_CARD } from "@/components/Ring";
import { IconBack, IconChevronDown, IconChevronUp } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { useTight } from "@/hooks/use-tight";
import { gaugeColor } from "@/lib/vitals";
import { YIELD_MS } from "@/lib/tempo";
import { CopyId } from "@/views/DetailPane";
import { ago, hopWord, lasting } from "@/words";

/**
 * One machine, opened from the devices pane.
 *
 * **This is the destination the machine rows were waiting for.** The
 * previous batch left those rows inert on a stated judgment — no
 * affordance may suggest a destination that does not exist — and the
 * judgment has not changed, only its input: there is a destination now,
 * so on *this* pane the row becomes a button. The files and browser
 * panes still list machines with nowhere to send you, and their rows
 * stay exactly as they were until their own batches give them one.
 *
 * No title here either (docs/UX.md). The machine's name is the content,
 * not a heading over it.
 */
export function MachineCard({
  row,
  narrow,
  onBack,
  hooks,
  hooksFailed,
  onInstallHooks,
  onUninstallHooks,
  onOpenTerm,
  onBrowseFiles,
  onBorrowWeb,
}: {
  row: DeviceRow | null;
  narrow: boolean;
  onBack: () => void;
  /** This machine's hooks, or `null` until the first answer is in. */
  hooks: HooksState | null;
  hooksFailed: boolean;
  onInstallHooks: () => void;
  onUninstallHooks: () => void;
  /** 开会话 — rejects with the refusal, which lands on the button. */
  onOpenTerm: (row: DeviceRow) => Promise<void>;
  onBrowseFiles: (row: DeviceRow) => void;
  /** 借网 — this machine's network, opened from where you stand
      (docs/NET.md 借网). Local navigation, so it cannot fail. */
  onBorrowWeb: (row: DeviceRow) => void;
}) {
  return (
    <section data-machine-card className="flex h-full min-w-0 flex-col">
      {/* **A strip only where something has to live in it.** Back exists
          on the narrow face, where the list really is off-screen; wide
          keeps the list beside this, so there is nothing to go back
          from. And the strip carries no name: the machine's name is the
          first thing in the card below, so putting it here too would be
          the same word twice on one screen — which is what a page title
          is, whatever it is called (docs/UX.md 所有页面不要标题). The
          session detail's header is not a counter-example: what it shows
          is not repeated under it. */}
      {narrow && (
        // **`min-h`, not `h`** — the same move the session detail's bar
        // made, for the same reason: the bar's look is still `--ctl-lg`,
        // but a 44 target inside a fixed 40 would light up a band that
        // is cut off, and the floor's own rule is that what paints the
        // pressed state paints the grown box (tokens.css `--touch`).
        <header className="flex min-h-ctl-lg flex-none items-center gap-2 border-b px-3">
          {/* The narrow face's only way back to the machine list, and it
              was an icon-only 30 square. There is no second exit from a
              detail screen on a phone, so this is the one control on
              this card that must not be under the floor. */}
          <Button
            variant="ghost"
            size="icon"
            aria-label={gui.back}
            data-back
            onClick={onBack}
            className="min-h-touch min-w-touch"
          >
            <IconBack />
          </Button>
        </header>
      )}
      {row && (
        // **Keyed by the machine**, so every piece of state on this card
        // belongs to the machine it was made on: a sort somebody set on
        // one machine's table is not an opinion about the next one's,
        // and neither is a turned-over dial or a scroll position.
        //
        // **And no `narrow` below this line.** Every part of the card
        // that changes shape measures its own box now — the drawer's
        // rows, the process table — because the card is not the window,
        // and a window breakpoint left a 106px overflow band the first
        // time it was tried (`use-tight.ts`). The strip above still
        // asks the shell: whether there is a way back is a fact about
        // the screen, not about a box on it.
        <Body
          key={row.id}
          row={row}
          hooks={hooks}
          hooksFailed={hooksFailed}
          onInstallHooks={onInstallHooks}
          onUninstallHooks={onUninstallHooks}
          onOpenTerm={onOpenTerm}
          onBrowseFiles={onBrowseFiles}
          onBorrowWeb={onBorrowWeb}
        />
      )}
    </section>
  );
}

/** One machine's card, from its head to its hook. Split from the
    section above only so that `key` can reset every bit of state on it
    when the machine changes. */
function Body({
  row,
  hooks,
  hooksFailed,
  onInstallHooks,
  onUninstallHooks,
  onOpenTerm,
  onBrowseFiles,
  onBorrowWeb,
}: {
  row: DeviceRow;
  hooks: HooksState | null;
  hooksFailed: boolean;
  onInstallHooks: () => void;
  onUninstallHooks: () => void;
  onOpenTerm: (row: DeviceRow) => Promise<void>;
  onBrowseFiles: (row: DeviceRow) => void;
  onBorrowWeb: (row: DeviceRow) => void;
}) {
  const [sort, setSort] = useState<Sort>(SORT_FIRST);
  const { load, why, shown, asking } = useLoad(row, sort);
  return (
        <div className="min-h-0 flex-1 overflow-y-auto p-4">
          <div className="flex flex-col gap-row">
            <Head
              row={row}
              onOpenTerm={onOpenTerm}
              onBrowseFiles={onBrowseFiles}
              onBorrowWeb={onBorrowWeb}
            />
            <Readings row={row} day={load?.cpu_day ?? null} top={load?.top ?? []} />
            <Busy
              load={load}
              why={why}
              sort={shown ?? sort}
              asking={asking}
              onSort={setSort}
            />
          {/* The subscription's arcs left this card on 2026-09-02
              (「机器页面不需要有token usage的环」) — the 额度 landing's
              foot is their one home now, where 额度 is read.
              `QuotaRings`' head carries the whole road. */}
          {/* **Only on this machine's card**, because the file it edits
              is on this machine. Putting it on every card would be a
              control that silently applies to one machine out of a list.
              Installing on a machine you are looking at from here
              belongs to the batch that adds remote verbs, and when it
              lands, this is where it goes. */}
          {row.me && (
            <Hooks
              state={hooks}
              failed={hooksFailed}
              onInstall={onInstallHooks}
              onUninstall={onUninstallHooks}
            />
          )}
          </div>
        </div>
  );
}

/**
 * The card's head: who this machine is, what you can do on it, and
 * which khor it is running.
 *
 * **Three lines and one row of verbs** (2026-09-03 to the user's
 * layout): the name with the id beside it small, under it the road and
 * how long the machine has been up, and on the right the verbs over
 * the version. The id is small **and** the control that copies it —
 * `CopyId`'s `id` face — because a string already on screen does not
 * need a button to fetch it.
 *
 * **The version moved up here from a foot** and brought the freshness
 * with it: both are facts about the *answer*, not about the machine's
 * hardware, and they belong beside the machine's name for the same
 * reason a letter's date belongs at its head. The pin went the other
 * way — off the card entirely (2026-09-03:「置顶统一和 session 一样,
 * 右键机器选择」): one gesture for one job across both lists beats a
 * button that only one of the two surfaces has.
 */
function Head({
  row,
  onOpenTerm,
  onBrowseFiles,
  onBorrowWeb,
}: {
  row: DeviceRow;
  onOpenTerm: (row: DeviceRow) => Promise<void>;
  onBrowseFiles: (row: DeviceRow) => void;
  onBorrowWeb: (row: DeviceRow) => void;
}) {
  const v = row.vitals;
  const days = v ? Math.floor(v.vitals.uptime_s / 86_400) : 0;
  const hours = v ? Math.floor(v.vitals.uptime_s / 3_600) : 0;
  return (
    <div className="flex flex-wrap items-start gap-row">
      {/* The same derivation as its row and as the rail's foot: one
          machine, one face, painted from what that machine said about
          itself. Bigger here, not different. */}
      <MachineAvatar face={row.face} className="size-avatar-lg" />
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 flex-wrap items-center gap-row">
          <span className="min-w-0 truncate text-title font-title">{row.name}</span>
          {/* The first twelve characters — a prefix to recognise, and
              pressing it puts all sixty-four on the clipboard. */}
          <CopyId id={row.id} face="id" />
        </div>
        <div className="flex flex-wrap items-center gap-row text-sm text-muted-foreground">
          {/* Which road this machine is being reached over
              (`khor_core::Hop`), the same reading `khor devices`
              prints, from the same call. Not a liveness signal — it
              lags the far end going away by minutes; the freshness on
              the right is what says whether this machine has been heard
              from.

              Three shapes for one slot. This machine reaches itself
              over nothing, so its word is 本机 and it is deliberately
              **not** `data-hop` — smoke asserts the own card carries no
              road. A satellite has no road of its own; its sponsor
              speaks for it (附属机器批). */}
          {row.me ? (
            <span data-machine-here className="flex-none">
              {gui.machine_here}
            </span>
          ) : row.sponsor ? (
            <span data-via-sponsor={row.sponsor} className="flex-none">
              {cli.via_sponsor(row.sponsor)}
            </span>
          ) : (
            row.hop && (
              <span data-hop={row.hop} className="flex-none">
                {hopWord(row.hop)}
              </span>
            )
          )}
          {v && v.vitals.uptime_s > 0 && (
            <span data-uptime className="flex-none">
              {days >= 1 ? cli.uptime_days(days) : cli.uptime_hours(hours)}
            </span>
          )}
        </div>
      </div>
      {/* **The facts sit over the verbs** (2026-09-03:「把操作按钮那一行
          与版本、时间一行交换位置」). Both halves of the head then read
          the same way down: what this is on the left, what it says on
          the right, and the things you can press along the bottom of
          both — rather than a row of buttons cutting across the middle
          of the identity. */}
      <div className="flex flex-col items-end gap-in">
        {v && (
          <div className="flex flex-wrap justify-end gap-row text-sm text-muted-foreground">
            {/* The version is one of `khor devices`' sentences and keeps
                its unit; the freshness is the age sentence for a machine
                reached over the wire (its own unit, counted with the
                readings) and the present's clock for this one. */}
            {v.vitals.version && <span data-vitals-unit="version">{cli.vitals_version(v.vitals.version)}</span>}
            {v.age_ms > 0 ? (
              <span data-vitals-age>{cli.vitals_taken(ago(Date.now() - v.age_ms))}</span>
            ) : (
              <span data-updated>{cli.updated_at(clock(Date.now()))}</span>
            )}
          </div>
        )}
        <Verbs
          row={row}
          onOpenTerm={onOpenTerm}
          onBrowseFiles={onBrowseFiles}
          onBorrowWeb={onBorrowWeb}
        />
      </div>
    </div>
  );
}

/**
 * The card's verbs, in one row — the pane stops being a pure dashboard
 * here (docs/KHOR.md 等价: the CLI can already `open --on`, `ls` and
 * `borrow` this machine from any terminal, so the card owes the same
 * moves).
 *
 * **Three verbs, one per landing this machine can be the subject of**
 * (2026-09-03:「右侧是终端、文件、借网三个操作按钮」): a session on it,
 * its disk, its network. 复制 id left this row the same day — the id is
 * on the card, and pressing it copies (`CopyId`'s `id` face), so a
 * button that fetched a visible string was a control with nothing left
 * to do.
 *
 * 开会话 is the one that can fail — the far machine can be away — and
 * the refusal lands on the button in the failure colour, the pin's
 * rule, then yields the offer back after a beat. While the call is out
 * the button is disabled rather than reworded: a second press would
 * open a second session, and "pressed twice, got two" is the kind of
 * surprise a disable is for.
 *
 * **The finger floor is worn here, not fixed in `ui/button.tsx`.** All
 * three are `size="sm"`, which is 24 tall, and this is the screen's
 * main row of actions. Raising the floor inside the `sm` variant would
 * have looked like the smaller edit and been the larger one: `sm` is a
 * *look* for controls that sit inline beside text (the `--ctl-*`
 * family), and tokens.css says in as many words that the touch floor is
 * not a fourth `--ctl-*`. The mechanism is worth knowing before copying
 * it: `min-height` beats `height`, so a caller-side `min-h-touch` lands
 * even though `cn()` cannot take `h-ctl-sm` off (tailwind-merge does not
 * read `ctl-sm` as a height — `ui/button.tsx` has the scar).
 */
function Verbs({
  row,
  onOpenTerm,
  onBrowseFiles,
  onBorrowWeb,
}: {
  row: DeviceRow;
  onOpenTerm: (row: DeviceRow) => Promise<void>;
  onBrowseFiles: (row: DeviceRow) => void;
  onBorrowWeb: (row: DeviceRow) => void;
}) {
  const [opening, setOpening] = useState(false);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    if (!failed) return;
    const t = window.setTimeout(() => setFailed(false), YIELD_MS);
    return () => window.clearTimeout(t);
  }, [failed]);
  return (
    <div data-machine-verbs className="flex flex-wrap items-center justify-end gap-in">
      <Button
        variant="outline"
        size="sm"
        data-open-term
        data-failed={failed}
        disabled={opening}
        className="min-h-touch data-[failed=true]:text-state-failed"
        onClick={() => {
          setOpening(true);
          onOpenTerm(row).then(
            () => setOpening(false),
            () => {
              setOpening(false);
              setFailed(true);
            },
          );
        }}
      >
        {failed ? gui.open_term_on_failed : gui.open_term_on}
      </Button>
      <Button
        variant="outline"
        size="sm"
        data-browse-files
        className="min-h-touch"
        onClick={() => onBrowseFiles(row)}
      >
        {gui.browse_files}
      </Button>
      {/* Outline like its two neighbours (动作排语法批): the verb row is
          this page's "what you can do here" list, three peer verbs in
          one dress — bare text among outlined buttons reads as a lower
          rank, or as a link. */}
      <Button
        variant="outline"
        size="sm"
        data-borrow-web
        className="min-h-touch"
        onClick={() => onBorrowWeb(row)}
      >
        {gui.borrow_web}
      </Button>
    </div>
  );
}

/**
 * The hook, and the one button that turns it on or off.
 *
 * **The button is the report.** It shows the job it will do next, so
 * "install" on screen means the hooks are not in place — one control
 * saying one thing, rather than a status line and a button that can
 * disagree with each other. Failure lands on the same button, in the
 * failure colour and under the word for what failed, exactly as a pin
 * does and for the same reason: this app has nowhere that collects
 * messages, and 做了但没变化 may not look like 失败 (docs/UX.md).
 *
 * Nothing is drawn until the state has been read once. A button guessing
 * "not installed" while the answer is still coming would offer to do
 * something already done, and the press would look like it did nothing.
 */
function Hooks({
  state,
  failed,
  onInstall,
  onUninstall,
}: {
  state: HooksState | null;
  failed: boolean;
  onInstall: () => void;
  onUninstall: () => void;
}) {
  if (!state) return null;
  const on = state.installed;
  return (
    <div data-hooks data-installed={on} className="flex flex-col gap-2 pt-pane">
      <Button
        variant="outline"
        size="sm"
        data-hooks-toggle
        data-failed={failed}
        onClick={on ? onUninstall : onInstall}
        className="min-h-touch self-start data-[failed=true]:text-state-failed"
      >
        {failed
          ? on
            ? gui.uninstall_hooks_failed
            : gui.install_hooks_failed
          : on
            ? gui.uninstall_hooks
            : gui.install_hooks}
      </Button>
      {/* What pressing it buys — **and only while it has not been
          pressed**. Before, this is the reason to press; after, it is a
          remark about something already true, and this app zeroes those
          rather than shortening them (docs/UX.md 文案: 装饰性说明归零).
          The word it promises is one of the six the list already shows,
          which is the only vocabulary this line is allowed (不解释实现);
          the catalog entry carries the rest of that judgment, including
          the tombstone for the second line that used to sit here. */}
      {!on && (
        <div data-hooks-buy className="text-sm text-muted-foreground">
          {gui.hooks_buy_you}
        </div>
      )}
    </div>
  );
}

/** One region of the card, in a box of its own.
 *
 * **The boxes arrived when the card stopped being one column of
 * paragraphs** (2026-09-03): readings, the day, the top and the hook
 * are four answers to four questions, and with nothing but headings
 * between them a person had to find the seams. What a box may not do is
 * make a second column — a row of two boxes side by side is a row that
 * cannot line up, and 「很容易对不齐」 is exactly what the user saw when
 * one was tried. So every panel is full width and they stack. */
function Panel({ children }: { children: React.ReactNode }) {
  return <div className="rounded-md border bg-card px-row py-row">{children}</div>;
}

/**
 * What the machine is doing, and how old that answer is.
 *
 * **Three states, three different things to say.** Never reached is not
 * an old reading and neither is a zero; an old reading is not the
 * present. Collapsing any two of them would put a number on the screen
 * that means something other than what it says (docs/SESSION.md 离线).
 *
 * **Strain is painted, and the colour is a new one** (user ruling
 * 2026-08-17 flipped the earlier "no severity colour" decision — the
 * trigger written on it was exactly 用户要). What survives from the old
 * ruling is its real content: colour here is spoken for — 中断 owns
 * amber, 失败 owns red, and a 95% disk wearing either would dress a
 * machine as a session. So strain has its own token (`--strain`), one
 * hue for one new thing, and the step from tight to critical is weight
 * rather than a second hue. The judgment itself — which resources, at
 * what thresholds, why — is `khor_core::Fill::strain`, minted into the
 * row by gui-core; this file only paints the word it was handed.
 *
 * # Four dials, and every one of them is a whole
 *
 * CPU, GPU, 内存, 磁盘 (2026-09-03:「就是 CPU、GPU、Mem、Disk。GPU 和
 * Disk 是汇总的,下面分硬件显示 bar」). A dial may only draw something
 * that has **one** denominator — that is what lets an arc say how full
 * without being learnt — so the two that are made of parts draw the
 * sum and hand the parts to a row apiece underneath.
 *
 * **The disk dial's percentage is the sum; its colour is the worst part
 * it is made of.** A machine with a full 1 TB scratch disk beside an
 * empty 2 TB one averages to comfortable, and the average is a true
 * number that answers the wrong question — what a person needs to know
 * is that *something* is out of room. The colour is not invented here
 * either: it is the worst of the strains gui-core already minted per
 * volume (`khor_core::Fill::strain`), so this file still only picks
 * among words it was handed.
 *
 * **`data-vitals-unit` marks a reading, and a whole made of parts is
 * not one.** The GPU and disk dials carry no unit when the parts are
 * there — the rows below are the readings `khor devices` prints, one
 * per card and one per volume, and the smoke holds the two faces to
 * that count. On an older node that reports only aggregates, the
 * aggregate *is* the reading and the dial is the unit.
 */
function Readings({
  row,
  day,
  top,
}: {
  row: DeviceRow;
  day: Load["cpu_day"] | null;
  /** The face's own process rows, for the two drawers that have no
      parts to show — see `Drawers`. */
  top: Load["top"];
}) {
  // **The leftmost is open on arrival**, so the block below the dials is
  // never an empty space waiting to be discovered. Pressing the open
  // one closes it: the press is a toggle, and a control whose press
  // does nothing when it is already on is a control that feels stuck.
  const [open, setOpen] = useState("cpu");
  if (!row.vitals) {
    return (
      <Panel>
        <div data-vitals-never className="text-sm text-muted-foreground">
          {cli.vitals_never}
        </div>
      </Panel>
    );
  }
  const { vitals: v, volume_strains } = row.vitals;
  const cards = v.gpus;
  const gpuMean = cards.length > 0 ? cards.reduce((a, c) => a + c.util_pct, 0) / cards.length : null;
  const gpuMem = cards.some((c) => c.mem)
    ? cards.reduce(
        (acc, c) => (c.mem ? { used: acc.used + c.mem.used, total: acc.total + c.mem.total } : acc),
        { used: 0, total: 0 },
      )
    : null;
  // The disk whole: every volume this machine reports, summed. An
  // older node that reports one disk and no volumes has that disk as
  // its whole; one that could not read either has no denominator, and
  // then only the track is drawn.
  const vols = v.volumes;
  const disk =
    vols.length > 0
      ? vols.reduce(
          (acc, x) => ({ used: acc.used + x.fill.used, total: acc.total + x.fill.total }),
          { used: 0, total: 0 },
        )
      : v.disk;
  const diskStrain =
    vols.length > 0
      ? (volume_strains.find((w) => w === "critical") ?? volume_strains.find((w) => w === "tight") ?? null)
      : row.vitals.disk_strain;
  const peak = (xs: (number | null)[]) => {
    const seen = xs.filter((x): x is number => x !== null);
    return seen.length > 0 ? Math.round(Math.max(...seen)) : null;
  };
  // **The rim marks, and only where real parts exist.** A card and a
  // volume are things the aggregate is made *of*; the processes under
  // the CPU and memory dials are occupants of a reading, not parts of
  // it, so those two dials carry no marks (`Face` carries why).
  const cardPips: Pip[] = cards.map((c, i) => ({ word: c.name || `GPU${i}`, v: c.util_pct }));
  const volPips: Pip[] = vols.map((x, i) => ({
    word: x.mount,
    v: x.fill.total > 0 ? (x.fill.used / x.fill.total) * 100 : null,
    strain: volume_strains[i] ?? null,
  }));
  const cpuPeak = day ? peak(day.buckets) : null;
  const memPeak = day ? peak(day.mem) : null;
  const left = (fill: { used: number; total: number }) => cli.fill_left(bytes(Math.max(0, fill.total - fill.used)));
  return (
    <Panel>
      <div data-vitals className="flex flex-wrap justify-around gap-row">
        <Dial
          dial="cpu"
          open={open === "cpu"}
          onOpen={setOpen}
          name="cpu"
          word={cli.vitals_name_cpu}
          fraction={v.cpu_pct / 100}
          sentence={cli.vitals_cpu(Math.round(v.cpu_pct), v.cores)}
          facts={[cli.cores_sub(v.cores), cpuPeak !== null ? cli.day_peak(`${cpuPeak}%`) : null]}
        />
        {cards.length > 0 && gpuMean !== null ? (
          <Dial
            dial="gpu"
            open={open === "gpu"}
            onOpen={setOpen}
            word={cli.vitals_name_gpu}
            pips={cardPips}
            fraction={gpuMean / 100}
            sentence={cli.vitals_gpu(Math.round(gpuMean), cards.length)}
            facts={[
              gpuMem ? cli.vitals_vram(bytes(gpuMem.used), bytes(gpuMem.total)) : cli.gpu_sub_nomem(cards.length),
            ]}
          />
        ) : (
          v.gpu && (
            <Dial
              dial="gpu"
              open={open === "gpu"}
              onOpen={setOpen}
              name="gpu"
              word={cli.vitals_name_gpu}
              fraction={v.gpu.util_pct / 100}
              sentence={cli.vitals_gpu(Math.round(v.gpu.util_pct), v.gpu.cards)}
              // An older node prints 显存 as a sentence of its own, and
              // the count of readings has to match — so on that node the
              // memory line *is* a unit, here on the back of the dial.
              facts={[
                v.gpu.mem ? (
                  <span
                    data-vitals-unit="vram"
                    data-vitals-detail="vram"
                    data-strain={row.vitals.vram_strain ?? undefined}
                    className="data-[strain]:text-strain data-[strain=critical]:font-label"
                  >
                    {cli.vitals_vram(bytes(v.gpu.mem.used), bytes(v.gpu.mem.total))}
                  </span>
                ) : (
                  cli.gpu_sub_nomem(v.gpu.cards)
                ),
              ]}
            />
          )
        )}
        <Dial
          dial="mem"
          open={open === "mem"}
          onOpen={setOpen}
          name="mem"
          word={cli.vitals_name_mem}
          fraction={v.mem.total > 0 ? v.mem.used / v.mem.total : null}
          strain={row.vitals.mem_strain}
          sentence={cli.vitals_mem(bytes(v.mem.used), bytes(v.mem.total))}
          facts={[left(v.mem), memPeak !== null ? cli.day_peak(`${memPeak}%`) : null]}
        />
        <Dial
          dial="disk"
          open={open === "disk"}
          onOpen={setOpen}
          word={cli.vitals_name_disk}
          pips={volPips}
          fraction={disk && disk.total > 0 ? disk.used / disk.total : null}
          strain={diskStrain}
          unknown={!disk}
          sentence={
            disk
              ? vols.length > 0
                ? cli.disk_whole(bytes(disk.total), vols.length)
                : cli.vitals_disk(bytes(disk.used), bytes(disk.total))
              : cli.vitals_disk_unknown
          }
          facts={[disk ? left(disk) : null]}
        />
      </div>
      <Drawers
        row={row}
        top={top}
        open={open}
        heads={{
          cpu: { word: cli.vitals_name_cpu, count: cli.procs_sub, fraction: v.cpu_pct / 100 },
          gpu: {
            word: cli.vitals_name_gpu,
            count: cli.gpu_sub_nomem,
            fraction: gpuMean !== null ? gpuMean / 100 : v.gpu ? v.gpu.util_pct / 100 : null,
          },
          mem: {
            word: cli.vitals_name_mem,
            count: cli.procs_sub,
            fraction: v.mem.total > 0 ? v.mem.used / v.mem.total : null,
            strain: row.vitals.mem_strain,
          },
          disk: {
            word: cli.vitals_name_disk,
            count: cli.vols_sub,
            fraction: disk && disk.total > 0 ? disk.used / disk.total : null,
            strain: diskStrain,
          },
        }}
      />
    </Panel>
  );
}

/**
 * The parts the two aggregate dials are made of: one row per GPU card,
 * then one per volume (2026-09-03:「GPU 和 Disk 是汇总的,下面分硬件显示
 * bar」).
 *
 * **One grid for both groups, and that is the whole point of the
 * component.** They were two blocks with two layouts once, and the bars
 * did not start on the same pixel — 「很容易对不齐」. Sharing a grid
 * makes alignment structural rather than a pair of numbers somebody has
 * to keep equal. The bar column is `1fr`, so the rows follow the card
 * rather than reserving a width the window may not have.
 *
 * **Each row is the unit** for one of `khor devices`' sentences and
 * carries that sentence word for word for the smoke to read (`sr-only`:
 * the row prints the same facts laid out, not the sentence twice).
 *
 * A volume is named by its mount point, spelled the way the machine
 * spells it (2026-09-03:「就写 /Volumes/disk0 就可以」). The short name
 * it wore before answered a question nobody asked — which of these is
 * "the disk" — and lost the one thing a person picks a volume by.
 */
function Drawers({
  row,
  top,
  open,
  heads,
}: {
  row: DeviceRow;
  /** The face's own two hundred process rows. **The CPU and memory
      drawers are computed from these and nothing is asked of the node**
      — there is no per-core array and no memory breakdown on the wire
      (`khor_core::Vitals` is one `cpu_pct` and one `Fill`), so what can
      honestly be shown there is who is taking the most. */
  top: Load["top"];
  open: string;
  /** Each group's heading: the reading's own glyph and its word. **The
      count is not passed in** — it is taken from the rows this
      component actually renders, so the number and the list cannot
      disagree. A machine whose node reports 「4 cards」 while sending no
      per-card list would otherwise print 「4 卡」 over an empty drawer,
      which is a heading lying about the thing directly under it. */
  heads: Record<string, { word: string; fraction: number | null; strain?: Strain | null; count: (n: number) => string }>;
}) {
  if (!row.vitals) return null;
  const { vitals: v, volume_strains } = row.vitals;
  const cards = v.gpus;
  const vols =
    v.volumes.length > 0
      ? v.volumes.map((vol, i) => ({
          unit: "vol",
          word: mountWord(vol.mount),
          fill: vol.fill,
          strain: volume_strains[i] ?? null,
          sentence: cli.vitals_vol(volWord(vol.mount), bytes(vol.fill.used), bytes(vol.fill.total)),
        }))
      : v.disk
        ? [
            {
              unit: "disk",
              word: cli.vitals_name_disk,
              fill: v.disk,
              strain: row.vitals.disk_strain,
              sentence: cli.vitals_disk(bytes(v.disk.used), bytes(v.disk.total)),
            },
          ]
        : [
            {
              unit: "disk",
              word: cli.vitals_name_disk,
              fill: null,
              strain: null,
              sentence: cli.vitals_disk_unknown,
            },
          ];
  // The busiest few, by the reading the drawer is under. Six, because
  // the drawer shows four without scrolling and the fifth and sixth are
  // what the scroll is for.
  const busiest = (by: "cpu" | "mem") =>
    [...top]
      .sort((a, b) => (by === "cpu" ? b.cpu_pct - a.cpu_pct : Number(b.mem) - Number(a.mem)))
      .slice(0, 6);
  const whole = v.cores > 0 ? v.cores * 100 : 100;
  // **What each drawer actually draws.** The heading counts these, not
  // what the node claims elsewhere — see `heads`.
  const drawn: Record<string, number> = {
    cpu: busiest("cpu").length,
    gpu: cards.length,
    mem: busiest("mem").length,
    disk: vols.length,
  };
  const box = useRef<HTMLDivElement>(null);
  // What the three columns need before they stop fitting: the two fixed
  // ones, the two gaps between them, and enough track left for a bar to
  // still read as a length. Written from the tokens rather than guessed
  // — `--part-name` 220 + `--part-num` 172 + two `--gap-row` 12 + 60.
  const tight = useTight(box, 476);
  return (
    <>
      {/* **One box, fixed height, and every group inside it.** The
          groups that are not open are hidden rather than dropped: the
          card's readings are counted against what `khor devices` prints
          (`smoke.mjs`'s `sameNumberOfReadings`), and a drawer that
          unmounted three of them would make the card disagree with the
          terminal — which reads as a broken backend, not as a drawer.
          Hidden elements are still in the document, so the count holds
          and only the eye is spared. */}
      <div
        ref={box}
        data-drawer={open}
        data-tight={tight ? "" : undefined}
        className="mt-row flex max-h-drawer-h min-h-drawer-h flex-col border-t pt-row text-sm tabular-nums"
      >
        {heads[open] && (
          <div data-drawer-head className="mb-in flex items-center gap-in text-aux text-muted-foreground">
            <Glyph fraction={heads[open].fraction} strain={heads[open].strain} />
            <span>{heads[open].word}</span>
            <span aria-hidden="true">·</span>
            <span data-drawer-count={drawn[open]} className="tabular-nums">
              {heads[open].count(drawn[open])}
            </span>
          </div>
        )}
        {/* **The rows scroll, the heading does not.** A heading that
            scrolled away would take with it the one line saying which
            reading these rows belong to — and four rows fit here without
            scrolling at all, which is what makes 「just fits」 and 「just
            does not」 different on screen. */}
        <div
          data-drawer-scroll
          className="min-h-0 flex-1 overflow-y-auto overscroll-contain"
          style={{ scrollSnapType: "y proximity" }}
        >
        <div data-parts="cpu" hidden={open !== "cpu"}>
          {busiest("cpu").map((r) => (
            <Occupant key={r.pid} row={r} share={r.cpu_pct / whole} value={`${Math.round(r.cpu_pct)}%`} />
          ))}
        </div>
        <div data-parts="gpu" hidden={open !== "gpu"}>
          {cards.map((c, i) => {
            const util = Math.round(c.util_pct);
            const sentence = c.mem
              ? cli.vitals_gpu_card_mem(c.name || cli.vitals_name_gpu, util, bytes(c.mem.used), bytes(c.mem.total))
              : cli.vitals_gpu_card(c.name || cli.vitals_name_gpu, util);
            return (
              <div key={`gpu${i}`} data-part={i}>
                <span
                  data-part-name
                  data-vitals-unit="gpu"
                  data-gpu-card={c.name || `GPU${i}`}
                  className="min-w-0 gap-in"
                >
                  <span className="flex-none font-label">GPU{i}</span>
                  {/* The driver's name, when it gave one; a card that
                      will not name itself (Apple's) is "GPU0" and
                      nothing more — not "GPU0 GPU". */}
                  {c.name && <span className="ml-in min-w-0 truncate text-muted-foreground">{c.name}</span>}
                  <span data-vitals-detail="gpu" className="sr-only">
                    {sentence}
                  </span>
                </span>
                <Bar pct={util} strain={null} />
                <span data-part-num className="justify-end gap-row text-right">
                  <span data-gpu-util={util}>{util}%</span>
                  <span className="text-muted-foreground">
                    {c.mem ? `${bytes(c.mem.used)} / ${bytes(c.mem.total)}` : ""}
                  </span>
                  {c.temp_c != null && (
                    <span className="text-muted-foreground">{`${Math.round(c.temp_c)}°C`}</span>
                  )}
                </span>
              </div>
            );
          })}
        </div>
        <div data-parts="mem" hidden={open !== "mem"}>
          {busiest("mem").map((r) => (
            <Occupant
              key={r.pid}
              row={r}
              share={v.mem.total > 0 ? Number(r.mem) / Number(v.mem.total) : null}
              value={bytes(r.mem)}
            />
          ))}
        </div>
        <div data-parts="disk" hidden={open !== "disk"}>
          {vols.map((r, i) => {
            const pct = r.fill && r.fill.total > 0 ? (r.fill.used / r.fill.total) * 100 : null;
            return (
              // By position, not by word: a Mac reports its data volume
              // and `/` as two mounts that can both print as `/`.
              <div key={`vol${i}`} data-part={i}>
                <span
                  data-part-name
                  data-vitals-unit={r.unit}
                  data-vitals-unknown={r.fill ? undefined : ""}
                  className="min-w-0 truncate"
                >
                  {r.word}
                  <span
                    data-vitals-detail={r.unit}
                    data-strain={r.strain ?? undefined}
                    className="sr-only data-[strain]:text-strain data-[strain=critical]:font-label"
                  >
                    {r.sentence}
                  </span>
                </span>
                <Bar pct={pct} strain={r.strain} />
                <span
                  data-part-num
                  data-strain={r.strain ?? undefined}
                  className="justify-end text-right text-muted-foreground data-[strain]:text-strain data-[strain=critical]:font-label"
                >
                  {r.fill ? `${bytes(r.fill.used)} / ${bytes(r.fill.total)}` : cli.vitals_disk_unknown}
                </span>
              </div>
            );
          })}
        </div>
        </div>
      </div>
      {/* **Outside the drawer, and that is not a detail.** Inside the
          scrolling box it made a four-row panel scroll, and then 「just
          fits」 and 「just does not」 look the same on screen.

          And it is not optional on these two: six occupants and six
          parts draw identically, while a part's readings add up to the
          ring above and an occupant's do not — there are several
          thousand more. Without the sentence the two blocks say
          different things in the same shape. */}
      {(open === "cpu" || open === "mem") && (
        <p data-drawer-note className="mt-in text-aux text-muted-foreground">
          {cli.top_share_note}
        </p>
      )}
    </>
  );
}

/** One occupant of a reading: a process, its share of the machine, and
    what it is taking. **Not a part** — see `Drawers`. */
function Occupant({ row, share, value }: { row: Load["top"][number]; share: number | null; value: string }) {
  return (
    <div data-part={row.pid} data-occupant={row.pid}>
      <span data-part-name className="min-w-0 gap-in truncate">
        <span className="truncate">{row.name}</span>
        {row.arg && <span className="ml-in min-w-0 truncate text-muted-foreground">{row.arg}</span>}
      </span>
      <Bar pct={share === null ? null : share * 100} strain={null} />
      <span data-part-num className="justify-end text-right">{value}</span>
    </div>
  );
}

/** One part's share, on the track every other part shares. `null` draws
    the track alone — a reading that could not be taken is not a zero. */
function Bar({ pct, strain }: { pct: number | null; strain: Strain | null }) {
  return (
    <span data-part-bar className="block h-1.5 w-full overflow-hidden rounded-full bg-foreground/10">
      {pct !== null && (
        <span
          data-part-ink
          className="block h-full rounded-full"
          style={{ width: `${Math.max(0, Math.min(100, pct))}%`, background: gaugeColor(strain) }}
        />
      )}
    </span>
  );
}

/** The wall clock for one instant, hours and minutes in the reader's
    own locale — the day chart's two ends and the head's 更新于. */
function clock(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** How deep the table goes. **A bound, and it is honest about being
    one**: the window shows the top 200 of whichever column it is
    ranked by, ranked on the machine itself. Not "every process" —
    turing has ten thousand, and 800 KB every five seconds to show nine
    rows is the kind of traffic the mesh should not spend on a screen.
    Two hundred is past any question a person asks a card ("what is
    eating this machine") and small enough to be free (~16 KB). */
const TOP_LIMIT = 200;

/** How often an open card re-asks. Five seconds is the vitals beat the
    rings above already move on; the table and the band moving on
    another would make the card describe two moments. */
const LOAD_MS = 5_000;
/** …and how long to wait after a refusal before asking again: a far
    machine that is away costs a dial each time, and the sentence on
    the card does not get truer by being re-fetched every five seconds. */
const LOAD_RETRY_MS = 30_000;

/** Which column the table is ordered by, and which way.
 *
 * **Never absent, and never a third state** (2026-09-03:「默认按 CPU
 * 降序。不存在现在这样,一个列名可以点三次,升、降、空」). A press
 * turns the column over and that is all it does: a control whose third
 * press lands somewhere the reader cannot name is a control that reads
 * as broken. The table opens on CPU, biggest first — the question a
 * person opens a machine card to ask. */
type Sort = { col: SortCol; dir: "up" | "down" };
const SORT_FIRST: Sort = { col: "cpu", dir: "down" };
type SortCol = "pid" | "name" | "user" | "cpu" | "mem" | "gpu" | "vram" | "time";

/**
 * What the machine is busy with (2026-09-02:「环下面显示进程 top,最近
 * 24 小时 cpu」): the CPU day as a chart, and the process top as a
 * table, one panel each.
 *
 * **Asked by the card, not carried on the row** — the rings read a
 * reading that rides the device beat, but a process table is asked for
 * only while somebody is looking at it, and the band is a day of
 * buckets that has no business on a two-second heartbeat. The ask
 * itself is `useLoad`, one level up, because the dials' 24-hour marks
 * read the same buckets this chart draws.
 *
 * **Three faces, and the empty one is never drawn.** Nothing until the
 * first answer (the ask is out); the chart and the table once it lands;
 * and the refusal's own sentence in their place when it does not — an
 * older khor over there, or a machine that is away. A blank table would
 * read as an idle machine, which is the one thing this surface must not
 * say by accident (docs/SESSION.md 认不出就不落词).
 */
function Busy({
  load,
  why,
  sort,
  asking,
  onSort,
}: {
  load: Load | null;
  why: string | null;
  /** **The order the rows on screen are in**, not the one just asked
      for — see `useLoad`. */
  sort: Sort;
  /** The column whose answer is still crossing, if any. */
  asking: SortCol | null;
  onSort: (s: Sort) => void;
}) {
  // **A refusal keeps the column heads and loses the rows.** It used to
  // replace the whole table, heads included, and that made an ordinary
  // mistake unrecoverable: ordering a remote by PID is refused by a khor
  // that predates the column, and with the heads gone there was no
  // second column to press — the only way back out was to open another
  // machine and come back (measured 2026-09-04; two probes timed out
  // reaching for a head that was no longer there).
  //
  // The rows still go. An empty table under live heads would say this
  // machine is running nothing, which is the reading the refusal exists
  // to prevent — so the sentence sits where the rows would be, and the
  // heads stay because pressing one is the way out.
  if (why) {
    return (
      <Panel>
        <Top rows={load?.top ?? []} sort={sort} asking={asking} onSort={onSort} why={why} />
      </Panel>
    );
  }
  if (!load) return null;
  return (
    <>
      <Panel>
        <div data-load-day>
          <DayChart day={load.cpu_day} />
        </div>
      </Panel>
      <Panel>
        <Scatter rows={load.top} />
        <Top rows={load.top} sort={sort} asking={asking} onSort={onSort} />
      </Panel>
    </>
  );
}

/**
 * The process table: what is eating this machine, by any column.
 *
 * # A window with a fixed height, not a fold
 *
 * Every process this machine is running is in here, and the box is a
 * fixed height that scrolls itself (2026-09-03:「就不要展开了,就是一个
 * 固定相对高度的窗口,鼠标在里面可以直接拖」). Both alternatives were
 * worse: a folded table hides how much there is, and an unfolding one
 * drops four hundred rows between the card's head and everything below
 * it — so the control that folds it back ends up a long scroll away
 * from the person who wants it. The height is `--top-h`, which carries
 * the rest of that reasoning.
 *
 * The head stays put while the rows move under it: a column head you
 * have to scroll back up to read is a sort control you cannot reach
 * from where the sorting matters.
 *
 * # A column of numbers with holes in it
 *
 * GPU and 显存 are `None` on every machine that cannot report them
 * (macOS reports no per-process GPU at all) and on every process that
 * is not on a card. **The two columns disappear when nothing on this
 * machine can fill them** — a column of blanks says "khor looked and
 * found nothing", which is the opposite of what happened.
 *
 * # Sorting puts absence last, both ways
 *
 * A row whose GPU is unknown is not a row with 0% GPU, so it does not
 * take a zero's place in the order. It sinks in both directions, which
 * is the only arrangement where reversing the sort does not turn "we
 * did not look" into "this is the smallest".
 */
function Top({
  rows,
  sort,
  asking,
  onSort,
  why,
}: {
  rows: Load["top"];
  /** The order these rows are actually in — see `useLoad`. */
  sort: Sort;
  /** The column whose answer has not landed yet. Its head says so, and
      **nothing else on the table moves until the rows do.** */
  asking: SortCol | null;
  onSort: (s: Sort) => void;
  /** Why the machine did not answer this ask. Set means: draw the heads
      and this sentence, and no rows — see `Busy`. The rows still come in
      because they decide which columns exist, and that is a fact about
      the machine rather than about this one failed answer. */
  why?: string | null;
}) {
  // **A column exists when the machine can fill it.** Two blank columns
  // read as 「khor looked and found nothing」, which is the opposite of
  // 「this machine has no cards」; and a whole column of blanks is worse
  // than no column, because it takes width from the ones that speak.
  const onCards = rows.some((r) => r.gpu_pct !== null || r.gpu_mem !== null);
  const onAges = rows.some((r) => r.run_secs != null);
  const ordered = sortRows(rows, sort);
  const box = useRef<HTMLDivElement>(null);
  // Eight columns need more room than the card has for most of its
  // life. What decides is the table's own width, not the window's —
  // the same lesson the drawer's rows cost (`use-tight.ts`).
  const tight = useTight(box, 620);
  const press = (col: SortCol) => {
    // Pressing the column it is already on turns it over. Pressing a
    // new one starts the way that column is read — biggest first for a
    // quantity, A to Z for a name.
    if (sort.col === col) {
      onSort({ col, dir: sort.dir === "down" ? "up" : "down" });
      return;
    }
    // A first press reads the column the way that column is usually
    // read: A to Z for a name, smallest first for a pid — and **longest
    // first for an age**, because 「哪个挂了最久」 is the question, not
    // 「哪个刚起」.
    onSort({ col, dir: col === "name" || col === "user" || col === "pid" ? "up" : "down" });
  };
  const cells = 5 + (onCards ? 2 : 0) + (onAges ? 1 : 0);
  return (
    <div
      ref={box}
      data-load-top-box
      data-tight={tight ? "" : undefined}
      className="max-h-top-h overflow-y-auto"
    >
      <div
        data-load-top
        data-panel
        data-nogpu={onCards ? undefined : ""}
        data-norun={onAges ? undefined : ""}
        className="text-sm tabular-nums"
      >
        <div data-top-heads className="sticky top-0 bg-card text-aux text-muted-foreground">
          {/* **The order the user named**: what you type, what it is,
              what it was started as, then the quantities with the two
              GPU ones beside the CPU rather than after the memory that
              has nothing to do with them. */}
          <Column col="pid" sort={sort} asking={asking} onPress={press} align="right">
            {cli.top_pid}
          </Column>
          <Column col="name" sort={sort} asking={asking} onPress={press} align="left">
            {cli.top_name}
          </Column>
          {/* **The command line is a column, and it does not sort.**
              Sorting by it would need a word on the wire that older
              nodes refuse (`khor_core::TopBy`), and nobody asks 「order
              these by their arguments」. */}
          <span data-c="cmd" className="font-body">
            {cli.top_cmd}
          </span>
          {onAges && (
            <Column col="time" sort={sort} asking={asking} onPress={press} align="right">
              {cli.top_time}
            </Column>
          )}
          <Column col="cpu" sort={sort} asking={asking} onPress={press} align="right">
            {cli.vitals_name_cpu}
          </Column>
          {onCards && (
            <Column col="gpu" sort={sort} asking={asking} onPress={press} align="right">
              {cli.vitals_name_gpu}
            </Column>
          )}
          {onCards && (
            <Column col="vram" sort={sort} asking={asking} onPress={press} align="right">
              {cli.vitals_name_vram}
            </Column>
          )}
          <Column col="mem" sort={sort} asking={asking} onPress={press} align="right">
            {cli.vitals_name_mem}
          </Column>
        </div>
        {why ? (
          <div data-load-why className="py-row text-sm text-muted-foreground" style={{ gridColumn: `span ${cells}` }}>
            {why}
          </div>
        ) : (
          ordered.map((r) => (
            <div key={r.pid} data-top-row={r.pid}>
              {/* **The pid is first and flush left, at every width.** On
                  turing the two vLLM workers carry the same name *and*
                  the same command line — the only things that tell them
                  apart are this number and their card memory. A layout
                  that drops it to save room drops the answer. */}
              {/* **Flush left, not right-aligned.** A pid is an
                  identifier, not a quantity — nobody compares their
                  magnitudes, and right-aligning them in a seven-column
                  track pushes the first thing read away from the edge
                  (measured here: 31.6px in). That is the same move that
                  put the pid 229px into the panel once. */}
              <span data-c="pid" className="whitespace-nowrap text-left text-muted-foreground">
                {r.pid}
              </span>
              <span data-c="name" className="min-w-0 truncate">
                {r.name}
              </span>
              <Cut text={r.arg} cells={cells} />
              {onAges && (
                <span data-c="time" data-top-time className="whitespace-nowrap text-right">
                  {r.run_secs == null ? "" : lasting(r.run_secs * 1000)}
                </span>
              )}
              <span data-c="cpu" className="whitespace-nowrap text-right">
                {Math.round(r.cpu_pct)}%
              </span>
              {onCards && (
                <span data-c="gpu" data-top-gpu className="whitespace-nowrap text-right">
                  {r.gpu_pct === null ? "" : `${Math.round(r.gpu_pct)}%`}
                </span>
              )}
              {onCards && (
                <span data-c="vram" data-top-vram className="whitespace-nowrap text-right">
                  {r.gpu_mem === null ? "" : bytes(r.gpu_mem)}
                </span>
              )}
              <span data-c="mem" className="whitespace-nowrap text-right">
                {bytes(r.mem)}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

/**
 * Every process as a point: **CPU across, memory up**, card memory as
 * the halo around it.
 *
 * # Why these two axes on every machine
 *
 * The pair this replaced was CPU against card memory, and it had two
 * faults that only show on a real fleet. On turing the two axes were
 * filled by **two non-overlapping sets of processes** — the compilers
 * are not on a card, so their `gpu_mem` is `None` — and a scatter whose
 * axes describe different populations is not read as a scatter. And a
 * machine with no card had no second axis at all, so it quietly became
 * memory: **a silent change of axis, which is the widest door to a
 * misreading there is.**
 *
 * CPU and memory exist for every process on every machine. So the three
 * cards can be read side by side, and every row is on the picture.
 *
 * # Card memory is the halo, and area is the encoding
 *
 * Radius by square root, so **area** is proportional to bytes — the
 * codex's rule for encoding a quantity as a size. The foot prints the
 * two largest readings and the pixel radii they produced, so anybody
 * can check on the spot that the two are told apart rather than taking
 * it on faith.
 *
 * **Three states, three appearances, never merged**: memory on the card
 * draws a halo; exactly zero draws a dashed ring (「looked, it is
 * zero」); nothing to read draws nothing at all (「did not look」). The
 * middle one is why they cannot be merged — a card that is idle and a
 * machine that cannot see cards are different facts.
 */
function Scatter({ rows }: { rows: Load["top"] }) {
  const W = 1000;
  const H = 260;
  const pad = 26;
  const maxCpu = Math.max(1, ...rows.map((r) => r.cpu_pct));
  const maxMem = Math.max(1, ...rows.map((r) => Number(r.mem)));
  const vrams = rows.map((r) => r.gpu_mem).filter((v): v is number => v !== null && v > 0);
  const maxVram = vrams.length > 0 ? Math.max(...vrams) : 0;
  const RMAX = 11;
  const halo = (v: number) => (maxVram > 0 ? RMAX * Math.sqrt(v / maxVram) : 0);
  const x = (v: number) => pad + (v / maxCpu) * (W - pad * 2);
  const y = (v: number) => H - pad - (v / maxMem) * (H - pad * 2);
  // The two biggest card readings, printed as bytes and as the radius
  // they became — the foot is what makes the encoding checkable.
  const two = [...vrams].sort((a, b) => b - a).slice(0, 2);
  return (
    <div data-top-plot className="flex flex-col gap-in">
      <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" className="h-40 w-full text-foreground">
        <line
          x1={pad}
          y1={H - pad}
          x2={W - pad}
          y2={H - pad}
          stroke="currentColor"
          strokeOpacity={0.15}
          vectorEffect="non-scaling-stroke"
        />
        <line
          x1={pad}
          y1={pad}
          x2={pad}
          y2={H - pad}
          stroke="currentColor"
          strokeOpacity={0.15}
          vectorEffect="non-scaling-stroke"
        />
        {rows.map((r) => {
          const cx = x(r.cpu_pct);
          const cy = y(Number(r.mem));
          return (
            <g key={r.pid} data-plot-point={r.pid}>
              {r.gpu_mem !== null && r.gpu_mem > 0 && (
                <circle
                  data-plot-halo={Math.round(halo(r.gpu_mem) * 10) / 10}
                  cx={cx}
                  cy={cy}
                  r={halo(r.gpu_mem)}
                  fill="currentColor"
                  fillOpacity={0.14}
                />
              )}
              {r.gpu_mem === 0 && (
                <circle
                  data-plot-zero
                  cx={cx}
                  cy={cy}
                  r={5}
                  fill="none"
                  stroke="currentColor"
                  strokeOpacity={0.35}
                  strokeDasharray="2 2"
                  vectorEffect="non-scaling-stroke"
                />
              )}
              <circle cx={cx} cy={cy} r={1.8} fill="currentColor" fillOpacity={0.75}>
                <title>{`${r.name} ${Math.round(r.cpu_pct)}% ${bytes(r.mem)}`}</title>
              </circle>
            </g>
          );
        })}
      </svg>
      <div className="flex justify-between gap-row text-aux text-muted-foreground">
        <span>
          {cli.vitals_name_cpu} × {cli.vitals_name_mem}
        </span>
        {two.length === 2 && (
          <span data-plot-foot className="tabular-nums">
            {cli.top_plot_halo(
              bytes(two[0]),
              Math.round(halo(two[0]) * 10) / 10,
              bytes(two[1]),
              Math.round(halo(two[1]) * 10) / 10,
            )}
          </span>
        )}
      </div>
    </div>
  );
}

/**
 * The command line's cell: the text, and **a mark when it did not fit**.
 *
 * # Why a mark and not an ellipsis
 *
 * An ellipsis reads as 「the value ends about here」 — it is punctuation
 * inside the text. What has to be said is 「there is more of this, and
 * you can have it」, which is a statement about the cell, not about the
 * string. So it is a small bordered square outside the words.
 *
 * # Why a press and not a hover
 *
 * The narrow card has no hover at all, and narrow is exactly where the
 * cell runs out of room first — a tooltip would put the answer where
 * the people who need it cannot reach. A press opens it in place;
 * keyboard reaches it the same way (`role`, `tabindex`, Enter and
 * Space), because a control only a pointer can work is a control half
 * the ways in cannot.
 *
 * # The mark is measured, never guessed
 *
 * `scrollWidth` against `clientWidth`, watched with a `ResizeObserver`.
 * A mark decided by counting characters would appear on cells that fit
 * (decoration) and miss cells that do not (a silent truncation), and
 * both errors look fine on the machine of whoever wrote the guess.
 */
function Cut({ text, cells }: { text: string | null; cells: number }) {
  const ref = useRef<HTMLSpanElement>(null);
  const [over, setOver] = useState(false);
  const [open, setOpen] = useState(false);
  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    // Only while closed: open, the text wraps and nothing overflows, so
    // measuring then would retract the mark and take the way back with
    // it.
    if (open) return;
    const measure = () => setOver(node.scrollWidth - node.clientWidth > 1);
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(node);
    return () => ro.disconnect();
  }, [text, open]);
  if (!text) {
    // **Nothing at all, not an empty cell.** A kernel thread has no
    // command line; an element that exists but is empty would make
    // 「count the ones that are missing」 impossible to ask.
    return <span data-c="cmd" />;
  }
  const cut = over;
  return (
    <span
      data-c="cmd"
      data-cut={cut ? "" : undefined}
      data-open={open ? "" : undefined}
      role={cut ? "button" : undefined}
      tabIndex={cut ? 0 : undefined}
      aria-expanded={cut ? open : undefined}
      onClick={cut ? () => setOpen((on) => !on) : undefined}
      onKeyDown={
        cut
          ? (e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                setOpen((on) => !on);
              }
            }
          : undefined
      }
      className={cn("flex min-w-0 items-baseline gap-in text-muted-foreground", cut && "cursor-pointer")}
      style={open ? { gridColumn: `span ${cells}` } : undefined}
    >
      <span ref={ref} className={cn("min-w-0", open ? "break-all" : "truncate")}>
        {text}
      </span>
      {cut && !open && (
        <span
          data-cut-mark
          aria-hidden="true"
          className="size-2.5 flex-none rounded-xs border border-muted-foreground/60"
        />
      )}
    </span>
  );
}

/** One column head: the word, and whether the table is ordered by it.
    A press is what sorts, so it is a button — `aria-sort` says the same
    thing to a reader who cannot see the chevron. */
function Column({
  col,
  sort,
  asking,
  onPress,
  align,
  grow,
  children,
}: {
  col: SortCol;
  sort: Sort;
  /** The column whose answer is still crossing — see `Top`. */
  asking: SortCol | null;
  onPress: (col: SortCol) => void;
  align: "left" | "right";
  /** The one column that takes the leftover width, where there is any
      to take — narrow only, and see `Top` for why. */
  grow?: boolean;
  children: React.ReactNode;
}) {
  const on = sort.col === col;
  // **`aria-sort` describes the rows, and the rows have not changed
  // yet.** It used to turn over on the press, which is a promise about
  // an answer still crossing the wire — and to a reader who cannot see
  // the table, a promise is all there is. `data-asking` is the only
  // thing a press moves.
  const waiting = asking === col;
  return (
    <th
      data-top-head={col}
      data-asking={waiting ? "" : undefined}
      aria-sort={on ? (sort.dir === "up" ? "ascending" : "descending") : "none"}
      aria-busy={waiting || undefined}
      className={cn(
        "font-body",
        align === "right" ? "text-right px-row" : "text-left",
        grow && "w-full",
        // Not a new visual language — the head simply goes quiet while
        // its answer is out. What this looks like finally is the design
        // lane's call.
        waiting && "opacity-60",
      )}
    >
      <button
        type="button"
        onClick={() => onPress(col)}
        // The floor on both axes (tokens.css `--touch`): the ink here
        // is two characters, and a head that is 44 tall and 25 wide is
        // a target you miss sideways. `w-full` spreads the press over
        // the whole column head rather than over the word alone, and
        // the floor is what the narrow columns grow to.
        className={cn(
          "inline-flex min-h-touch min-w-touch w-full items-center gap-in hover:text-foreground",
          align === "right" ? "flex-row-reverse justify-start" : "justify-start",
          on && "text-foreground",
        )}
      >
        {children}
        {on && (sort.dir === "up" ? <IconChevronUp className="size-3" /> : <IconChevronDown className="size-3" />)}
      </button>
    </th>
  );
}

/** The order a column press asks for.
 *
 * **Absence sinks in both directions.** A process khor could not read a
 * GPU figure for is not a process using none of the GPU, so it may not
 * take the place a zero would: reversing the sort would otherwise turn
 * 「没看到」 into 「最小的那个」, which is the same neighbouring-value
 * mistake the readings are built to avoid.
 */
function sortRows(rows: Load["top"], sort: Sort): Load["top"] {
  const dir = sort.dir === "up" ? 1 : -1;
  const num = (r: Load["top"][number]): number | null => {
    switch (sort.col) {
      case "cpu":
        return r.cpu_pct;
      case "mem":
        return r.mem;
      case "gpu":
        return r.gpu_pct;
      case "vram":
        return r.gpu_mem;
      case "time":
        return r.run_secs;
      default:
        return null;
    }
  };
  const text = (r: Load["top"][number]) => (sort.col === "user" ? r.user : r.name);
  return [...rows].sort((a, b) => {
    if (sort.col === "name" || sort.col === "user") {
      return dir * text(a).localeCompare(text(b)) || a.pid - b.pid;
    }
    const [x, y] = [num(a), num(b)];
    if (x === null && y === null) return a.pid - b.pid;
    if (x === null) return 1;
    if (y === null) return -1;
    return dir * (x - y) || a.pid - b.pid;
  });
}

/**
 * The one ask behind both the dials' 24-hour marks and the chart.
 *
 * Its cadence is the vitals beat the dials already move on; a refusal
 * backs off, because a far machine that is away costs a dial each time
 * and the sentence on the card does not get truer by being re-fetched
 * every five seconds.
 */
function useLoad(row: DeviceRow | null, sort: Sort) {
  const [load, setLoad] = useState<Load | null>(null);
  const [why, setWhy] = useState<string | null>(null);
  // **Which order the rows on screen are actually in.**
  //
  // Not the same thing as the order somebody just asked for, and the
  // gap between the two is where a real bug lived (measured
  // 2026-09-04): pressing PID re-sorted the rows already on screen
  // while the true answer was still crossing, so the table showed **the
  // busiest 200 re-sorted by pid** — `[10, 11, 25, 31, 43, 386, 410,
  // 422]` where the machine's answer is `[1, 2, 3, 4, 6, 9, …]` — and
  // then silently became the real thing a poll later. That is the
  // 「重排冒充排名」 this file's own comment warns about, and it is
  // worse than a blank: a blank says 「还没好」, while this says
  // 「好了」 and is wrong, for up to five seconds, looking exactly like
  // a true answer.
  //
  // So the rows are drawn in the order of the answer that produced
  // them, and this is that order. `null` until the first one lands.
  const [shown, setShown] = useState<Sort | null>(null);
  const id = row?.id ?? null;
  const me = row?.me ?? false;
  const name = row?.name ?? "";
  // **Clearing belongs to the machine changing, not to the column.**
  // Pressing 内存 used to blank the chart, the table and everything
  // under them for one round trip — because the answer was dropped
  // while the new one was in the air, and the day chart is drawn from
  // the same answer. The day did not change; only the order did. So
  // the old answer stays on screen until the new one lands, and the
  // only thing that empties the card is picking a different machine.
  useEffect(() => {
    setLoad(null);
    setWhy(null);
    setShown(null);
  }, [id]);
  useEffect(() => {
    if (id === null) return;
    let stopped = false;
    let timer = 0;
    const ask = async () => {
      let wait = LOAD_MS;
      try {
        // A name, not an id: the node resolves machines by the name its
        // table knows them by, and its own by being asked for itself
        // (`Node::load_on`).
        // **The ranking travels; the rows do not.** turing answered
        // with 10 175 processes the first time this asked for all of
        // them — 800 KB every five seconds to show nine. So the column
        // goes over with the ask and the machine ranks before it cuts
        // (`khor_core::TopBy`), which makes「按显存排的前 200」a true
        // answer rather than a re-sort of the busiest 200 by CPU.
        const l = await fetchLoad(me ? "" : name, TOP_LIMIT, sort.col, sort.dir === "down");
        if (stopped) return;
        setLoad(l);
        setShown({ col: sort.col, dir: sort.dir });
        setWhy(null);
      } catch (e: unknown) {
        if (stopped) return;
        setWhy(String((e as { message?: string })?.message ?? e));
        wait = LOAD_RETRY_MS;
      }
      if (!stopped) timer = window.setTimeout(() => void ask(), wait);
    };
    void ask();
    return () => {
      stopped = true;
      window.clearTimeout(timer);
    };
  }, [id, me, name, sort?.col, sort?.dir]);
  // `asking` is the column somebody pressed whose answer has not landed
  // yet — the head wears it, and nothing else moves until the rows do.
  const asking = shown === null || shown.col !== sort.col || shown.dir !== sort.dir ? sort.col : null;
  return { load, why, shown, asking };
}

/**
 * The machine's last day, one reading to a row (2026-09-03:「GPU、CPU、
 * Mem、Disk 都绘制」).
 *
 * # Why rows and not four lines in one frame
 *
 * Four series over one baseline cross, and every crossing is a place
 * where two numbers become one shape — the reader has to pick them
 * apart by colour or dash, which is a second vocabulary to learn before
 * the chart says anything. Small multiples on a **shared time axis**
 * cost vertical room and give back the two things that matter: each row
 * is read on its own, and a spike at 14:20 lines up vertically across
 * all four. That is the comparison somebody opens this chart for —
 * 「CPU 冲高的时候显存也满了吗」.
 *
 * Each row carries its own name and its own high-water mark, because a
 * shape without a number answers 什么时候 and never 多高.
 *
 * # Still one cell per bucket, and the gate still counts them
 *
 * Every row is 288 adjacent rectangles in a `data-day-cell` group, so a
 * `null` bucket is a group with nothing in it: the recorder was not
 * running, and the chart says so by drawing nothing rather than a zero
 * or a bridge. **A series the recorder never knew is not a row at all**
 * — an older node sends two series, not four flat lines claiming its
 * cards were idle all day.
 *
 * `preserveAspectRatio="none"` lets the width follow the card; the
 * hairline baselines keep their weight through `non-scaling-stroke`.
 */
function DayChart({ day }: { day: Load["cpu_day"] }) {
  // **One reading, one cursor, one set of peaks — held here.** The
  // three readings are three ways of looking at the same numbers, so
  // the state that says 「which moment」 belongs to the chart and not to
  // any one of them: keep it inside a row and switching readings drops
  // the moment somebody was looking at, which would make them three
  // charts wearing one heading.
  const [view, setView] = useState<"heat" | "ridge" | "para">("heat");
  const [at, setAt] = useState<number | null>(null);
  const rows = [
    { key: "cpu", word: cli.vitals_name_cpu, series: day.buckets },
    { key: "gpu", word: cli.vitals_name_gpu, series: day.gpu },
    { key: "mem", word: cli.vitals_name_mem, series: day.mem },
    { key: "disk", word: cli.vitals_name_disk, series: day.disk },
    // **The cards' memory, and only where a machine has such a thing.**
    // On unified memory it is empty by construction, because those
    // bytes are already the 内存 row — see `khor_core::CpuDay::vram`.
    // So an NVIDIA box draws five rows and this Mac draws four, from
    // one code path and no branch about which machine it is.
    { key: "vram", word: cli.vitals_name_vram, series: day.vram },
    // A series nobody recorded is not drawn — see the head. `some`
    // rather than `length`: a node can send 288 holes, and a row of
    // holes with a name over it reads as a reading that is missing
    // rather than as one this machine never takes.
  ].filter((r) => r.series.some((v) => v !== null));
  const n = Math.max(1, day.buckets.length);
  const startMs = day.now_ms - day.bucket_ms * n;
  const sameDay = new Date(startMs).toDateString() === new Date(day.now_ms).toDateString();
  return (
    <div data-day-band-axis className="flex flex-col gap-in">
      <div className="flex items-center justify-between gap-row text-aux text-muted-foreground">
        <span>{cli.day_trend}</span>
        {/* The two ends of the window, and nothing between them: the
            rows below share this axis, so naming it once is naming it
            for all four. */}
        <span className="tabular-nums">
          {sameDay ? clock(startMs) : cli.yesterday_at(clock(startMs))} → {clock(day.now_ms)}
        </span>
      </div>
      {view === "para" ? (
        // **All of them in one frame, on one baseline.** This is the
        // arrangement the stacked rows were chosen over — four lines
        // cross, and every crossing is a place where two numbers become
        // one shape — and that is exactly why it is worth having as a
        // reading somebody asks for rather than as the one they get: to
        // read a crossing you have to want to.
        <Overlay rows={rows} at={at} onAt={setAt} />
      ) : (
        rows.map((r) => (
          <DayRow key={r.key} name={r.key} word={r.word} series={r.series} view={view} at={at} onAt={setAt} />
        ))
      )}
      <div className="flex items-center justify-between gap-row text-aux text-muted-foreground">
        {/* The reading, as three words. **Not tabs**: a tab says 「three
            places」, and this is one place the whole time. */}
        <span data-day-views className="flex items-center gap-row">
          {(
            [
              ["heat", cli.day_view_heat],
              ["ridge", cli.day_view_ridge],
              ["para", cli.day_view_para],
            ] as const
          ).map(([key, word]) => (
            <button
              key={key}
              type="button"
              data-day-view-pick={key}
              data-on={view === key}
              aria-pressed={view === key}
              onClick={() => setView(key)}
              // **Wears the touch floor rather than waiving it.** These
              // three sit on a line of their own with nothing to take
              // room from — the floor is a floor under things free to
              // grow, and these are free. (The truncation mark in the
              // table is the case where it is not, and that one is
              // waived by name with what the waiver costs.)
              className="inline-flex min-h-touch min-w-touch items-center justify-center rounded-md px-in data-[on=true]:font-label data-[on=true]:text-foreground"
            >
              {word}
            </button>
          ))}
        </span>
        {/* What the shapes cannot say: the moment under the cursor, and
            every reading at it. Absent until there is one — a readout
            of nothing would be a row of dashes pretending to be data. */}
        {at !== null && (
          <span data-day-at className="tabular-nums">
            {cli.day_at(
              clock(startMs + at * day.bucket_ms),
              rows
                .map((r) => `${r.word} ${r.series[at] === null ? "—" : `${Math.round(r.series[at] as number)}%`}`)
                .join("  "),
            )}
          </span>
        )}
      </div>
    </div>
  );
}

/** Every reading in one frame, as lines over a shared baseline — the
    「叠放」 reading. One path per unbroken run, so a gap stays a gap
    rather than a bridge between two moments that were never measured. */
function Overlay({
  rows,
  at,
  onAt,
}: {
  rows: { key: string; word: string; series: (number | null)[] }[];
  at: number | null;
  onAt: (i: number | null) => void;
}) {
  const n = Math.max(1, rows[0]?.series.length ?? 1);
  const W = 1000;
  const H = 100;
  const w = W / n;
  const y = (pct: number) => H - (Math.max(0, Math.min(100, pct)) / 100) * H;
  return (
    <div data-day-row="all" className="flex items-center gap-row">
      <span className="w-8 flex-none text-aux text-muted-foreground" />
      <svg
        data-day-band
        data-day-view="para"
        viewBox={`0 0 ${W} ${H}`}
        preserveAspectRatio="none"
        className="h-24 min-w-0 flex-1 text-foreground"
        onPointerMove={(e) => {
          const box = e.currentTarget.getBoundingClientRect();
          onAt(Math.max(0, Math.min(n - 1, Math.floor(((e.clientX - box.left) / box.width) * n))));
        }}
        onPointerLeave={() => onAt(null)}
      >
        <line x1={0} y1={H} x2={W} y2={H} stroke="currentColor" strokeOpacity={0.15} vectorEffect="non-scaling-stroke" />
        {rows.map((r, k) => {
          const runs: string[] = [];
          let run: string[] = [];
          r.series.forEach((v, i) => {
            if (v === null) {
              if (run.length > 1) runs.push(run.join(" "));
              run = [];
              return;
            }
            run.push(`${run.length === 0 ? "M" : "L"}${(i * w + w / 2).toFixed(1)},${y(v).toFixed(1)}`);
          });
          if (run.length > 1) runs.push(run.join(" "));
          return runs.map((d, j) => (
            <path
              key={`${r.key}-${j}`}
              data-day-line={r.key}
              d={d}
              fill="none"
              stroke="currentColor"
              strokeOpacity={0.35 + k * 0.14}
              strokeWidth={1.2}
              vectorEffect="non-scaling-stroke"
            />
          ));
        })}
        {at !== null && (
          <line
            data-day-cursor
            x1={at * w + w / 2}
            y1={0}
            x2={at * w + w / 2}
            y2={H}
            stroke="currentColor"
            strokeOpacity={0.5}
            vectorEffect="non-scaling-stroke"
          />
        )}
      </svg>
      <span className="w-16 flex-none" />
    </div>
  );
}

/** One reading's day: its name, its shape, and how high it got.
 *
 * The name sits to the left at a fixed width so the four shapes start
 * on one edge — the same reason the parts under the dials share a grid
 * (`Parts`). Without it each row would begin wherever its own word
 * ended, and a vertical comparison across rows would be off by the
 * length of a Chinese word. */
/**
 * How dark one bucket is drawn, and whether it is drawn in the strain
 * colour at all.
 *
 * **The value is the ink.** Five steps, and the top boundary is 90 —
 * the number the product already treats as the edge (`Fill::strain`),
 * so the band and the dials break at the same place rather than at two
 * numbers a reader has to hold apart.
 *
 * # Which rows may wear the strain colour, and why not the others
 *
 * Only 内存 and 磁盘. Those are the two readings where being full is a
 * thing going wrong: a filesystem at 95 starts failing writes.
 *
 * **A busy CPU is not a warning, and neither is a busy card.** A
 * machine at 95% CPU is a machine doing its job, and `TopRow::gpu_mem`
 * already records the same for the cards — 「a card can read 0% busy
 * while holding 43 GiB」 — so a card at 90% memory is the normal state
 * of a box that is being used. Painting those in the alarm colour would
 * light the alarm on every working machine, and this repo's own ruling
 * is that a badge which is never zero is not a badge.
 */
function shadeOf(pct: number, row: string): { key: string; opacity: number; strain: boolean } {
  const v = Math.max(0, Math.min(100, pct));
  // `>=` at 90, pinned deliberately: the boundary bucket belongs to the
  // top step, the way `Fill::strain` reads its own.
  if (v >= 90) {
    const strain = row === "mem" || row === "disk";
    return { key: strain ? "strain" : "4", opacity: 1, strain };
  }
  if (v >= 70) return { key: "3", opacity: 0.66, strain: false };
  if (v >= 45) return { key: "2", opacity: 0.4, strain: false };
  if (v >= 20) return { key: "1", opacity: 0.22, strain: false };
  return { key: "0", opacity: 0.1, strain: false };
}

function DayRow({
  name,
  word,
  series,
  view,
  at,
  onAt,
}: {
  name: string;
  word: string;
  series: (number | null)[];
  /** Which reading: the ink carries the value, or the height does. */
  view: "heat" | "ridge";
  /** The moment under the cursor, shared by every row — see `DayChart`. */
  at: number | null;
  onAt: (i: number | null) => void;
}) {
  const n = Math.max(1, series.length);
  const W = 1000;
  const H = 100;
  const w = W / n;
  const seen = series.filter((v): v is number => v !== null);
  const peak = seen.length > 0 ? Math.round(Math.max(...seen)) : null;
  return (
    <div data-day-row={name} className="flex items-center gap-row">
      <span className="w-8 flex-none text-aux text-muted-foreground">{word}</span>
      <svg
        data-day-band
        data-day-view={view}
        viewBox={`0 0 ${W} ${H}`}
        preserveAspectRatio="none"
        // **`h-6`, not `h-8`.** Five readings at the old height would
        // be 160px where four were 128px; at 24px apiece they are
        // 120px — a row was added and the block got *shorter*. The
        // inner coordinates do not move: `preserveAspectRatio="none"`
        // lets the same 0–100 box stretch into whatever height it is.
        className="h-6 min-w-0 flex-1 text-foreground"
        aria-hidden="true"
        onPointerMove={(e) => {
          const box = e.currentTarget.getBoundingClientRect();
          onAt(Math.max(0, Math.min(n - 1, Math.floor(((e.clientX - box.left) / box.width) * n))));
        }}
        onPointerLeave={() => onAt(null)}
      >
        <line
          x1={0}
          y1={H}
          x2={W}
          y2={H}
          stroke="currentColor"
          strokeOpacity={0.15}
          vectorEffect="non-scaling-stroke"
        />
        {series.map((b, i) => {
          const step = b === null ? null : shadeOf(b, name);
          return (
            <g key={i} data-day-cell data-gap={b === null ? "" : undefined}>
              {b !== null && step !== null && (
                <rect
                  data-day-shade={step.key}
                  x={i * w}
                  // **Full height, every cell.** The value is in the
                  // ink, not in the height: a column whose height was
                  // the reading made five stacked rows into five
                  // little charts to be read one at a time, and the
                  // comparison this block exists for is vertical —
                  // 「CPU 冲高的时候显存也满了吗」 — which wants one
                  // column of one colour per moment.
                  // **Heat puts the value in the ink; ridge puts it in
                  //     the height.** Same numbers, two readings — and
                  //     the ridge is the one that answers 「how much」
                  //     at a glance while the heat answers 「when」.
                  y={view === "ridge" ? H - (Math.max(0, Math.min(100, b)) / 100) * H : 0}
                  width={w + 0.6}
                  height={view === "ridge" ? (Math.max(0, Math.min(100, b)) / 100) * H : H}
                  fill={step.strain ? "var(--strain)" : "currentColor"}
                  fillOpacity={view === "ridge" ? (step.strain ? 1 : 0.5) : step.opacity}
                >
                  <title>{`${Math.round(b)}%`}</title>
                </rect>
              )}
            </g>
          );
        })}
        {/* Twenty-four hour marks, one an hour. **Not decoration**: they
            replaced a hair per bucket, and the difference is that an
            hour is a unit somebody can count. 288 five-minute buckets
            at three pixels each cannot be counted, so a tick per bucket
            was a decoration claiming to be a scale. */}
        {at !== null && (
          <line
            data-day-cursor
            x1={at * w + w / 2}
            y1={0}
            x2={at * w + w / 2}
            y2={H}
            stroke="currentColor"
            strokeOpacity={0.5}
            vectorEffect="non-scaling-stroke"
          />
        )}
        {/* The marks *between* the hours, so the last one does not land
            on the band's own right edge where it is a border rather
            than a scale: 24 hours have 23 boundaries inside them. */}
        {Array.from({ length: Math.max(0, Math.ceil(n / 12) - 1) }, (_, k) => (
          <line
            key={k}
            data-day-hour
            x1={(k + 1) * 12 * w}
            y1={0}
            x2={(k + 1) * 12 * w}
            y2={H}
            stroke="currentColor"
            strokeOpacity={0.12}
            vectorEffect="non-scaling-stroke"
          />
        ))}
      </svg>
      {/* What the shape cannot say. Absent when nothing was measured —
          a peak of 0% would be a reading. */}
      <span data-day-peak className="w-16 flex-none text-right text-aux tabular-nums text-muted-foreground">
        {peak === null ? "" : cli.day_row_peak(`${peak}%`)}
      </span>
    </div>
  );
}

/**
 * One dial: an arc with its share inside, its name under it, and the
 * quantities behind the share on its back.
 *
 * **Only the arc turns** (2026-09-03:「翻转只翻转环,下面的标题保留」).
 * The name is what a person is looking for while scanning four of
 * these, and a name that vanishes mid-turn makes them wait for the
 * animation to know which reading they are reading.
 *
 * **What the back says is what the front cannot.** The front is a share
 * — 96% — and the back is the quantity a person acts on: 还剩 51.6G, the
 * day's high-water mark, how many cards or volumes the whole is made
 * of. Under those sits the sentence `khor devices` prints, which is
 * both the provenance of the numbers above it and what smoke compares
 * word for word.
 *
 * The unit and the sentence's mark ride together only when this dial
 * *is* a reading — see `Readings` on why an aggregate over parts is
 * not.
 */
function Dial({
  dial,
  open,
  onOpen,
  name,
  word,
  facts,
  sentence,
  fraction,
  strain,
  unknown,
  pips,
}: {
  /** **Which reading this dial is, for anything that has to find it.**
      Separate from `name`, which is the key on the wire and is absent
      on the dials whose parts are counted elsewhere — leaving those two
      unaddressable, which is how a gate came to measure the wrong
      element (2026-09-04). Every dial has one of these. */
  dial: string;
  /** Whether this dial's drawer is the one showing. */
  open: boolean;
  onOpen: (dial: string) => void;
  /** The reading's key on the wire, or none for a dial that is a
      picture of readings counted elsewhere (the GPU and disk dials
      above their per-part rows). Without it the unit and detail marks
      are not set. */
  name?: string;
  /** The reading's name, under the dial and never turning with it. */
  word: string;
  /** One or two lines of what the share is made of. `null` entries are
      dropped, so a caller may pass a fact that does not exist yet (a
      machine with no day recorded has no 24-hour mark). */
  facts: (React.ReactNode | null)[];
  /** What `khor devices` prints for this reading, word for word. */
  sentence: string;
  /** `null` when there is no denominator to divide by, and then only the
      track is drawn — a full circle of arc would read as "this is at
      100%", and no circle at all as "this is at zero". */
  fraction: number | null;
  /** The word gui-core minted, absent for the units that carry none
      (CPU, GPU) as much as for a reading below the first step. */
  strain?: Strain | null;
  /** This reading could not be taken at all, as opposed to being zero. */
  unknown?: boolean;
  /** The parts this reading is made of — cards, volumes — each marked
      on the rim at its own reading. **Only where parts exist**: the
      processes under CPU and memory are occupants, not parts (`Face`
      carries why). */
  pips?: Pip[];
}) {
  // A percentage, because the arc is one and the two should agree at a
  // glance. The exact quantity — bytes, cores, cards — is the back's job.
  const value = fraction === null ? null : `${Math.round(fraction * 100)}%`;
  const lines = facts.filter((f) => f !== null && f !== undefined);
  return (
    <div className="flex flex-col items-center gap-in">
      <button
        type="button"
        data-metric
        data-dial={dial}
        data-open={open}
        data-vitals-unit={name}
        data-vitals-unknown={unknown ? "" : undefined}
        aria-expanded={open}
        // The sentence, not "CPU" — a name that reads out the short form
        // would make a screen reader turn the card over for the number.
        aria-label={sentence}
        // **A toggle, not an open.** Pressing the dial that is already
        // showing closes it — anything else makes the press do nothing
        // when it is already on, which reads as a stuck control.
        onClick={() => onOpen(open ? "" : dial)}
        className="rounded-md"
        // Wider than the arc it holds, because the back has to fit a
        // sentence: the two faces share one box, and sizing it to the
        // circle would wrap 「内存 12.3G / 16.0G」 into four lines.
        style={{ width: RING_SIZE_CARD * 1.6, height: RING_SIZE_CARD }}
      >
        <span data-metric-faces className="block h-full w-full">
          <span data-metric-face="front" className="grid h-full w-full place-items-center">
            <Face
              fraction={unknown ? null : fraction}
              strain={strain}
              pips={pips}
              unknown={unknown}
              size={RING_SIZE_CARD}
              stroke={RING_STROKE_CARD}
            >
              {value === null || unknown ? "—" : value}
            </Face>
          </span>
          {/* **The attribute marks the sentence itself**, not the box
              around it: it is what `khor devices` prints word for word,
              and it is compared as text — so it has to name exactly the
              sentence and not a box that also holds the quantities. */}
          {/* **Every sentence stays in the document.** This used to be
              the dial's other face, turned to by pressing; the press
              opens the drawer now, and the sentences would have gone
              with the flip. They cannot: `smoke.mjs` counts the card's
              readings against what `khor devices` prints, word for
              word, so dropping them makes the card disagree with the
              terminal. Read aloud, not drawn. */}
          <span data-metric-face="back" className="sr-only">
            {lines.map((line, i) => (
              <span key={i} className="text-sm tabular-nums">
                {line}
              </span>
            ))}
            <span
              data-vitals-detail={name}
              data-strain={strain ?? undefined}
              className="text-aux tabular-nums text-muted-foreground data-[strain]:text-strain data-[strain=critical]:font-label"
            >
              {sentence}
            </span>
          </span>
        </span>
      </button>
      <span className="text-sm">{word}</span>
    </div>
  );
}

/**
 * Bytes as a person reads them: 1024-based, one decimal, no space.
 *
 * The CLI has its own copy of this (`crates/cli` `bytes`), deliberately:
 * formatting is painting, not judgment. The wire carries bytes, nothing
 * depends on the two producing identical characters, and a library
 * handing back strings would be deciding what a screen it cannot see has
 * room for.
 */
/** How a volume is **named on the card**: its mount point as the
    machine spells it (2026-09-03:「就写 /Volumes/disk0 就可以」), with
    one substitution — macOS mounts the volume everything a person
    touches lives on at `/System/Volumes/Data`, and nobody calls it
    that. Every other mount keeps every character, because the path is
    what a person picks a volume by and the last step alone (`disk0`)
    threw away where it is. */
function mountWord(mount: string): string {
  return mount === "/System/Volumes/Data" ? "/" : mount;
}

/** A volume's short name **in the sentence `khor devices` prints** —
    `/System/Volumes/Data` and `/` both draw as `/`, the rest keep their
    last step. The CLI keeps a copy of its own (`vol_word`), the `bytes`
    precedent below: painting is not judgment, and the wire carries the
    honest mount. */
function volWord(mount: string): string {
  if (mount === "/System/Volumes/Data" || mount === "/") return "/";
  const steps = mount.split("/").filter(Boolean);
  return steps[steps.length - 1] ?? mount;
}

function bytes(n: number): string {
  const units = ["B", "K", "M", "G", "T"];
  let v = n;
  let u = 0;
  while (v >= 1024 && u + 1 < units.length) {
    v /= 1024;
    u += 1;
  }
  return u === 0 ? `${n}${units[0]}` : `${v.toFixed(1)}${units[u]}`;
}
