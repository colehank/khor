import type { DeviceRow } from "../api";
import { cli } from "../gen/catalog";

export function DevicesList({ rows }: { rows: DeviceRow[] }) {
  return (
    <div>
      {rows.map((d) => (
        <div key={d.id} className="row">
          <span className="avatar-blank" aria-hidden="true" />
          <span className="row-main">
            <div className="row-title">
              {d.name}
              {d.me ? ` ${cli.this_machine}` : ""}
            </div>
            <div className="row-sub">
              <span>{d.id.slice(0, 12)}</span>
            </div>
          </span>
        </div>
      ))}
    </div>
  );
}
