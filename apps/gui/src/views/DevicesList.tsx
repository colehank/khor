import type { DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { cli, gui } from "@/gen/catalog";

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
 * **A row here does nothing at all yet.** Machine cards are a later
 * batch, so the row is not a button, does not light up under the
 * pointer and carries no chevron: an affordance that answers nothing
 * teaches people to stop trying the ones that do. That leaves a list
 * you can only read, and that is the honest shape of it today — the
 * first thing this row is going to be able to do is open the machine's
 * card, and until that exists there is nothing here to offer.
 */
export function DevicesList({ rows, query }: { rows: DeviceRow[]; query: string }) {
  const shown = visibleDevices(rows, query);
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
          className="flex items-center gap-3 px-4 py-2"
        >
          <MachineAvatar face={d.face} className="size-avatar" />
          <span className="min-w-0 flex-1">
            <span className="block truncate">
              {d.name}
              {d.me ? ` ${cli.this_machine}` : ""}
            </span>
            <span className="block truncate text-sm text-muted-foreground">{d.id.slice(0, 12)}</span>
          </span>
        </div>
      ))}
    </div>
  );
}
