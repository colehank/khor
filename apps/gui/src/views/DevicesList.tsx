import type { DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { PinButton } from "@/components/PinButton";
import { cli, gui } from "@/gen/catalog";
import { pinnedFirst } from "@/lib/pins";

/** Search matches the name people call the machine and the id they
    paste — the two strings the row itself shows. */
export function visibleDevices(rows: DeviceRow[], query: string) {
  const q = query.trim().toLowerCase();
  if (q === "") return rows;
  return rows.filter((d) => `${d.name} ${d.id}`.toLowerCase().includes(q));
}

/**
 * The machine list. Three panes draw it today — devices, files and
 * browser — because "which machine" is the first step of all three, and
 * each of the latter two grows its own content in its own batch.
 *
 * **A row here goes nowhere yet.** Machine cards are a later batch, so
 * the row is not a button, does not light up under the pointer and
 * carries no chevron: an affordance that answers nothing teaches people
 * to stop trying the ones that do. The pin is the one thing you can
 * operate, and it says so by being the only thing that reacts.
 *
 * `pins` is per-pane, so this component is told which set it is drawing
 * rather than reaching for one — see `@/lib/pins` for why the panes do
 * not share.
 */
export function DevicesList({
  rows,
  query,
  pinned,
  onTogglePin,
}: {
  rows: DeviceRow[];
  query: string;
  pinned: ReadonlySet<string>;
  onTogglePin: (key: string) => void;
}) {
  const shown = pinnedFirst(visibleDevices(rows, query), (d) => d.id, pinned);
  // An empty device table cannot happen — this machine is always in it —
  // so the only way to reach zero rows is by filtering, and that gets
  // said in the words for it rather than borrowing the sessions pane's.
  if (shown.length === 0) {
    return rows.length === 0 ? null : (
      <div data-empty className="p-4 text-sm text-muted-foreground">
        {gui.no_matches}
      </div>
    );
  }
  return (
    <div>
      {shown.map((d) => (
        <div
          key={d.id}
          data-device={d.name}
          data-row={d.id}
          className="flex items-center gap-1 py-2 pr-2 pl-4"
        >
          <MachineAvatar face={d.face} className="size-avatar" />
          <span className="ml-3 min-w-0 flex-1">
            <span className="block truncate">
              {d.name}
              {d.me ? ` ${cli.this_machine}` : ""}
            </span>
            <span className="block truncate text-sm text-muted-foreground">{d.id.slice(0, 12)}</span>
          </span>
          <PinButton pinned={pinned.has(d.id)} onToggle={() => onTogglePin(d.id)} />
        </div>
      ))}
    </div>
  );
}
