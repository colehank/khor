import type { DeviceRow } from "@/api";
import { cli } from "@/gen/catalog";

export function DevicesList({ rows }: { rows: DeviceRow[] }) {
  return (
    <div>
      {rows.map((d) => (
        <div key={d.id} className="flex items-center gap-3 px-4 py-2">
          <span aria-hidden="true" className="size-avatar flex-none rounded-full bg-muted" />
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
