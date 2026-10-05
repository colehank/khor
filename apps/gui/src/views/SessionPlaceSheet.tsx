/**
 * **The terminal pill opens where the session is** (UX-17, the owner
 * 2026-10-05: the pill's tap opened the 24-hour view and then nothing).
 * One sheet, two tabs:
 *
 *   目录  the session's directory — where the shell is now (OSC 7, the
 *        terminal's marks), else where the session started (`place`) —
 *        as the machine's files: the listing, and a file opened from it
 *        is the file page (`MachineFiles`, the machine page's own);
 *   日志  the session's 24 hours (`DayStrip`), unchanged.
 *
 * 目录 first; the last tab is kept per session. Phone and a Mac window
 * dragged narrow (the deck's pill); the wide face keeps its own road (the
 * front tab drops the day).
 */
import { useState } from "react";

import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { cli, gui } from "@/gen/catalog";
import { DayStrip, type Day } from "@/views/DayStrip";
import { MachineFiles } from "@/views/MachineFiles";

type Tab = "dir" | "log";

const KEPT = "khor.session-sheet.";

function keptTab(session: string): Tab {
  try {
    return window.localStorage.getItem(KEPT + session) === "log" ? "log" : "dir";
  } catch {
    return "dir";
  }
}

function keepTab(session: string, tab: Tab) {
  try {
    window.localStorage.setItem(KEPT + session, tab);
  } catch {
    // A private window keeps nothing; the sheet opens on 目录 next time.
  }
}

export function SessionPlaceSheet({
  open,
  onOpenChange,
  session,
  machine,
  here,
  dir,
  day,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  session: string;
  /** The machine the session lives on, by name (`SessionRow.home`). */
  machine: string;
  here: boolean;
  /** Where the session is: its shell's directory, else its place; `null`
      when neither is known, and the listing starts at the machine's home. */
  dir: string | null;
  day: Day;
}) {
  const [tab, setTab] = useState<Tab>(() => keptTab(session));
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent form data-place-sheet className="flex h-[80dvh] flex-col !gap-0 overflow-hidden">
        <DialogTitle className="sr-only">{gui.new_session_dir}</DialogTitle>
        <SegmentedControl<Tab>
          label={gui.new_session_dir}
          value={tab}
          onChange={(t) => {
            setTab(t);
            keepTab(session, t);
          }}
          mark={{ "data-place-tabs": true }}
          className="mx-auto mb-in flex-none"
          options={[
            { key: "dir", label: gui.new_session_dir, mark: { "data-place-tab": "dir" } },
            { key: "log", label: cli.day_view_heat, mark: { "data-place-tab": "log" } },
          ]}
        />
        {tab === "dir" ? (
          <div data-place-dir className="relative flex min-h-0 flex-1 flex-col">
            <MachineFiles key={`${machine} ${dir ?? ""}`} machine={machine} here={here} startDir={dir} />
          </div>
        ) : (
          <div data-place-log className="min-h-0 flex-1 overflow-y-auto">
            <DayStrip runs={day} />
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
