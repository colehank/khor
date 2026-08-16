import type { DeviceRow, HooksState } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { IconBack, IconPin } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { ago } from "@/words";

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
              <div className="truncate text-lg">
                {row.name}
                {row.me ? ` ${cli.this_machine}` : ""}
              </div>
              {/* The whole id, not the row's twelve characters: this is
                  the screen somebody is on when they need to paste it. */}
              <div data-machine-id className="break-all text-sm text-muted-foreground">
                {row.id}
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
    <div data-hooks data-installed={on} className="flex flex-col gap-2 pt-8">
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
 * **No severity colour, and that is a decision rather than an
 * omission.** Colour in this app is spoken for: the six state words own
 * it, 待批 is the only cool colour anywhere, and red means a session
 * sank to the bottom of the list. Turning a 91%-full disk red would
 * dress it as a failed session. Choosing a threshold at which a disk
 * becomes a warning is also a product judgment nobody has made — and a
 * threshold set wrong is an alarm that teaches people to ignore alarms.
 * The numbers say how full it is; the bar shows it at a glance.
 */
function Readings({ row }: { row: DeviceRow }) {
  if (!row.vitals) {
    return (
      <div data-vitals-never className="pt-6 text-sm text-muted-foreground">
        {cli.vitals_never}
      </div>
    );
  }
  const { vitals: v, age_ms } = row.vitals;
  return (
    <div data-vitals className="flex flex-col gap-3 pt-6">
      <Unit
        name="cpu"
        label={cli.vitals_cpu(Math.round(v.cpu_pct), v.cores)}
        fraction={v.cpu_pct / 100}
      />
      <Unit
        name="mem"
        label={cli.vitals_mem(bytes(v.mem.used), bytes(v.mem.total))}
        fraction={v.mem.total > 0 ? v.mem.used / v.mem.total : null}
      />
      {v.disk ? (
        <Unit
          name="disk"
          label={cli.vitals_disk(bytes(v.disk.used), bytes(v.disk.total))}
          fraction={v.disk.total > 0 ? v.disk.used / v.disk.total : null}
        />
      ) : (
        // Said, not left out: a line that is simply absent and a machine
        // that could not answer read the same, and `0 / 0` would read as
        // an empty disk.
        <div data-vitals-unit="disk" data-vitals-unknown className="text-sm text-muted-foreground">
          {cli.vitals_disk_unknown}
        </div>
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
            label={cli.vitals_gpu(Math.round(v.gpu.util_pct), v.gpu.cards)}
            fraction={v.gpu.util_pct / 100}
          />
          {v.gpu.mem && (
            <Unit
              name="vram"
              label={cli.vitals_vram(bytes(v.gpu.mem.used), bytes(v.gpu.mem.total))}
              fraction={v.gpu.mem.total > 0 ? v.gpu.mem.used / v.gpu.mem.total : null}
            />
          )}
        </>
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
  label,
  fraction,
}: {
  name: string;
  label: string;
  /** `null` when there is no denominator to divide by, and then no bar
      is drawn — an empty track reads as "this is at zero". */
  fraction: number | null;
}) {
  return (
    <div data-vitals-unit={name}>
      <div className="text-sm">{label}</div>
      {fraction !== null && (
        <div className="mt-1 h-1 w-full overflow-hidden rounded-full bg-secondary">
          <div
            data-vitals-bar
            className="h-full bg-muted-foreground"
            style={{ width: `${Math.min(100, Math.max(0, fraction * 100))}%` }}
          />
        </div>
      )}
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
