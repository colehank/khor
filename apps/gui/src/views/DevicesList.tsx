import type { DeviceRow } from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { cli } from "@/gen/catalog";

export function DevicesList({ rows }: { rows: DeviceRow[] }) {
  return (
    <div>
      {rows.map((d) => (
        <div key={d.id} data-device={d.name} className="flex items-center gap-3 px-4 py-2">
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
