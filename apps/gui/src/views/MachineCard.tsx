import { useEffect, useState } from "react";

import type { DeviceRow, HooksState, Load, Strain } from "@/api";
import { fetchLoad } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { Ring, RING_SIZE } from "@/components/Ring";
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
          <Busy row={row} />
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
  const { vitals: v, age_ms, volume_strains } = row.vitals;
  return (
    <div data-vitals className="flex flex-wrap gap-row pt-pane">
      <Unit
        name="cpu"
        word={cli.vitals_name_cpu}
        label={cli.vitals_cpu(Math.round(v.cpu_pct), v.cores)}
        fraction={v.cpu_pct / 100}
      />
      <Unit
        name="mem"
        word={cli.vitals_name_mem}
        label={cli.vitals_mem(bytes(v.mem.used), bytes(v.mem.total))}
        fraction={v.mem.total > 0 ? v.mem.used / v.mem.total : null}
        strain={row.vitals.mem_strain}
      />
      {v.volumes.length > 0 ? (
        // The volumes one by one (2026-09-02:「硬盘存储」明细) — the
        // home volume leads (the node sorts), each ring wearing the
        // strain the node minted for its own fill. The two branches
        // below are the older shape's fallback, and 「说不上」 belongs
        // only there: an empty survey is an old reporter, never an
        // empty machine.
        v.volumes.map((vol, i) => (
          <Unit
            key={vol.mount}
            name="vol"
            word={volWord(vol.mount)}
            label={cli.vitals_vol(volWord(vol.mount), bytes(vol.fill.used), bytes(vol.fill.total))}
            fraction={vol.fill.total > 0 ? vol.fill.used / vol.fill.total : null}
            strain={volume_strains[i]}
          />
        ))
      ) : v.disk ? (
        <Unit
          name="disk"
          word={cli.vitals_name_disk}
          label={cli.vitals_disk(bytes(v.disk.used), bytes(v.disk.total))}
          fraction={v.disk.total > 0 ? v.disk.used / v.disk.total : null}
          strain={row.vitals.disk_strain}
        />
      ) : (
        // Said, not left out: a line that is simply absent and a machine
        // that could not answer read the same, and `0 / 0` would read as
        // an empty disk.
        //
        // **A ring with no reading rather than a bare line.** `fraction`
        // of null draws the track and no arc, which is the shape of "no
        // answer" — and it keeps this reading in the same row as its
        // neighbours, where a lone line of text would have read as a
        // different *kind* of thing rather than the same kind with
        // nothing in it.
        <Unit name="disk" word={cli.vitals_name_disk} label={cli.vitals_disk_unknown} fraction={null} unknown />
      )}
      {/* **Absent rather than explained**, which is the opposite of the
          disk right above, and the asymmetry is the judgment. khor's home
          is always on some filesystem, so failing to name it means khor
          could not answer. A machine with no GPU — or one khor cannot ask
          — is an ordinary machine, and a line announcing that on every
          desktop without a card reports a non-event. Same again one level
          in for the video memory: a unified-memory machine has none to
          report, and those bytes are already in the 内存 line above. */}
      {v.gpus.length > 0 ? (
        // One ring per card (2026-09-02:「多个GPU，应该分多个环」— the
        // aggregate hides the idle card somebody is looking for,
        // `khor_core::GpuCard`); its VRAM rides the flip side of the
        // same ring, because a per-card row plus an aggregate VRAM
        // ring would be the same bytes on screen twice. The ring count
        // stays equal to `khor devices`' sentence count — smoke holds
        // the two faces to it.
        v.gpus.map((c, i) => (
          <Unit
            key={i}
            name="gpu"
            word={v.gpus.length > 1 ? `${cli.vitals_name_gpu} ${i}` : cli.vitals_name_gpu}
            label={
              c.mem
                ? cli.vitals_gpu_card_mem(
                    c.name || cli.vitals_name_gpu,
                    Math.round(c.util_pct),
                    bytes(c.mem.used),
                    bytes(c.mem.total),
                  )
                : cli.vitals_gpu_card(c.name || cli.vitals_name_gpu, Math.round(c.util_pct))
            }
            fraction={c.util_pct / 100}
          />
        ))
      ) : (
        v.gpu && (
          <>
            <Unit
              name="gpu"
              word={cli.vitals_name_gpu}
              label={cli.vitals_gpu(Math.round(v.gpu.util_pct), v.gpu.cards)}
              fraction={v.gpu.util_pct / 100}
            />
            {v.gpu.mem && (
              <Unit
                name="vram"
                word={cli.vitals_name_vram}
                label={cli.vitals_vram(bytes(v.gpu.mem.used), bytes(v.gpu.mem.total))}
                fraction={v.gpu.mem.total > 0 ? v.gpu.mem.used / v.gpu.mem.total : null}
                strain={row.vitals.vram_strain}
              />
            )}
          </>
        )
      )}
      {/* Which khor that machine runs. It rides with the readings
          because it is the same kind of fact — something a machine says
          about itself, on the cadence it already says the others on
          (`Vitals::version`) — and it is drawn with no bar because it
          divides by nothing.

          **Silent when unknown, and that silence is the answer**: a peer
          whose khor predates this field says nothing here, and that is
          exactly the machine an upgrade sweep is looking for. Printing
          a placeholder would hide the one row worth finding. */}
      {v.version && (
        <div data-vitals-unit="version" className="text-sm text-muted-foreground">
          {cli.vitals_version(v.version)}
        </div>
      )}
      {/* Printed on every reading that was not taken to answer this very
          call — which is every machine but this one. A number with no age
          beside it is read as the present. */}
      {age_ms > 0 && (
        <div data-vitals-age className="text-sm text-muted-foreground">
          {cli.vitals_taken(ago(Date.now() - age_ms))}
        </div>
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
        <div className="text-aux text-muted-foreground">{cli.top_day}</div>
        <DayBand day={load.cpu_day} />
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
 * The CPU day as a band: one cell per bucket, oldest at the left, the
 * bar's height the bucket's mean. A `null` bucket is a cell with no bar
 * — the recorder was not running, and the band says so by drawing
 * nothing there rather than a zero or a bridge. Every cell is drawn
 * (the gate counts 288) so a gap holds its width and the axis stays
 * a day long whatever was measured.
 */
function DayBand({ day }: { day: Load["cpu_day"] }) {
  return (
    <div data-day-band className="flex h-10 w-full items-end gap-px">
      {day.buckets.map((b, i) => (
        <div
          key={i}
          data-day-cell
          data-gap={b === null ? "" : undefined}
          className="min-w-0 flex-1 self-stretch"
          title={b === null ? undefined : `${Math.round(b)}%`}
        >
          {b !== null && (
            <div
              className="h-full w-full bg-foreground/50"
              style={{ height: `${Math.max(2, Math.min(100, b))}%`, marginTop: "auto" }}
            />
          )}
        </div>
      ))}
    </div>
  );
}

function Unit({
  name,
  word,
  label,
  fraction,
  strain,
  unknown,
}: {
  name: string;
  /** The reading's name on its own, for under the ring. */
  word: string;
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
      style={{ minWidth: RING_SIZE }}
    >
      <span data-metric-faces>
        <span data-metric-face="front" className="flex flex-col items-center gap-in">
          <Ring fraction={fraction} color={gaugeColor(strain)}>
            {value}
          </Ring>
          <span className="text-aux text-muted-foreground">{word}</span>
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
