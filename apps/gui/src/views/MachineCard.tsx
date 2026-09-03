import { useEffect, useState } from "react";

import type { DeviceRow, HooksState, Load, Strain } from "@/api";
import { fetchLoad } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { Ring, RING_SIZE_CARD, RING_STROKE_CARD } from "@/components/Ring";
import { IconBack, IconPin } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { gaugeColor } from "@/lib/vitals";
import { YIELD_MS } from "@/lib/tempo";
import { CopyId } from "@/views/DetailPane";
import { ago, hopWord } from "@/words";

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
  onPin,
  pinFailed,
  hooks,
  hooksFailed,
  onInstallHooks,
  onUninstallHooks,
  onOpenTerm,
  onBrowseFiles,
}: {
  row: DeviceRow | null;
  narrow: boolean;
  onBack: () => void;
  onPin: (row: DeviceRow) => void;
  /** Rows whose last pin attempt did not take — see `App`. */
  pinFailed: Set<string>;
  /** This machine's hooks, or `null` until the first answer is in. */
  hooks: HooksState | null;
  hooksFailed: boolean;
  onInstallHooks: () => void;
  onUninstallHooks: () => void;
  /** 在这台开终端 — rejects with the refusal, which lands on the button. */
  onOpenTerm: (row: DeviceRow) => Promise<void>;
  onBrowseFiles: (row: DeviceRow) => void;
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
        <div className="min-h-0 flex-1 overflow-y-auto p-4">
          <div className="flex items-start gap-4">
            {/* The same derivation as its row and as the rail's foot: one
                machine, one face, painted from what that machine said
                about itself. Bigger here, not different. */}
            <MachineAvatar face={row.face} className="size-avatar-lg" />
            <div className="min-w-0 flex-1">
              {/* The row's arrangement, larger: the name over the two
                  facts that qualify one machine (2026-09-02:「机器页面
                  打开,标题那也是左边头像,右上名称,右下序列号与连接
                  方式」). The suffix the name used to carry is gone —
                  「本机不用写一个括号在机器名旁边」— the connection
                  slot says the same thing as a reading instead of a
                  bracket. */}
              <div className="truncate text-title font-title">{row.name}</div>
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                {/* The first twelve characters — a prefix to recognise,
                    not to paste. The whole id stays behind 复制 id in
                    the verb row, and smoke ties the two together (the
                    prefix must open a card that hands over the full
                    id). */}
                <span data-card-id className="truncate">{row.id.slice(0, 12)}</span>
                {/* Which road this machine is being reached over
                    (`khor_core::Hop`), the same reading `khor devices`
                    prints, from the same call. Not a liveness signal —
                    it lags the far end going away by minutes; the age
                    beside the readings below is what says whether this
                    machine has been heard from.

                    Three shapes for one slot. This machine reaches
                    itself over nothing, so its word is 本机 and it is
                    deliberately **not** `data-hop` — smoke asserts the
                    own card carries no road. A satellite has no road of
                    its own; its sponsor speaks for it (附属机器批). */}
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
              </div>
            </div>
            {/* The same control as on the row, with the same two names
                and the same failure face — it is one fact about this
                machine, and a second way to express it would be a second
                thing to keep in step. */}
            <Button
              variant="ghost"
              size="icon"
              data-card-pin
              data-on={row.pinned}
              data-pin-failed={pinFailed.has(row.id)}
              aria-label={
                pinFailed.has(row.id)
                  ? row.pinned
                    ? gui.unpin_failed
                    : gui.pin_failed
                  : row.pinned
                    ? gui.unpin
                    : gui.pin
              }
              onClick={() => onPin(row)}
              // The finger floor, the same one the row's pin carries in
              // both lists — one control, one target size wherever it
              // stands (tokens.css `--touch`). It grows the box the
              // ghost's hover block paints, not the glyph.
              className="min-h-touch min-w-touch flex-none text-muted-foreground data-[on=true]:text-foreground data-[pin-failed=true]:text-state-failed"
            >
              <IconPin pinned={row.pinned} />
            </Button>
          </div>
          <Verbs row={row} onOpenTerm={onOpenTerm} onBrowseFiles={onBrowseFiles} />
          <Readings row={row} />
          <GpuDetail row={row} />
          <Busy row={row} />
          <Storage row={row} />
          <Foot row={row} />
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
      )}
    </section>
  );
}

