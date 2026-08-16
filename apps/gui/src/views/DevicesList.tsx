import type { DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { IconPin } from "@/components/icons";
import { Button } from "@/components/ui/button";
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
 * **The row itself still goes nowhere.** Machine cards are a later
 * batch, so the row is not a button, does not light up under the
 * pointer and carries no chevron: an affordance that answers nothing
 * teaches people to stop trying the ones that do.
 *
 * The pin is not a counter-example to that. It is a control that does
 * the whole of what it promises the moment it is pressed — what the
 * previous batch refused was an affordance suggesting a *destination*
 * that does not exist. So the pin sits on the row while the row stays
 * inert, and that difference is the point rather than an inconsistency.
 */
export function DevicesList({
  rows,
  query,
  onPin,
}: {
  rows: DeviceRow[];
  query: string;
  onPin: (row: DeviceRow) => void;
}) {
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
          data-pinned={d.pinned}
          className="flex items-center gap-3 py-2 pr-2 pl-4"
        >
          <MachineAvatar face={d.face} className="size-avatar" />
          <span className="min-w-0 flex-1">
            <span className="block truncate">
              {d.name}
              {d.me ? ` ${cli.this_machine}` : ""}
            </span>
            <span className="block truncate text-sm text-muted-foreground">{d.id.slice(0, 12)}</span>
          </span>
          <Button
            variant="ghost"
            size="icon"
            data-row-pin
            data-on={d.pinned}
            aria-label={d.pinned ? gui.unpin : gui.pin}
            onClick={() => onPin(d)}
            className="flex-none text-muted-foreground data-[on=true]:text-foreground"
          >
            <IconPin pinned={d.pinned} />
          </Button>
        </div>
      ))}
    </div>
  );
}
