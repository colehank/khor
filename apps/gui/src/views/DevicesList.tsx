import type { DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { IconPin } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { cli, gui } from "@/gen/catalog";
import { cn } from "@/lib/utils";

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
 * # Whether a row goes anywhere is the caller's to say
 *
 * The rule has not changed since the batch that wrote it down: **no
 * affordance may suggest a destination that does not exist**, because
 * one that answers nothing teaches people to stop trying the ones that
 * do. What changed is the input — the devices pane now opens a machine
 * card, so on that pane the row is a button, lights under the pointer
 * and is reachable by keyboard. Files and browser have no second step
 * yet, so they pass no `onOpen` and their rows stay exactly as inert as
 * they were.
 *
 * That is why this is a prop and not a flag read from a landing name:
 * the thing that decides is "is there somewhere to go", and the only
 * code that knows it is the code that would take you there. A pane that
 * grows a destination turns its rows into buttons by handing one over,
 * and there is no way to get the button without it.
 *
 * The pin was never a counter-example. It is a control that does the
 * whole of what it promises the moment it is pressed; what was refused
 * was an affordance suggesting a *destination*. So the pin sat on the
 * row through the batch where the row was inert, and it sits on the same
 * strip now that the row opens.
 */
export function DevicesList({
  rows,
  query,
  onPin,
  onOpen,
  selected,
  pinFailed,
}: {
  rows: DeviceRow[];
  query: string;
  onPin: (row: DeviceRow) => void;
  /** Where a row goes, when it goes anywhere. Absent on the panes that
      have no second step yet — see above. */
  onOpen?: (row: DeviceRow) => void;
  selected?: string | null;
  /** Rows whose last pin attempt did not take — see `App`. */
  pinFailed: Set<string>;
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
          className={cn(
            "flex items-center pr-2",
            // Only a row that opens something reacts to the pointer.
            onOpen && "hover:bg-secondary",
            onOpen && d.id === selected && "bg-accent hover:bg-accent",
          )}
        >
          {/* A strip holding the identity and the pin, exactly as a
              session row does — the pin cannot be nested inside the
              button that opens the row, because a button inside a button
              is not a thing the DOM has. Whether the identity half is a
              button or an inert span is the only difference between the
              panes, and it is one element deep. */}
          {onOpen ? (
            <button
              type="button"
              data-row-open
              onClick={() => onOpen(d)}
              className="flex min-w-0 flex-1 items-center gap-3 py-2 pl-4 text-left"
            >
              <Identity row={d} />
            </button>
          ) : (
            <span className="flex min-w-0 flex-1 items-center gap-3 py-2 pl-4">
              <Identity row={d} />
            </span>
          )}
          {/* Same failure face as a session row, for the same reason
              and by the same rule — see `SessionsList`. */}
          <Button
            variant="ghost"
            size="icon"
            data-row-pin
            data-on={d.pinned}
            data-pin-failed={pinFailed.has(d.id)}
            aria-label={
              pinFailed.has(d.id)
                ? d.pinned
                  ? gui.unpin_failed
                  : gui.pin_failed
                : d.pinned
                  ? gui.unpin
                  : gui.pin
            }
            onClick={() => onPin(d)}
            className="flex-none text-muted-foreground data-[on=true]:text-foreground data-[pin-failed=true]:text-state-failed"
          >
            <IconPin pinned={d.pinned} />
          </Button>
        </div>
      ))}
    </div>
  );
}

/** What a row says about a machine, the same either way — so the two
    branches above differ in whether they answer a click and in nothing
    else a reader can see. */
function Identity({ row }: { row: DeviceRow }) {
  return (
    <>
      <MachineAvatar face={row.face} className="size-avatar" />
      <span className="min-w-0 flex-1">
        <span className="block truncate">
          {row.name}
          {row.me ? ` ${cli.this_machine}` : ""}
        </span>
        <span className="block truncate text-sm text-muted-foreground">{row.id.slice(0, 12)}</span>
      </span>
    </>
  );
}