/**
 * The card's verbs, in one row — the pane stops being a pure dashboard
 * here (docs/KHOR.md 等价: the CLI can already `open --on`, `ls`, and
 * copy this machine's id from any terminal, so the card owes the same
 * moves).
 *
 * 开终端 is the one that can fail — the far machine can be away — and
 * the refusal lands on the button in the failure colour, the pin's
 * rule, then yields the offer back after a beat (`CopyId`'s cadence:
 * the report is about the press, not about the row). While the call is
 * out the button is disabled rather than reworded: a second press
 * would open a second terminal, and "pressed twice, got two" is the
 * kind of surprise a disable is for.
 *
 * The id keeps its 64 characters out of the header: it is the string a
 * person pastes, not a fact they read, so it travels behind the same
 * 复制 id button the session detail uses — one control for one job on
 * both cards.
 *
 * **The finger floor is worn here, not fixed in `ui/button.tsx`.** All
 * three of these are `size="sm"`, which is 24 tall, and this is the
 * screen's main row of actions — so all three took `min-h-touch`
 * (`CopyId` carries its own). Raising the floor inside the `sm` variant
 * would have looked like the smaller edit and been the larger one: `sm`
 * is a *look* for controls that sit inline beside text (the `--ctl-*`
 * family), and tokens.css says in as many words that the touch floor is
 * not a fourth `--ctl-*` — the two move for different reasons. Changing
 * the variant would also regrow every `sm` button in the app, in files
 * two other lanes have open right now, for a reading nobody on those
 * screens asked for.
 *
 * The mechanism is worth knowing before copying it: `min-height` beats
 * `height`, so a caller-side `min-h-touch` lands even though `cn()`
 * cannot take `h-ctl-sm` off (tailwind-merge does not read `ctl-sm` as a
 * height — `ui/button.tsx` has the scar). What the caller cannot do is
 * make the box *smaller*; the floor is the only direction that works
 * from here, which is the direction we want.
 */
function Verbs({
  row,
  onOpenTerm,
  onBrowseFiles,
}: {
  row: DeviceRow;
  onOpenTerm: (row: DeviceRow) => Promise<void>;
  onBrowseFiles: (row: DeviceRow) => void;
}) {
  const [opening, setOpening] = useState(false);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    if (!failed) return;
    const t = window.setTimeout(() => setFailed(false), YIELD_MS);
    return () => window.clearTimeout(t);
  }, [failed]);
  return (
    <div data-machine-verbs className="flex flex-wrap items-center gap-2 pt-pane">
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
      {/* Outline like its two neighbours (动作排语法批): the verb row
          is this page's "what you can do here" list, three peer verbs
          in one dress — bare text among outlined buttons reads as a
          lower rank, or as a link. The session header keeps its ghost:
          that row is all quiet text, and each row speaks one language. */}
      <CopyId id={row.id} variant="outline" />
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
 */
function Readings({ row }: { row: DeviceRow }) {
  if (!row.vitals) {
    return (
      <div data-vitals-never className="pt-pane text-sm text-muted-foreground">
        {cli.vitals_never}
      </div>
    );
  }
  const { vitals: v } = row.vitals;
  // The GPU ring when the node reports per card: one gauge for the
  // machine (the cards' mean, their memory summed), **no unit of its
  // own** — the 明细 rows below are the readings `khor devices` prints,
  // one per card, and the smoke holds the two faces to that count. On an
  // older node the aggregate is the reading, and the ring is the unit.
  const cards = v.gpus;
  const gpuMean = cards.length > 0 ? cards.reduce((a, c) => a + c.util_pct, 0) / cards.length : null;
  const gpuMem = cards.some((c) => c.mem)
    ? cards.reduce(
        (acc, c) => (c.mem ? { used: acc.used + c.mem.used, total: acc.total + c.mem.total } : acc),
        { used: 0, total: 0 },
      )
    : null;
  return (
    <div data-vitals className="flex flex-wrap justify-around gap-row pt-pane">
      <Unit
        name="cpu"
        word={cli.vitals_name_cpu}
        sub={cli.cores_sub(v.cores)}
        label={cli.vitals_cpu(Math.round(v.cpu_pct), v.cores)}
        fraction={v.cpu_pct / 100}
      />
      {cards.length > 0 && gpuMean !== null ? (
        <Unit
          word={cli.vitals_name_gpu}
          sub={
            gpuMem
              ? cli.gpu_sub(cards.length, bytes(gpuMem.used), bytes(gpuMem.total))
              : cli.gpu_sub_nomem(cards.length)
          }
          label={cli.vitals_gpu(Math.round(gpuMean), cards.length)}
          fraction={gpuMean / 100}
        />
      ) : (
        v.gpu && (
          <Unit
            name="gpu"
            word={cli.vitals_name_gpu}
            // The memory rides under the ring as its own reading: an
            // older node prints 显存 as a sentence of its own, and the
            // count has to match — so the sub-line *is* the unit.
            sub={
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
              )
            }
            label={cli.vitals_gpu(Math.round(v.gpu.util_pct), v.gpu.cards)}
            fraction={v.gpu.util_pct / 100}
          />
        )
      )}
      <Unit
        name="mem"
        word={cli.vitals_name_mem}
        sub={`${bytes(v.mem.used)} / ${bytes(v.mem.total)}`}
        label={cli.vitals_mem(bytes(v.mem.used), bytes(v.mem.total))}
        fraction={v.mem.total > 0 ? v.mem.used / v.mem.total : null}
        strain={row.vitals.mem_strain}
      />
    </div>
  );
}

