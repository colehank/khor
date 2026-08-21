import { useState } from "react";

import type { DeviceRow, HooksState, Strain } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { Ring, RING_SIZE } from "@/components/Ring";
import { IconBack, IconPin } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { gaugeColor } from "@/lib/vitals";
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
        <header className="flex h-ctl-lg flex-none items-center gap-2 border-b px-3">
          <Button variant="ghost" size="icon" aria-label={gui.back} data-back onClick={onBack}>
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
              {/* The three tiers by name rather than by size: this is
                  the title, the readings under it are the facts, and
                  the id below is what qualifies one. Same numbers as
                  before — `text-lg` and `text-sm` are what these two
                  tiers resolve to — so nothing moves; what changes is
                  that the next person edits a role instead of a size. */}
              <div className="truncate text-title">
                {row.name}
                {row.me ? ` ${cli.this_machine}` : ""}
              </div>
              {/* The whole id, not the row's twelve characters: this is
                  the screen somebody is on when they need to paste it. */}
              <div data-machine-id className="break-all text-aux text-muted-foreground">
                {row.id}
              </div>
              {/* Which road this machine is using to reach that one
                  (`khor_core::Hop`), the same reading `khor devices`
                  prints, from the same call.

                  **Nothing for this machine itself** — the row carries
                  no reading at all there, so this renders nothing
                  without having to know why.

                  Not a liveness signal, and deliberately not dressed as
                  one — it lags the far end going away by minutes, and
                  the age beside the readings below is what says whether
                  this machine has been heard from. Two facts, two
                  places. */}
              {row.hop && (
                <div data-hop={row.hop} className="text-sm text-muted-foreground">
                  {hopWord(row.hop)}
                </div>
              )}
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
              className="flex-none text-muted-foreground data-[on=true]:text-foreground data-[pin-failed=true]:text-state-failed"
            >
              <IconPin pinned={row.pinned} />
            </Button>
          </div>
          <Readings row={row} />
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
        className="self-start data-[failed=true]:text-state-failed"
      >
        {failed
          ? on
            ? gui.uninstall_hooks_failed
            : gui.install_hooks_failed
          : on
            ? gui.uninstall_hooks
            : gui.install_hooks}
      </Button>
      {/* What pressing it buys, and which claude it touches. Two facts
          rather than one sentence: the second is not a detail of the
          first, it is the answer to "whose machine". */}
      <div className="text-sm text-muted-foreground">{gui.hooks_buy_you}</div>
      <div className="text-sm text-muted-foreground">{gui.hooks_are_local}</div>
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
  const { vitals: v, age_ms } = row.vitals;
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
      {v.disk ? (
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
      {v.gpu && (
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
            />
          )}
        </>
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
          className="grid place-items-center text-center text-aux text-muted-foreground data-[strain]:text-strain data-[strain=critical]:font-medium"
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
