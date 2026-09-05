import { useEffect, useRef, useState, useSyncExternalStore } from "react";

import type { DeviceRow, HooksState, Load, Strain } from "@/api";
import { fetchLoad } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { Face, Glyph, type Pip } from "@/components/Face";
import { RING_SIZE_CARD, RING_STROKE_CARD } from "@/components/Ring";
import { IconBack, IconChevronDown, IconChevronUp } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";
import { mount as mountDay } from "@/day/day-core.js";
import type { DayPanel, DayWords } from "@/day/day-core.js";
import "@/day/day.css";
import { useHiddenRows, useOverflows, useTight } from "@/hooks/use-tight";
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
  const { load, why, shown, asking, seen } = useLoad(row, sort);
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
              seen={seen}
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
  // **Nothing is open on arrival, and the slot below the dials shows
  // the day.** It used to open the leftmost dial, because the slot had
  // nothing else to hold and an empty box under four dials is a space
  // waiting to be discovered. It has something else to hold now: the
  // 24-hour figure moved in here (2026-09-04), so the resting state of
  // this block is 「four readings now, and the day behind them」 and a
  // ring is pressed to trade the day for what that ring is made of.
  //
  // Pressing the open one closes it — back to the day. The press is a
  // toggle, and a control whose press does nothing when it is already
  // on is a control that feels stuck (`Dial`'s `onOpen`, which has
  // always sent `""` for that).
  const [open, setOpen] = useState("");
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
      {/* **One slot, one height, two things it can show.** See
          `--slot-h`: the figure and the drawers are the same height, so
          the card does not move under a press. */}
      <div data-ring-slot>
      <Drawers
        row={row}
        top={top}
        open={open}
        heads={{
          cpu: { kind: "occupants", word: cli.vitals_name_cpu, count: cli.procs_sub, fraction: v.cpu_pct / 100 },
          gpu: {
            kind: "parts",
            word: cli.vitals_name_gpu,
            count: cli.gpu_sub_nomem,
            fraction: gpuMean !== null ? gpuMean / 100 : v.gpu ? v.gpu.util_pct / 100 : null,
          },
          mem: {
            kind: "occupants",
            word: cli.vitals_name_mem,
            count: cli.procs_sub,
            fraction: v.mem.total > 0 ? v.mem.used / v.mem.total : null,
            strain: row.vitals.mem_strain,
          },
          disk: {
            kind: "parts",
            word: cli.vitals_name_disk,
            count: cli.vols_sub,
            fraction: disk && disk.total > 0 ? disk.used / disk.total : null,
            strain: diskStrain,
          },
        }}
      />
        {/* **The day lives in the same slot as the drawers**
            (2026-09-04). It is not a fourth thing under the dials, it
            is what this slot shows when no ring is pressed — and the
            two share one height so pressing a ring does not make the
            card jump.

            **Hidden, never unmounted.** `smoke.mjs` counts the
            readings on this card against what `khor devices` prints,
            and a block that unmounted its figure or its rows would make
            the card disagree with the terminal — which reads as a
            broken backend rather than as a closed drawer. */}
        <div data-load-day hidden={open !== ""}>
          {day && <DayChart day={day} />}
        </div>
      </div>
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
      which is a heading lying about the thing directly under it.

      `kind` is **what the rows under it are**, and it is the one place
      that fact is written down. A card and a volume are *parts* of the
      ring above — their readings add up to it. A process is only an
      *occupant*, and the few listed here do not add up to anything;
      there are several thousand more. Six of either draws identically,
      so the difference has to be said, and it is said **in the count's
      own grammar**: `parts` wear a bare count (「3 卷」), `occupants`
      wear a qualifier in front of the number (「占用最多的 6 个进程」).

      That used to be a footnote under the drawer instead
      (`top-share-note`, 「占用最多的几个,加起来不等于上面那个环」).
      2026-09-04:「也不要,很蠢」. What went is the apology, not the
      fact — a line that explains the block above it after the fact
      costs a row and is read last, while the heading is read first and
      is where somebody is already looking for what these rows are.
      `check-face` measures the grammar, on both arms. */
  heads: Record<
    string,
    { kind: "parts" | "occupants"; word: string; fraction: number | null; strain?: Strain | null; count: (n: number) => string }
  >;
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
  // **An occupant's bar is drawn against the biggest occupant shown,
  // not against the machine.** Against the machine it is invisible and
  // therefore says nothing: turing's hungriest process is 41% of one
  // core on a box with sixty-four of them, so a share-of-machine bar
  // for the row at the top of the list draws a hairline and every row
  // below it draws nothing at all — a column that is blank for every
  // value it can hold is not a reading.
  //
  // What it does say is 「how these few compare to each other」, which
  // is the question the drawer is answering. **It is not a fraction of
  // the ring above**, and the foot says so in as many words — that
  // sentence is doing real work here rather than being a formality.
  const against = (xs: number[]) => Math.max(1, ...xs);
  // **What each drawer actually draws.** The heading counts these, not
  // what the node claims elsewhere — see `heads`.
  const drawn: Record<string, number> = {
    cpu: busiest("cpu").length,
    gpu: cards.length,
    mem: busiest("mem").length,
    disk: vols.length,
  };
  const box = useRef<HTMLDivElement>(null);
  const scroll = useRef<HTMLDivElement>(null);
  // **What is out of sight right now**, so the heading can say it —
  // and stop saying it the moment somebody scrolls to the bottom.
  const hidden = useHiddenRows(scroll, open);
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
        // **The slot decides the height now**, not this box: the day
        // and the drawers share one, so pressing a ring cannot move
        // anything below (`--slot-h`, and `[data-slot]` in app.css).
        // It used to carry `min-h-drawer-h max-h-drawer-h`, a height
        // written for a block that stood on its own.
        hidden={open === ""}
        // **The rule above the slot and the room under it belong to the
        // slot**, not to this box: they were here, and the day beside
        // it had neither, so the drawer's heading sat 13px below the
        // reading words — two things that have to be the same line
        // (`[data-ring-slot]`, and the assertion that caught it).
        className="flex h-full flex-col text-sm tabular-nums"
      >
        {heads[open] && (
          <div
            data-drawer-head
            data-drawer-kind={heads[open].kind}
            className="mb-in flex items-center gap-in text-aux text-muted-foreground"
          >
            <Glyph fraction={heads[open].fraction} strain={heads[open].strain} />
            <span>{heads[open].word}</span>
            <span aria-hidden="true">·</span>
            <span data-drawer-count={drawn[open]} className="tabular-nums">
              {heads[open].count(drawn[open])}
            </span>
            {/* **The number is on the element, not in the sentence.**
                It used to be read out — 「还有 3 个在下面,拖一下」 —
                and 2026-09-04 the answer to that was 「你不觉得这个画很
                蠢吗,不要这个。下滑查看更多」. Nobody acts on the
                three: seeing that there is more is what makes somebody
                scroll, and whether it is three or seven changes
                nothing they do next.

                **What did not change is when it appears.** It is still
                counted off the rows the bottom edge actually clips, so
                it is absent when everything fits and goes the moment
                somebody reaches the end — a hint that is always there
                is not a hint, it is part of the heading. `check-face`
                reads the count off `data-drawer-more`, which is why
                the words could go and the state could stay. */}
            {hidden > 0 && <span data-drawer-more={hidden}>· {cli.drawer_more}</span>}
          </div>
        )}
        {/* **The rows scroll, the heading does not.** A heading that
            scrolled away would take with it the one line saying which
            reading these rows belong to — and four rows fit here without
            scrolling at all, which is what makes 「just fits」 and 「just
            does not」 different on screen. */}
        <div
          ref={scroll}
          data-drawer-scroll
          className="min-h-0 flex-1 overflow-y-auto overscroll-contain"
          style={{ scrollSnapType: "y proximity" }}
        >
        <div data-parts="cpu" hidden={open !== "cpu"}>
          {busiest("cpu").map((r, _i, all) => (
            <Occupant
              key={r.pid}
              row={r}
              share={r.cpu_pct / against(all.map((x) => x.cpu_pct))}
              value={`${Math.round(r.cpu_pct)}%`}
            />
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
          {busiest("mem").map((r, _i, all) => (
            <Occupant
              key={r.pid}
              row={r}
              share={Number(r.mem) / against(all.map((x) => Number(x.mem)))}
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
      {/* **Nothing under the drawer.** A sentence used to sit here
          saying that the rows above do not add up to the ring
          (`top-share-note`); 2026-09-04:「也不要,很蠢」.

          The fact it was defending is real and is still said — it
          moved **into the heading's grammar**, where `kind` explains
          it: `occupants` count themselves with a qualifier in front of
          the number, `parts` with a bare count. That is better than
          the footnote was, and not only shorter: the footnote sat
          *after* the block it was about, so it could only correct a
          reading somebody had already taken, while the heading is
          above the rows and is read on the way in. */}
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
  return (
    <span data-part-bar className="flex w-full items-center">
      <span className="block h-1.5 w-full overflow-hidden rounded-full bg-foreground/10">
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

/** The wall clock for one instant, hours and minutes in the reader's
    own locale — the head's 更新于. (It printed the day chart's two ends
    as well until 2026-09-04; the window is drawn on that figure's axis
    and does not need writing out beside it.) */
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
type SortCol = "pid" | "name" | "user" | "cpu" | "mem" | "gpu" | "vram" | "time" | "status" | "cputime" | "io";

/**
 * **The table's columns, in the order they are drawn, with everything
 * that differs between them written once.**
 *
 * The head word, the track width and the alignment used to live in
 * three places — a list of tracks, a run of `<Column>`s, and a run of
 * cells — and 2026-09-05 the table went from seven columns to eleven.
 * Three lists of eleven kept in agreement by somebody remembering is
 * the arrangement that already failed once at seven: a machine with
 * per-process 显存 and no per-process GPU had six cells meeting five
 * tracks, and 「命令」 collapsed to 55px. **The gate that catches that
 * is still there** (`tracks` against heads) — this is what stops it
 * having to fire.
 *
 * The order is the one the user named (2026-09-05): the pid you type
 * next, then what the process *is* (状态 · 用户), then the two times,
 * then the quantities, and **「命令」 last** because it is the only
 * column whose content has no bound, so it is the one that can absorb
 * width without opening a canyon between two numbers.
 */
const COLS = {
  pid: { head: () => cli.top_pid, w: "var(--cw-pid)", fr: 1, align: "left" },
  status: { head: () => cli.top_status, w: "var(--cw-status)", fr: 1, align: "left" },
  user: { head: () => cli.top_user, w: "var(--cw-user)", fr: 1, align: "left" },
  time: { head: () => cli.top_time, w: "var(--cw-run)", fr: 1, align: "right" },
  cputime: { head: () => cli.top_cputime, w: "var(--cw-cputime)", fr: 1, align: "right" },
  cpu: { head: () => cli.vitals_name_cpu, w: "var(--cw-cpu)", fr: 1, align: "right" },
  gpu: { head: () => cli.vitals_name_gpu, w: "var(--cw-gpu)", fr: 1, align: "right" },
  mem: { head: () => cli.vitals_name_mem, w: "var(--cw-mem)", fr: 1, align: "right" },
  vram: { head: () => cli.vitals_name_vram, w: "var(--cw-vram)", fr: 1, align: "right" },
  io: { head: () => cli.top_io, w: "var(--cw-io)", fr: 1, align: "right" },
  // **The one nudge away from even**, and it is the column the person
  // asking for even shares also called out as the one whose content
  // 「有长有短」. Half again, not four times: at four the table is the
  // old shape wearing a new formula.
  name: { head: () => cli.top_cmd, w: "var(--cw-cmd)", fr: 1.5, align: "left" },
} as const satisfies Record<SortCol, { head: () => string; w: string; fr: number; align: "left" | "right" }>;
const COL_ORDER = ["pid", "status", "user", "time", "cputime", "cpu", "gpu", "mem", "vram", "io", "name"] as const;


/**
 * **Which optional columns this machine has ever filled**, one latch
 * each.
 *
 * Four of the seven are 「an older khor over there does not send this
 * field at all」 and three are 「this machine has nothing to put in
 * it」, and they behave the same way on screen, so they are one
 * mechanism.
 *
 * **The question is 「can any row fill me」, not 「does this machine
 * have the hardware」** — turing reports two RTX A5000s and not one of
 * its two hundred processes carries a per-process figure, so asking
 * about the cards drew two empty columns and spent 136px on them.
 *
 * **And it latches, only ever upward.** Read off the rows in front of
 * you it is a fact about *this page*: order turing by GPU and the two
 * hundred rows that come back can be all-`null`, so pressing 「GPU」
 * deletes the GPU column and the press has no head left to press again
 * (user, 2026-09-04:「一点整个表就变了」). Cleared only when the machine
 * changes.
 */
type Seen = { status: boolean; user: boolean; time: boolean; cputime: boolean; gpu: boolean; vram: boolean; io: boolean };
/** What counts as a row filling each of them.
 *
 *  **`user` is the odd one**: it is a `String` on the wire and its
 *  absence is spelled `""`, because a machine that will not name an
 *  account has no id to send either. So the emptiness has to be read,
 *  not the null. */
const FILLS: { [K in keyof Seen]: (r: Load["top"][number]) => boolean } = {
  status: (r) => r.status !== null,
  user: (r) => r.user !== "",
  time: (r) => r.run_secs !== null,
  cputime: (r) => r.cpu_ms !== null,
  gpu: (r) => r.gpu_pct !== null,
  vram: (r) => r.gpu_mem !== null,
  io: (r) => r.io_bytes !== null,
};
const SEEN_KEYS = Object.keys(FILLS) as (keyof Seen)[];
const SEEN_NONE: Seen = { status: false, user: false, time: false, cputime: false, gpu: false, vram: false, io: false };

/** The word for a status token, or the token itself.
 *
 *  **A token with no word is printed as it stands** — `unknown-13` on
 *  the glass is something a person can look up, and it is the honest
 *  answer for a status a newer node knows about and this build does
 *  not. What it must never do is fall through to 「运行」: the node
 *  goes to some trouble not to say that word wrongly
 *  (`khor_node::top::status_token`), and a lookup with a default would
 *  undo all of it here. */
const STATUS_WORD: Record<string, string> = {
  run: cli.top_st_run,
  sleep: cli.top_st_sleep,
  idle: cli.top_st_idle,
  spawning: cli.top_st_spawning,
  diskwait: cli.top_st_diskwait,
  dead: cli.top_st_dead,
  zombie: cli.top_st_zombie,
  stopped: cli.top_st_stopped,
  tracing: cli.top_st_tracing,
  parked: cli.top_st_parked,
  wakekill: cli.top_st_wakekill,
  waking: cli.top_st_waking,
  suspended: cli.top_st_suspended,
  lockblocked: cli.top_st_lockblocked,
  unclear: cli.top_st_unclear,
};

/**
 * Accumulated CPU as a **stopwatch**, where 时长 next to it is a
 * duration in words.
 *
 * The two columns are both times and they are not the same time — one
 * is the wall clock, the other is core-seconds, and a process on four
 * cores has burned four times the first. Printed in the same shape they
 * read as a contradiction (「起了 2 小时,烧了 8 小时」 looks like a bug);
 * printed as 「2小时」 and 「8:03:41」 they read as two kinds of thing,
 * which is what they are.
 */
const cpuClock = (ms: number) => {
  const s = Math.round(ms / 1000);
  const two = (n: number) => String(n).padStart(2, "0");
  const h = Math.floor(s / 3600);
  const m = two(Math.floor((s % 3600) / 60));
  // **Past a hundred hours the seconds go.** hinton's `go.d.plugin`
  // came back at `1374:35:10` — ten characters, wider than the column,
  // and it spilled into the gap beside it (seen on a real card). The
  // seconds on a figure that size are not a reading anybody uses; the
  // magnitude is. Under a hundred hours they stay, because there they
  // are the difference between a process that just started burning and
  // one that has been at it a while.
  if (h >= 100) return `${h}:${m}`;
  return h > 0 ? `${h}:${m}:${two(s % 60)}` : `${Math.floor(s / 60)}:${two(s % 60)}`;
};

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
  seen,
}: {
  load: Load | null;
  why: string | null;
  /** Which optional columns any row on this machine has ever filled.
      **Latched 「seen」 facts, not properties of the hardware or of the
      far khor's version**, and together they decide which columns the
      table has — see `Seen`, `useLoad` and `Top`. */
  seen: Seen;
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
        <Top rows={load?.top ?? []} sort={sort} asking={asking} onSort={onSort} why={why} seen={seen} />
      </Panel>
    );
  }
  if (!load) return null;
  //   **The day is not a panel of its own any more** (2026-09-04): it
  //   moved into the slot under the four rings, which is where it is
  //   read against them. See `Readings`.
  return (
    <Panel>
      {/* **The table first, the figure under it** (2026-09-05:「列表和
          散点图换个位置」). The reader's question when a card opens is
          「这台机器在忙什么」, and the answer to that is rows; the
          scatter answers a second question — 「这两个量之间有什么关系」
          — which is only asked once the first is answered.

          It also retires a bug at the root rather than by treatment:
          the scatter's axis words used to sit directly above the
          table's column heads, and 「横 CPU 内存 显存 时长」 read as the
          columns below it (2026-09-04:「上面的散点图的 X 和 y 设置是
          下面表的表头」). Two rows of small words cannot be mistaken
          for each other when one of them is a screen away. */}
      <Top rows={load.top} sort={sort} asking={asking} onSort={onSort} seen={seen} />
      {/* **A rule between the table and the figure.** They are one
          block — the same two hundred processes, listed and then drawn
          — so a hairline and `--gap-group` on both sides rather than
          two cards; splitting them would say they are two subjects.

          **`--gap-group` is the token whose whole definition is this
          case** (「between groups that must not read as one list」);
          `--gap-row` was tried first and measured 25px against an 18px
          row, which is a break narrower than twice the rhythm it is
          breaking. */}
      <div data-top-seam className="mt-group border-t pt-group">
        <Scatter rows={load.top} />
      </div>
    </Panel>
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
  seen,
}: {
  rows: Load["top"];
  /** The order these rows are actually in — see `useLoad`. */
  sort: Sort;
  /** The column whose answer has not landed yet. Its head says so, and
      **nothing else on the table moves until the rows do.** */
  asking: SortCol | null;
  onSort: (s: Sort) => void;
  /** Why the machine did not answer this ask. Set means: draw the heads
      and this sentence, and no rows — see `Busy`. */
  why?: string | null;
  /** **Which columns this table has, decided by the machine and not by
      the rows in front of you** — see `Seen`.

      It used to be two props (`cards` and `ages`) and before that
      `rows.some(r => r.gpu_pct !== null)` read inline here, which made
      the column set a function of the current answer.

      The old note, kept because it is what the shape costs:

      It used to be `rows.some(r => r.gpu_pct !== null)`, and that made
      the column set a function of the current answer: order turing by
      GPU and the two hundred rows that come back can be all-`null`, so
      **pressing 「GPU」 deleted the GPU column** — the press had no head
      left to press again (user, 2026-09-04:「一点整个表就变了，比如
      GPU，一点就没有GPU这列了」). A column that appears and disappears
      with the ordering is not a column, it is a reading of the page.

      So it comes from above and it latches — see `useLoad`. */
  seen: Seen;
}) {
  // **A column exists when a row on this machine has ever filled it.**
  // A whole column of blanks is worse than no column: it takes width
  // from the ones that speak, and it reads as 「khor looked and found
  // nothing」, which is not the same as 「nothing here runs on a card」.
  // Latched in `useLoad`, so an ordering cannot take a column away.
  //
  // **One list, and the heads, tracks and cells are all read off it.**
  // See `COLS` for why that is not tidiness.
  const shown = COL_ORDER.filter((k) => !(k in FILLS) || seen[k as keyof Seen]);
  const ordered = sortRows(rows, sort);
  const box = useRef<HTMLDivElement>(null);
  const grid = useRef<HTMLDivElement>(null);
  // **The table stacks when its tracks do not fit, and that is asked
  // of the browser** rather than of a constant — the column count runs
  // from seven to eleven, so the width where it stops fitting is not a
  // number anybody can write down once (`useOverflows` carries what
  // writing it down cost). It is asked of the table's own box and not
  // the window's: the card has a rail and a list beside it, and a
  // window breakpoint once left a 106px overflow band invisible to any
  // two-point check.
  const tight = useOverflows(box, grid, shown.join(","));
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
  const cells = shown.length;
  // **The tracks, in the order the cells are written below** — the same
  // list, so they cannot disagree. Only the order and the share are
  // here; every floor is a token.
  //
  // **`minmax(floor, share fr)`, not `floor` with the leftover dumped
  // on the last column** (2026-09-05:「我们要不按页面宽度均分列宽。或
  // 稍微微调一些,大致是均分」). The old arrangement gave every column
  // exactly its floor and handed the whole remainder to 「命令」, which
  // on a wide card left that one column holding hundreds of pixels of
  // blank while the numbers were crammed at the left.
  //
  // Each track keeps its floor — the head's own word plus the sort
  // chevron, and the widest reading that column has actually shown — and
  // the free space above the floors is shared. **The floors are what
  // stop 「even」 from becoming 「too narrow to read」**: a share smaller
  // than a column's floor simply does not apply, and a table narrower
  // than the floors added up stops being a grid at all (`useOverflows`).
  const tracks = shown.map((k) => `minmax(${COLS[k].w}, ${COLS[k].fr}fr)`).join(" ");
  return (
    <div
      ref={box}
      data-load-top-box
      data-tight={tight ? "" : undefined}
      className="max-h-top-h overflow-y-auto"
    >
      {/* `data-top-cols` is **which columns this table actually has, in
          order**, and `data-top-frs` the share each was given — so a
          gate can check the division without keeping its own copy of
          the weights. — one attribute rather than a flag per absent column.
          The three it replaces (`data-nogpu` / `data-novram` /
          `data-norun`) were left over from the four hand-written grid
          variants that a generated track list retired; nothing read
          them any more, and at eleven columns the flag-per-column shape
          would have been seven of them. */}
      <div
        ref={grid}
        data-load-top
        data-panel
        data-top-cols={shown.join(" ")}
        data-top-frs={shown.map((k) => `${k}:${COLS[k].fr}`).join(" ")}
        style={{ gridTemplateColumns: tracks }}
        className="text-sm tabular-nums"
      >
        <div data-top-heads className="bg-card text-aux text-muted-foreground">
          {/* **The order the user named** (2026-09-05:「PID · 状态 ·
              USER · 时长 · CPU时间 · CPU · GPU · 内存 · 显存 · 读写 ·
              命令」): the pid you type next, then what the process *is*,
              then the two times, then the quantities — the two GPU ones
              beside the CPU rather than after the memory that has
              nothing to do with them — and **「命令」 last**.

              **Last is what makes the table fill without a canyon.**
              「命令」 is the only column whose content has no bound, so
              it is the one the leftover width goes to; while it sat
              second, that width opened **between the name and the
              numbers** — measured 700+px on turing at 1500, which is
              the same visual event as the 799px hole this card was
              rebuilt to remove. At the end, the same slack lands inside
              it and runs to the table's own right edge, and no two
              columns are pushed apart at all.

              **One column sorts by a word that is not its own**:
              「命令」 carries a process's short name and its identifying
              word together and orders by the name, which is the text
              the column starts with. Its head is `cmd` in the DOM
              because the column is; the sort is `name` because that is
              what `khor_core::TopBy` knows. */}
          {shown.map((k) => (
            <Column
              key={k}
              col={k}
              head={k === "name" ? "cmd" : undefined}
              sort={sort}
              asking={asking}
              onPress={press}
              align={COLS[k].align}
            >
              {COLS[k].head()}
            </Column>
          ))}
        </div>
        {why ? (
          <div data-load-why className="py-row text-sm text-muted-foreground" style={{ gridColumn: `span ${cells}` }}>
            {why}
          </div>
        ) : (
          ordered.map((r) => (
            <div key={r.pid} data-top-row={r.pid}>
              {shown.map((k) =>
                k === "name" ? (
                  /* Last, and it takes the table's leftover width —
                     see the head. */
                  <Cut key={k} name={r.name} arg={r.arg} />
                ) : (
                  <span
                    key={k}
                    data-c={k}
                    {...{ [`data-top-${k}`]: "" }}
                    // **The one cell that can say more than it shows.**
                    // A status this build has no word for prints its
                    // token, and a token is longer than the column
                    // (`unknown-13` against 「休眠」) — the number in it
                    // is the whole reason it was kept, so it must be
                    // readable somewhere even when the cell clips.
                    title={k === "status" && r.status !== null ? r.status : undefined}
                    className={cn(
                      "whitespace-nowrap",
                      COLS[k].align === "right" ? "text-right" : "text-left",
                      // **The pid is muted, and it is the only cell
                      // that is.** It is an identifier rather than a
                      // reading — nobody compares the magnitudes of two
                      // pids — so it stands back from the numbers that
                      // are being compared. (It is also what the head's
                      // 「heavier than every cell」 gate measures
                      // against, which is why that gate had to stop
                      // sampling 「the first cell」 and start sampling
                      // the loudest one.)
                      k === "pid" && "text-muted-foreground",
                      // The one column whose content has no bound and
                      // no room: an account name runs off the end, so
                      // it is cut rather than allowed to push the
                      // numbers over.
                      (k === "user" || k === "status") && "overflow-hidden text-ellipsis",
                    )}
                  >
                    {cellOf(k, r)}
                  </span>
                ),
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

/**
 * What one cell says, for every column but 「命令」 (which is a run of
 * text with its own truncation — see `Cut`).
 *
 * **An empty string is 「读不到」 and it is deliberate.** Every one of
 * these that can be absent prints nothing rather than a zero: a `0%`
 * where the machine never looked is the neighbouring-value mistake, and
 * on this table it would be sorted, compared and believed.
 */
function cellOf(k: Exclude<SortCol, "name">, r: Load["top"][number]): string {
  switch (k) {
    case "pid":
      return String(r.pid);
    // A token this build has no word for prints as the token — never
    // as a neighbour's word. See `STATUS_WORD`.
    case "status":
      return r.status === null ? "" : (STATUS_WORD[r.status] ?? r.status);
    case "user":
      return r.user;
    case "time":
      return r.run_secs === null ? "" : lasting(r.run_secs * 1000);
    case "cputime":
      return r.cpu_ms === null ? "" : cpuClock(r.cpu_ms);
    case "cpu":
      return `${Math.round(r.cpu_pct)}%`;
    case "gpu":
      return r.gpu_pct === null ? "" : `${Math.round(r.gpu_pct)}%`;
    case "mem":
      return bytes(r.mem);
    case "vram":
      return r.gpu_mem === null ? "" : bytes(r.gpu_mem);
    // Absent on most rows of a shared box, because khor may not read
    // another account's — and absent is not zero
    // (`khor_node::top::io_readable`).
    case "io":
      return r.io_bytes === null ? "" : bytes(r.io_bytes);
  }
}

/**
 * The four quantities either axis of the scatter can be, and how each
 * one is read off a row and written on a tick.
 *
 * **The same four the table has columns for** (2026-09-04:「表要可以
 * 设定 x 和 y 轴」), because those are the quantities a person is
 * already looking at when they decide they want to see two of them
 * against each other.
 *
 * `of` returns `null` for 「did not read」, and a row that answers
 * `null` on either axis **is not drawn** — see `Scatter`.
 */
const AXES = {
  //   **The tick keeps its significant figures.** `Math.round` was
  //   enough while every axis was a square root — its ticks are even
  //   fractions of the maximum, so the small ones are still whole
  //   percents. On a log axis the ticks are even fractions of the
  //   *axis*, and the first few land at 0.03%, 0.2%, 1.3% — measured on
  //   this Mac, where they came out as four ticks all reading 「0%」.
  cpu: {
    word: () => cli.vitals_name_cpu,
    of: (r: Load["top"][number]) => r.cpu_pct,
    tick: (v: number) => `${v >= 10 || v === 0 ? Math.round(v) : v < 1 ? v.toFixed(2) : v.toFixed(1)}%`,
  },
  mem: { word: () => cli.vitals_name_mem, of: (r: Load["top"][number]) => Number(r.mem), tick: (v: number) => bytes(v) },
  vram: { word: () => cli.vitals_name_vram, of: (r: Load["top"][number]) => r.gpu_mem, tick: (v: number) => bytes(v) },
  time: { word: () => cli.top_time, of: (r: Load["top"][number]) => r.run_secs, tick: (v: number) => lasting(v * 1000) },
} as const;
type AxisKey = keyof typeof AXES;
const AXIS_KEYS = Object.keys(AXES) as AxisKey[];

/**
 * **How one axis turns a reading into a position, and which ruler it
 * had to use to do it.**
 *
 * 2026-09-04:「因 X 或 y scale 不一无法绘制出有意义的图的时候,用 log 或
 * 什么方法非线性的绘制呀」.
 *
 * # The measurement that decides, so it is not switched unconditionally
 *
 * `max / median`. Under a square root a ratio `R` puts the median at
 * `1/√R` of the axis, so **`R > 44` is exactly 「half the points are in
 * the first 15%」** — the shape of a picture that answers nothing. It is
 * measured on the readings actually being drawn, not assumed from the
 * quantity: on the live fleet CPU comes out 400–1600 on all three
 * machines, while 内存 is 16 / 34 / 52 and 时长 is 5.5 / 803 / 10.3.
 * **Two of the four are degenerate on one machine and not on another**,
 * which is why the ruler is printed on the axis rather than assumed.
 *
 * A median of 0 is `R = ∞` and takes the log branch: half the readings
 * at zero is the worst case of the same problem, and √ cannot separate
 * the other half either.
 *
 * # Why `log1p` of a ratio, and not `log`
 *
 * **`log 0` is undefined and 0 is the commonest value in this table**
 * (three machines' CPU medians are 0.02 / 1.6 / 0.12 against maxima in
 * the tens and thousands, and plenty of rows read exactly 0). `log1p`
 * is defined there and **`log1p(0) = 0`, so zero is drawn at zero** —
 * no `v + 1` fudge, no ruler whose labels lie about where they sit.
 *
 * The argument is `v / s`, not `v`: `log1p` only compresses above 1, so
 * on a quantity whose values are all below 1 (CPU percent) it is
 * indistinguishable from linear. `s` is the **smallest positive reading
 * on this axis** — the data's own resolution — floored at `max / 1e6`
 * so a single tiny outlier cannot stretch the ruler. Measured with it,
 * the three machines' CPU medians land at 0.219 / 0.221 / 0.176 of the
 * axis instead of 0.025 / 0.027 / 0.05.
 *
 * # Ticks
 *
 * **On the square root the ticks are even fractions of the value** and
 * the transform decides where they sit, so they crowd towards the top —
 * and that crowding is the evidence that the ruler is not linear.
 *
 * **On the log they are even fractions of the axis** and the *labels*
 * come out uneven (0, 0.03%, 0.2%, 1.3%, 8%, 47%). The same evidence,
 * on the other channel, and it has to be that way round here: even
 * value fractions on a log axis put every tick but the first in the top
 * fifth, leaving the stretch where all the points are unlabelled.
 * Either way **a tick's label is a real reading**, never a transformed
 * number.
 */
function ruler(vals: number[]): { kind: "sqrt" | "log"; at: (v: number) => number; tick: (f: number) => number } {
  const max = Math.max(1e-9, ...vals);
  const sorted = [...vals].sort((a, b) => a - b);
  const med = sorted.length ? sorted[Math.floor(sorted.length / 2)] : 0;
  const wide = !(med > 0) || max / med > 44;
  if (!wide) {
    return { kind: "sqrt", at: (v) => Math.sqrt(Math.max(0, v) / max), tick: (f) => max * f };
  }
  const pos = vals.filter((v) => v > 0);
  const s = Math.max(pos.length ? Math.min(...pos) : max / 1e6, max / 1e6);
  const span = Math.log1p(max / s);
  return {
    kind: "log",
    at: (v) => Math.log1p(Math.max(0, v) / s) / span,
    // The value whose position is `f` — so the label is a real reading
    // and the tick is where that reading belongs.
    tick: (f) => (Math.exp(f * span) - 1) * s,
  };
}

/**
 * Which pair of axes the scatter is drawn against — **one pair for
 * every machine**, which is why it is not `useState` inside the card.
 *
 * A card unmounts when somebody opens a different machine, so state
 * kept in it would reset to the default on every hop, and three cards
 * read side by side would be three different pictures. **A silent
 * change of axis is the widest door to a misreading there is** and this
 * component's own history is the proof: the pair before this one fell
 * back to memory on a machine with no card, and nothing said so.
 *
 * Deliberately not persisted to disk. It is a way of looking, not a
 * setting, and it costs one press to get back.
 */
const axesStore = {
  pair: { x: "cpu" as AxisKey, y: "mem" as AxisKey },
  subs: new Set<() => void>(),
  get() {
    return this.pair;
  },
  set(next: { x: AxisKey; y: AxisKey }) {
    if (next.x === this.pair.x && next.y === this.pair.y) return;
    this.pair = next;
    this.subs.forEach((f) => f());
  },
  subscribe(f: () => void) {
    this.subs.add(f);
    return () => {
      this.subs.delete(f);
    };
  },
};
function useAxes() {
  return useSyncExternalStore(
    (f) => axesStore.subscribe(f),
    () => axesStore.get(),
  );
}

/**
 * Every process as a point, on **two axes the reader picks**, with card
 * memory as the halo around it.
 *
 * # Why the pair defaults to CPU and memory
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
 * CPU and memory exist for every process on every machine, so as a
 * default the three cards can be read side by side and every row is on
 * the picture. **The reader may now choose otherwise, and that is not
 * the same thing**: a chosen axis is on screen in the foot, and the
 * count of rows it left out is printed beside it. What was forbidden is
 * the app changing an axis on its own.
 *
 * # A row with nothing to say on an axis is not drawn at zero
 *
 * 显存 and 时长 are not readings every process has. Measured on the
 * live fleet: **200 of 200 rows on Mac-mini and on hinton carry no card
 * memory at all, and 3 of 200 do on turing.** Drawing those at 0 would
 * put 「没看到」 exactly where 「最小的那个」 goes, which is the
 * neighbouring-value mistake this whole card is shaped to avoid. So
 * they are left out — **and the number left out is printed**, because a
 * picture missing two thirds of its points and a machine that is idle
 * look identical.
 *
 * # One ruler for all four
 *
 * Square root, on whichever quantity is chosen. **Not because √ centres
 * these distributions — it does not** — but because the alternative is
 * a ruler that changes with the axis, which is the silent change again.
 * Measured 2026-09-04, where the median lands along a √ axis:
 *
 *     axis     Mac-mini   hinton   turing
 *     CPU         0.025    0.027    0.05
 *     内存        0.171    0.249    0.139
 *     时长        0.426    0.035    0.311
 *
 * 时长 is the one worth checking, since it spans five orders of
 * magnitude — and it comes out **no worse than CPU**, the axis this
 * picture already shipped with. (Linear, for comparison, puts CPU's
 * median at 0.0006.)
 *
 * `√` and not `log` is structural rather than a matter of taste:
 * **`log 0` is undefined and 「0」 is the commonest value here.** A log
 * axis has to be `log(v+1)`, and then the ruler is no longer the one
 * its own labels claim. `√0 = 0`, so zero is drawn at zero.
 *
 * # Card memory is the halo, and area is the encoding
 *
 * Radius by square root, so **area** is proportional to bytes — the
 * codex's rule for encoding a quantity as a size. The foot prints the
 * two largest readings and the pixel radii they produced, so anybody
 * can check on the spot that the two are told apart rather than taking
 * it on faith.
 *
 * **The halo stays 显存 even when 显存 is one of the axes.** It then
 * says one thing twice, which is redundant and harmless. The two
 * alternatives are not: swapping the halo to some other quantity, or
 * dropping it, would both be **the app changing an encoding without
 * being asked** — the exact failure this component's first version was
 * rewritten for.
 *
 * **Three states, three appearances, never merged**: memory on the card
 * draws a halo; exactly zero draws a dashed ring (「looked, it is
 * zero」); nothing to read draws nothing at all (「did not look」). The
 * middle one is why they cannot be merged — a card that is idle and a
 * machine that cannot see cards are different facts.
 */
function Scatter({ rows }: { rows: Load["top"] }) {
  const box = useRef<HTMLDivElement>(null);
  const axes = useAxes();
  // **The picture is drawn in real pixels, not in a stretched box.**
  // It used to carry `preserveAspectRatio="none"`, which stretches the
  // viewBox to the element — and a circle under a non-uniform stretch
  // is an ellipse. The halo is an *area*, and the foot promises a
  // radius in pixels; both were false by however much the box was
  // wider than it was tall. So the viewBox tracks the element's own
  // size and one unit is one pixel.
  const [size, setSize] = useState({ w: 900, h: 170 });
  useEffect(() => {
    const node = box.current;
    if (!node) return;
    const ro = new ResizeObserver(([e]) =>
      setSize({ w: Math.max(1, e.contentRect.width), h: Math.max(1, e.contentRect.height) }),
    );
    ro.observe(node);
    return () => ro.disconnect();
  }, []);
  const { w: W, h: H } = size;
  const padL = 46;
  const padB = 22;
  const padT = 12;
  const padR = 14;
  const ax = AXES[axes.x];
  const ay = AXES[axes.y];
  // **The rows this picture can actually place.** Both readings, or the
  // row is not on it — see the head.
  const placed = rows
    .map((r) => ({ r, x: ax.of(r), y: ay.of(r) }))
    .filter((p): p is { r: Load["top"][number]; x: number; y: number } => p.x !== null && p.y !== null);
  const off = rows.length - placed.length;
  const rx = ruler(placed.map((p) => p.x));
  const ry = ruler(placed.map((p) => p.y));
  const vrams = rows.map((r) => r.gpu_mem).filter((v): v is number => v !== null && v > 0);
  const maxVram = vrams.length > 0 ? Math.max(...vrams) : 0;
  const RMAX = 11;
  const halo = (v: number) => (maxVram > 0 ? RMAX * Math.sqrt(v / maxVram) : 0);
  const sx = (v: number) => padL + rx.at(v) * (W - padL - padR);
  const sy = (v: number) => H - padB - ry.at(v) * (H - padT - padB);
  // **Ticks at round fractions of the real value**, placed where the
  // root puts them — so they crowd together towards the right, and
  // that crowding is itself the evidence that the ruler is not linear.
  // An axis that looks evenly spaced while a transform hides underneath
  // is the one that misleads.
  const steps = [0, 1, 2, 3, 4, 5, 6].map((k) => k / 6);
  const two = [...vrams].sort((a, b) => b - a).slice(0, 2);
  return (
    <div
      data-top-plot
      data-plot-axes={`${axes.x}x${axes.y}`}
      data-plot-scale-x={rx.kind}
      data-plot-scale-y={ry.kind}
      className="flex flex-col gap-in"
    >
      {/* **The vertical axis says what it is, at the top of itself.**
          Written across, not turned ninety degrees: rotated Latin is
          read by tilting the head, and rotated 中文 is a different
          writing mode with its own rules — neither is worth making a
          reader do for two characters. */}
      <AxisPick which="y" chosen={axes.y} onPick={(k) => axesStore.set({ ...axes, y: k })} align="left" />
      <div ref={box} className="h-44 w-full">
        {/* **Named**, because it is no longer the only `<svg>` in this
            block: the axis labels carry chevrons, and a probe reaching
            for 「the figure」 with `querySelector("svg")` picked a 12px
            chevron and reported the drawing as 12 pixels tall
            (2026-09-05, measured — the assertion was about the picker
            opening in place, and it was reading an icon). */}
        <svg data-plot-svg viewBox={`0 0 ${W} ${H}`} width={W} height={H} className="text-foreground">
          <line x1={padL} y1={H - padB} x2={W - padR} y2={H - padB} stroke="currentColor" strokeOpacity={0.15} />
          <line x1={padL} y1={padT} x2={padL} y2={H - padB} stroke="currentColor" strokeOpacity={0.15} />
          {/* **Which ruler each axis is on, written on the axis.**
              Whether a reading needs the log one is decided from that
              machine's own numbers (see `ruler`), so the same pair can
              be two different rulers on two machines — and then 「the
              same position」 means two different things. This is the
              only honest way out of that: say it, on the axis, where
              somebody reading a position is already looking.

              **Both of them, not only the one that changed.** Marking
              only the exception asks the reader to know that no mark
              means a square root, which is a rule nobody taught them. */}
          {/* Inside the plot and just off each axis's far end, because
              outside it the tick labels are already there: the x mark
              landed on the last percentage and the y mark on the top
              byte figure (measured, and visible in a screenshot as
              「5|3%」 and a ruler word through 「606.8M」). */}
          <text
            data-plot-ruler="x"
            x={W - padR}
            y={H - padB - 5}
            fontSize={9}
            textAnchor="end"
            fill="currentColor"
            fillOpacity={0.4}
          >
            {rx.kind === "log" ? cli.top_plot_log : cli.top_plot_sqrt}
          </text>
          <text
            data-plot-ruler="y"
            x={padL + 4}
            y={padT + 7}
            fontSize={9}
            textAnchor="start"
            fill="currentColor"
            fillOpacity={0.4}
          >
            {ry.kind === "log" ? cli.top_plot_log : cli.top_plot_sqrt}
          </text>
          {steps.map((f) => {
            const v = rx.tick(f);
            return (
              <g key={`x${f}`} data-plot-tick-x={Math.round(v * 10) / 10}>
                <line x1={sx(v)} y1={H - padB} x2={sx(v)} y2={H - padB + 3} stroke="currentColor" strokeOpacity={0.3} />
                <text x={sx(v)} y={H - padB + 14} fontSize={10} textAnchor="middle" fill="currentColor" fillOpacity={0.5}>
                  {ax.tick(v)}
                </text>
              </g>
            );
          })}
          {steps.map((f) => {
            const v = ry.tick(f);
            return (
              <g key={`y${f}`} data-plot-tick-y={Math.round(v)}>
                <line x1={padL - 3} y1={sy(v)} x2={padL} y2={sy(v)} stroke="currentColor" strokeOpacity={0.3} />
                <text x={padL - 6} y={sy(v) + 3} fontSize={10} textAnchor="end" fill="currentColor" fillOpacity={0.5}>
                  {ay.tick(v)}
                </text>
              </g>
            );
          })}
          {placed.map(({ r, x, y }) => {
            const cx = sx(x);
            const cy = sy(y);
            return (
              <g key={r.pid} data-plot-point={r.pid} data-plot-x={Math.round(cx * 10) / 10} data-plot-y={Math.round(cy * 10) / 10}>
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
                  />
                )}
                <circle cx={cx} cy={cy} r={1.8} fill="currentColor" fillOpacity={0.75}>
                  <title>{`${r.name} ${ax.tick(x)} ${ay.tick(y)}`}</title>
                </circle>
              </g>
            );
          })}
        </svg>
      </div>
      {/* And the horizontal one under itself, centred on the axis it
          names. */}
      <AxisPick which="x" chosen={axes.x} onPick={(k) => axesStore.set({ ...axes, x: k })} align="center" />
      <div className="flex flex-wrap items-baseline justify-between gap-row text-aux text-muted-foreground">
        <span className="flex flex-wrap items-baseline gap-in">
          {/* **Not optional.** See the head: a picture that quietly
              dropped two thirds of its rows draws the same as an idle
              machine. */}
          {off > 0 && <span data-plot-off={off}>{cli.top_plot_off(off)}</span>}
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

/** **One axis's label, which is also the control that changes it**
 * (2026-09-05:「散点图搞成带X和Y label的。通过点击label,去切换X或y的值」).
 *
 * # Why it is a label and not a row of four words
 *
 * It used to be four words per axis, all eight sitting in the figure's
 * foot under a 「横」/「纵」. Two things were wrong with that and only one
 * of them was the position: **an axis with four quantities beside it
 * does not say what it is.** A reader glancing at the vertical ruler
 * saw four candidates and had to work out which one the picture was
 * drawn with; now the axis carries one word, and it is the right one.
 *
 * # Why it can be pressed, without a ground or a border
 *
 * The chevron. **Not a new mark** — it is the same one the column heads
 * next door wear to say 「this word is the control」, so the card is not
 * being taught a second vocabulary for the same idea. A capsule or a
 * bordered box would say 「button」 louder and would also be the first
 * such box on this card, which is the wrong thing to be first at.
 *
 * # Why the height cannot move
 *
 * Collapsed it is one button, open it is four, and **both are a row of
 * `min-h-touch` buttons** — so opening the picker cannot push the
 * figure down or squeeze it. That is the same rule the 24-hour block's
 * three reading words and the drawer's ⊙ title follow: one place, the
 * contents swapped, the height fixed.
 *
 * The floor is on the hit area rather than on the word: each of these
 * is 44 tall and 44 wide to a finger while the ink stays the
 * footnote's size (`day.css`'s `.sw button`, the same arrangement) —
 * which is also why the gap between the four is zero. At the floor's
 * width they are already far enough apart.
 */
function AxisPick({
  which,
  chosen,
  onPick,
  align,
}: {
  which: "x" | "y";
  chosen: AxisKey;
  onPick: (k: AxisKey) => void;
  /** Where the label sits under the axis it names: the horizontal one
      is centred on its axis, the vertical one starts at the top of
      its own. */
  align: "left" | "center";
}) {
  const [open, setOpen] = useState(false);
  return (
    <span
      data-plot-pick={which}
      data-open={open ? "" : undefined}
      className={cn(
        "flex items-baseline gap-0 text-aux",
        align === "center" ? "justify-center" : "justify-start",
      )}
    >
      {open
        ? AXIS_KEYS.map((k) => (
            <button
              key={k}
              type="button"
              data-plot-axis={k}
              data-on={k === chosen ? "true" : "false"}
              aria-pressed={k === chosen}
              onClick={() => {
                onPick(k);
                setOpen(false);
              }}
              className={cn(
                "inline-flex min-h-touch min-w-touch items-center justify-center px-in hover:text-foreground",
                k === chosen ? "font-title text-foreground" : "text-muted-foreground",
              )}
            >
              {AXES[k].word()}
            </button>
          ))
        : (
            <button
              type="button"
              data-plot-label={which}
              aria-expanded={false}
              onClick={() => setOpen(true)}
              className="inline-flex min-h-touch min-w-touch items-center justify-center gap-in px-in font-title text-foreground hover:text-foreground"
            >
              {AXES[chosen].word()}
              <IconChevronDown className="size-3" />
            </button>
          )}
    </span>
  );
}

/**
 * The 「命令」 cell: **a process's short name and the one word that/**
 * The 「命令」 cell: **a process's short name and the one word that
 * identifies it, as one run of text**, with a mark when it did not fit.
 *
 * # One width, not two
 *
 * 2026-09-04:「这俩就是俩字符串,放一列是有原因的,干嘛还要单独显示宽度呢?
 * 他俩加一块共享列宽」.
 *
 * The version this replaces gave each half its own width — a floor on
 * the word, a ceiling on the name, and shrink factors deciding which
 * gave way first. **What that produced is what he saw: a name squeezed
 * out entirely and replaced by a `▪`.** A cell that cannot show what
 * the process *is* has not truncated anything, it has deleted the
 * answer; the identifying word beside it is then a word identifying
 * nothing.
 *
 * So there is one text flow and one truncation, at the end, where a
 * truncation belongs. **Neither half has a `min-width` any more**, and
 * the name comes first, so the first characters on screen are always
 * the ones that say which process this is.
 *
 * The two inks stay (name in the body ink, word in the muted one).
 * **That is semantics, not layout** — it says 「this part is the name
 * and that part qualifies it」 and it costs no width at all.
 *
 * # What may not be squeezed out
 *
 * **The pid is a track of its own and is never dropped**: on turing the
 * two vLLM workers carry the same name *and* the same identifying word
 * — `argv[0]` is `VLLM::EngineCore` for both — so the only things that
 * tell those two rows apart are the pid and the card memory.
 *
 * **When the word repeats the name it is not drawn twice.** That is the
 * vLLM case above, and printing `VLLM::EngineCore VLLM::EngineCore`
 * would spend a column's width saying one thing.
 *
 * # ⚠️ Tombstone: the bordered square is gone, and the argument for it
 * was wrong in a way the argument could not see
 *
 * There used to be a 10px bordered square at the end of a cut cell, and
 * a section here headed 「Why a mark and not an ellipsis」 arguing for
 * it: an ellipsis is punctuation *inside the text* saying 「the value
 * ends about here」, while what has to be said is 「there is more of
 * this, and you can have it」 — a statement about the cell. That
 * reasoning is sound and it is beside the point.
 *
 * 2026-09-05, from the person using this, looking at a real screen:
 * **「关键是它看着就是字体缺字呀。用户看着很难受」**. An empty box
 * immediately after text is what a missing glyph looks like — tofu — and
 * a screen that looks like a font failed to load says 「this software is
 * broken」, not 「there is more here」. **The mark was not overruled by a
 * preference; it was overruled by what it actually looked like.** It
 * meant to say one thing and delivered another, which no amount of
 * correctness about cells and strings can fix.
 *
 * The replacement needed inventing nothing: the line already wears
 * Tailwind's `truncate` (`overflow:hidden; text-overflow:ellipsis`), so
 * the ellipsis was **already being drawn** — measured on a real card,
 * the cell read `…Stable-a44adf7f…` **and then the square**. Deleting
 * the square was the whole change.
 *
 * # Why a press, and a hover as well
 *
 * **The press is the one that must not be lost.** The narrow card has
 * no hover at all, and narrow is exactly where the cell runs out of room
 * first — an answer that lives only in a tooltip is missing where it is
 * needed most. So a press opens the line in place, and the keyboard
 * reaches it the same way (`role`, `tabindex`, Enter and Space).
 *
 * The hover is **added** on top of that (2026-09-05:「命令这种显示不全
 * 的,鼠标悬浮后显示」), for the pointer case where opening the row in
 * place is more disruption than the question deserves. Radix's tooltip
 * because it is the app's existing one: it opens on **focus** as well as
 * hover, and it portals out of the table's scroll box, which an
 * absolutely-positioned panel inside `overflow: auto` cannot do without
 * being clipped.
 *
 * **Only on cells that are actually cut**, so a tooltip never repeats a
 * line the reader can already see in full.
 *
 * # The mark is measured, never guessed
 *
 * `scrollWidth` against `clientWidth` on the one flowing line, watched
 * with a `ResizeObserver`. A mark decided by counting characters would
 * appear on cells that fit (decoration) and miss cells that do not (a
 * silent truncation), and both errors look fine on the machine of
 * whoever wrote the guess.
 */
function Cut({ name, arg }: { name: string; arg: string | null }) {
  const line = useRef<HTMLSpanElement>(null);
  const [over, setOver] = useState(false);
  const [open, setOpen] = useState(false);
  // **Not drawn twice** — see the head of this component.
  const word = arg && arg !== name ? arg : null;
  useEffect(() => {
    const node = line.current;
    if (!node) return;
    // Only while closed: open, the text wraps and nothing overflows, so
    // measuring then would retract the mark and take the way back with
    // it.
    if (open) return;
    // ⚠️ **A box with no width is not a reading.** When the mark
    // appears the cell gains a tooltip trigger around it, which
    // unmounts this line and mounts a new one — and the observer, still
    // holding the detached node, fires 0×0. Read as a measurement that
    // is 「it fits」, and the mark retracts on a cell that visibly does
    // not fit. Absent is not zero here either.
    const measure = () => {
      if (node.clientWidth === 0) return;
      setOver(node.scrollWidth - node.clientWidth > 1);
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(node);
    return () => ro.disconnect();
    // `over` is a dependency **because the answer changes the tree**:
    // the trigger wraps this line only when it is cut, so the node this
    // effect measured is not the node on screen a render later. Without
    // it the observer is left on the old one for good.
  }, [name, word, open, over]);
  const cut = over;
  const cell = (
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
      // **Opening wraps inside this column; it does not span the
      //     table.** Spanning was the first version and it pushed every
      //     following cell of that row into the next grid line — the
      //     whole table below the opened cell came apart (seen in a
      //     screenshot; the assertion that the cell grew and shrank was
      //     perfectly happy, because it was true). The row grows in
      //     height instead, and the columns stay where they are.
      className={cn("flex min-w-0 items-baseline gap-in", cut && "cursor-pointer")}
    >
      {/* **One box, one line, one truncation.** The name and the word
          are two spans only so they can wear two inks; they are not two
          columns, and neither carries a width of its own. */}
      <span ref={line} data-cmd-line className={cn("min-w-0", open ? "break-all" : "truncate")}>
        <span data-cmd-name>{name}</span>
        {word && (
          <>
            {" "}
            <span data-cmd-arg className="text-muted-foreground">
              {word}
            </span>
          </>
        )}
      </span>
    </span>
  );
  // **A cell that fits is not a trigger.** Wrapping every one of them
  // would hand two hundred rows Radix's pointer handlers on elements
  // that say they are not targets — which the face gate reads, rightly,
  // as a press with no way in for a keyboard.
  //
  // ⚠️ **And the wrap is downstream of a measurement, which is why the
  // two guards above exist.** The first version had this conditional
  // and nothing else: the moment `over` flipped true the line was
  // unmounted and remounted inside the trigger, the `ResizeObserver`
  // was left holding the detached node, it fired 0×0, `over` flipped
  // back — and the effect never re-ran, because none of its
  // dependencies had changed. Stuck false, on a cell that visibly did
  // not fit. Measured: **0 of 23 marked, 1 silent**, on a build whose
  // marking logic was untouched.
  //
  // **Opening does not unwrap it either**, and that is the same lesson
  // one step along: the first fix dropped the trigger while the line
  // was open, so the press replaced the element under the reader's
  // pointer and the gate measuring 「it opened」 was left holding a
  // removed node (`18 → 0 → 0`). The wrapper survives the press; only
  // the tooltip's *content* goes, because a tooltip repeating a line
  // that is already showing in full is noise.
  if (!cut) return cell;
  return (
    <Tooltip>
      <TooltipTrigger asChild>{cell}</TooltipTrigger>
      {/* **Only where there is something the reader cannot already
          see.** A tooltip that repeats a line showing in full is noise
          on two hundred rows. Wide enough to be worth opening and
          narrow enough to stay on the card; `break-all` because what
          overflows here is a path, and a path has no spaces to break
          at. */}
      {!open && (
        <TooltipContent side="top" className="max-w-[var(--tip-cmd-max)] break-all whitespace-normal text-left">
          <span data-cmd-full>{word ? `${name} ${word}` : name}</span>
        </TooltipContent>
      )}
    </Tooltip>
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
  head,
  grow,
  children,
}: {
  col: SortCol;
  sort: Sort;
  /** The column whose answer is still crossing — see `Top`. */
  asking: SortCol | null;
  onPress: (col: SortCol) => void;
  align: "left" | "right";
  /** What this head is called **in the DOM**, when that is not the
      wire word it sorts by. One column only: 「命令」 carries a
      process's short name and its identifying word together, and it
      orders by the name — which is the text that column starts with,
      so 「order this column」 and 「order by the name」 are the same
      request. The head is `cmd` because the column is, and the sort is
      `name` because that is the word `khor_core::TopBy` knows. */
  head?: string;
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
      data-top-head={head ?? col}
      data-asking={waiting ? "" : undefined}
      aria-sort={on ? (sort.dir === "up" ? "ascending" : "descending") : "none"}
      aria-busy={waiting || undefined}
      className={cn(
        "font-body",
        // **No padding of its own.** It carried `px-row` on the
        // right-aligned heads, which inset the word 8px from the edge
        // its own numbers are flush against — the head and its column
        // did not line up, and the 8px was also what pushed 「CPU」 out
        // of a 48px track (it rendered 「CPL」). Alignment and room, one
        // deletion.
        align === "right" ? "text-right" : "text-left",
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
      //   **The pid is a number here too**, and leaving it out was a
      //   bug rather than an omission: every column that falls through
      //   to `null` ends at the tiebreak, and **the tiebreak is not
      //   turned over by the direction** — so pressing PID a second
      //   time drew the arrow the other way and left the rows in
      //   ascending order. It was invisible against a real node, which
      //   re-ranks and sends the rows back already reversed; it is what
      //   the reader sees in the second between the press and that
      //   answer, and it is what a stand-in that ignores the ask shows
      //   the whole time. Found 2026-09-05 by asking every head to read
      //   differently both ways.
      case "pid":
        return r.pid;
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
      case "cputime":
        return r.cpu_ms;
      case "io":
        return r.io_bytes;
      default:
        return null;
    }
  };
  // **状态 groups; it does not rank** — there is no order in which 僵尸
  // is greater than 休眠 that is not somebody's opinion. What a press
  // is for is 「把不正常的那几行凑到一块」, and it sorts on the **token**
  // rather than the word so a clump cannot split along a translation —
  // the same key `khor_core::TopBy::Status` uses on the machine, so the
  // order here and the order there are one order.
  const text = (r: Load["top"][number]) =>
    sort.col === "user" ? r.user : sort.col === "status" ? (r.status ?? "") : r.name;
  return [...rows].sort((a, b) => {
    if (sort.col === "name" || sort.col === "user" || sort.col === "status") {
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
  // **Whether this machine's khor puts an age on a process — a fact
  // about the far side, kept here rather than read off the rows on
  // screen.**
  //
  // Read off the rows it is a fact about *this page*, and the page is
  // whatever the current ordering cut: press a column and the answer
  // can come back without a single filled reading in it, and the column
  // vanishes under the finger that just pressed it — the bug the person
  // using this named as 「一点就没有 GPU 这列了」. So it latches: once a
  // machine has shown one, it can report them, and no later ordering
  // unsays it. It only ever goes up, and `id` below clears it when the
  // machine changes.
  //
  // 2026-09-05:「A column the machine cannot fill does not exist」 was
  // once asked of the *machine* — 「does it report video memory」 — and
  // that is the wrong level. turing reports two RTX A5000s and **not
  // one of its two hundred processes carries a per-process figure**, so
  // it drew two empty columns and spent 136px on them. The question a
  // column has to answer is 「can any row fill me」.
  //
  // **What this reports is 「seen」, not a property of the hardware or
  // of the far khor's version.** A machine whose only card process
  // starts after the card is opened gains the columns then and keeps
  // them; one that never runs anything on its cards never grows them.
  // That is a weaker claim than the old one and it is the true one.
  //
  // **One object rather than one state per column**, since 2026-09-05
  // took it from three to seven: seven `useState`s, seven resets and
  // seven merge lines is seven chances for the reset and the merge to
  // disagree about which columns exist.
  const [seen, setSeen] = useState<Seen>(SEEN_NONE);
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
    setSeen(SEEN_NONE);
  }, [id]);
  useEffect(() => {
    if (id === null) return;
    let stopped = false;
    let timer = 0;
    // **Whether this is the ask a press caused.** The effect restarts
    // when the ordering changes, so the first ask of a run is the one
    // somebody asked for and every ask after it is the beat — see where
    // this is read, below.
    let pressed = true;
    const ask = async () => {
      let wait = LOAD_MS;
      const reorder = pressed;
      pressed = false;
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
        // **A press re-orders the table and touches nothing else.**
        // 2026-09-04:「表点一些 sorting 方式,趋势和 CPU、GPU、内存都在
        // 变。不应该这样」.
        //
        // One ask carries both the rows and the day, so the answer to
        // 「order these by memory」 arrives with a *newer day* attached
        // — a different `now_ms`, and with it a redrawn chart. Nothing
        // about the last 24 hours changed; the reader asked for the
        // rows to move and the figure under them moved too.
        //
        // Measured against a control, because 「it changed after I
        // clicked」 and 「it changes anyway」 look identical on a live
        // machine: five presses rebuilt the chart 5/5 times, five waits
        // of the same length without a press rebuilt it 1/5 — the beat.
        // (**The dials came out 3/5 either way**, which is how it was
        // settled that their numbers moving is the beat and not the
        // press. They are drawn from `row.vitals`, which this ask never
        // touches.)
        //
        // So a press keeps the day already on screen, and the next beat
        // brings a new one. There is no old day to keep on the first
        // ask for a machine, and none is wanted — `id` cleared it.
        setLoad((was) => (reorder && was ? { ...l, cpu_day: was.cpu_day } : l));
        setShown({ col: sort.col, dir: sort.dir });
        setWhy(null);
        // **Per column, never as a group**: they are separate
        // readings and a machine answers some and not others — a
        // process can hold card memory with no utilisation figure and
        // the other way round, and an older khor sends four of these
        // seven and nothing else.
        //
        // The same object comes back when nothing was learned, so a
        // beat that teaches this nothing does not re-render the table.
        setSeen((was) => {
          const next = { ...was };
          let grew = false;
          for (const k of SEEN_KEYS) {
            if (!next[k] && l.top.some(FILLS[k])) {
              next[k] = true;
              grew = true;
            }
          }
          return grew ? next : was;
        });
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
  return { load, why, shown, asking, seen };
}

/**
 * The machine's last day — **the prototype's panel, mounted, not
 * re-drawn.**
 *
 * Everything inside the box below is built by `day/day-core.js`, which
 * is `card-whole.html`'s `KhorDay` moved across whole. This component
 * owns the box, the heading, and the one call that hands the module a
 * new answer. It owns no geometry, no scale, no colour and no cursor —
 * those are the module's, and that is the point: there is no second
 * implementation to keep in step.
 *
 * **The thing this replaced was a React re-implementation written from
 * the spec's description of the same drawing**, and everything that was
 * wrong with it came out of that gap — the ridge drawn with the
 * vertical-hair fill the design had measured and thrown away, the
 * parallel coordinates with no axes, sizes and weights off by whatever
 * the prose did not pin. 「artifacts 已经写好代码画了,为什么还要自己画
 * 一遍」.
 *
 * The heading stays here because it is the card's, not the figure's:
 * the two ends of the window are the same fact the dials above are read
 * against.
 */
function DayChart({ day }: { day: Load["cpu_day"] }) {
  const box = useRef<HTMLDivElement>(null);
  const panel = useRef<DayPanel | null>(null);
  // **Every word the drawing says, from the catalog.** The module holds
  // no Chinese; this is where the two meet.
  const words: DayWords = {
    cpu: cli.vitals_name_cpu,
    gpu: cli.vitals_name_gpu,
    mem: cli.vitals_name_mem,
    disk: cli.vitals_name_disk,
    vram: cli.vitals_name_vram,
    heat: cli.day_view_heat,
    ridge: cli.day_view_ridge,
    para: cli.day_view_para,
    gap: cli.day_gap,
    now: cli.day_now,
    shade: cli.day_shade,
    strainLegend: cli.day_strain_legend(90),
    peakAt: (pct, at) => cli.day_peak_at(pct, at),
    brushHead: (what, n, hours) => cli.day_brush_head(what, n, hours),
    brushMedian: (name, pct) => cli.day_brush_median(name, pct),
    brushNone: (name) => cli.day_brush_none(name),
    brushEmpty: (name, band) => cli.day_brush_empty(name, band),
  };
  useEffect(() => {
    const node = box.current;
    if (!node) return;
    panel.current = mountDay(node);
    return () => {
      panel.current?.destroy();
      panel.current = null;
    };
  }, []);
  // **A new answer redraws; it does not remount.** The reading somebody
  // picked and the moment they left the cursor on live inside the
  // module, and tearing it down every five seconds would take both.
  //
  // ⚠️ **A new *answer*, not a new render.** This effect had no
  // dependency list, so it ran after every render of this component —
  // and a machine card re-renders for a dozen reasons that have nothing
  // to do with the day. Each run rebuilt the whole figure. Measured
  // 2026-09-04 on the real face: eleven of sixteen one-second samples
  // caught a fresh node set, most of them mid-fade. The signature below
  // is what a redraw is *for*: the window moved, or its shape changed.
  // `day` itself cannot be the dependency — it is parsed fresh from the
  // wire on every poll, so its identity changes even when it says the
  // same thing.
  const answer = `${day.now_ms}/${day.bucket_ms}/${day.buckets.length}`;
  useEffect(() => {
    panel.current?.update(day, words);
    // `day` and `words` are read, not depended on: the signature above
    // is the dependency, and `words` is the same catalog every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [answer]);
  // **No heading, and no window written out.** It used to carry
  // 「24 小时趋势」 and 「昨天 15:34 → 15:34」 above the figure;
  // 2026-09-04 both went, for two different reasons.
  //
  // The window is **drawn on the axis** — hour marks every three hours
  // and 「现在」 at its right end — so printing it again was the same
  // fact said twice, and the second telling is the one that goes stale
  // in somebody's memory. The heading went because the slot this block
  // now lives in has one line at the top for whatever it is showing,
  // and for the figure that line is the three reading words. A title
  // there would push them out or push the figure down.
  //
  // What that buys is the thing the merge is for: **the reading words
  // do not vanish and come back** when a ring is pressed. They are
  // replaced, in place, by the drawer's own heading — one line, one
  // position, one height, so nothing moves.
  return <div ref={box} data-day-band-axis className="h-full w-full" />;
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
      {/* **The open ring's name says so, in weight and ink and nothing
          else** (2026-09-04:「开着抽屉时那个环要看得出是选中的」). The
          slot under these four belongs to whichever one is open, and
          without a mark on the ring the block below is a set of rows
          with no stated owner.

          No ground, no border, no capsule: the same two channels the
          24-hour block's reading words use and the rail's own rule —
          a plate says 「a place」, and this is one place looked at from
          four sides. The word and not the arc, because the arc is a
          reading and a reading may not be dimmed to mean something
          about the interface. */}
      <span data-dial-word data-on={open ? "true" : "false"} className={cn("text-sm", open && "font-title text-foreground")}>
        {word}
      </span>
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
