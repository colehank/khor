// The browser landing before a machine is picked: every pinned page,
// each a jump straight to opening it through the machine it was pinned
// against (docs/NET.md 借网). The pins are the network's
// (khor_sync::webpins — pin on the phone, listed here); the machine names
// come looked-up from the backend, and a machine that left the table
// shows a short id rather than nothing, because the pin is still real and
// hiding it would strand the unpin.
import { useEffect, useState } from "react";

import { fetchWebPins, type WebPinRow } from "@/api";
import { gui } from "@/gen/catalog";

export function PinnedWebs({ onOpen }: { onOpen: (device: string, url: string) => void }) {
  const [rows, setRows] = useState<WebPinRow[] | null>(null);

  useEffect(() => {
    let stopped = false;
    fetchWebPins()
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
        {gui.no_pinned_webs}
      </div>
    );
  }
  return (
    <div data-pinned-webs className="min-h-0 overflow-y-auto p-2">
      {rows.map((r) => (
        <button
          key={`${r.device}:${r.url}`}
          type="button"
          data-pinned-web={r.url}
          onClick={() => onOpen(r.device, r.url)}
          className="flex w-full items-baseline gap-2 rounded-md px-3 py-2 text-left hover:bg-muted/50"
        >
          <span title={r.url} className="min-w-0 flex-1 truncate text-sm font-medium">
            {r.url}
          </span>
          <span className="flex-none text-xs text-muted-foreground">{r.name}</span>
        </button>
      ))}
    </div>
  );
}