/**
 * The per-card GPU rows under the rings (2026-09-02:「环下面显示…
 * GPU明细」; 2026-09-03 to the user's reference): index, the driver's
 * name, a bar and the share in use, memory used over total, and the
 * die temperature when the driver has one. **Each row is the unit** for
 * one of `khor devices`' per-card sentences, and carries that sentence
 * word for word for the smoke to read (`sr-only`: the row prints the
 * same facts laid out, not the sentence twice). Absent when the node
 * reported no cards: the aggregate ring is then the reading, and an
 * empty section would read as "no cards".
 */
function GpuDetail({ row }: { row: DeviceRow }) {
  const cards = row.vitals?.vitals.gpus ?? [];
  if (cards.length === 0) return null;
  return (
    <div data-gpu-detail className="flex flex-col gap-in pt-pane">
      <div className="text-aux text-muted-foreground">{cli.gpu_detail}</div>
      {cards.map((c, i) => {
        const util = Math.round(c.util_pct);
        const sentence = c.mem
          ? cli.vitals_gpu_card_mem(c.name || cli.vitals_name_gpu, util, bytes(c.mem.used), bytes(c.mem.total))
          : cli.vitals_gpu_card(c.name || cli.vitals_name_gpu, util);
        return (
          <div
            key={i}
            data-vitals-unit="gpu"
            data-gpu-card={i}
            className="flex items-center gap-row rounded-md bg-muted/40 px-row py-in text-sm tabular-nums"
          >
            <span className="flex-none font-label">GPU{i}</span>
            <span className="w-48 min-w-0 truncate text-muted-foreground">{c.name || cli.vitals_name_gpu}</span>
            <span className="h-1.5 min-w-0 flex-1 overflow-hidden rounded-full bg-foreground/10">
              <span
                className="block h-full rounded-full"
                style={{ width: `${Math.max(0, Math.min(100, util))}%`, background: gaugeColor(null) }}
              />
            </span>
            <span className="w-10 flex-none text-right">{util}%</span>
            {c.mem && (
              <span className="flex-none text-muted-foreground">
                {bytes(c.mem.used)} / {bytes(c.mem.total)}
              </span>
            )}
            {c.temp_c != null && <span className="flex-none text-muted-foreground">{Math.round(c.temp_c)}°C</span>}
            <span data-vitals-detail="gpu" className="sr-only">
              {sentence}
            </span>
          </div>
        );
      })}
    </div>
  );
}

/**
 * Storage, a bar per volume (2026-09-03 to the user's reference; the
 * rings the volumes wore before were the user's「硬盘存储」ask read
 * too literally — a bar is how a person reads "how full"). Each bar is
 * the unit for one of `khor devices`' volume sentences, and prints that
 * sentence at its right, word for word, wearing the volume's strain. An
 * older node that reports one disk gets one bar; one that could not
 * name it gets the unknown sentence and no bar.
 */
