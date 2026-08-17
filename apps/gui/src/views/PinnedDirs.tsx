// The files landing before a machine is picked: every pinned
// directory, each a jump straight to where it points. The pins are the
// network's (khor_sync::dirpins — pin on the phone, listed here), and
// the machine names come looked-up from the backend: a machine that
// left the table shows a short id rather than nothing, because the pin
// is still real on that disk and hiding it would strand the unpin.
import { useEffect, useState } from "react";

import { fetchDirPins, type DirPinRow } from "@/api";
import { gui } from "@/gen/catalog";

export function PinnedDirs({ onOpen }: { onOpen: (device: string, path: string) => void }) {
  const [rows, setRows] = useState<DirPinRow[] | null>(null);

  useEffect(() => {
    let stopped = false;
    fetchDirPins()
      .then((r) => {
        if (!stopped) setRows(r);
      })
      .catch(() => {
        if (!stopped) setRows([]);
      });
    return () => {
      stopped = true;
    };
  }, []);

  if (rows === null) return null;
  if (rows.length === 0) {
    return (
      <div className="grid h-full place-items-center text-sm text-muted-foreground">
        {gui.no_pinned_dirs}
      </div>
    );
  }
  return (
    <div data-pinned-dirs className="min-h-0 overflow-y-auto p-2">
      {rows.map((r) => {
        const leaf = r.path.slice(r.path.lastIndexOf("/") + 1) || r.path;
        return (
          <button
            key={`${r.device}:${r.path}`}
            type="button"
            data-pinned-dir={r.path}
            onClick={() => onOpen(r.device, r.path)}
            className="flex w-full items-baseline gap-2 rounded-md px-3 py-2 text-left hover:bg-muted/50"
          >
            <span className="flex-none text-sm font-medium">{leaf}</span>
            <span className="flex-none text-xs text-muted-foreground">{r.name}</span>
            <span title={r.path} className="min-w-0 flex-1 truncate text-xs text-muted-foreground">
              {r.path}
            </span>
          </button>
        );
      })}
    </div>
  );
}
