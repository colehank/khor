import type { DeviceRow, SessionRow, Usage, QuotaAnswer } from "@/api";
import { MandalaMap } from "@/views/MandalaMap";
import { WaitingCall } from "@/views/MeshNow";
import { UsagePanel } from "@/views/UsagePanel";
import { cn } from "@/lib/utils";

/**
 * The mark's place: the mesh, and what it cost.
 *
 * **Side by side wide, stacked narrow, and the two are one argument
 * rather than two panels that happen to share a screen.** The picture
 * names the machines and says which of them khor has heard from; the
 * numbers are every machine's added together and name none. Each is the
 * caption the other would otherwise have needed printed under it.
 *
 * The map is handed no `onOpen` here: there is no machine card in this
 * space to open into, and an affordance that answers nothing is what the
 * mark itself was forbidden from being until it opened something
 * (`MandalaMap` says why that is a prop).
 *
 * **A file of its own rather than an expression inside the shell**, and
 * the reason is about how this repo is worked rather than about React:
 * this surface and the session list are reshaped by different people at
 * the same time, and while it lived in `App.tsx` those two changes were
 * one file's changes. The shell now names it and nothing else.
 */
export function MarkPlace({
  rows,
  devices,
  usage,
  quota,
  narrow,
  onPickWaiting,
}: {
  rows: SessionRow[];
  devices: DeviceRow[];
  usage: Usage | null;
  quota: QuotaAnswer | null;
  narrow: boolean;
  /** Where the waiting call leads, when anything is waiting. */
  onPickWaiting: (row: SessionRow) => void;
}) {
  return (
    <div data-mark-place className="flex h-full min-h-0 min-w-0 flex-col">
      {/* Above both halves rather than inside either: on the narrow face
          the right-hand column is the bottom half of the screen, and a
          call to answer something would have been the last thing seen
          instead of the first. */}
      <WaitingCall rows={rows} onPick={onPickWaiting} />
      <div className={cn("flex min-h-0 min-w-0 flex-1", narrow ? "flex-col" : "flex-row")}>
        <div className={cn("min-h-0 min-w-0", narrow ? "flex-1" : "flex-[3]")}>
          <MandalaMap rows={devices} />
        </div>
        <div
          className={cn(
            "min-h-0 min-w-0",
            narrow ? "max-h-1/2 flex-none border-t" : "w-list flex-none border-l",
          )}
        >
          <UsagePanel usage={usage} quota={quota} />
        </div>
      </div>
    </div>
  );
}