function Storage({ row }: { row: DeviceRow }) {
  if (!row.vitals) return null;
  const { vitals: v, volume_strains } = row.vitals;
  const rows =
    v.volumes.length > 0
      ? v.volumes.map((vol, i) => ({
          name: "vol",
          word: volWord(vol.mount),
          fill: vol.fill,
          strain: volume_strains[i] ?? null,
          label: cli.vitals_vol(volWord(vol.mount), bytes(vol.fill.used), bytes(vol.fill.total)),
        }))
      : v.disk
        ? [
            {
              name: "disk",
              word: cli.vitals_name_disk,
              fill: v.disk,
              strain: row.vitals.disk_strain,
              label: cli.vitals_disk(bytes(v.disk.used), bytes(v.disk.total)),
            },
          ]
        : [{ name: "disk", word: cli.vitals_name_disk, fill: null, strain: null, label: cli.vitals_disk_unknown }];
  return (
    <div data-storage className="flex flex-col gap-row pt-pane">
      {rows.map((r, i) => {
        const pct = r.fill && r.fill.total > 0 ? (r.fill.used / r.fill.total) * 100 : null;
        return (
          <div
            // By position, not by word: a Mac reports its data volume
            // and `/` as two mounts that both print as `/`.
            key={i}
            data-vitals-unit={r.name}
            data-vitals-unknown={r.fill ? undefined : ""}
            className="flex flex-col gap-in"
          >
            <div className="flex items-baseline justify-between gap-row text-sm">
              <span className="text-muted-foreground">
                {cli.storage} · {r.word}
              </span>
              <span
                data-vitals-detail={r.name}
                data-strain={r.strain ?? undefined}
                className="tabular-nums text-muted-foreground data-[strain]:text-strain data-[strain=critical]:font-label"
              >
                {r.label}
              </span>
            </div>
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-foreground/10">
              {pct !== null && (
                <div
                  className="h-full rounded-full"
                  style={{ width: `${Math.max(0, Math.min(100, pct))}%`, background: gaugeColor(r.strain) }}
                />
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}

/** The wall clock for one instant, hours and minutes in the reader's
    own locale — the day chart's two ends and the foot's 更新于. */
function clock(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/**
 * The card's foot: which khor this machine runs (the version is one of
 * `khor devices`' sentences and keeps its unit), how long it has been
 * up, and when this reading is from — the age sentence for a machine
 * reached over the wire (its own unit, counted with the readings), the
 * present's clock for this one.
 */
function Foot({ row }: { row: DeviceRow }) {
  if (!row.vitals) return null;
  const { vitals: v, age_ms } = row.vitals;
  const days = Math.floor(v.uptime_s / 86_400);
  const hours = Math.floor(v.uptime_s / 3_600);
  return (
    <div data-foot className="flex flex-wrap gap-row pt-pane text-sm text-muted-foreground">
      {v.version && <span data-vitals-unit="version">{cli.vitals_version(v.version)}</span>}
      {v.uptime_s > 0 && <span data-uptime>{days >= 1 ? cli.uptime_days(days) : cli.uptime_hours(hours)}</span>}
      {age_ms > 0 ? (
        <span data-vitals-age>{cli.vitals_taken(ago(Date.now() - age_ms))}</span>
      ) : (
        <span data-updated>{cli.updated_at(clock(Date.now()))}</span>
      )}
    </div>
  );
}

/** How many rows the card's top shows. Eight: `top`'s own fold on a
    short terminal, and the number past which the table is a list of
    idle helpers rather than an answer to 在忙什么. */
const TOP_SHOWN = 8;
/** How often an open card re-asks. Five seconds is the vitals beat the
    rings above already move on; the table and the band moving on
    another would make the card describe two moments. */
const LOAD_MS = 5_000;
/** …and how long to wait after a refusal before asking again: a far
    machine that is away costs a dial each time, and the sentence on
    the card does not get truer by being re-fetched every five seconds. */
const LOAD_RETRY_MS = 30_000;

/**
 * What the machine is busy with, under its rings (2026-09-02:「环下面
 * 显示进程top,最近24小时cpu」): the process top as a table and the CPU
 * day as a band.
 *
 * **Asked by this card, not carried on the row** — the rings read a
 * reading that rides the device beat, but a process table is asked for
 * only while somebody is looking at it, and the band is a day of
 * buckets that has no business on a two-second heartbeat.
 *
 * **Three faces, and the empty one is never drawn.** Nothing until the
 * first answer (the ask is out); the table and the band once it lands;
 * and the refusal's own sentence in their place when it does not — an
 * older khor over there, or a machine that is away. A blank table would
 * read as an idle machine, which is the one thing this surface must not
 * say by accident (docs/SESSION.md 认不出就不落词).
 *
 * The band paints absence as absence: a bucket the recorder never saw
 * (`null`) is a cell with no bar, never a bar of height zero and never
 * a line bridged from its neighbours (`khor_core::CpuDay`).
 */
function Busy({ row }: { row: DeviceRow }) {
  const [load, setLoad] = useState<Load | null>(null);
  const [why, setWhy] = useState<string | null>(null);
  useEffect(() => {
    let stopped = false;
    let timer = 0;
    setLoad(null);
    setWhy(null);
    const ask = async () => {
      let wait = LOAD_MS;
      try {
        // A name, not an id: the node resolves machines by the name
        // its table knows them by, and its own by being asked for
        // itself (`Node::load_on`).
        const l = await fetchLoad(row.me ? "" : row.name, TOP_SHOWN);
        if (stopped) return;
        setLoad(l);
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
  }, [row.id, row.me, row.name]);

  if (why) {
    return (
      <div data-load-why className="pt-pane text-sm text-muted-foreground">
        {why}
      </div>
    );
  }
  if (!load) return null;
  return (
    <div data-load className="flex flex-col gap-row pt-pane">
      <div data-load-day>
        <DayChart day={load.cpu_day} />
      </div>
      <table data-load-top className="w-full text-sm tabular-nums">
        <thead className="text-aux text-muted-foreground">
          <tr>
            <th className="text-left font-body">{cli.top_name}</th>
            <th className="text-left font-body">{cli.top_user}</th>
            <th className="text-right font-body">{cli.vitals_name_cpu}</th>
            <th className="text-right font-body">{cli.vitals_name_mem}</th>
          </tr>
        </thead>
        <tbody>
          {load.top.map((r) => (
            <tr key={r.pid} data-top-row={r.pid}>
              {/* The name, then the pid in the muted colour: the pid
                  is what a person would type after the name, not a
                  fact they read, and a column of its own would push
                  the name off a narrow card. */}
              <td className="max-w-0 truncate pr-2">
                {r.name} <span className="text-muted-foreground">{r.pid}</span>
              </td>
              <td className="truncate pr-2 text-muted-foreground">{r.user}</td>
              <td className="text-right">{Math.round(r.cpu_pct)}%</td>
              <td className="text-right">{bytes(r.mem)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/**
 * The CPU day as an area with the memory day dashed over it
 * (2026-09-03 to the user's reference), on a hairline track with the
 * two ends' clocks named.
 *
 * **Still one cell per bucket, and the gate still counts them.** The
 * area is 288 adjacent rectangles — one per bucket, in a `data-day-cell`
 * group — so a `null` bucket is a group with nothing in it: the recorder
 * was not running, and the chart says so by drawing nothing there rather
 * than a zero or a bridge. The memory line is a sibling of the cells,
 * never a child, and it too breaks over a gap (one path per run of
 * readings). `preserveAspectRatio="none"` lets the width follow the card;
 * the dashed line keeps its weight through `non-scaling-stroke`.
 */
function DayChart({ day }: { day: Load["cpu_day"] }) {
  const n = Math.max(1, day.buckets.length);
  const W = 1000;
  const H = 100;
  const x = (i: number) => (i / n) * W;
  const w = W / n;
  const y = (pct: number) => H - (Math.max(0, Math.min(100, pct)) / 100) * H;
  // The memory line, one path per unbroken run — a gap ends a run.
  const memRuns: string[] = [];
  let run: string[] = [];
  day.mem.forEach((m, i) => {
    if (m === null) {
      if (run.length > 1) memRuns.push(run.join(" "));
      run = [];
      return;
    }
    run.push(`${run.length === 0 ? "M" : "L"}${(x(i) + w / 2).toFixed(1)},${y(m).toFixed(1)}`);
  });
  if (run.length > 1) memRuns.push(run.join(" "));
  const startMs = day.now_ms - day.bucket_ms * n;
  return (
    <div data-day-band-axis className="flex flex-col gap-in">
      <div className="flex items-center justify-between gap-row text-aux text-muted-foreground">
        <span className="flex items-center gap-row">
          <span>{cli.day_trend}</span>
          <span className="flex items-center gap-in">
            <span className="inline-block h-0.5 w-3 bg-foreground/60" />
            {cli.vitals_name_cpu}
          </span>
          {day.mem.length > 0 && (
            <span className="flex items-center gap-in">
              <span className="inline-block h-0 w-3 border-t border-dashed border-foreground/80" />
              {cli.vitals_name_mem}
            </span>
          )}
        </span>
        <span className="tabular-nums">
          {clock(startMs)} → {clock(day.now_ms)}
        </span>
      </div>
      <svg
        data-day-band
        viewBox={`0 0 ${W} ${H}`}
        preserveAspectRatio="none"
        className="h-16 w-full text-foreground"
        aria-hidden="true"
      >
        <line x1={0} y1={H} x2={W} y2={H} stroke="currentColor" strokeOpacity={0.15} vectorEffect="non-scaling-stroke" />
        {day.buckets.map((b, i) => (
          <g key={i} data-day-cell data-gap={b === null ? "" : undefined}>
            {b !== null && (
              <rect x={x(i)} y={y(b)} width={w + 0.6} height={H - y(b)} fill="currentColor" fillOpacity={0.5}>
                <title>{`${Math.round(b)}%`}</title>
              </rect>
            )}
          </g>
        ))}
        {memRuns.map((d, k) => (
          <path
            key={k}
            data-day-mem
            d={d}
            fill="none"
            stroke="currentColor"
            strokeOpacity={0.85}
            strokeWidth={1.5}
            strokeDasharray="4 3"
            vectorEffect="non-scaling-stroke"
          />
        ))}
      </svg>
    </div>
  );
}

function Unit({
  name,
  word,
  sub,
  label,
  fraction,
  strain,
  unknown,
}: {
  /** The reading's key on the wire, or none for a ring that is a
      picture of readings counted elsewhere (the GPU ring above the
      per-card rows). Without it the unit and detail marks are not set. */
  name?: string;
  /** The reading's name on its own, for under the ring. */
  word: string;
  /** The line under the name — the fact a person reads without
      turning the ring: cores, memory used over total, cards. */
  sub?: React.ReactNode;
  label: string;
  /** `null` when there is no denominator to divide by, and then only the
      track is drawn — a full circle of arc would read as "this is at
      100%", and no circle at all as "this is at zero". */
  fraction: number | null;
  /** The word gui-core minted, absent for the units that carry none
      (CPU, GPU) as much as for a reading below the first step. */
  strain?: Strain | null;
  /** This reading could not be taken at all, as opposed to being zero. */
  unknown?: boolean;
}) {
  const [flipped, setFlipped] = useState(false);
  // A percentage, because the arc is one and the two should agree at a
  // glance. The exact quantity — bytes, cores, cards — is the back's job.
  const value = fraction === null ? null : `${Math.round(fraction * 100)}%`;
  return (
    <button
      type="button"
      data-metric
      data-flipped={flipped}
      data-vitals-unit={name}
      data-vitals-unknown={unknown ? "" : undefined}
      aria-pressed={flipped}
      // The sentence, not "CPU" — a name that reads out the short form
      // would make a screen reader turn the card over for the number.
      aria-label={label}
      onClick={() => setFlipped((on) => !on)}
      className="rounded-md p-in"
      style={{ minWidth: RING_SIZE_CARD }}
    >
      <span data-metric-faces>
        <span data-metric-face="front" className="flex flex-col items-center gap-in">
          <Ring fraction={fraction} color={gaugeColor(strain)} size={RING_SIZE_CARD} stroke={RING_STROKE_CARD}>
            {value}
          </Ring>
          <span className="text-sm">{word}</span>
          {sub != null && <span className="text-aux tabular-nums text-muted-foreground">{sub}</span>}
        </span>
        {/* **The attribute marks the sentence itself**, not the unit
            around it: it is what `khor devices` prints word for word, and
            it is compared as text — so it has to name exactly the
            sentence and not a box that also holds a number in a ring. */}
        <span
          data-metric-face="back"
          data-vitals-detail={name}
          data-strain={strain ?? undefined}
          className="grid place-items-center text-center text-aux tabular-nums text-muted-foreground data-[strain]:text-strain data-[strain=critical]:font-label"
        >
          {label}
        </span>
      </span>
    </button>
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
/** A volume's short name — `/System/Volumes/Data` and `/` both draw as
    `/` (the disk a person means), the rest keep their last step. The
    CLI keeps a copy of its own (`vol_word`), the `bytes` precedent
    below: painting is not judgment, and the wire carries the honest
    mount. */
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
